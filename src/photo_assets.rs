//! Immutable original image bytes. Only the Store decides whether an asset is registered.
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};
use image::{
    ImageDecoder, Limits,
    codecs::{jpeg::JpegDecoder, png::PngDecoder},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fmt,
    io::{self, Cursor, Read, Write},
    path::Path,
};
use uuid::Uuid;

pub const MAX_PHOTO_BYTES: usize = 10 * 1024 * 1024;
pub const MAX_PHOTO_SIDE: u32 = 8192;
pub const MAX_PHOTO_PIXELS: u64 = 20_000_000;
const MAX_DECODE_BYTES: u64 = 128 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhotoFormat {
    Jpeg,
    Png,
}
impl PhotoFormat {
    pub fn mime(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
        }
    }
    pub fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhotoAssetDescriptor {
    pub sha256: String,
    pub asset_key: String,
    pub format: PhotoFormat,
    pub byte_length: u64,
    pub width: u32,
    pub height: u32,
}

// No serde or public fields: untrusted descriptor JSON cannot mint a validated upload.
pub struct PhotoUpload {
    bytes: Vec<u8>,
    format_hint: PhotoFormat,
}
pub struct ValidatedPhoto {
    bytes: Vec<u8>,
    descriptor: PhotoAssetDescriptor,
}
pub struct PublishedPhoto {
    descriptor: PhotoAssetDescriptor,
}
pub struct PhotoBytes {
    pub descriptor: PhotoAssetDescriptor,
    pub bytes: Vec<u8>,
}
impl ValidatedPhoto {
    pub fn descriptor(&self) -> &PhotoAssetDescriptor {
        &self.descriptor
    }
}
impl PublishedPhoto {
    pub fn descriptor(&self) -> &PhotoAssetDescriptor {
        &self.descriptor
    }
}

#[derive(Debug)]
pub enum PhotoAssetError {
    Validation(String),
    Unavailable(String),
}
impl fmt::Display for PhotoAssetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(message) | Self::Unavailable(message) => f.write_str(message),
        }
    }
}
impl std::error::Error for PhotoAssetError {}
fn invalid(message: &str) -> PhotoAssetError {
    PhotoAssetError::Validation(message.into())
}
fn unavailable(message: &str) -> PhotoAssetError {
    PhotoAssetError::Unavailable(message.into())
}

pub fn photo_upload(
    filename: &str,
    content_type: Option<&str>,
    bytes: Vec<u8>,
) -> Result<PhotoUpload, PhotoAssetError> {
    if filename.is_empty()
        || filename.chars().count() > 255
        || filename.starts_with('.')
        || filename.ends_with(['.', ' '])
        || filename
            .chars()
            .any(|c| c.is_control() || "/\\:<>\"|?*".contains(c))
    {
        return Err(invalid("사진 파일 이름을 확인해 주세요."));
    }
    let (stem, extension) = filename
        .rsplit_once('.')
        .ok_or_else(|| invalid("JPEG 또는 PNG 파일을 선택해 주세요."))?;
    let device = stem
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    if matches!(device.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            device
                .strip_prefix(prefix)
                .is_some_and(|n| matches!(n, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"))
        })
    {
        return Err(invalid("사진 파일 이름을 확인해 주세요."));
    }
    let format_hint = match extension.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => PhotoFormat::Jpeg,
        "png" => PhotoFormat::Png,
        _ => return Err(invalid("JPEG 또는 PNG 파일을 선택해 주세요.")),
    };
    if content_type
        .is_some_and(|mime| mime != format_hint.mime() && mime != "application/octet-stream")
    {
        return Err(invalid("사진 형식과 전송 형식이 다릅니다."));
    }
    if bytes.is_empty() || bytes.len() > MAX_PHOTO_BYTES {
        return Err(invalid(
            "사진은 비어 있지 않은 10MiB 이하 파일이어야 합니다.",
        ));
    }
    Ok(PhotoUpload { bytes, format_hint })
}

