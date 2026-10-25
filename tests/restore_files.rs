use sunchang_hansang::{
    restore::{PreviewRestore, RestoreTargetId, RestoreTargetQuery},
    restore_files::{
        MAX_RESTORE_UPLOAD_BYTES, publish_restore_import, read_restore_import,
        read_restore_journal, reject_duplicate_json_keys, restore_upload, verify_restore_bytes,
        verify_restore_upload,
    },
};

#[test]
fn closed_target_ids_and_cursors_reject_paths_and_noncanonical_values() {
    for value in [
        "journal:0",
        "journal:01",
        "journal:+1",
        "journal: 1",
        "journal:9223372036854775808",
        "import:../x",
        "C:\\x",
        "https://example.invalid/a",
        "journal:1:stream",
    ] {
        assert!(RestoreTargetId::parse(value).is_err(), "{value}");
    }
    assert_eq!(
        RestoreTargetId::parse("journal:1").unwrap(),
        RestoreTargetId::Journal(1)
    );
    let hash = "a".repeat(64);
    assert!(RestoreTargetId::parse(&format!("import:{hash}")).is_ok());
    assert!(RestoreTargetId::parse(&format!("import:{}", hash.to_uppercase())).is_err());
    for text in [
        r#"{"cursor":""}"#,
        r#"{"cursor":"01"}"#,
        r#"{"source":"import","cursor":"1"}"#,
    ] {
        assert!(
            serde_json::from_str::<RestoreTargetQuery>(text)
                .unwrap()
                .validate()
                .is_err()
        );
    }
    assert!(serde_json::from_str::<RestoreTargetQuery>(r#"{"path":"x"}"#).is_err());
    assert!(serde_json::from_str::<RestoreTargetQuery>(r#"{"source":"all"}"#).is_err());
    assert!(
        PreviewRestore {
            target_id: format!("import:{hash}"),
            expected_sha256: "b".repeat(64)
        }
        .validate()
        .is_err()
    );
}

#[test]
fn duplicate_keys_are_rejected_at_every_depth_including_escaped_equivalence() {
    for bytes in [
        br#"{"a":1,"a":2}"#.as_slice(),
        br#"{"a":1,"\u0061":2}"#,
        br#"[{"x":{"v":1,"v":2}}]"#,
        br#"{} {}"#,
        b"{\"x\":\"\xff\"}",
        b"\xef\xbb\xbf{}",
        br#"{"x":NaN}"#,
    ] {
        assert!(reject_duplicate_json_keys(bytes).is_err(), "{bytes:?}");
    }
    reject_duplicate_json_keys(br#"{"a":[{"x":null},{"x":true},-1,0.5,"\u0061"]}"#).unwrap();
    let deep = format!("{}0{}", "[".repeat(140), "]".repeat(140));
    assert!(reject_duplicate_json_keys(deep.as_bytes()).is_err());
}

#[test]
fn upload_limit_is_exact_and_local_verified_documents_have_no_http_cap() {
    let mut exact = vec![b' '; MAX_RESTORE_UPLOAD_BYTES];
    exact[0] = b'{';
    exact[1] = b'}';
    let upload = restore_upload(exact.clone()).unwrap();
    assert_eq!(
        verify_restore_upload(upload).unwrap().byte_length(),
        MAX_RESTORE_UPLOAD_BYTES as u64
    );
    exact.push(b' ');
    assert!(restore_upload(exact.clone()).is_err());
    assert_eq!(
        verify_restore_bytes(exact).unwrap().byte_length(),
        MAX_RESTORE_UPLOAD_BYTES as u64 + 1
    );
    assert!(restore_upload(Vec::new()).is_err());
}

#[test]
fn immutable_publication_preserves_exact_bytes_and_never_overwrites_mismatches() {
    let root = tempfile::tempdir().unwrap();
    let bytes = b"{ \"synthetic\": [1, 2] }\n".to_vec();
    let verified = verify_restore_bytes(bytes.clone()).unwrap();
    let published = publish_restore_import(root.path(), &verified).unwrap();
    assert_eq!(published.sha256(), verified.sha256());
    assert_eq!(published.byte_length(), bytes.len() as u64);
    publish_restore_import(root.path(), &verified).unwrap();
    assert_eq!(
        read_restore_import(root.path(), published.sha256(), published.byte_length())
            .unwrap()
            .bytes(),
        bytes
    );
    let path = root
        .path()
        .join("imports")
        .join(format!("{}.json", published.sha256()));
    std::fs::write(&path, b"different").unwrap();
    assert!(publish_restore_import(root.path(), &verified).is_err());
    assert!(read_restore_import(root.path(), published.sha256(), published.byte_length()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"different");
    assert_eq!(
        std::fs::read_dir(root.path().join("imports"))
            .unwrap()
            .count(),
        1
    );
}

#[test]
fn fixed_journal_names_and_nonregular_destinations_fail_closed() {
    let root = tempfile::tempdir().unwrap();
    let verified = verify_restore_bytes(b"{\"original\":true}".to_vec()).unwrap();
    std::fs::create_dir(root.path().join("snapshots")).unwrap();
    std::fs::write(
        root.path().join("snapshots/00000000000000000001.json"),
        verified.bytes(),
    )
    .unwrap();
    assert_eq!(
        read_restore_journal(root.path(), 1, verified.sha256(), verified.byte_length())
            .unwrap()
            .bytes(),
        verified.bytes()
    );
    assert!(
        read_restore_journal(root.path(), 0, verified.sha256(), verified.byte_length()).is_err()
    );
    assert!(read_restore_journal(root.path(), 1, &"f".repeat(64), verified.byte_length()).is_err());
    assert!(read_restore_import(root.path(), "../x", 2).is_err());
    let destination = root
        .path()
        .join("imports")
        .join(format!("{}.json", verified.sha256()));
    std::fs::create_dir_all(&destination).unwrap();
    std::fs::write(destination.join("sentinel"), b"preserve").unwrap();
    assert!(publish_restore_import(root.path(), &verified).is_err());
    assert_eq!(
        std::fs::read(destination.join("sentinel")).unwrap(),
        b"preserve"
    );
}

#[cfg(windows)]
#[test]
fn real_windows_junction_cannot_redirect_archive_or_journal_access() {
    for name in ["imports", "snapshots"] {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let link = root.path().join(name);
        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:RESTORE_TEST_LINK -Target $env:RESTORE_TEST_TARGET | Out-Null"])
            .env("RESTORE_TEST_LINK", &link).env("RESTORE_TEST_TARGET", outside.path()).status().unwrap();
        assert!(status.success(), "junction setup must execute");
        let verified = verify_restore_bytes(b"{}".to_vec()).unwrap();
        if name == "imports" {
            assert!(publish_restore_import(root.path(), &verified).is_err());
        } else {
            std::fs::write(
                outside.path().join("00000000000000000001.json"),
                verified.bytes(),
            )
            .unwrap();
            assert!(
                read_restore_journal(root.path(), 1, verified.sha256(), verified.byte_length())
                    .is_err()
            );
            assert_eq!(
                std::fs::read(outside.path().join("00000000000000000001.json")).unwrap(),
                verified.bytes()
            );
        }
        std::fs::remove_dir(link).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn symlink_archive_files_and_directories_are_never_followed() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let verified = verify_restore_bytes(b"{}".to_vec()).unwrap();
    symlink(outside.path(), root.path().join("imports")).unwrap();
    assert!(publish_restore_import(root.path(), &verified).is_err());
    std::fs::remove_file(root.path().join("imports")).unwrap();
    std::fs::create_dir(root.path().join("imports")).unwrap();
    let target = outside.path().join("source.json");
    std::fs::write(&target, verified.bytes()).unwrap();
    symlink(
        &target,
        root.path()
            .join("imports")
            .join(format!("{}.json", verified.sha256())),
    )
    .unwrap();
    assert!(read_restore_import(root.path(), verified.sha256(), verified.byte_length()).is_err());
    assert!(publish_restore_import(root.path(), &verified).is_err());
    assert_eq!(std::fs::read(target).unwrap(), verified.bytes());
}
