use image::{ExtendedColorType, ImageEncoder};
use sunchang_hansang::photo_assets::{
    MAX_PHOTO_BYTES, PhotoFormat, photo_upload, publish_photo, read_photo,
    validate_asset_descriptor, validate_photo, verify_photo,
};

fn png() -> Vec<u8> {
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(&[11, 29, 47, 255], 1, 1, ExtendedColorType::Rgba8)
        .unwrap();
    bytes
}

fn crc(bytes: &[u8]) -> u32 {
    let mut value = !0u32;
    for byte in bytes {
        value ^= u32::from(*byte);
        for _ in 0..8 {
            value = (value >> 1) ^ (0xedb88320u32 & (0u32.wrapping_sub(value & 1)));
        }
    }
    !value
}

fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut value = (data.len() as u32).to_be_bytes().to_vec();
    value.extend(kind);
    value.extend(data);
    value.extend(crc(&value[4..]).to_be_bytes());
    value
}

#[test]
fn jpeg_and_png_are_decoded_but_original_bytes_and_derived_identity_are_preserved() {
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new(&mut jpeg)
        .write_image(&[11, 29, 47], 1, 1, ExtendedColorType::Rgb8)
        .unwrap();
    for (filename, mime, bytes, format) in [
        ("가상.png", "image/png", png(), PhotoFormat::Png),
        ("synthetic.JPEG", "image/jpeg", jpeg, PhotoFormat::Jpeg),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let photo =
            validate_photo(photo_upload(filename, Some(mime), bytes.clone()).unwrap()).unwrap();
        assert_eq!(photo.descriptor().format, format);
        assert_eq!(
            (photo.descriptor().width, photo.descriptor().height),
            (1, 1)
        );
        assert_eq!(photo.descriptor().byte_length, bytes.len() as u64);
        let published = publish_photo(directory.path(), &photo).unwrap();
        assert_eq!(published.descriptor(), photo.descriptor());
        assert_eq!(
            read_photo(directory.path(), photo.descriptor())
                .unwrap()
                .bytes,
            bytes
        );
        verify_photo(directory.path(), photo.descriptor()).unwrap();
        let again = publish_photo(directory.path(), &photo).unwrap();
        assert_eq!(again.descriptor(), photo.descriptor());
    }
}

#[test]
fn filenames_mime_and_signatures_never_become_storage_paths() {
    for name in [
        "../a.png", "a/b.png", "a\\b.png", "C:a.png", "CON.png", "nul.PNG", "LPT1.jpg", "a\0.png",
        "a.png ", ".png",
    ] {
        assert!(
            photo_upload(name, Some("image/png"), png()).is_err(),
            "{name:?}"
        );
    }
    assert!(photo_upload("a.png", Some("text/html"), png()).is_err());
    assert!(photo_upload("a.png", Some("image/png"), vec![]).is_err());
    assert!(photo_upload("a.png", Some("image/png"), vec![0; MAX_PHOTO_BYTES + 1]).is_err());
    for bytes in [
        b"<svg xmlns='http://www.w3.org/2000/svg'/>".to_vec(),
        b"GIF89a".to_vec(),
        png()[..40].to_vec(),
    ] {
        assert!(validate_photo(photo_upload("a.png", Some("image/png"), bytes).unwrap()).is_err());
    }
    assert!(validate_photo(photo_upload("a.jpg", Some("image/jpeg"), png()).unwrap()).is_err());
}

#[test]
fn dimensions_and_animation_are_rejected_before_large_output_allocation() {
    for (width, height) in [(8193u32, 1u32), (5000, 5000), (0, 1)] {
        let mut bytes = png();
        bytes[16..20].copy_from_slice(&width.to_be_bytes());
        bytes[20..24].copy_from_slice(&height.to_be_bytes());
        let checksum = crc(&bytes[12..29]);
        bytes[29..33].copy_from_slice(&checksum.to_be_bytes());
        assert!(
            validate_photo(photo_upload("large.png", Some("image/png"), bytes).unwrap()).is_err()
        );
    }
    let original = png();
    let mut animated = original[..33].to_vec();
    animated.extend(chunk(b"acTL", &[0, 0, 0, 1, 0, 0, 0, 0]));
    animated.extend(chunk(
        b"fcTL",
        &[
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 10, 0, 0,
        ],
    ));
    animated.extend(&original[33..]);
    assert!(
        validate_photo(photo_upload("animation.png", Some("image/png"), animated).unwrap())
            .is_err()
    );
}

