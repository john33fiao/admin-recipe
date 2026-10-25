//! Restore decoding separates active facts from immutable replay evidence.
//! The original source bytes are never reserialized in the archive.
use super::*;
use crate::{
    photo_assets::PhotoAssetDescriptor,
    restore::{RestoreArea, RestoreAreaCount, RestoreResult, RestoreSourceKind, RestoreSummary},
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::store) struct PreservedLegacyRequest {
    pub request_id: String,
    pub name: String,
    pub unit: String,
    pub material: Material,
    pub epoch: String,
    pub sequence: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::store) struct PreservedRestoreRecord {
    pub request_id: String,
    pub previous_epoch: String,
    pub result: RestoreResult,
}

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::store) struct PreservedState {
    pub mutation_requests: Vec<MutationRecord>,
    pub creation_requests: Vec<PreservedLegacyRequest>,
    pub restore_records: Vec<PreservedRestoreRecord>,
    pub photo_assets: Vec<PhotoAssetDescriptor>,
}

pub(in crate::store) struct RestoreSourceHeader {
    pub format_version: u32,
    pub schema_version: i64,
    pub sequence: i64,
    pub request_id: Option<String>,
    pub recorded_at: String,
    pub source_kind: RestoreSourceKind,
}

pub(in crate::store) struct DecodedRestore {
    pub header: RestoreSourceHeader,
    pub preserved: PreservedState,
    pub(super) active: SnapshotStateV14,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotV2Schema15 {
    format_version: u32,
    schema_version: i64,
    source_kind: RestoreSourceKind,
    sequence: String,
    request_id: Option<String>,
    recorded_at: String,
    state: SnapshotStateV14,
    preserved: PreservedState,
}

fn invalid(message: impl std::fmt::Display) -> StoreError {
    StoreError::Validation(format!("복원 자료를 확인할 수 없습니다: {message}"))
}

fn uuid(value: &str) -> Result<(), StoreError> {
    if Uuid::parse_str(value).map_err(invalid)?.to_string() != value {
        return Err(invalid("UUID가 정규 형식이 아닙니다"));
    }
    Ok(())
}

fn sequence(value: &str, allow_zero: bool) -> Result<i64, StoreError> {
    let n = value.parse::<i64>().map_err(invalid)?;
    if n < i64::from(!allow_zero) || n.to_string() != value {
        return Err(invalid("순번이 정규 정수가 아닙니다"));
    }
    Ok(n)
}

fn time(value: &str) -> Result<(), StoreError> {
    chrono::DateTime::parse_from_rfc3339(value).map_err(invalid)?;
    Ok(())
}

fn strict_json(bytes: &[u8]) -> Result<(), StoreError> {
    crate::restore_files::reject_duplicate_json_keys(bytes).map_err(invalid)
}

fn closed_source<T: serde::de::DeserializeOwned + Serialize>(
    raw: &str,
    value: &serde_json::Value,
) -> Result<T, StoreError> {
    let decoded: T = serde_json::from_str(raw).map_err(invalid)?;
    if !historical_fields_match(value, &serde_json::to_value(&decoded).map_err(invalid)?) {
        return Err(invalid("원문에 알 수 없거나 누락된 필드가 있습니다"));
    }
    Ok(decoded)
}

// Old frozen shapes share a few optional DTO fields that were added later.
// Their absent null values are a compatibility view, never invented facts.
// Every original key and value must survive; unknown input is still rejected.
fn historical_fields_match(original: &serde_json::Value, decoded: &serde_json::Value) -> bool {
    use serde_json::Value;
    match (original, decoded) {
        (Value::Object(before), Value::Object(after)) => {
            before.iter().all(|(key, value)| {
                after
                    .get(key)
                    .is_some_and(|decoded| historical_fields_match(value, decoded))
            }) && after
                .iter()
                .all(|(key, value)| before.contains_key(key) || value.is_null())
        }
        (Value::Array(before), Value::Array(after)) => {
            before.len() == after.len()
                && before
                    .iter()
                    .zip(after)
                    .all(|(a, b)| historical_fields_match(a, b))
        }
        _ => original == decoded,
    }
}

pub(in crate::store) fn decode_restore_source(bytes: &[u8]) -> Result<DecodedRestore, StoreError> {
    strict_json(bytes)?;
    let raw = std::str::from_utf8(bytes).map_err(invalid)?;
    let value: serde_json::Value = serde_json::from_str(raw).map_err(invalid)?;
    let format = value["format_version"]
        .as_u64()
        .ok_or_else(|| invalid("형식 버전 없음"))?;
    let schema = value["schema_version"]
        .as_i64()
        .ok_or_else(|| invalid("스키마 버전 없음"))?;
    if schema > 15 || !matches!((format, schema), (1, 1..=2) | (2, 3..=15)) {
        return Err(invalid("지원하지 않는 형식 또는 미래 스키마입니다"));
    }
    if schema == 15 {
        let snapshot: SnapshotV2Schema15 = serde_json::from_str(raw).map_err(invalid)?;
        // Shared historical HTTP DTOs may tolerate unknown fields. New restore
        // documents must not silently discard them, including nested metadata.
        if serde_json::to_value(&snapshot).map_err(invalid)? != value {
            return Err(invalid("스키마15에 알 수 없거나 누락된 필드가 있습니다"));
        }
        return decode_v15(snapshot);
    }
    let seq = if format == 1 {
        value["sequence"]
            .as_i64()
            .filter(|n| *n > 0)
            .ok_or_else(|| invalid("순번 오류"))?
    } else {
        sequence(
            value["sequence"]
                .as_str()
                .ok_or_else(|| invalid("순번 오류"))?,
            false,
        )?
    };
    validate_source(raw, seq, schema).map_err(invalid)?;
    let request_id = value["request_id"]
        .as_str()
        .ok_or_else(|| invalid("요청 ID 없음"))?
        .to_owned();
    uuid(&request_id)?;
    let recorded_at = value["recorded_at"]
        .as_str()
        .ok_or_else(|| invalid("기록 시각 없음"))?
        .to_owned();
    time(&recorded_at)?;
    let active = match schema {
        1 | 2 => expand_v1(closed_source(raw, &value)?, bytes)?,
        3 => expand_state(
            serde_json::to_value(closed_source::<SnapshotV2Schema3>(raw, &value)?.state)
                .map_err(invalid)?,
        )?,
        4 => expand_state(
            serde_json::to_value(closed_source::<SnapshotV2Schema4>(raw, &value)?.state)
                .map_err(invalid)?,
        )?,
        5 => expand_state(
            serde_json::to_value(closed_source::<SnapshotV2Schema5>(raw, &value)?.state)
                .map_err(invalid)?,
        )?,
        6 => expand_state(
            serde_json::to_value(closed_source::<frozen6::Snapshot>(raw, &value)?.into_state()?)
                .map_err(invalid)?,
        )?,
        7 => expand_state(
            serde_json::to_value(closed_source::<frozen7::Snapshot>(raw, &value)?.into_state()?)
                .map_err(invalid)?,
        )?,
        8 => closed_source::<frozen8::Snapshot>(raw, &value)?.into_state()?,
        9 => closed_source::<frozen9::Snapshot>(raw, &value)?.into_state()?,
        10 => closed_source::<frozen10::Snapshot>(raw, &value)?.into_state()?,
        11 => closed_source::<frozen11::Snapshot>(raw, &value)?.into_state()?,
        12 => closed_source::<frozen12::Snapshot>(raw, &value)?.into_state()?,
        13 => closed_source::<frozen13::Snapshot>(raw, &value)?.into_state()?,
        14 => closed_source::<SnapshotV2Schema14>(raw, &value)?.state,
        _ => return Err(invalid("지원하지 않는 자료입니다")),
    };
    validate_active(&active, seq)?;
    let preserved = preservation_from_active(&active)?;
    validate_preserved(&preserved, &active, seq)?;
    Ok(DecodedRestore {
        header: RestoreSourceHeader {
            format_version: format as u32,
            schema_version: schema,
            sequence: seq,
            request_id: Some(request_id),
            recorded_at,
            source_kind: RestoreSourceKind::Journal,
        },
        active,
        preserved,
    })
}

fn decode_v15(snapshot: SnapshotV2Schema15) -> Result<DecodedRestore, StoreError> {
    if snapshot.format_version != 2 || snapshot.schema_version != 15 {
        return Err(invalid("스키마15 헤더 오류"));
    }
    let capture = matches!(snapshot.source_kind, RestoreSourceKind::BeforeRestore);
    let seq = sequence(&snapshot.sequence, capture)?;
    match (&snapshot.request_id, capture) {
        (Some(id), false) => {
            uuid(id)?;
            if !snapshot.state.mutation_requests.iter().any(|request| {
                request.request_id == *id
                    && request.sequence == snapshot.sequence
                    && request.epoch == snapshot.state.catalog_state.epoch
            }) {
                return Err(invalid("변경 기록 헤더의 원요청 근거가 없습니다"));
            }
        }
        (None, true) => (),
        _ => {
            return Err(invalid(
                "변경 기록과 직전 상태 보존의 요청 ID가 맞지 않습니다",
            ));
        }
    }
    time(&snapshot.recorded_at)?;
    validate_active(&snapshot.state, seq)?;
    validate_preserved(&snapshot.preserved, &snapshot.state, seq)?;
    Ok(DecodedRestore {
        header: RestoreSourceHeader {
            format_version: 2,
            schema_version: 15,
            sequence: seq,
            request_id: snapshot.request_id,
            recorded_at: snapshot.recorded_at,
            source_kind: snapshot.source_kind,
        },
        active: snapshot.state,
        preserved: snapshot.preserved,
    })
}

fn derived_v1_id(source_hash: &[u8], purpose: &str, material_id: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(b"sunchang-hansang.restore.v1\0");
    hash.update(source_hash);
    hash.update(purpose.as_bytes());
    hash.update([0]);
    hash.update(material_id.as_bytes());
    let digest = hash.finalize();
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    // UUIDv8 identifies derived migration metadata, not a historical event.
    bytes[6] = (bytes[6] & 0x0f) | 0x80;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes).to_string()
}

