//! Immutable JSON archives. File existence alone never registers a restore target.
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};
use serde::{
    Deserialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fmt,
    io::{self, Read, Write},
    path::Path,
};
use uuid::Uuid;

pub const MAX_RESTORE_UPLOAD_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug)]
pub enum RestoreFileError {
    Validation(String),
    Unavailable(String),
}
impl fmt::Display for RestoreFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) | Self::Unavailable(message) => f.write_str(message),
        }
    }
}
impl std::error::Error for RestoreFileError {}
fn invalid(message: &str) -> RestoreFileError {
    RestoreFileError::Validation(message.into())
}
fn unavailable(message: &str) -> RestoreFileError {
    RestoreFileError::Unavailable(message.into())
}

// No serde/public fields: uploaded descriptors cannot claim a verified hash.
pub struct RestoreUpload {
    bytes: Vec<u8>,
}
pub struct VerifiedRestoreBytes {
    bytes: Vec<u8>,
    sha256: String,
}
pub struct PublishedRestoreImport {
    sha256: String,
    byte_length: u64,
}
impl VerifiedRestoreBytes {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    pub fn byte_length(&self) -> u64 {
        self.bytes.len() as u64
    }
}
impl PublishedRestoreImport {
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    pub fn byte_length(&self) -> u64 {
        self.byte_length
    }
}

pub fn restore_upload(bytes: Vec<u8>) -> Result<RestoreUpload, RestoreFileError> {
    if bytes.is_empty() || bytes.len() > MAX_RESTORE_UPLOAD_BYTES {
        return Err(invalid(
            "복원 JSON 파일은 비어 있지 않아야 하며 64MiB 이하여야 합니다.",
        ));
    }
    Ok(RestoreUpload { bytes })
}
pub fn verify_restore_upload(
    upload: RestoreUpload,
) -> Result<VerifiedRestoreBytes, RestoreFileError> {
    verify_restore_bytes(upload.bytes)
}
/// Syntax and content identity only. The snapshot decoder must still validate
/// the closed format, references and semantics before publication/registration.
/// Local journals and before-restore captures do not inherit the HTTP size cap.
pub fn verify_restore_bytes(bytes: Vec<u8>) -> Result<VerifiedRestoreBytes, RestoreFileError> {
    reject_duplicate_json_keys(&bytes)?;
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    Ok(VerifiedRestoreBytes { bytes, sha256 })
}

struct UniqueJson;
impl<'de> Deserialize<'de> for UniqueJson {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(UniqueJsonVisitor)
    }
}
struct UniqueJsonVisitor;
impl<'de> Visitor<'de> for UniqueJsonVisitor {
    type Value = UniqueJson;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a JSON value with unique object keys")
    }
    fn visit_bool<E: de::Error>(self, _: bool) -> Result<UniqueJson, E> {
        Ok(UniqueJson)
    }
    fn visit_i64<E: de::Error>(self, _: i64) -> Result<UniqueJson, E> {
        Ok(UniqueJson)
    }
    fn visit_u64<E: de::Error>(self, _: u64) -> Result<UniqueJson, E> {
        Ok(UniqueJson)
    }
    fn visit_f64<E: de::Error>(self, _: f64) -> Result<UniqueJson, E> {
        Ok(UniqueJson)
    }
    fn visit_str<E: de::Error>(self, _: &str) -> Result<UniqueJson, E> {
        Ok(UniqueJson)
    }
    fn visit_unit<E: de::Error>(self) -> Result<UniqueJson, E> {
        Ok(UniqueJson)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<UniqueJson, A::Error> {
        while sequence.next_element::<UniqueJson>()?.is_some() {}
        Ok(UniqueJson)
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<UniqueJson, A::Error> {
        let mut keys = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            keys.try_reserve(1)
                .map_err(|_| de::Error::custom("JSON object exceeds available memory"))?;
            if !keys.insert(key) {
                return Err(de::Error::custom("duplicate JSON object key"));
            }
            map.next_value::<UniqueJson>()?;
        }
        Ok(UniqueJson)
    }
}
pub fn reject_duplicate_json_keys(bytes: &[u8]) -> Result<(), RestoreFileError> {
    // Explicit UTF-8 validation also covers strings which a skipping parser might
    // otherwise not materialize. Default serde_json recursion limits stay enabled.
    std::str::from_utf8(bytes).map_err(|_| invalid("복원 JSON은 올바른 UTF-8이어야 합니다."))?;
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    UniqueJson::deserialize(&mut deserializer)
        .and_then(|_| deserializer.end())
        .map_err(|_| invalid("복원 JSON은 중복 키 없이 완전한 단일 문서여야 합니다."))
}