#[test]
fn forged_descriptors_and_changed_registered_bytes_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let photo = validate_photo(photo_upload("a.png", Some("image/png"), png()).unwrap()).unwrap();
    publish_photo(directory.path(), &photo).unwrap();
    let descriptor = photo.descriptor();
    let mut wrong_dimensions = descriptor.clone();
    wrong_dimensions.width = 2;
    assert!(read_photo(directory.path(), &wrong_dimensions).is_err());
    for change in [0, 1, 2, 3] {
        let mut forged = descriptor.clone();
        match change {
            0 => forged.asset_key = "../outside.png".into(),
            1 => forged.sha256 = forged.sha256.to_uppercase(),
            2 => forged.width = 8193,
            _ => forged.byte_length = 0,
        }
        assert!(validate_asset_descriptor(&forged).is_err());
        assert!(read_photo(directory.path(), &forged).is_err());
    }
    std::fs::write(directory.path().join(&descriptor.asset_key), b"changed").unwrap();
    assert!(read_photo(directory.path(), descriptor).is_err());
    assert!(publish_photo(directory.path(), &photo).is_err());
    assert_eq!(
        std::fs::read(directory.path().join(&descriptor.asset_key)).unwrap(),
        b"changed"
    );
}

#[test]
fn a_nonregular_destination_is_not_replaced_or_removed() {
    let directory = tempfile::tempdir().unwrap();
    let photo = validate_photo(photo_upload("a.png", Some("image/png"), png()).unwrap()).unwrap();
    let destination = directory.path().join(&photo.descriptor().asset_key);
    std::fs::create_dir_all(&destination).unwrap();
    std::fs::write(destination.join("sentinel"), b"preserve").unwrap();
    assert!(publish_photo(directory.path(), &photo).is_err());
    assert_eq!(
        std::fs::read(destination.join("sentinel")).unwrap(),
        b"preserve"
    );
    assert!(read_photo(directory.path(), photo.descriptor()).is_err());
}

#[cfg(unix)]
#[test]
fn linked_directories_and_files_are_rejected_without_touching_the_target() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), directory.path().join("assets")).unwrap();
    let photo = validate_photo(photo_upload("a.png", Some("image/png"), png()).unwrap()).unwrap();
    assert!(publish_photo(directory.path(), &photo).is_err());
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
    std::fs::remove_file(directory.path().join("assets")).unwrap();
    std::fs::create_dir_all(directory.path().join("assets/photos")).unwrap();
    let target = outside.path().join("outside.png");
    std::fs::write(&target, png()).unwrap();
    symlink(
        &target,
        directory.path().join(&photo.descriptor().asset_key),
    )
    .unwrap();
    assert!(read_photo(directory.path(), photo.descriptor()).is_err());
    assert!(publish_photo(directory.path(), &photo).is_err());
    assert_eq!(std::fs::read(target).unwrap(), png());
}

#[cfg(windows)]
#[test]
fn windows_junction_directory_is_rejected_and_external_contents_preserved() {
    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let junction = directory.path().join("assets");
    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:PHOTO_TEST_LINK -Target $env:PHOTO_TEST_TARGET | Out-Null"])
        .env("PHOTO_TEST_LINK", &junction)
        .env("PHOTO_TEST_TARGET", outside.path())
        .status().unwrap();
    assert!(status.success(), "junction setup must really execute");
    let photo = validate_photo(photo_upload("a.png", Some("image/png"), png()).unwrap()).unwrap();
    assert!(publish_photo(directory.path(), &photo).is_err());
    assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
    // Remove only this empty junction, never recursively traverse the target.
    std::fs::remove_dir(junction).unwrap();
}