fn expand_v1(old: super::super::Snapshot, raw: &[u8]) -> Result<SnapshotStateV14, StoreError> {
    let source_hash = Sha256::digest(raw);
    let mut versions = Vec::with_capacity(old.materials.len());
    let mut heads = Vec::with_capacity(old.materials.len());
    for material in &old.materials {
        let unit = crate::domain::Unit::parse(&material.unit).map_err(invalid)?;
        let id = derived_v1_id(&source_hash, "material-version", &material.id);
        heads.push(Head {
            material_id: material.id.clone(),
            current_version_id: id.clone(),
        });
        versions.push(MaterialVersion {
            id,
            material_id: material.id.clone(),
            name: material.name.clone(),
            unit: material.unit.clone(),
            canonical_unit: unit.canonical().into(),
            dimension: unit.dimension().into(),
            kind: "unclassified".into(),
            description: String::new(),
            archived: false,
            recorded_at: material.created_at.clone(),
            seq: None,
            source: "legacy_migration".into(),
        });
    }
    expand_state(serde_json::json!({
        "materials": old.materials, "material_versions": versions, "material_heads": heads,
        "material_prices": [], "catalog_state": {"epoch": derived_v1_id(&source_hash, "source-epoch", ""), "revision": old.sequence.to_string()},
        "creation_requests": [], "mutation_requests": []
    }))
}