fn limits() -> Limits {
    let mut limits = Limits::default();
    limits.max_image_width = Some(MAX_PHOTO_SIDE);
    limits.max_image_height = Some(MAX_PHOTO_SIDE);
    // image documents max_alloc as best effort, not a process RSS bound.
    limits.max_alloc = Some(MAX_DECODE_BYTES);
    limits
}
fn dimensions(width: u32, height: u32) -> Result<(), PhotoAssetError> {
    if width == 0
        || height == 0
        || width > MAX_PHOTO_SIDE
        || height > MAX_PHOTO_SIDE
        || u64::from(width) * u64::from(height) > MAX_PHOTO_PIXELS
    {
        return Err(invalid(
            "사진은 각 변 8192px, 전체 2000만 픽셀 이하여야 합니다.",
        ));
    }
    Ok(())
}
fn full_decode(decoder: impl ImageDecoder) -> Result<(u32, u32), PhotoAssetError> {
    let (width, height) = decoder.dimensions();
    dimensions(width, height)?;
    let length = decoder.total_bytes();
    if length > MAX_DECODE_BYTES {
        return Err(invalid("사진 디코딩 크기가 너무 큽니다."));
    }
    let length = usize::try_from(length).map_err(|_| invalid("사진 디코딩 크기가 너무 큽니다."))?;
    let mut decoded = Vec::new();
    decoded
        .try_reserve_exact(length)
        .map_err(|_| unavailable("사진을 검증할 메모리를 확보하지 못했습니다."))?;
    decoded.resize(length, 0);
    decoder
        .read_image(&mut decoded)
        .map_err(|_| invalid("사진 전체를 읽을 수 없습니다. 원본 파일을 확인해 주세요."))?;
    Ok((width, height))
}
pub fn validate_photo(upload: PhotoUpload) -> Result<ValidatedPhoto, PhotoAssetError> {
    let format = if upload.bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        PhotoFormat::Png
    } else if upload.bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        PhotoFormat::Jpeg
    } else {
        return Err(invalid("실제 JPEG 또는 PNG 사진이 아닙니다."));
    };
    if format != upload.format_hint {
        return Err(invalid("사진 확장자와 실제 형식이 다릅니다."));
    }
    let (width, height) = match format {
        PhotoFormat::Png => {
            let decoder = PngDecoder::with_limits(Cursor::new(&upload.bytes), limits())
                .map_err(|_| invalid("PNG 사진을 읽을 수 없습니다."))?;
            if decoder
                .is_apng()
                .map_err(|_| invalid("PNG 사진 정보를 읽을 수 없습니다."))?
            {
                return Err(invalid("움직이는 PNG는 지원하지 않습니다."));
            }
            full_decode(decoder)?
        }
        PhotoFormat::Jpeg => {
            let mut decoder = JpegDecoder::new(Cursor::new(&upload.bytes))
                .map_err(|_| invalid("JPEG 사진을 읽을 수 없습니다."))?;
            decoder
                .set_limits(limits())
                .map_err(|_| invalid("JPEG 사진 크기가 너무 큽니다."))?;
            full_decode(decoder)?
        }
    };
    let sha256 = format!("{:x}", Sha256::digest(&upload.bytes));
    let descriptor = PhotoAssetDescriptor {
        asset_key: format!("assets/photos/{sha256}.{}", format.extension()),
        sha256,
        format,
        byte_length: upload.bytes.len() as u64,
        width,
        height,
    };
    Ok(ValidatedPhoto {
        bytes: upload.bytes,
        descriptor,
    })
}
pub fn validate_asset_descriptor(value: &PhotoAssetDescriptor) -> Result<(), PhotoAssetError> {
    if value.sha256.len() != 64
        || !value
            .sha256
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        || value.asset_key
            != format!(
                "assets/photos/{}.{}",
                value.sha256,
                value.format.extension()
            )
        || value.byte_length == 0
        || value.byte_length > MAX_PHOTO_BYTES as u64
    {
        return Err(invalid("사진 자산 정보가 올바르지 않습니다."));
    }
    dimensions(value.width, value.height)
}

