//! Whole-state replacement and immutable cross-generation request evidence.
use super::{Store, StoreError, catalog, snapshot, unavailable};
use crate::{catalog::CatalogState, photo_assets::PhotoAssetDescriptor, restore::*};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};

mod lifecycle;
mod schema;
pub(super) use schema::{migrate_restore, validate_restore_schema};

fn invalid(error: impl std::fmt::Display) -> StoreError {
    StoreError::Validation(error.to_string())
}
fn json(value: &impl Serialize) -> Result<String, StoreError> {
    serde_json::to_string(value).map_err(unavailable)
}
fn decode<T: DeserializeOwned>(value: &str) -> Result<T, StoreError> {
    serde_json::from_str(value).map_err(unavailable)
}
fn conflict() -> StoreError {
    StoreError::Conflict("같은 요청 ID에 다른 입력 또는 처리 기록을 사용할 수 없습니다".into())
}
pub(super) fn generation(state: CatalogState, processed: bool) -> StoreError {
    StoreError::StaleGeneration {
        request_status: if processed {
            RestoreRequestStatus::AlreadyProcessed
        } else {
            RestoreRequestStatus::NotProcessed
        },
        state,
    }
}

pub(super) fn read_preserved_state(c: &Connection) -> Result<snapshot::PreservedState, StoreError> {
    let requests=c.prepare("SELECT request_id,operation,payload,response,epoch,sequence FROM preserved_mutation_requests ORDER BY sequence,request_id").map_err(unavailable)?
        .query_map([],|r|Ok(snapshot::MutationRecord{request_id:r.get(0)?,operation:r.get(1)?,payload:r.get(2)?,response:r.get(3)?,epoch:r.get(4)?,sequence:r.get::<_,i64>(5)?.to_string()})).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    let legacy=c.prepare("SELECT request_id,name,unit,material_json,epoch,sequence FROM preserved_creation_requests ORDER BY request_id").map_err(unavailable)?
        .query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,Option<i64>>(5)?))).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    let restores = c
        .prepare(
            "SELECT request_id,previous_epoch,result_json FROM restore_records ORDER BY request_id",
        )
        .map_err(unavailable)?
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(unavailable)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(unavailable)?;
    Ok(snapshot::PreservedState {
        mutation_requests: requests,
        creation_requests: legacy
            .into_iter()
            .map(|(request_id, name, unit, material, epoch, sequence)| {
                Ok(snapshot::PreservedLegacyRequest {
                    request_id,
                    name,
                    unit,
                    material: decode(&material)?,
                    epoch,
                    sequence: sequence.map(|s| s.to_string()),
                })
            })
            .collect::<Result<_, StoreError>>()?,
        restore_records: restores
            .into_iter()
            .map(|(request_id, previous_epoch, result)| {
                Ok(snapshot::PreservedRestoreRecord {
                    request_id,
                    previous_epoch,
                    result: decode(&result)?,
                })
            })
            .collect::<Result<_, StoreError>>()?,
        photo_assets: preserved_assets(c)?,
    })
}