fn expand_state(mut value: serde_json::Value) -> Result<SnapshotStateV14, StoreError> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| invalid("전체 상태가 객체가 아닙니다"))?;
    for field in ACTIVE_COLLECTIONS {
        object
            .entry(*field)
            .or_insert_with(|| serde_json::json!([]));
    }
    serde_json::from_value(value).map_err(invalid)
}

fn validate_active(state: &SnapshotStateV14, seq: i64) -> Result<(), StoreError> {
    uuid(&state.catalog_state.epoch)?;
    if sequence(&state.catalog_state.revision, true)? != seq {
        return Err(invalid("현재 상태 개정과 원문 순번이 다릅니다"));
    }
    let mut materials = HashSet::new();
    for material in &state.materials {
        uuid(&material.id)?;
        time(&material.created_at)?;
        if !materials.insert(material.id.as_str()) {
            return Err(invalid("중복 품목 ID"));
        }
        super::super::validate_input(super::super::CreateMaterial {
            request_id: material.id.clone(),
            name: material.name.clone(),
            unit: material.unit.clone(),
        })
        .map_err(invalid)?;
    }
    let mut versions = HashSet::new();
    for row in &state.material_versions {
        uuid(&row.id)?;
        uuid(&row.material_id)?;
        time(&row.recorded_at)?;
        if !versions.insert(row.id.as_str()) || !materials.contains(row.material_id.as_str()) {
            return Err(invalid("품목 버전의 중복 또는 참조 오류"));
        }
        if let Some(s) = &row.seq {
            if sequence(s, false)? > seq {
                return Err(invalid("품목 버전 순번 오류"));
            }
        } else if row.source != "legacy_migration" {
            return Err(invalid("품목 버전의 변경 근거가 없습니다"));
        }
        let unit = crate::domain::Unit::parse(&row.unit).map_err(invalid)?;
        if unit.canonical() != row.canonical_unit || unit.dimension() != row.dimension {
            return Err(invalid("품목 단위와 차원이 일치하지 않습니다"));
        }
    }
    let mut heads = HashSet::new();
    for row in &state.material_heads {
        if !heads.insert(row.material_id.as_str())
            || !state
                .material_versions
                .iter()
                .any(|v| v.id == row.current_version_id && v.material_id == row.material_id)
        {
            return Err(invalid("품목의 현재 버전 참조 오류"));
        }
    }
    if materials != heads {
        return Err(invalid("품목의 현재 버전 누락"));
    }
    validate_current_state(state, seq).map_err(invalid)
}