struct ArchiveDirectory {
    _parent: Dir,
    _root: Dir,
    files: Dir,
}
fn archive_directory(
    root: &Path,
    name: &'static str,
    create: bool,
) -> Result<ArchiveDirectory, RestoreFileError> {
    let absolute = std::path::absolute(root)
        .map_err(|_| unavailable("복원 자료 기준 경로를 열 수 없습니다."))?;
    let parent = absolute
        .parent()
        .ok_or_else(|| unavailable("복원 자료 기준 경로가 올바르지 않습니다."))?;
    let root_name = absolute
        .file_name()
        .ok_or_else(|| unavailable("복원 자료 기준 경로가 올바르지 않습니다."))?;
    // Single ambient boundary: root comes from Store, never an HTTP argument.
    let parent = Dir::open_ambient_dir(parent, ambient_authority())
        .map_err(|_| unavailable("복원 자료 기준 폴더를 열 수 없습니다."))?;
    let root = parent
        .open_dir_nofollow(root_name)
        .map_err(|_| unavailable("복원 자료 기준 폴더를 안전하게 열 수 없습니다."))?;
    let files = match root.open_dir_nofollow(name) {
        Ok(directory) => directory,
        Err(error) if create && error.kind() == io::ErrorKind::NotFound => {
            match root.create_dir(name) {
                Ok(()) => sync_directory(&root)?,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
                Err(_) => return Err(unavailable("복원 자료 폴더를 만들 수 없습니다.")),
            }
            root.open_dir_nofollow(name)
                .map_err(|_| unavailable("복원 자료 폴더가 안전한 디렉터리가 아닙니다."))?
        }
        Err(_) => return Err(unavailable("복원 자료 폴더를 안전하게 열 수 없습니다.")),
    };
    Ok(ArchiveDirectory {
        _parent: parent,
        _root: root,
        files,
    })
}
fn sync_directory(directory: &Dir) -> Result<(), RestoreFileError> {
    #[cfg(unix)]
    directory
        .try_clone()
        .and_then(|dir| dir.into_std_file().sync_all())
        .map_err(|_| unavailable("복원 자료 폴더 기록을 완료하지 못했습니다."))?;
    // Windows has no equivalent directory fsync guarantee. File bytes are synced;
    // retained directory handles deny rename/deletion during this operation.
    #[cfg(not(unix))]
    let _ = directory;
    Ok(())
}
fn read_in(
    directory: &Dir,
    filename: &str,
    hash: &str,
    length: u64,
) -> Result<VerifiedRestoreBytes, RestoreFileError> {
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    #[cfg(windows)]
    {
        use cap_std::fs::OpenOptionsExt;
        options.share_mode(1); // FILE_SHARE_READ only; reject write/rename races.
    }
    let mut file = directory
        .open_with(filename, &options)
        .map_err(|_| unavailable("등록된 복원 파일을 안전하게 열 수 없습니다."))?;
    let metadata = file
        .metadata()
        .map_err(|_| unavailable("복원 파일 정보를 읽을 수 없습니다."))?;
    if !metadata.is_file() || metadata.len() != length {
        return Err(unavailable(
            "복원 파일 형식 또는 길이가 등록 정보와 다릅니다.",
        ));
    }
    let capacity = usize::try_from(length)
        .map_err(|_| unavailable("복원 파일이 지원 가능한 메모리 범위를 넘었습니다."))?;
    let max_read = length
        .checked_add(1)
        .ok_or_else(|| unavailable("복원 파일 길이가 지원 범위를 넘었습니다."))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| unavailable("복원 자료를 읽을 메모리를 확보하지 못했습니다."))?;
    (&mut file)
        .take(max_read)
        .read_to_end(&mut bytes)
        .map_err(|_| unavailable("복원 파일을 끝까지 읽지 못했습니다."))?;
    if bytes.len() as u64 != length || format!("{:x}", Sha256::digest(&bytes)) != hash {
        return Err(unavailable("복원 파일 내용이 등록 정보와 다릅니다."));
    }
    verify_restore_bytes(bytes)
        .map_err(|_| unavailable("등록된 복원 JSON 형식이 올바르지 않습니다."))
}
pub fn read_restore_import(
    root: &Path,
    sha256: &str,
    expected_length: u64,
) -> Result<VerifiedRestoreBytes, RestoreFileError> {
    crate::restore::validate_restore_sha256(sha256).map_err(RestoreFileError::Validation)?;
    let directory = archive_directory(root, "imports", false)?;
    read_in(
        &directory.files,
        &format!("{sha256}.json"),
        sha256,
        expected_length,
    )
}
pub fn read_restore_journal(
    root: &Path,
    sequence: i64,
    expected_sha256: &str,
    expected_length: u64,
) -> Result<VerifiedRestoreBytes, RestoreFileError> {
    if sequence <= 0 {
        return Err(invalid("변경 기록 순번은 양수여야 합니다."));
    }
    crate::restore::validate_restore_sha256(expected_sha256)
        .map_err(RestoreFileError::Validation)?;
    let directory = archive_directory(root, "snapshots", false)?;
    read_in(
        &directory.files,
        &format!("{sequence:020}.json"),
        expected_sha256,
        expected_length,
    )
}
/// Caller must first run the snapshot semantic decoder. Publication does not
/// register this hash; only a committed Store registry row grants lookup access.
pub fn publish_restore_import(
    root: &Path,
    verified: &VerifiedRestoreBytes,
) -> Result<PublishedRestoreImport, RestoreFileError> {
    let directory = archive_directory(root, "imports", true)?;
    let temporary = format!(".restore-{}.tmp", Uuid::new_v4());
    let destination = format!("{}.json", verified.sha256());
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create_new(true)
        .follow(FollowSymlinks::No);
    let mut file = directory
        .files
        .open_with(&temporary, &options)
        .map_err(|_| unavailable("복원 자료 임시파일을 만들 수 없습니다."))?;
    let result = (|| {
        file.write_all(verified.bytes())
            .and_then(|_| file.sync_all())
            .map_err(|_| unavailable("복원 자료 원본 기록을 완료하지 못했습니다."))?;
        drop(file);
        // Never fall back to overwrite/rename when hard-link publication fails.
        match directory
            .files
            .hard_link(&temporary, &directory.files, &destination)
        {
            Ok(()) => (),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
            Err(_) => return Err(unavailable("복원 파일을 덮어쓰지 않고 게시할 수 없습니다.")),
        }
        let actual = read_in(
            &directory.files,
            &destination,
            verified.sha256(),
            verified.byte_length(),
        )?;
        if actual.bytes() != verified.bytes() {
            return Err(unavailable("같은 해시 이름의 복원 파일 내용이 다릅니다."));
        }
        sync_directory(&directory.files)?;
        Ok(PublishedRestoreImport {
            sha256: verified.sha256().into(),
            byte_length: verified.byte_length(),
        })
    })();
    // Only our own random temporary name is removed; registered/orphan archives
    // and previous journals are preserved. Interrupted temps are never listed.
    let _ = directory.files.remove_file(&temporary);
    result
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;
    #[test]
    fn restore_directory_handles_block_replacement_until_verification_finishes() {
        let root = tempfile::tempdir().unwrap();
        let directory = archive_directory(root.path(), "imports", true).unwrap();
        let original = root.path().join("imports");
        let replacement = root.path().join("moved");
        assert!(std::fs::rename(&original, &replacement).is_err());
        drop(directory);
        std::fs::rename(&original, &replacement).unwrap();
    }
}