pub(super) fn preserved_assets(c: &Connection) -> Result<Vec<PhotoAssetDescriptor>, StoreError> {
    let rows = c
        .prepare("SELECT sha256,descriptor_json FROM preserved_photo_assets ORDER BY sha256")
        .map_err(unavailable)?
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(unavailable)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(unavailable)?;
    rows.into_iter()
        .map(|(sha, raw)| {
            let asset: PhotoAssetDescriptor = decode(&raw)?;
            if asset.sha256 != sha {
                return Err(unavailable("사진 보존 기록이 일치하지 않습니다"));
            }
            crate::photo_assets::validate_asset_descriptor(&asset).map_err(unavailable)?;
            Ok(asset)
        })
        .collect()
}
pub(super) fn preserved_asset(
    c: &Connection,
    sha: &str,
) -> Result<Option<PhotoAssetDescriptor>, StoreError> {
    let raw: Option<String> = c
        .query_row(
            "SELECT descriptor_json FROM preserved_photo_assets WHERE sha256=?1",
            [sha],
            |r| r.get(0),
        )
        .optional()
        .map_err(unavailable)?;
    raw.map(|raw| {
        let asset: PhotoAssetDescriptor = decode(&raw)?;
        if asset.sha256 != sha {
            return Err(unavailable("사진 보존 기록이 일치하지 않습니다"));
        }
        crate::photo_assets::validate_asset_descriptor(&asset).map_err(unavailable)?;
        Ok(asset)
    })
    .transpose()
}
pub(super) fn preserve_asset(
    c: &Connection,
    asset: &PhotoAssetDescriptor,
) -> Result<(), StoreError> {
    if let Some(old) = preserved_asset(c, &asset.sha256)? {
        if old != *asset {
            return Err(conflict());
        }
    } else {
        c.execute(
            "INSERT INTO preserved_photo_assets(sha256,descriptor_json) VALUES(?1,?2)",
            params![asset.sha256, json(asset)?],
        )
        .map_err(unavailable)?;
    }
    Ok(())
}
pub(super) fn preserved_request(
    c: &Connection,
    id: &str,
) -> Result<Option<snapshot::MutationRecord>, StoreError> {
    c.query_row("SELECT request_id,operation,payload,response,epoch,sequence FROM preserved_mutation_requests WHERE request_id=?1",[id],|r|Ok(snapshot::MutationRecord{request_id:r.get(0)?,operation:r.get(1)?,payload:r.get(2)?,response:r.get(3)?,epoch:r.get(4)?,sequence:r.get::<_,i64>(5)?.to_string()})).optional().map_err(unavailable)
}
pub(super) fn preserved_legacy(
    c: &Connection,
    id: &str,
) -> Result<Option<snapshot::PreservedLegacyRequest>, StoreError> {
    let row=c.query_row("SELECT name,unit,material_json,epoch,sequence FROM preserved_creation_requests WHERE request_id=?1",[id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,Option<i64>>(4)?))).optional().map_err(unavailable)?;
    row.map(|(name, unit, material, epoch, sequence)| {
        Ok(snapshot::PreservedLegacyRequest {
            request_id: id.into(),
            name,
            unit,
            material: decode(&material)?,
            epoch,
            sequence: sequence.map(|s| s.to_string()),
        })
    })
    .transpose()
}
pub(super) fn has_restored(c: &Connection) -> Result<bool, StoreError> {
    c.query_row("SELECT EXISTS(SELECT 1 FROM restore_records)", [], |r| {
        r.get(0)
    })
    .map_err(unavailable)
}
fn preserve_request(c: &Connection, r: &snapshot::MutationRecord) -> Result<(), StoreError> {
    if let Some(old) = preserved_request(c, &r.request_id)? {
        if json(&old)? != json(r)? {
            return Err(conflict());
        }
    } else {
        if let Some(legacy) = preserved_legacy(c, &r.request_id)? {
            if r.operation != "legacy.material.create" {
                return Err(conflict());
            }
            let input: super::CreateMaterial = decode(&r.payload)?;
            if input.request_id != legacy.request_id
                || input.name != legacy.name
                || input.unit != legacy.unit
            {
                return Err(conflict());
            }
        }
        let sequence = r.sequence.parse::<i64>().map_err(invalid)?;
        c.execute(
            "INSERT OR IGNORE INTO state_sequences(sequence) VALUES(?1)",
            [sequence],
        )
        .map_err(unavailable)?;
        c.execute("INSERT INTO preserved_mutation_requests(request_id,operation,payload,response,epoch,sequence) VALUES(?1,?2,?3,?4,?5,?6)",params![r.request_id,r.operation,r.payload,r.response,r.epoch,sequence]).map_err(unavailable)?;
    }
    Ok(())
}
fn preserve_legacy(c: &Connection, r: &snapshot::PreservedLegacyRequest) -> Result<(), StoreError> {
    if let Some(old) = preserved_legacy(c, &r.request_id)? {
        // Old v1/v2 requests did not record a generation. Expansion metadata
        // cannot turn the same immutable legacy registration into a conflict.
        if old.name != r.name || old.unit != r.unit || json(&old.material)? != json(&r.material)? {
            return Err(conflict());
        }
    } else {
        if let Some(request) = preserved_request(c, &r.request_id)? {
            if request.operation != "legacy.material.create" {
                return Err(conflict());
            }
            let input: super::CreateMaterial = decode(&request.payload)?;
            if input.name != r.name || input.unit != r.unit || input.request_id != r.request_id {
                return Err(conflict());
            }
        }
        let sequence = r
            .sequence
            .as_deref()
            .map(str::parse::<i64>)
            .transpose()
            .map_err(invalid)?;
        if let Some(sequence) = sequence {
            c.execute(
                "INSERT OR IGNORE INTO state_sequences(sequence) VALUES(?1)",
                [sequence],
            )
            .map_err(unavailable)?;
        }
        c.execute("INSERT INTO preserved_creation_requests(request_id,name,unit,material_json,epoch,sequence) VALUES(?1,?2,?3,?4,?5,?6)",params![r.request_id,r.name,r.unit,json(&r.material)?,r.epoch,sequence]).map_err(unavailable)?;
    }
    Ok(())
}
pub(super) fn merge_preserved(
    c: &Connection,
    state: &snapshot::PreservedState,
) -> Result<(), StoreError> {
    for r in &state.mutation_requests {
        preserve_request(c, r)?;
    }
    for r in &state.creation_requests {
        preserve_legacy(c, r)?;
    }
    for r in &state.restore_records {
        let old: Option<(String, String)> = c
            .query_row(
                "SELECT previous_epoch,result_json FROM restore_records WHERE request_id=?1",
                [&r.request_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(unavailable)?;
        let result = json(&r.result)?;
        if let Some((epoch, body)) = old {
            if epoch != r.previous_epoch || body != result {
                return Err(conflict());
            }
        } else {
            c.execute("INSERT INTO restore_records(request_id,previous_epoch,result_json) VALUES(?1,?2,?3)",params![r.request_id,r.previous_epoch,result]).map_err(unavailable)?;
        }
    }
    for asset in &state.photo_assets {
        preserve_asset(c, asset)?;
    }
    Ok(())
}

// Called in the same transaction as every accepted ordinary mutation.
pub(super) fn preserve_active_requests(c: &Connection) -> Result<(), StoreError> {
    let epoch = catalog::state(c)?.epoch;
    let requests=c.prepare("SELECT request_id,operation,payload,response,epoch,sequence FROM mutation_requests ORDER BY sequence").map_err(unavailable)?
        .query_map([],|r|Ok(snapshot::MutationRecord{request_id:r.get(0)?,operation:r.get(1)?,payload:r.get(2)?,response:r.get(3)?,epoch:r.get(4)?,sequence:r.get::<_,i64>(5)?.to_string()})).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    for r in &requests {
        preserve_request(c, r)?;
    }
    let legacy=c.prepare("SELECT r.request_id,r.name,r.unit,m.id,m.name,m.unit,m.created_at FROM creation_requests r JOIN materials m ON m.id=r.material_id ORDER BY r.request_id").map_err(unavailable)?
        .query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,super::Material{id:r.get(3)?,name:r.get(4)?,unit:r.get(5)?,created_at:r.get(6)?}))).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    for (request_id, name, unit, material) in legacy {
        if preserved_legacy(c, &request_id)?.is_some() {
            continue;
        }
        let request = requests.iter().find(|r| r.request_id == request_id);
        preserve_legacy(
            c,
            &snapshot::PreservedLegacyRequest {
                request_id: request_id.clone(),
                name,
                unit,
                material,
                epoch: request.map_or_else(|| epoch.clone(), |r| r.epoch.clone()),
                sequence: request.map(|r| r.sequence.clone()),
            },
        )?;
    }
    Ok(())
}