fn preservation_from_active(state: &SnapshotStateV14) -> Result<PreservedState, StoreError> {
    let mut legacy = Vec::with_capacity(state.creation_requests.len());
    for row in &state.creation_requests {
        let material = state
            .materials
            .iter()
            .find(|m| m.id == row.material_id)
            .ok_or_else(|| invalid("등록 요청의 품목이 없습니다"))?;
        let mutation = state
            .mutation_requests
            .iter()
            .find(|r| r.request_id == row.request_id);
        legacy.push(PreservedLegacyRequest {
            request_id: row.request_id.clone(),
            name: row.name.clone(),
            unit: row.unit.clone(),
            material: material.clone(),
            epoch: mutation.map_or_else(|| state.catalog_state.epoch.clone(), |m| m.epoch.clone()),
            sequence: mutation.map(|m| m.sequence.clone()),
        });
    }
    Ok(PreservedState {
        mutation_requests: state.mutation_requests.clone(),
        creation_requests: legacy,
        restore_records: vec![],
        photo_assets: state
            .photo_assets
            .iter()
            .map(|r| r.descriptor.clone())
            .collect(),
    })
}

fn same<T: Serialize>(a: &T, b: &T) -> Result<bool, StoreError> {
    Ok(serde_json::to_value(a).map_err(invalid)? == serde_json::to_value(b).map_err(invalid)?)
}