// Retain every directory handle until the operation ends. cap-primitives opens
// Windows directory handles without FILE_SHARE_DELETE; no-follow rejects junctions.
struct PhotoDirectory {
    _parent: Dir,
    _root: Dir,
    _assets: Dir,
    photos: Dir,
}
fn child(parent: &Dir, name: &str, create: bool) -> Result<Dir, PhotoAssetError> {
    match parent.open_dir_nofollow(name) {
        Ok(dir) => Ok(dir),
        Err(error) if create && error.kind() == io::ErrorKind::NotFound => {
            match parent.create_dir(name) {
                Ok(()) => sync_directory(parent)?,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
                Err(_) => return Err(unavailable("사진 저장 폴더를 만들 수 없습니다.")),
            }
            parent
                .open_dir_nofollow(name)
                .map_err(|_| unavailable("사진 저장 폴더가 안전한 디렉터리가 아닙니다."))
        }
        Err(_) => Err(unavailable("사진 저장 폴더를 안전하게 열 수 없습니다.")),
    }
}
fn photo_directory(root: &Path, create: bool) -> Result<PhotoDirectory, PhotoAssetError> {
    let absolute = std::path::absolute(root)
        .map_err(|_| unavailable("사진 저장 기준 경로를 열 수 없습니다."))?;
    let parent = absolute
        .parent()
        .ok_or_else(|| unavailable("사진 저장 기준 경로가 올바르지 않습니다."))?;
    let name = absolute
        .file_name()
        .ok_or_else(|| unavailable("사진 저장 기준 경로가 올바르지 않습니다."))?;
    // This is the single ambient boundary; callers pass Store.root, never HTTP data.
    let parent = Dir::open_ambient_dir(parent, ambient_authority())
        .map_err(|_| unavailable("사진 저장 기준 폴더를 열 수 없습니다."))?;
    let root = parent
        .open_dir_nofollow(name)
        .map_err(|_| unavailable("사진 저장 기준 폴더가 안전한 디렉터리가 아닙니다."))?;
    let assets = child(&root, "assets", create)?;
    let photos = child(&assets, "photos", create)?;
    Ok(PhotoDirectory {
        _parent: parent,
        _root: root,
        _assets: assets,
        photos,
    })
}
fn sync_directory(dir: &Dir) -> Result<(), PhotoAssetError> {
    #[cfg(unix)]
    dir.try_clone()
        .and_then(|dir| dir.into_std_file().sync_all())
        .map_err(|_| unavailable("사진 폴더 기록을 완료하지 못했습니다."))?;
    // Windows does not offer the same directory fsync guarantee. Files are synced.
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}
fn read_in(dir: &Dir, descriptor: &PhotoAssetDescriptor) -> Result<Vec<u8>, PhotoAssetError> {
    let filename = format!("{}.{}", descriptor.sha256, descriptor.format.extension());
    let mut options = OpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    #[cfg(windows)]
    {
        use cap_std::fs::OpenOptionsExt;
        options.share_mode(1);
    } // FILE_SHARE_READ only.
    let mut file = dir
        .open_with(filename, &options)
        .map_err(|_| unavailable("등록된 사진 파일을 안전하게 열 수 없습니다."))?;
    let metadata = file
        .metadata()
        .map_err(|_| unavailable("등록된 사진 정보를 읽을 수 없습니다."))?;
    if !metadata.is_file() || metadata.len() != descriptor.byte_length {
        return Err(unavailable(
            "등록된 사진 파일 형식 또는 길이가 바뀌었습니다.",
        ));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(descriptor.byte_length as usize)
        .map_err(|_| unavailable("사진을 읽을 메모리를 확보하지 못했습니다."))?;
    (&mut file)
        .take(MAX_PHOTO_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| unavailable("등록된 사진을 읽을 수 없습니다."))?;
    if bytes.len() as u64 != descriptor.byte_length
        || format!("{:x}", Sha256::digest(&bytes)) != descriptor.sha256
    {
        return Err(unavailable("등록된 사진 내용이 저장 정보와 다릅니다."));
    }
    // The hash authenticates the bytes against the descriptor, not its MIME or
    // dimensions. Check their header agreement too, without decoding every GET.
    let header_dimensions = match descriptor.format {
        PhotoFormat::Png if bytes.starts_with(b"\x89PNG\r\n\x1a\n") => {
            let decoder = PngDecoder::with_limits(Cursor::new(&bytes), limits())
                .map_err(|_| unavailable("등록된 PNG 사진 정보가 올바르지 않습니다."))?;
            if decoder
                .is_apng()
                .map_err(|_| unavailable("등록된 PNG 정보를 읽을 수 없습니다."))?
            {
                return Err(unavailable("등록된 사진이 움직이는 PNG입니다."));
            }
            decoder.dimensions()
        }
        PhotoFormat::Jpeg if bytes.starts_with(&[0xff, 0xd8, 0xff]) => {
            let mut decoder = JpegDecoder::new(Cursor::new(&bytes))
                .map_err(|_| unavailable("등록된 JPEG 사진 정보가 올바르지 않습니다."))?;
            decoder
                .set_limits(limits())
                .map_err(|_| unavailable("등록된 JPEG 크기가 한도를 넘었습니다."))?;
            decoder.dimensions()
        }
        _ => return Err(unavailable("등록된 사진의 실제 형식이 다릅니다.")),
    };
    if header_dimensions != (descriptor.width, descriptor.height) {
        return Err(unavailable("등록된 사진 치수가 저장 정보와 다릅니다."));
    }
    Ok(bytes)
}
pub fn read_photo(
    root: &Path,
    descriptor: &PhotoAssetDescriptor,
) -> Result<PhotoBytes, PhotoAssetError> {
    validate_asset_descriptor(descriptor)
        .map_err(|_| unavailable("등록된 사진 자산 정보가 올바르지 않습니다."))?;
    let directory = photo_directory(root, false)?;
    let bytes = read_in(&directory.photos, descriptor)?;
    Ok(PhotoBytes {
        descriptor: descriptor.clone(),
        bytes,
    })
}
pub fn verify_photo(root: &Path, descriptor: &PhotoAssetDescriptor) -> Result<(), PhotoAssetError> {
    read_photo(root, descriptor).map(|_| ())
}

/// Restore preflight classification, independent of presentation/error text.
/// Existing upload, registered GET and publication keep their original API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhotoCheckError {
    Missing,
    UnsafePath,
    Corrupt,
}
fn check_io(error: io::Error) -> PhotoCheckError {
    match error.kind() {
        io::ErrorKind::NotFound => PhotoCheckError::Missing,
        io::ErrorKind::NotADirectory | io::ErrorKind::IsADirectory => PhotoCheckError::UnsafePath,
        // Permission/device/read errors are unavailable contents, never proof of
        // absence. The check blocks restore instead of silently skipping them.
        _ => PhotoCheckError::Corrupt,
    }
}
fn checked_photo_directory(parent: &Dir, path: &Path) -> Result<Dir, PhotoCheckError> {
    let metadata = parent.symlink_metadata(path).map_err(check_io)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(PhotoCheckError::UnsafePath);
    }
    parent.open_dir_nofollow(path).map_err(check_io)
}
pub fn check_photo(root: &Path, descriptor: &PhotoAssetDescriptor) -> Result<(), PhotoCheckError> {
    validate_asset_descriptor(descriptor).map_err(|_| PhotoCheckError::Corrupt)?;
    let absolute = std::path::absolute(root).map_err(check_io)?;
    let parent_path = absolute.parent().ok_or(PhotoCheckError::UnsafePath)?;
    let root_name = absolute.file_name().ok_or(PhotoCheckError::UnsafePath)?;
    let parent = Dir::open_ambient_dir(parent_path, ambient_authority()).map_err(check_io)?;
    let root = checked_photo_directory(&parent, Path::new(root_name))?;
    let assets = checked_photo_directory(&root, Path::new("assets"))?;
    let photos = checked_photo_directory(&assets, Path::new("photos"))?;
    let filename = format!("{}.{}", descriptor.sha256, descriptor.format.extension());
    let metadata = photos.symlink_metadata(&filename).map_err(check_io)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(PhotoCheckError::UnsafePath);
    }
    // read_in uses a no-follow final handle, checks its regular type/length and
    // hash, and never trusts the earlier metadata lookup to authorize reading.
    let bytes = read_in(&photos, descriptor).map_err(|_| PhotoCheckError::Corrupt)?;
    let upload = photo_upload(
        &format!("verified.{}", descriptor.format.extension()),
        Some(descriptor.format.mime()),
        bytes,
    )
    .map_err(|_| PhotoCheckError::Corrupt)?;
    let decoded = validate_photo(upload).map_err(|_| PhotoCheckError::Corrupt)?;
    if decoded.descriptor() != descriptor {
        return Err(PhotoCheckError::Corrupt);
    }
    Ok(())
}
pub fn publish_photo(
    root: &Path,
    photo: &ValidatedPhoto,
) -> Result<PublishedPhoto, PhotoAssetError> {
    let directory = photo_directory(root, true)?;
    let temporary = format!(".upload-{}.tmp", Uuid::new_v4());
    let destination = format!(
        "{}.{}",
        photo.descriptor.sha256,
        photo.descriptor.format.extension()
    );
    let mut options = OpenOptions::new();
    options
        .write(true)
        .create_new(true)
        .follow(FollowSymlinks::No);
    let mut file = directory
        .photos
        .open_with(&temporary, &options)
        .map_err(|_| unavailable("사진 임시파일을 만들 수 없습니다."))?;
    let result = (|| {
        file.write_all(&photo.bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| unavailable("사진 원본 기록을 완료하지 못했습니다."))?;
        drop(file);
        // Hard-link publication never overwrites. Unsupported filesystems fail closed.
        match directory
            .photos
            .hard_link(&temporary, &directory.photos, destination)
        {
            Ok(()) => (),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => (),
            Err(_) => return Err(unavailable("사진 파일을 덮어쓰지 않고 게시할 수 없습니다.")),
        }
        let bytes = read_in(&directory.photos, &photo.descriptor)?;
        if bytes != photo.bytes {
            return Err(unavailable("같은 이름의 사진 내용이 다릅니다."));
        }
        sync_directory(&directory.photos)?;
        Ok(PublishedPhoto {
            descriptor: photo.descriptor.clone(),
        })
    })();
    // Only remove our random temporary name. An interrupted process may leave it;
    // the registered-asset API never exposes temporary names or unregistered blobs.
    let _ = directory.photos.remove_file(&temporary);
    result
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;

    #[test]
    fn open_directory_handles_prevent_path_replacement_until_the_operation_finishes() {
        let root = tempfile::tempdir().unwrap();
        let directory = photo_directory(root.path(), true).unwrap();
        let original = root.path().join("assets/photos");
        let replacement = root.path().join("assets/moved");
        assert!(std::fs::rename(&original, &replacement).is_err());
        assert!(original.is_dir());
        assert!(!replacement.exists());
        drop(directory);
        std::fs::rename(&original, &replacement).unwrap();
        assert!(replacement.is_dir());
    }
}
