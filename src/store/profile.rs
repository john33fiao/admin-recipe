use super::{Store, StoreError, catalog, unavailable};
use crate::{
    catalog::{MaterialVersion, MutationMeta},
    photo_assets::{self, PhotoAssetDescriptor, PhotoAssetError, PhotoBytes, PhotoUpload},
    profile::*,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path};
use uuid::Uuid;

mod validation;
pub(super) use validation::{Validation, validate_snapshot};

fn invalid(message: impl std::fmt::Display) -> StoreError {
    StoreError::Validation(message.to_string())
}
fn photo_error(error: PhotoAssetError) -> StoreError {
    match error {
        PhotoAssetError::Validation(message) => StoreError::Validation(message),
        PhotoAssetError::Unavailable(message) => StoreError::Unavailable(message),
    }
}
fn stale() -> StoreError {
    StoreError::StaleState("품목 또는 사진·구입처가 변경되었습니다. 최신 내용을 확인하세요".into())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AssetRow {
    pub descriptor: PhotoAssetDescriptor,
    pub recorded_at: String,
    pub seq: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct VersionRow {
    pub id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub previous_version_id: Option<String>,
    pub copied_from_version_id: Option<String>,
    pub recorded_at: String,
    pub seq: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Head {
    pub material_id: String,
    pub current_version_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PhotoRow {
    pub profile_version_id: String,
    pub photo: ProfilePhoto,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SupplierRow {
    pub profile_version_id: String,
    pub supplier: ProfileSupplier,
}
pub(super) struct SnapshotRows {
    pub assets: Vec<AssetRow>,
    pub versions: Vec<VersionRow>,
    pub heads: Vec<Head>,
    pub photos: Vec<PhotoRow>,
    pub suppliers: Vec<SupplierRow>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalRequest {
    material_id: String,
    request: ReviseMaterialProfile,
    upload: Option<PhotoAssetDescriptor>,
}

fn text(value: &mut String, max: usize, label: &str) -> Result<(), StoreError> {
    *value = value
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim()
        .to_owned();
    if value.chars().count() > max || value.chars().any(|c| c.is_control() && c != '\n') {
        return Err(invalid(format!(
            "{label}은 줄바꿈 이외의 제어문자 없이 {max}자 이내로 입력하세요"
        )));
    }
    Ok(())
}
fn normalize_supplier(input: &mut SupplierInput) -> Result<(), StoreError> {
    text(
        &mut input.description,
        MAX_SUPPLIER_DESCRIPTION_CHARS,
        "구입처 설명",
    )?;
    if let Some(raw) = &input.url {
        // Reject controls before trimming: a URL parser may silently remove them.
        if raw.chars().any(char::is_control) {
            return Err(invalid("구입처 주소에 제어문자를 사용할 수 없습니다"));
        }
        let raw = raw.trim();
        if raw.is_empty() {
            input.url = None;
        } else {
            if raw.chars().count() > MAX_SUPPLIER_URL_CHARS {
                return Err(invalid("구입처 주소가 너무 깁니다"));
            }
            let parsed = url::Url::parse(raw)
                .map_err(|_| invalid("절대 http 또는 https 주소를 입력하세요"))?;
            if !matches!(parsed.scheme(), "http" | "https")
                || !parsed.has_host()
                || !parsed.username().is_empty()
                || parsed.password().is_some()
            {
                return Err(invalid(
                    "사용자 정보가 없는 절대 http 또는 https 주소를 입력하세요",
                ));
            }
            input.url = Some(raw.to_owned());
        }
    }
    if input.url.is_none() && input.description.is_empty() {
        return Err(invalid("구입처 주소 또는 설명을 입력하세요"));
    }
    Ok(())
}
fn normalize(request: &mut ReviseMaterialProfile) -> Result<MutationMeta, StoreError> {
    let mut meta: MutationMeta = request.meta.clone().into();
    catalog::normalize_meta(&mut meta)?;
    request.meta.request_id = meta.request_id.clone();
    request.meta.epoch = meta.epoch.clone();
    request.meta.expected_revision = meta.expected_revision.clone();
    request.expected_material_version_id =
        catalog::uuid(&request.expected_material_version_id, "품목 버전")?;
    request.expected_profile_version_id = request
        .expected_profile_version_id
        .as_deref()
        .map(|id| catalog::uuid(id, "사진·구입처 버전"))
        .transpose()?;
    if request.photos.len() > MAX_PROFILE_PHOTOS || request.suppliers.len() > MAX_PROFILE_SUPPLIERS
    {
        return Err(invalid("사진과 구입처는 각각 20개까지 저장할 수 있습니다"));
    }
    if (request.photos.is_empty() && request.primary_position.is_some())
        || (!request.photos.is_empty()
            && !request
                .primary_position
                .is_some_and(|p| p < request.photos.len()))
    {
        return Err(invalid("사진 목록에서 대표 사진 한 장을 선택하세요"));
    }
    let mut existing = HashSet::new();
    let mut uploads = 0;
    for photo in &mut request.photos {
        let caption = match photo {
            ProfilePhotoInput::Existing { photo_id, caption } => {
                *photo_id = catalog::uuid(photo_id, "사진 ID")?;
                if !existing.insert(photo_id.clone()) {
                    return Err(invalid("같은 사진을 목록에 중복 연결할 수 없습니다"));
                }
                caption
            }
            ProfilePhotoInput::Upload { caption } => {
                uploads += 1;
                caption
            }
        };
        text(caption, MAX_PROFILE_CAPTION_CHARS, "사진 설명")?;
    }
    if uploads > 1 {
        return Err(invalid("새 사진은 한 번에 한 장만 등록할 수 있습니다"));
    }
    for supplier in &mut request.suppliers {
        normalize_supplier(supplier)?;
    }
    if serde_json::to_vec(request).map_err(unavailable)?.len() > MAX_PROFILE_METADATA_BYTES {
        return Err(invalid("사진·구입처 입력이 허용 크기를 초과했습니다"));
    }
    Ok(meta)
}

fn asset(connection: &Connection, sha: &str) -> Result<Option<AssetRow>, StoreError> {
    connection.query_row("SELECT sha256,asset_key,format,byte_length,width,height,recorded_at,sequence FROM photo_assets WHERE sha256=?1", [sha], asset_row).optional().map_err(unavailable)
}
fn asset_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssetRow> {
    let format: String = row.get(2)?;
    let format = serde_json::from_value(serde_json::Value::String(format)).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
    })?;
    Ok(AssetRow {
        descriptor: PhotoAssetDescriptor {
            sha256: row.get(0)?,
            asset_key: row.get(1)?,
            format,
            byte_length: u64::try_from(row.get::<_, i64>(3)?).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Integer,
                    Box::new(error),
                )
            })?,
            width: row.get(4)?,
            height: row.get(5)?,
        },
        recorded_at: row.get(6)?,
        seq: row.get::<_, i64>(7)?.to_string(),
    })
}
fn position(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<usize> {
    usize::try_from(row.get::<_, i64>(index)?).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}
fn version_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<VersionRow> {
    Ok(VersionRow {
        id: row.get(0)?,
        material_id: row.get(1)?,
        material_version_id: row.get(2)?,
        previous_version_id: row.get(3)?,
        copied_from_version_id: row.get(4)?,
        recorded_at: row.get(5)?,
        seq: row.get::<_, i64>(6)?.to_string(),
    })
}
const VERSION_COLUMNS: &str = "id,material_id,material_version_id,previous_version_id,copied_from_version_id,recorded_at,sequence";
fn versions(
    connection: &Connection,
    material: Option<&str>,
) -> Result<Vec<VersionRow>, StoreError> {
    connection.prepare(&format!("SELECT {VERSION_COLUMNS} FROM material_profile_versions WHERE (?1 IS NULL OR material_id=?1) ORDER BY sequence,id")).map_err(unavailable)?
        .query_map([material], version_row).map_err(unavailable)?.collect::<rusqlite::Result<_>>().map_err(unavailable)
}
fn current(connection: &Connection, material: &str) -> Result<Option<VersionRow>, StoreError> {
    let id: Option<String> = connection
        .query_row(
            "SELECT current_version_id FROM material_profile_heads WHERE material_id=?1",
            [material],
            |r| r.get(0),
        )
        .optional()
        .map_err(unavailable)?;
    id.map(|id| connection.query_row(&format!("SELECT {VERSION_COLUMNS} FROM material_profile_versions WHERE id=?1 AND material_id=?2"), params![id, material], version_row).map_err(unavailable)).transpose()
}
fn full(connection: &Connection, row: VersionRow) -> Result<MaterialProfileVersion, StoreError> {
    let raw = connection.prepare("SELECT id,position,asset_sha256,caption,is_primary FROM material_profile_photos WHERE profile_version_id=?1 ORDER BY position").map_err(unavailable)?
        .query_map([&row.id], |r| Ok((r.get::<_,String>(0)?,position(r,1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,bool>(4)?))).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    let photos = raw
        .into_iter()
        .map(|(id, position, sha, caption, is_primary)| {
            Ok(ProfilePhoto {
                id,
                position,
                asset: asset(connection, &sha)?
                    .ok_or_else(|| unavailable("사진 자산 참조가 없습니다"))?
                    .descriptor,
                caption,
                is_primary,
            })
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    let suppliers = connection.prepare("SELECT position,url,description FROM material_profile_suppliers WHERE profile_version_id=?1 ORDER BY position").map_err(unavailable)?
        .query_map([&row.id], |r| Ok(ProfileSupplier{position:position(r,0)?,url:r.get(1)?,description:r.get(2)?})).map_err(unavailable)?.collect::<rusqlite::Result<_>>().map_err(unavailable)?;
    Ok(MaterialProfileVersion {
        id: row.id,
        material_id: row.material_id,
        material_version_id: row.material_version_id,
        previous_version_id: row.previous_version_id,
        copied_from_version_id: row.copied_from_version_id,
        photos,
        suppliers,
        recorded_at: row.recorded_at,
        seq: row.seq,
    })
}
fn summary(v: MaterialProfileVersion) -> MaterialProfileVersionSummary {
    MaterialProfileVersionSummary {
        id: v.id,
        material_version_id: v.material_version_id,
        previous_version_id: v.previous_version_id,
        copied_from_version_id: v.copied_from_version_id,
        photo_count: v.photos.len(),
        supplier_count: v.suppliers.len(),
        primary_photo_id: v.photos.iter().find(|p| p.is_primary).map(|p| p.id.clone()),
        recorded_at: v.recorded_at,
        seq: v.seq,
    }
}
fn verify_version(root: &Path, version: &MaterialProfileVersion) -> Result<(), StoreError> {
    let mut seen = HashSet::new();
    for photo in &version.photos {
        if seen.insert(&photo.asset.sha256) {
            photo_assets::verify_photo(root, &photo.asset).map_err(photo_error)?;
        }
    }
    Ok(())
}
fn insert_version(
    connection: &Connection,
    version: &MaterialProfileVersion,
) -> Result<(), StoreError> {
    connection.execute("INSERT INTO material_profile_versions(id,material_id,material_version_id,previous_version_id,copied_from_version_id,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![version.id,version.material_id,version.material_version_id,version.previous_version_id,version.copied_from_version_id,version.recorded_at,version.seq.parse::<i64>().map_err(unavailable)?]).map_err(unavailable)?;
    for p in &version.photos {
        connection.execute("INSERT INTO material_profile_photos(profile_version_id,id,position,asset_sha256,caption,is_primary) VALUES(?1,?2,?3,?4,?5,?6)",params![version.id,p.id,p.position as i64,p.asset.sha256,p.caption,p.is_primary]).map_err(unavailable)?;
    }
    for s in &version.suppliers {
        connection.execute("INSERT INTO material_profile_suppliers(profile_version_id,position,url,description) VALUES(?1,?2,?3,?4)",params![version.id,s.position as i64,s.url,s.description]).map_err(unavailable)?;
    }
    connection.execute("INSERT INTO material_profile_heads(material_id,current_version_id) VALUES(?1,?2) ON CONFLICT(material_id) DO UPDATE SET current_version_id=excluded.current_version_id",params![version.material_id,version.id]).map_err(unavailable)?;
    Ok(())
}

impl Store {
    pub fn profile_current(
        &self,
        material_id: &str,
    ) -> Result<Option<MaterialProfileDetail>, StoreError> {
        let id = catalog::uuid(material_id, "품목 ID")?;
        let Some(owner) = catalog::current_version(&self.connection, &id)? else {
            return Ok(None);
        };
        Ok(Some(MaterialProfileDetail {
            material: catalog::summary(&self.connection, owner)?,
            version: current(&self.connection, &id)?
                .map(|row| full(&self.connection, row))
                .transpose()?,
            state: catalog::state(&self.connection)?,
        }))
    }
    pub fn profile_versions(
        &self,
        material_id: &str,
    ) -> Result<Option<MaterialProfileHistory>, StoreError> {
        let id = catalog::uuid(material_id, "품목 ID")?;
        if catalog::current_version(&self.connection, &id)?.is_none() {
            return Ok(None);
        }
        let versions = versions(&self.connection, Some(&id))?
            .into_iter()
            .rev()
            .map(|v| full(&self.connection, v).map(summary))
            .collect::<Result<_, _>>()?;
        Ok(Some(MaterialProfileHistory {
            material_id: id,
            versions,
            state: catalog::state(&self.connection)?,
        }))
    }
    pub fn profile_version(
        &self,
        material_id: &str,
        version_id: &str,
    ) -> Result<Option<MaterialProfileDetail>, StoreError> {
        let id = catalog::uuid(material_id, "품목 ID")?;
        let version = catalog::uuid(version_id, "사진·구입처 버전")?;
        let Some(owner) = catalog::current_version(&self.connection, &id)? else {
            return Ok(None);
        };
        let row = self.connection.query_row(&format!("SELECT {VERSION_COLUMNS} FROM material_profile_versions WHERE id=?1 AND material_id=?2"),params![version,id],version_row).optional().map_err(unavailable)?;
        row.map(|row| {
            Ok(MaterialProfileDetail {
                material: catalog::summary(&self.connection, owner)?,
                version: Some(full(&self.connection, row)?),
                state: catalog::state(&self.connection)?,
            })
        })
        .transpose()
    }
    pub fn profile_revise(
        &mut self,
        material_id: &str,
        mut request: ReviseMaterialProfile,
        upload: Option<PhotoUpload>,
    ) -> Result<MaterialProfileDetail, StoreError> {
        let material_id = catalog::uuid(material_id, "품목 ID")?;
        let meta = normalize(&mut request)?;
        let wants_upload = request
            .photos
            .iter()
            .any(|p| matches!(p, ProfilePhotoInput::Upload { .. }));
        if wants_upload != upload.is_some() {
            return Err(invalid("새 사진 입력과 실제 파일이 일치하지 않습니다"));
        }
        let validated = upload
            .map(photo_assets::validate_photo)
            .transpose()
            .map_err(photo_error)?;
        let descriptor = validated.as_ref().map(|p| p.descriptor().clone());
        let payload = serde_json::to_string(&CanonicalRequest {
            material_id: material_id.clone(),
            request: request.clone(),
            upload: descriptor.clone(),
        })
        .map_err(unavailable)?;
        // File publication precedes the DB transaction. An unregistered orphan
        // after a rejected transaction is retained, but never served by GET.
        if let Some(photo) = &validated {
            photo_assets::publish_photo(&self.root, photo).map_err(photo_error)?;
        }
        let root = self.root.clone();
        let result = self.mutate(
            Some(&meta),
            &meta.request_id,
            "profile.revise",
            &payload,
            move |tx, seq, time, state| {
                let owner = catalog::current_version(tx, &material_id)?
                    .ok_or_else(|| invalid("품목을 찾을 수 없습니다"))?;
                if owner.id != request.expected_material_version_id {
                    return Err(stale());
                }
                let prior = current(tx, &material_id)?.map(|row| full(tx, row)).transpose()?;
                if prior.as_ref().map(|v| v.id.as_str()) != request.expected_profile_version_id.as_deref() {
                    return Err(stale());
                }
                if let Some(d) = &descriptor {
                    super::restore::preserve_asset(tx, d)?;
                    if let Some(existing) = asset(tx, &d.sha256)? {
                        if existing.descriptor != *d {
                            return Err(unavailable("등록된 사진 자산 정보가 일치하지 않습니다"));
                        }
                    } else {
                        let format = serde_json::to_value(d.format).map_err(unavailable)?;
                        let byte_length = i64::try_from(d.byte_length).map_err(unavailable)?;
                        tx.execute(
                            "INSERT INTO photo_assets(sha256,asset_key,format,byte_length,width,height,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                            params![d.sha256,d.asset_key,format.as_str().ok_or_else(|| unavailable("사진 형식이 올바르지 않습니다"))?,byte_length,d.width,d.height,time,seq],
                        ).map_err(unavailable)?;
                    }
                }
                let mut photos = Vec::new();
                for (position, input) in request.photos.into_iter().enumerate() {
                    let (id, asset, caption) = match input {
                        ProfilePhotoInput::Existing { photo_id, caption } => {
                            let old = prior.as_ref()
                                .and_then(|v| v.photos.iter().find(|p| p.id == photo_id))
                                .ok_or_else(|| invalid("기대 사진·구입처 버전에 속한 사진만 연결할 수 있습니다"))?;
                            (photo_id, old.asset.clone(), caption)
                        }
                        ProfilePhotoInput::Upload { caption } => (
                            Uuid::new_v4().to_string(),
                            descriptor.clone().ok_or_else(|| invalid("새 사진 파일이 없습니다"))?,
                            caption,
                        ),
                    };
                    photos.push(ProfilePhoto {
                        id,
                        position,
                        asset,
                        caption,
                        is_primary: request.primary_position == Some(position),
                    });
                }
                let version = MaterialProfileVersion {
                    id: Uuid::new_v4().to_string(),
                    material_id: material_id.clone(),
                    material_version_id: owner.id.clone(),
                    previous_version_id: prior.map(|v| v.id),
                    copied_from_version_id: None,
                    photos,
                    suppliers: request.suppliers.into_iter().enumerate()
                        .map(|(position, s)| ProfileSupplier { position, url: s.url, description: s.description }).collect(),
                    recorded_at: time.into(),
                    seq: seq.to_string(),
                };
                verify_version(&root, &version)?;
                insert_version(tx, &version)?;
                Ok(MaterialProfileDetail {
                    material: catalog::summary(tx, owner)?,
                    version: Some(version),
                    state: state.clone(),
                })
            },
        )?;
        if let Some(version) = &result.version {
            verify_version(&self.root, version)?;
        }
        Ok(result)
    }
    pub fn photo_asset_bytes(&self, sha256: &str) -> Result<Option<PhotoBytes>, StoreError> {
        if sha256.len() != 64
            || !sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid("사진 자산 ID가 올바르지 않습니다"));
        }
        let active = asset(&self.connection, sha256)?;
        let preserved = super::restore::preserved_asset(&self.connection, sha256)?;
        if let Some(active) = active
            && preserved.as_ref() != Some(&active.descriptor)
        {
            return Err(unavailable("활성 사진 자산과 보존 등록 정보가 다릅니다"));
        }
        preserved
            .map(|descriptor| {
                photo_assets::read_photo(&self.root, &descriptor).map_err(photo_error)
            })
            .transpose()
    }
}

pub(super) fn copy_profile(
    connection: &Connection,
    root: &Path,
    source: &str,
    target: &MaterialVersion,
    seq: i64,
    time: &str,
) -> Result<(), StoreError> {
    let Some(row) = current(connection, source)? else {
        return Ok(());
    };
    let mut version = full(connection, row)?;
    verify_version(root, &version)?;
    version.copied_from_version_id = Some(version.id);
    version.id = Uuid::new_v4().to_string();
    version.material_id = target.material_id.clone();
    version.material_version_id = target.id.clone();
    version.previous_version_id = None;
    version.recorded_at = time.into();
    version.seq = seq.to_string();
    for p in &mut version.photos {
        p.id = Uuid::new_v4().to_string();
    }
    insert_version(connection, &version)
}
pub(super) fn verify_copy_result(
    connection: &Connection,
    root: &Path,
    material: &str,
    seq: &str,
) -> Result<(), StoreError> {
    for row in versions(connection, Some(material))?
        .into_iter()
        .filter(|v| v.seq == seq)
    {
        verify_version(root, &full(connection, row)?)?;
    }
    Ok(())
}
pub(super) fn verify_assets(connection: &Connection, root: &Path) -> Result<(), StoreError> {
    for descriptor in super::restore::preserved_assets(connection)? {
        photo_assets::verify_photo(root, &descriptor).map_err(photo_error)?;
    }
    Ok(())
}
pub(super) fn snapshot_rows(connection: &Connection) -> Result<SnapshotRows, StoreError> {
    let assets=connection.prepare("SELECT sha256,asset_key,format,byte_length,width,height,recorded_at,sequence FROM photo_assets ORDER BY sequence,sha256").map_err(unavailable)?.query_map([],asset_row).map_err(unavailable)?.collect::<rusqlite::Result<_>>().map_err(unavailable)?;
    let versions = versions(connection, None)?;
    let heads=connection.prepare("SELECT material_id,current_version_id FROM material_profile_heads ORDER BY material_id").map_err(unavailable)?.query_map([],|r|Ok(Head{material_id:r.get(0)?,current_version_id:r.get(1)?})).map_err(unavailable)?.collect::<rusqlite::Result<_>>().map_err(unavailable)?;
    let mut photos = Vec::new();
    let mut suppliers = Vec::new();
    for row in &versions {
        let version = full(connection, row.clone())?;
        photos.extend(version.photos.into_iter().map(|photo| PhotoRow {
            profile_version_id: row.id.clone(),
            photo,
        }));
        suppliers.extend(version.suppliers.into_iter().map(|supplier| SupplierRow {
            profile_version_id: row.id.clone(),
            supplier,
        }));
    }
    Ok(SnapshotRows {
        assets,
        versions,
        heads,
        photos,
        suppliers,
    })
}
pub(super) fn validate_profile_schema(connection: &Connection) -> Result<(), StoreError> {
    let rows = snapshot_rows(connection)?;
    let materials = catalog::versions(connection, None)?;
    let mutations=connection.prepare("SELECT sequence,operation,request_id,payload,response FROM mutation_requests ORDER BY sequence").map_err(unavailable)?.query_map([],|r|Ok((r.get::<_,i64>(0)?.to_string(),r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?))).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    let mutations = mutations
        .iter()
        .map(|r| {
            (
                r.0.as_str(),
                r.1.as_str(),
                r.2.as_str(),
                r.3.as_str(),
                r.4.as_str(),
            )
        })
        .collect::<Vec<_>>();
    let sequence = catalog::state(connection)?
        .revision
        .parse()
        .map_err(unavailable)?;
    validate_snapshot(Validation {
        assets: &rows.assets,
        versions: &rows.versions,
        heads: &rows.heads,
        photos: &rows.photos,
        suppliers: &rows.suppliers,
        materials: &materials,
        mutations: &mutations,
        sequence,
    })
}
pub(super) fn migrate_profiles(connection: &Connection) -> Result<(), StoreError> {
    connection.execute_batch("CREATE TABLE photo_assets(sha256 TEXT PRIMARY KEY,asset_key TEXT NOT NULL UNIQUE,format TEXT NOT NULL CHECK(format IN ('jpeg','png')),byte_length INTEGER NOT NULL CHECK(byte_length>0),width INTEGER NOT NULL CHECK(width>0),height INTEGER NOT NULL CHECK(height>0),recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence));
CREATE TABLE material_profile_versions(id TEXT PRIMARY KEY,material_id TEXT NOT NULL REFERENCES materials(id),material_version_id TEXT NOT NULL REFERENCES material_versions(id),previous_version_id TEXT REFERENCES material_profile_versions(id),copied_from_version_id TEXT REFERENCES material_profile_versions(id),recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),UNIQUE(material_id,sequence));
CREATE TABLE material_profile_heads(material_id TEXT PRIMARY KEY REFERENCES materials(id),current_version_id TEXT NOT NULL UNIQUE REFERENCES material_profile_versions(id));
CREATE TABLE material_profile_photos(profile_version_id TEXT NOT NULL REFERENCES material_profile_versions(id),id TEXT NOT NULL,position INTEGER NOT NULL CHECK(position>=0),asset_sha256 TEXT NOT NULL REFERENCES photo_assets(sha256),caption TEXT NOT NULL,is_primary INTEGER NOT NULL CHECK(is_primary IN (0,1)),PRIMARY KEY(profile_version_id,id),UNIQUE(profile_version_id,position));
CREATE TABLE material_profile_suppliers(profile_version_id TEXT NOT NULL REFERENCES material_profile_versions(id),position INTEGER NOT NULL CHECK(position>=0),url TEXT,description TEXT NOT NULL,PRIMARY KEY(profile_version_id,position));").map_err(unavailable)
}