fn validate_preserved(
    preserved: &PreservedState,
    state: &SnapshotStateV14,
    max: i64,
) -> Result<(), StoreError> {
    let mut requests = BTreeMap::new();
    for row in &preserved.mutation_requests {
        uuid(&row.request_id)?;
        uuid(&row.epoch)?;
        if sequence(&row.sequence, false)? > max
            || requests.insert(row.request_id.as_str(), row).is_some()
        {
            return Err(invalid("보존 요청의 중복 또는 순번 오류"));
        }
        if !matches!(
            row.operation.as_str(),
            "material.create"
                | "material.revise"
                | "material.archive"
                | "material.price"
                | "legacy.material.create"
                | "business.create"
                | "business.revise"
                | "business.confirm"
                | "business.archive"
                | "recipe.create"
                | "recipe.revise"
                | "recipe.copy"
                | "profile.revise"
                | "state.restore"
        ) {
            return Err(invalid("알 수 없는 보존 요청 종류"));
        }
        strict_json(row.payload.as_bytes())?;
        strict_json(row.response.as_bytes())?;
    }
    for row in &state.mutation_requests {
        if !requests
            .get(row.request_id.as_str())
            .map(|old| same(*old, row))
            .transpose()?
            .unwrap_or(false)
        {
            return Err(invalid("활성 요청과 보존 원응답이 다릅니다"));
        }
    }
    let mut legacy = BTreeMap::new();
    for row in &preserved.creation_requests {
        uuid(&row.request_id)?;
        uuid(&row.epoch)?;
        uuid(&row.material.id)?;
        time(&row.material.created_at)?;
        if row.name != row.material.name
            || row.unit != row.material.unit
            || row
                .sequence
                .as_deref()
                .map(|s| sequence(s, false))
                .transpose()?
                .is_some_and(|n| n > max)
            || legacy.insert(row.request_id.as_str(), row).is_some()
        {
            return Err(invalid("보존 등록 요청 오류"));
        }
        if let Some(request) = requests.get(row.request_id.as_str()) {
            if request.operation != "legacy.material.create"
                || request.epoch != row.epoch
                || row.sequence.as_ref() != Some(&request.sequence)
            {
                return Err(invalid("등록 요청의 처리 근거가 다릅니다"));
            }
            let input: crate::store::CreateMaterial =
                serde_json::from_str(&request.payload).map_err(invalid)?;
            let response: crate::catalog::CatalogMutation<Material> =
                serde_json::from_str(&request.response).map_err(invalid)?;
            if input.request_id != row.request_id
                || input.name != row.name
                || input.unit != row.unit
                || !same(&response.value, &row.material)?
                || response.state.epoch != row.epoch
                || response.state.revision != request.sequence
            {
                return Err(invalid("등록 요청과 원처리 결과가 다릅니다"));
            }
        }
    }
    for row in &state.creation_requests {
        if !legacy.get(row.request_id.as_str()).is_some_and(|old| {
            old.name == row.name && old.unit == row.unit && old.material.id == row.material_id
        }) {
            return Err(invalid("활성 등록 요청의 보존 근거가 없습니다"));
        }
    }
    let mut assets = BTreeMap::new();
    for descriptor in &preserved.photo_assets {
        crate::photo_assets::validate_asset_descriptor(descriptor).map_err(invalid)?;
        if assets
            .insert(descriptor.sha256.as_str(), descriptor)
            .is_some()
        {
            return Err(invalid("중복 보존 사진"));
        }
    }
    for row in &state.photo_assets {
        if !assets
            .get(row.descriptor.sha256.as_str())
            .map(|old| same(*old, &row.descriptor))
            .transpose()?
            .unwrap_or(false)
        {
            return Err(invalid("활성 사진과 보존 자산이 다릅니다"));
        }
    }
    let mut restores = HashSet::new();
    let mut restore_requests = HashSet::new();
    for row in &preserved.restore_records {
        uuid(&row.request_id)?;
        uuid(&row.previous_epoch)?;
        uuid(&row.result.restore_id)?;
        uuid(&row.result.state.epoch)?;
        time(&row.result.recorded_at)?;
        let seq = sequence(&row.result.state.revision, false)?;
        let request = requests
            .get(row.request_id.as_str())
            .ok_or_else(|| invalid("복원 요청의 처리 근거 없음"))?;
        validate_target(&row.result.target, seq)?;
        validate_target(&row.result.previous_target, seq)?;
        let input: crate::restore::ConfirmRestore =
            serde_json::from_str(&request.payload).map_err(invalid)?;
        if seq > max
            || !restores.insert(row.result.restore_id.as_str())
            || !restore_requests.insert(row.request_id.as_str())
            || row.previous_epoch == row.result.state.epoch
            || input.meta.request_id != row.request_id
            || input.meta.epoch != row.previous_epoch
            || input.target_id != row.result.target.target_id
            || input.expected_sha256 != row.result.target.sha256
            || input.meta.expected_revision != row.result.previous_target.sequence
            || request.operation != "state.restore"
            || request.epoch != row.result.state.epoch
            || request.sequence != row.result.state.revision
            || !same(
                &serde_json::from_str::<RestoreResult>(&request.response).map_err(invalid)?,
                &row.result,
            )?
        {
            return Err(invalid("복원 기록과 원응답이 일치하지 않습니다"));
        }
    }
    if requests.values().any(|r| {
        r.operation == "state.restore" && !restore_requests.contains(r.request_id.as_str())
    }) {
        return Err(invalid("보존 복원 요청의 결과 근거가 없습니다"));
    }
    Ok(())
}

