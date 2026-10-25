use super::*;
use crate::{
    business::BusinessMeta,
    restore_files::{self, RestoreFileError, RestoreUpload, VerifiedRestoreBytes},
};
use chrono::{SecondsFormat, Utc};
use std::path::Path;
use uuid::Uuid;

fn file_error(error: RestoreFileError) -> StoreError {
    match error {
        RestoreFileError::Validation(m) => invalid(m),
        RestoreFileError::Unavailable(m) => unavailable(m),
    }
}
fn target(
    verified: &VerifiedRestoreBytes,
    decoded: &snapshot::DecodedRestore,
    source: RestoreTargetSource,
) -> Result<RestoreTarget, StoreError> {
    Ok(RestoreTarget {
        target_id: match source {
            RestoreTargetSource::Journal => format!("journal:{}", decoded.header.sequence),
            RestoreTargetSource::Import => format!("import:{}", verified.sha256()),
        },
        source,
        source_kind: decoded.header.source_kind,
        sha256: verified.sha256().into(),
        byte_length: verified.byte_length().to_string(),
        format_version: decoded.header.format_version,
        schema_version: Some(u32::try_from(decoded.header.schema_version).map_err(invalid)?),
        sequence: decoded.header.sequence.to_string(),
        request_id: decoded.header.request_id.clone(),
        recorded_at: decoded.header.recorded_at.clone(),
    })
}
fn register(c: &Connection, value: &RestoreTarget) -> Result<(), StoreError> {
    let old: Option<String> = c
        .query_row(
            "SELECT target_json FROM restore_imports WHERE sha256=?1",
            [&value.sha256],
            |r| r.get(0),
        )
        .optional()
        .map_err(unavailable)?;
    let raw = json(value)?;
    if let Some(old) = old {
        if decode::<RestoreTarget>(&old)? != *value {
            return Err(unavailable("등록된 복원 지점 정보가 다릅니다"));
        }
    } else {
        c.execute(
            "INSERT INTO restore_imports(sha256,byte_length,target_json) VALUES(?1,?2,?3)",
            params![
                value.sha256,
                value.byte_length.parse::<i64>().map_err(invalid)?,
                raw
            ],
        )
        .map_err(unavailable)?;
    }
    Ok(())
}
fn load(
    c: &Connection,
    root: &Path,
    id: &RestoreTargetId,
) -> Result<(RestoreTarget, snapshot::DecodedRestore), StoreError> {
    match id {
        RestoreTargetId::Journal(sequence) => {
            let raw: Option<String> = c
                .query_row(
                    "SELECT payload FROM snapshot_outbox WHERE sequence=?1",
                    [sequence],
                    |r| r.get(0),
                )
                .optional()
                .map_err(unavailable)?;
            let verified = restore_files::verify_restore_bytes(
                raw.ok_or_else(|| invalid("복원 지점을 찾을 수 없습니다"))?
                    .into_bytes(),
            )
            .map_err(file_error)?;
            let bytes = restore_files::read_restore_journal(
                root,
                *sequence,
                verified.sha256(),
                verified.byte_length(),
            )
            .map_err(file_error)?;
            let decoded = snapshot::decode_restore_source(bytes.bytes())?;
            if decoded.header.sequence != *sequence
                || decoded.header.source_kind != RestoreSourceKind::Journal
            {
                return Err(unavailable("변경 기록의 원문과 순번이 다릅니다"));
            }
            Ok((
                target(&bytes, &decoded, RestoreTargetSource::Journal)?,
                decoded,
            ))
        }
        RestoreTargetId::Import(sha) => {
            let row: Option<(i64, String)> = c
                .query_row(
                    "SELECT byte_length,target_json FROM restore_imports WHERE sha256=?1",
                    [sha],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(unavailable)?;
            let (length, raw) =
                row.ok_or_else(|| invalid("등록된 복원 자료를 찾을 수 없습니다"))?;
            let bytes = restore_files::read_restore_import(
                root,
                sha,
                u64::try_from(length).map_err(unavailable)?,
            )
            .map_err(file_error)?;
            let decoded = snapshot::decode_restore_source(bytes.bytes())?;
            let actual = target(&bytes, &decoded, RestoreTargetSource::Import)?;
            if actual != decode::<RestoreTarget>(&raw)? {
                return Err(unavailable("등록된 복원 자료 정보와 원문이 다릅니다"));
            }
            Ok((actual, decoded))
        }
    }
}
fn rollback_preflight<T>(
    c: &Connection,
    work: impl FnOnce() -> Result<T, StoreError>,
) -> Result<T, StoreError> {
    let owns_transaction = c.is_autocommit();
    c.execute_batch("SAVEPOINT restore_preflight")
        .map_err(unavailable)?;
    let result = work();
    // Releasing an outer savepoint after ROLLBACK TO still commits an empty
    // write transaction and changes SQLite's header. Roll back the whole
    // transaction we own, preserving the original file bytes as well as rows.
    let rollback = if owns_transaction {
        "ROLLBACK"
    } else {
        "ROLLBACK TO restore_preflight; RELEASE restore_preflight"
    };
    c.execute_batch(rollback).map_err(unavailable)?;
    result
}
fn preflight(c: &Connection, decoded: &snapshot::DecodedRestore) -> Result<(), StoreError> {
    rollback_preflight(c, || {
        snapshot::replace_restore_state(c, decoded)?;
        let bad: Option<String> = c
            .query_row("PRAGMA foreign_key_check", [], |r| r.get(0))
            .optional()
            .map_err(unavailable)?;
        if bad.is_some() {
            return Err(invalid("복원 자료의 참조가 올바르지 않습니다"));
        }
        snapshot::verify_restored_active(c, decoded)
    })
}
fn preserved_conflict(
    c: &Connection,
    decoded: &snapshot::DecodedRestore,
) -> Result<bool, StoreError> {
    match rollback_preflight(c, || merge_preserved(c, &decoded.preserved)) {
        Ok(()) => Ok(false),
        Err(StoreError::Conflict(_)) => Ok(true),
        Err(e) => Err(e),
    }
}
fn photos(root: &Path, assets: &[PhotoAssetDescriptor]) -> RestorePhotoCheck {
    let mut result = RestorePhotoCheck {
        required: assets.len(),
        ..Default::default()
    };
    for asset in assets {
        let checked = crate::photo_assets::check_photo(root, asset);
        let problem = match checked {
            Ok(()) => {
                result.verified += 1;
                continue;
            }
            Err(crate::photo_assets::PhotoCheckError::Missing) => {
                result.missing += 1;
                RestorePhotoProblemKind::Missing
            }
            Err(crate::photo_assets::PhotoCheckError::UnsafePath) => {
                result.unsafe_path += 1;
                RestorePhotoProblemKind::UnsafePath
            }
            Err(crate::photo_assets::PhotoCheckError::Corrupt) => {
                result.corrupt += 1;
                RestorePhotoProblemKind::Corrupt
            }
        };
        if result.examples.len() < RESTORE_PHOTO_EXAMPLE_LIMIT {
            result.examples.push(RestorePhotoProblem {
                sha256: asset.sha256.clone(),
                problem,
            });
        } else {
            result.examples_truncated = true;
        }
    }
    result
}
fn check_preview(
    c: &Connection,
    root: &Path,
    target: RestoreTarget,
    decoded: &snapshot::DecodedRestore,
) -> Result<RestorePreview, StoreError> {
    preflight(c, decoded)?;
    let mut all_assets = std::collections::BTreeMap::new();
    for asset in preserved_assets(c)?
        .into_iter()
        .chain(decoded.required_assets())
    {
        if let Some(old) = all_assets.insert(asset.sha256.clone(), asset.clone())
            && old != asset
        {
            return Err(conflict());
        }
    }
    let photos = photos(root, &all_assets.into_values().collect::<Vec<_>>());
    let mut blockers = Vec::new();
    if photos.missing > 0 {
        blockers.push(RestoreBlocker {
            code: RestoreBlockerCode::MissingAssets,
            count: photos.missing,
            message: "사진 원본이 없는 복원 자료입니다".into(),
        });
    }
    if photos.corrupt + photos.unsafe_path > 0 {
        blockers.push(RestoreBlocker {
            code: RestoreBlockerCode::InvalidAssets,
            count: photos.corrupt + photos.unsafe_path,
            message: "사진 원본 또는 경로를 확인할 수 없습니다".into(),
        });
    }
    if preserved_conflict(c, decoded)? {
        blockers.push(RestoreBlocker {
            code: RestoreBlockerCode::RequestConflict,
            count: 1,
            message: "기존 요청 ID와 다른 처리 근거가 있어 복원할 수 없습니다".into(),
        });
    }
    let current_max: i64 = c
        .query_row(
            "SELECT COALESCE(MAX(sequence),0) FROM state_sequences",
            [],
            |r| r.get(0),
        )
        .map_err(unavailable)?;
    if current_max.max(decoded.max_sequence()) == i64::MAX {
        blockers.push(RestoreBlocker {
            code: RestoreBlockerCode::SequenceExhausted,
            count: 1,
            message: "새 변경 기록 순번을 만들 수 없습니다".into(),
        });
    }
    Ok(RestorePreview {
        target,
        state: catalog::state(c)?,
        before: snapshot::current_restore_summary(c)?,
        after: decoded.summary()?,
        photos,
        can_restore: blockers.is_empty(),
        blockers,
    })
}

impl Store {
    pub fn restore_targets(
        &self,
        query: RestoreTargetQuery,
    ) -> Result<RestoreTargetPage, StoreError> {
        query.validate().map_err(invalid)?;
        let mut rows = Vec::new();
        match query.source {
            RestoreTargetSource::Journal => {
                let cursor = query
                    .cursor
                    .as_deref()
                    .map(str::parse::<i64>)
                    .transpose()
                    .map_err(invalid)?;
                let ids=self.connection.prepare("SELECT sequence FROM snapshot_outbox WHERE (?1 IS NULL OR sequence<?1) ORDER BY sequence DESC LIMIT 26").map_err(unavailable)?
                    .query_map([cursor],|r|r.get::<_,i64>(0)).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
                for id in ids {
                    rows.push(load(&self.connection, &self.root, &RestoreTargetId::Journal(id))?.0);
                }
            }
            RestoreTargetSource::Import => {
                let ids=self.connection.prepare("SELECT sha256 FROM restore_imports WHERE (?1 IS NULL OR sha256>?1) ORDER BY sha256 LIMIT 26").map_err(unavailable)?
                    .query_map([query.cursor.as_deref()],|r|r.get::<_,String>(0)).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
                for id in ids {
                    rows.push(load(&self.connection, &self.root, &RestoreTargetId::Import(id))?.0);
                }
            }
        }
        let more = rows.len() > RESTORE_TARGET_PAGE_SIZE;
        rows.truncate(RESTORE_TARGET_PAGE_SIZE);
        let next_cursor = more
            .then(|| {
                rows.last().map(|r| match query.source {
                    RestoreTargetSource::Journal => r.sequence.clone(),
                    RestoreTargetSource::Import => r.sha256.clone(),
                })
            })
            .flatten();
        Ok(RestoreTargetPage {
            state: catalog::state(&self.connection)?,
            source: query.source,
            rows,
            next_cursor,
        })
    }
    pub fn restore_import(&mut self, upload: RestoreUpload) -> Result<RestoreTarget, StoreError> {
        self.recover()?;
        let bytes = restore_files::verify_restore_upload(upload).map_err(file_error)?;
        let decoded = snapshot::decode_restore_source(bytes.bytes())?;
        preflight(&self.connection, &decoded)?;
        let target = target(&bytes, &decoded, RestoreTargetSource::Import)?;
        restore_files::publish_restore_import(&self.root, &bytes).map_err(file_error)?;
        register(&self.connection, &target)?;
        Ok(target)
    }
    pub fn restore_preview(&mut self, input: PreviewRestore) -> Result<RestorePreview, StoreError> {
        let id = input.validate().map_err(invalid)?;
        self.recover()?;
        let (target, decoded) = load(&self.connection, &self.root, &id)?;
        if input.expected_sha256 != target.sha256 {
            return Err(invalid("선택한 자료의 원문이 다릅니다"));
        }
        check_preview(&self.connection, &self.root, target, &decoded)
    }
    pub fn restore(&mut self, mut input: ConfirmRestore) -> Result<RestoreResult, StoreError> {
        let id = input.target().map_err(invalid)?;
        let mut meta: crate::catalog::MutationMeta = input.meta.clone().into();
        catalog::normalize_meta(&mut meta)?;
        input.meta = BusinessMeta {
            request_id: meta.request_id.clone(),
            epoch: meta.epoch.clone(),
            expected_revision: meta.expected_revision.clone(),
        };
        let payload = json(&input)?;
        if let Some(old) = preserved_request(&self.connection, &meta.request_id)? {
            if old.operation != "state.restore" || old.payload != payload {
                return Err(conflict());
            }
            self.recover()?;
            let result: RestoreResult = decode(&old.response)?;
            let state = catalog::state(&self.connection)?;
            if result.state.epoch != state.epoch {
                return Err(generation(state, true));
            }
            return Ok(result);
        }
        if preserved_legacy(&self.connection, &meta.request_id)?.is_some() {
            return Err(conflict());
        }
        self.recover()?;
        let current = catalog::state(&self.connection)?;
        if current.epoch != meta.epoch {
            return Err(generation(current, false));
        }
        if current.revision != meta.expected_revision {
            return Err(StoreError::StaleState(
                "자료가 변경되었습니다. 다시 미리보기를 확인하세요".into(),
            ));
        }
        let (target, decoded) = load(&self.connection, &self.root, &id)?;
        if target.sha256 != input.expected_sha256 {
            return Err(invalid("선택한 자료의 원문이 다릅니다"));
        }
        // The decoder requires every active request to have matching preserved
        // evidence. Check imported identities before publishing a before-capture;
        // a new restore cannot reuse an original or legacy request's UUID.
        if decoded
            .preserved
            .mutation_requests
            .iter()
            .any(|row| row.request_id == meta.request_id)
            || decoded
                .preserved
                .creation_requests
                .iter()
                .any(|row| row.request_id == meta.request_id)
        {
            return Err(conflict());
        }
        let preview = check_preview(&self.connection, &self.root, target.clone(), &decoded)?;
        if !preview.can_restore {
            if preview
                .blockers
                .iter()
                .any(|b| b.code == RestoreBlockerCode::RequestConflict)
            {
                return Err(conflict());
            }
            return Err(invalid(
                preview
                    .blockers
                    .iter()
                    .map(|b| b.message.as_str())
                    .collect::<Vec<_>>()
                    .join("; "),
            ));
        }
        let time = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
        let before = snapshot::encode_before_restore(&self.connection, &time)?;
        let before =
            restore_files::verify_restore_bytes(before.into_bytes()).map_err(file_error)?;
        let before_decoded = snapshot::decode_restore_source(before.bytes())?;
        let previous_target = target_from_before(&before, &before_decoded)?;
        restore_files::publish_restore_import(&self.root, &before).map_err(file_error)?;
        // Registration and active replacement commit together. A failed attempt
        // may leave an unregistered immutable file, never an applied restore.
        let tx = self.connection.transaction().map_err(unavailable)?;
        if catalog::state(&tx)? != current {
            return Err(StoreError::StaleState("자료가 변경되었습니다".into()));
        }
        if decoded.max_sequence() > 0 {
            tx.execute(
                "INSERT OR IGNORE INTO state_sequences(sequence) VALUES(?1)",
                [decoded.max_sequence()],
            )
            .map_err(unavailable)?;
        }
        tx.execute("INSERT INTO state_sequences DEFAULT VALUES", [])
            .map_err(unavailable)?;
        let sequence = tx.last_insert_rowid();
        tx.execute(
            "INSERT INTO snapshot_outbox(sequence,payload,published) VALUES(?1,'',0)",
            [sequence],
        )
        .map_err(unavailable)?;
        merge_preserved(&tx, &decoded.preserved)?;
        snapshot::replace_restore_state(&tx, &decoded)?;
        snapshot::verify_restored_active(&tx, &decoded)?;
        let state = CatalogState {
            epoch: Uuid::new_v4().to_string(),
            revision: sequence.to_string(),
        };
        tx.execute(
            "UPDATE catalog_state SET epoch=?1,revision=?2 WHERE singleton=1",
            params![state.epoch, sequence],
        )
        .map_err(unavailable)?;
        let result = RestoreResult {
            restore_id: Uuid::new_v4().to_string(),
            target,
            previous_target: previous_target.clone(),
            recorded_at: time.clone(),
            state: state.clone(),
        };
        let response = json(&result)?;
        tx.execute("INSERT INTO mutation_requests(request_id,operation,payload,response,epoch,sequence) VALUES(?1,'state.restore',?2,?3,?4,?5)",params![meta.request_id,payload,response,state.epoch,sequence]).map_err(unavailable)?;
        preserve_active_requests(&tx)?;
        tx.execute(
            "INSERT INTO restore_records(request_id,previous_epoch,result_json) VALUES(?1,?2,?3)",
            params![meta.request_id, current.epoch, response],
        )
        .map_err(unavailable)?;
        register(&tx, &previous_target)?;
        super::super::validate_schema(&tx).map_err(unavailable)?;
        let snapshot = snapshot::encode(&tx, sequence, &meta.request_id, &time)?;
        snapshot::validate(&snapshot, sequence)?;
        tx.execute(
            "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=?2",
            params![snapshot, sequence],
        )
        .map_err(unavailable)?;
        tx.commit().map_err(unavailable)?;
        self.recover().map_err(|e|unavailable(format!("복원은 DB에 저장되었지만 JSON 발행을 마치지 못했습니다. 같은 요청으로 확인하세요: {e}")))?;
        Ok(result)
    }
}
fn target_from_before(
    bytes: &VerifiedRestoreBytes,
    decoded: &snapshot::DecodedRestore,
) -> Result<RestoreTarget, StoreError> {
    target(bytes, decoded, RestoreTargetSource::Import)
}