fn validate_target(target: &crate::restore::RestoreTarget, upper: i64) -> Result<(), StoreError> {
    use crate::restore::{RestoreTargetId, RestoreTargetSource, validate_restore_sha256};
    validate_restore_sha256(&target.sha256).map_err(invalid)?;
    let id = RestoreTargetId::parse(&target.target_id).map_err(invalid)?;
    let capture = target.source_kind == RestoreSourceKind::BeforeRestore;
    let seq = sequence(&target.sequence, capture)?;
    if seq >= upper {
        return Err(invalid("복원 근거가 새 기록보다 앞서지 않습니다"));
    }
    let bytes = target.byte_length.parse::<u64>().map_err(invalid)?;
    if bytes == 0 || bytes.to_string() != target.byte_length {
        return Err(invalid("복원 근거 길이 오류"));
    }
    match (id, target.source) {
        (RestoreTargetId::Journal(value), RestoreTargetSource::Journal)
            if value == seq && !capture => {}
        (RestoreTargetId::Import(hash), RestoreTargetSource::Import) if hash == target.sha256 => (),
        _ => return Err(invalid("복원 근거 대상이 일치하지 않습니다")),
    }
    if !matches!(
        (target.format_version, target.schema_version),
        (1, Some(1..=2)) | (2, Some(3..=15))
    ) || (capture && target.schema_version != Some(15))
    {
        return Err(invalid("복원 근거 형식 오류"));
    }
    match (&target.request_id, capture) {
        (Some(id), false) => uuid(id)?,
        (None, true) => (),
        _ => return Err(invalid("복원 근거 요청 ID 오류")),
    }
    time(&target.recorded_at)
}

pub(in crate::store) fn encode_journal(
    connection: &Connection,
    seq: i64,
    request_id: &str,
    recorded_at: &str,
) -> Result<String, StoreError> {
    encode_state(
        connection,
        seq,
        Some(request_id.to_owned()),
        recorded_at,
        RestoreSourceKind::Journal,
    )
}

pub(in crate::store) fn encode_before_restore(
    connection: &Connection,
    recorded_at: &str,
) -> Result<String, StoreError> {
    let current = catalog::state(connection)?;
    encode_state(
        connection,
        sequence(&current.revision, true)?,
        None,
        recorded_at,
        RestoreSourceKind::BeforeRestore,
    )
}

fn encode_state(
    connection: &Connection,
    seq: i64,
    request_id: Option<String>,
    recorded_at: &str,
    source_kind: RestoreSourceKind,
) -> Result<String, StoreError> {
    let snapshot = SnapshotV2Schema15 {
        format_version: 2,
        schema_version: 15,
        source_kind,
        sequence: seq.to_string(),
        request_id,
        recorded_at: recorded_at.to_owned(),
        state: read_current_state(connection)?,
        preserved: super::super::restore::read_preserved_state(connection)?,
    };
    Ok(serde_json::to_string_pretty(&snapshot).map_err(unavailable)? + "\n")
}

pub(in crate::store) fn validate_v15(payload: &str, seq: i64) -> Result<(), StoreError> {
    let decoded = decode_restore_source(payload.as_bytes())?;
    if decoded.header.sequence != seq
        || !matches!(decoded.header.source_kind, RestoreSourceKind::Journal)
    {
        return Err(invalid("변경 기록의 종류 또는 순번 오류"));
    }
    Ok(())
}

impl DecodedRestore {
    pub(in crate::store) fn summary(&self) -> Result<RestoreSummary, StoreError> {
        summary(&self.active)
    }
    pub(in crate::store) fn required_assets(&self) -> Vec<PhotoAssetDescriptor> {
        self.preserved.photo_assets.clone()
    }
    pub(in crate::store) fn max_sequence(&self) -> i64 {
        self.header.sequence
    }
}

pub(in crate::store) fn current_restore_summary(
    connection: &Connection,
) -> Result<RestoreSummary, StoreError> {
    summary(&read_current_state(connection)?)
}

pub(in crate::store) fn validate_current_preserved(
    connection: &Connection,
) -> Result<(), StoreError> {
    let active = read_current_state(connection)?;
    let max = sequence(&active.catalog_state.revision, true).map_err(unavailable)?;
    let preserved = super::super::restore::read_preserved_state(connection)?;
    validate_preserved(&preserved, &active, max).map_err(unavailable)
}

fn summary(state: &SnapshotStateV14) -> Result<RestoreSummary, StoreError> {
    use RestoreArea::*;
    let counts = [
        (
            Materials,
            state.materials.len(),
            state.material_versions.len(),
        ),
        (
            Recipes,
            state.recipe_heads.len(),
            state.recipe_versions.len(),
        ),
        (
            Business,
            state.business_records.len(),
            state.business_revisions.len(),
        ),
        (
            Orders,
            state.purchase_orders.len(),
            state.purchase_order_versions.len(),
        ),
        (Receipts, state.receipts.len(), state.receipts.len()),
        (
            Productions,
            state.productions.len(),
            state.productions.len(),
        ),
        (Sales, state.sales.len(), state.sale_versions.len()),
        (
            Stocktakes,
            state.stocktake_observations.len(),
            state.stocktake_versions.len(),
        ),
        (
            Allocations,
            state.consumption_heads.len(),
            state.consumption_versions.len(),
        ),
        (Wastes, state.wastes.len(), state.wastes.len()),
        (
            PurchaseReturns,
            state.purchase_returns.len(),
            state.purchase_returns.len(),
        ),
        (
            Reconciliations,
            state.receipt_reconciliations.len(),
            state.inclusion_link_versions.len(),
        ),
        (
            Corrections,
            state.correction_records.len(),
            state.correction_event_versions.len(),
        ),
        (
            Profiles,
            state.material_profile_heads.len(),
            state.material_profile_versions.len(),
        ),
        (Photos, state.photo_assets.len(), state.photo_assets.len()),
    ];
    let drafts = state
        .business_heads
        .iter()
        .filter(|head| {
            state.business_revisions.iter().any(|v| {
                v.id == head.current_revision_id
                    && v.status == crate::business::BusinessStatus::Draft
            })
        })
        .count();
    let photo_bytes = state.photo_assets.iter().try_fold(0_u64, |sum, row| {
        sum.checked_add(row.descriptor.byte_length)
            .ok_or_else(|| invalid("사진 길이 합계 초과"))
    })?;
    Ok(RestoreSummary {
        areas: counts
            .into_iter()
            .map(|(area, records, versions)| RestoreAreaCount {
                area,
                records,
                versions,
            })
            .collect(),
        drafts,
        photo_bytes: photo_bytes.to_string(),
    })
}

pub(in crate::store) fn verify_restored_active(
    connection: &Connection,
    decoded: &DecodedRestore,
) -> Result<(), StoreError> {
    let actual = read_current_state(connection)?;
    validate_active(&actual, decoded.header.sequence)?;
    let canonical = |state: &SnapshotStateV14| -> Result<serde_json::Value, StoreError> {
        let mut value = serde_json::to_value(state).map_err(unavailable)?;
        for field in ACTIVE_COLLECTIONS {
            if let Some(rows) = value[*field].as_array_mut() {
                rows.sort_by_key(|v| v.to_string());
            }
        }
        Ok(value)
    };
    if canonical(&actual)? != canonical(&decoded.active)? {
        return Err(invalid("복원한 전체 상태가 선택 자료와 다릅니다"));
    }
    Ok(())
}

const ACTIVE_COLLECTIONS: &[&str] = &[
    "materials",
    "material_versions",
    "material_heads",
    "material_prices",
    "creation_requests",
    "mutation_requests",
    "business_records",
    "business_revisions",
    "business_heads",
    "business_confirmations",
    "business_catalog_price_results",
    "business_corrections",
    "purchase_orders",
    "purchase_order_versions",
    "purchase_order_heads",
    "receipts",
    "holding_versions",
    "holding_heads",
    "order_receipt_closures",
    "recipe_versions",
    "recipe_lines",
    "recipe_heads",
    "productions",
    "production_inputs",
    "production_outputs",
    "production_effects",
    "sales",
    "sale_versions",
    "sale_heads",
    "sale_cost_rows",
    "stocktake_batches",
    "stocktake_observations",
    "stocktake_versions",
    "stocktake_heads",
    "consumption_versions",
    "consumption_heads",
    "consumption_rows",
    "wastes",
    "receipt_reconciliations",
    "inclusion_links",
    "inclusion_link_versions",
    "inclusion_link_heads",
    "inclusion_reservations",
    "reconciliation_material_effects",
    "correction_records",
    "correction_event_versions",
    "correction_event_heads",
    "correction_projection_versions",
    "correction_projection_heads",
    "correction_price_bases",
    "purchase_returns",
    "photo_assets",
    "material_profile_versions",
    "material_profile_heads",
    "material_profile_photos",
    "material_profile_suppliers",
];
