use super::{Store, StoreError, business, catalog, holding, stocktake, unavailable};
use crate::{
    business::{
        BusinessEffect, BusinessIssue, BusinessKind, BusinessPayload, BusinessResult,
        BusinessStatus,
    },
    catalog::MaterialVersion,
    domain::{Money, Quantity},
    waste::*,
};
use rusqlite::{Connection, params};
use std::collections::HashSet;
use uuid::Uuid;

fn validation(error: impl std::fmt::Display) -> StoreError {
    StoreError::Validation(error.to_string())
}
fn issue(
    field: &str,
    code: &str,
    message: impl ToString,
) -> (Vec<BusinessIssue>, Option<BusinessEffect>) {
    (
        vec![BusinessIssue {
            field: field.into(),
            code: code.into(),
            message: message.to_string(),
        }],
        None,
    )
}
fn reason_text(reason: WasteReason) -> &'static str {
    match reason {
        WasteReason::Discard => "discard",
        WasteReason::StaffMeal => "staff_meal",
    }
}

pub(super) fn material_name(
    connection: &Connection,
    data: &DraftWasteV1,
) -> Result<Option<String>, StoreError> {
    let Some((material, version)) = data
        .material_id
        .as_ref()
        .zip(data.material_version_id.as_ref())
    else {
        return Ok(None);
    };
    Ok(catalog::versions(connection, Some(material))?
        .into_iter()
        .find(|v| &v.id == version)
        .map(|v| v.name))
}

pub(super) fn inspect_waste(
    connection: &Connection,
    data: &DraftWasteV1,
) -> Result<(Vec<BusinessIssue>, Option<BusinessEffect>), StoreError> {
    let Some(id) = data.material_id.as_deref() else {
        return Ok(issue("material_id", "required", "폐기한 품목을 선택하세요"));
    };
    let Some(material) = catalog::current_version(connection, id)? else {
        return Ok(issue(
            "material_id",
            "invalid_reference",
            "폐기 품목을 찾을 수 없습니다",
        ));
    };
    if data.material_version_id.as_deref() != Some(material.id.as_str()) {
        return Ok(issue(
            "material_version_id",
            "stale_reference",
            "품목 정보가 바뀌었습니다. 입력을 유지하고 현재 정보를 다시 확인하세요",
        ));
    }
    let context = WasteMaterialContext {
        material_id: material.material_id,
        material_version_id: material.id,
        material_name: material.name,
        unit: material.canonical_unit,
        holding: holding::current_input(connection, id)?,
    };
    match calculate_waste(data, &context) {
        Ok(WasteCheck::Ready(waste)) => Ok((vec![], Some(BusinessEffect::Waste { waste }))),
        Ok(WasteCheck::NeedsReconciliation(conflict)) => Ok(issue(
            &conflict.field,
            "needs_reconciliation",
            conflict.message,
        )),
        Err(error) => Ok(issue("quantity", "invalid_input", error)),
    }
}

pub(super) fn apply_waste(
    connection: &Connection,
    data: &DraftWasteV1,
    confirmation: &str,
    seq: i64,
    time: &str,
) -> Result<(), StoreError> {
    let (issues, effect) = inspect_waste(connection, data)?;
    if !issues.is_empty() {
        return Err(validation(
            issues
                .iter()
                .map(|i| i.message.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        ));
    }
    let Some(BusinessEffect::Waste { waste }) = effect else {
        return Err(unavailable("폐기 미리보기 결과가 없습니다"));
    };
    let id = Uuid::new_v4().to_string();
    let before = holding::current_input(connection, &waste.material_id)?;
    let effect_json = serde_json::to_string(&waste).map_err(unavailable)?;
    connection.execute("INSERT INTO wastes(id,confirmation_id,material_id,material_version_id,before_holding_version_id,quantity_micros,loss_value_micros,reason,note,effect_json,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)", params![id,confirmation,waste.material_id,waste.material_version_id,waste.before.source_id,Quantity::parse(&waste.quantity).map_err(unavailable)?.micros(),Money::parse(&waste.loss_value).map_err(unavailable)?.micros(),reason_text(waste.reason),data.note,effect_json,time,seq]).map_err(unavailable)?;
    holding::append(
        connection,
        holding::AppendHolding {
            material_id: &waste.material_id,
            material_version_id: &waste.material_version_id,
            receipt_id: None,
            production_effect_id: None,
            observation_version_id: None,
            reconciliation_effect_id: None,
            waste_id: Some(&id),
            before: &before,
            after: &waste.after,
            seq,
            time,
        },
    )?;
    Ok(())
}

struct StoredWaste {
    id: String,
    confirmation_id: String,
    business_record_id: String,
    material_id: String,
    material_version_id: String,
    before_id: String,
    quantity: i64,
    loss_value: i64,
    reason: String,
    note: String,
    effect_json: String,
    recorded_at: String,
    seq: i64,
}

pub(super) fn records(
    connection: &Connection,
    material: Option<&str>,
    id: Option<&str>,
) -> Result<Vec<WasteRecord>, StoreError> {
    let rows = connection.prepare("SELECT w.id,w.confirmation_id,c.record_id,w.material_id,w.material_version_id,w.before_holding_version_id,w.quantity_micros,w.loss_value_micros,w.reason,w.note,w.effect_json,w.recorded_at,w.sequence FROM wastes w JOIN business_confirmations c ON c.id=w.confirmation_id WHERE (?1 IS NULL OR w.material_id=?1) AND (?2 IS NULL OR w.id=?2) ORDER BY w.sequence,w.id").map_err(unavailable)?
        .query_map(params![material,id], |r| Ok(StoredWaste { id:r.get(0)?,confirmation_id:r.get(1)?,business_record_id:r.get(2)?,material_id:r.get(3)?,material_version_id:r.get(4)?,before_id:r.get(5)?,quantity:r.get(6)?,loss_value:r.get(7)?,reason:r.get(8)?,note:r.get(9)?,effect_json:r.get(10)?,recorded_at:r.get(11)?,seq:r.get(12)? })).map_err(unavailable)?
        .collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    let holdings = holding::versions(connection, material)?;
    rows.into_iter()
        .map(|row| {
            let mut effect: WasteEffect =
                serde_json::from_str(&row.effect_json).map_err(unavailable)?;
            if effect.after.source_id.is_some()
                || effect.material_id != row.material_id
                || effect.material_version_id != row.material_version_id
                || effect.before.source_id.as_deref() != Some(row.before_id.as_str())
                || effect.quantity
                    != Quantity::from_micros(row.quantity)
                        .map_err(unavailable)?
                        .decimal()
                || effect.loss_value
                    != Money::from_micros(row.loss_value)
                        .map_err(unavailable)?
                        .decimal()
                || reason_text(effect.reason) != row.reason
            {
                return Err(unavailable("폐기 원본과 보존한 계산 근거가 다릅니다"));
            }
            let mut after = holdings
                .iter()
                .filter(|h| h.waste_id.as_deref() == Some(row.id.as_str()));
            let h = after
                .next()
                .ok_or_else(|| unavailable("폐기의 보유 결과가 없습니다"))?;
            if after.next().is_some()
                || h.material_id != effect.material_id
                || h.material_version_id != effect.material_version_id
                || h.quantity != effect.after.quantity
                || Some(h.value.as_str()) != effect.after.value.as_deref()
                || h.previous_version_id.as_deref() != Some(row.before_id.as_str())
                || h.seq != row.seq.to_string()
                || h.recorded_at != row.recorded_at
            {
                return Err(unavailable("폐기와 보유 결과의 연결이 다릅니다"));
            }
            effect.after.source_id = Some(h.id.clone());
            Ok(WasteRecord {
                id: row.id,
                confirmation_id: row.confirmation_id,
                business_record_id: row.business_record_id,
                effect,
                note: row.note,
                recorded_at: row.recorded_at,
                seq: row.seq.to_string(),
            })
        })
        .collect()
}

pub(super) fn result(
    connection: &Connection,
    confirmation: &str,
) -> Result<BusinessResult, StoreError> {
    let id = connection
        .query_row(
            "SELECT id FROM wastes WHERE confirmation_id=?1",
            [confirmation],
            |r| r.get(0),
        )
        .map_err(unavailable)?;
    Ok(BusinessResult::Waste { waste_id: id })
}
pub(super) fn snapshot_rows(connection: &Connection) -> Result<Vec<WasteRecord>, StoreError> {
    records(connection, None, None)
}

impl Store {
    pub fn waste_list(&self, filter: WasteFilter) -> Result<WasteList, StoreError> {
        let material = filter
            .material_id
            .as_deref()
            .map(|id| catalog::uuid(id, "품목 ID"))
            .transpose()?;
        let mut items = records(&self.connection, material.as_deref(), None)?;
        items.reverse();
        Ok(WasteList {
            items,
            state: catalog::state(&self.connection)?,
        })
    }
    pub fn waste_detail(&self, id: &str) -> Result<Option<WasteDetail>, StoreError> {
        let id = catalog::uuid(id, "폐기 ID")?;
        let Some(item) = records(&self.connection, None, Some(&id))?.pop() else {
            return Ok(None);
        };
        Ok(Some(WasteDetail {
            item,
            state: catalog::state(&self.connection)?,
        }))
    }
}

pub(super) fn migrate_wastes(connection: &Connection) -> anyhow::Result<()> {
    connection.execute_batch("CREATE TABLE wastes(
        id TEXT PRIMARY KEY NOT NULL,
        confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),
        material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,
        before_holding_version_id TEXT NOT NULL REFERENCES holding_versions_next(id),
        quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),
        loss_value_micros INTEGER NOT NULL CHECK(loss_value_micros>=0 AND loss_value_micros<=9000000000000000000),
        reason TEXT NOT NULL CHECK(reason IN('discard','staff_meal')),note TEXT NOT NULL,
        effect_json TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL UNIQUE REFERENCES snapshot_outbox(sequence),
        FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id));
        CREATE INDEX wastes_material ON wastes(material_id);")?;
    Ok(())
}

pub(super) struct Validation<'a> {
    pub records: &'a [WasteRecord],
    pub holdings: &'a [holding::HoldingVersion],
    pub observations: &'a [stocktake::VersionRow],
    pub material_versions: &'a [MaterialVersion],
    pub business_records: &'a [business::Record],
    pub revisions: &'a [crate::business::BusinessRevision],
    pub confirmations: &'a [business::ConfirmationRow],
    pub mutations: &'a [(&'a str, &'a str, &'a str)],
    pub sequence: i64,
}
fn identifier(text: &str) -> Result<(), StoreError> {
    if Uuid::parse_str(text).map_err(unavailable)?.to_string() != text {
        return Err(unavailable("폐기 ID 형식이 올바르지 않습니다"));
    }
    Ok(())
}
fn sequence(text: &str) -> Result<i64, StoreError> {
    let seq = text.parse::<i64>().map_err(unavailable)?;
    if seq <= 0 || seq.to_string() != text {
        return Err(unavailable("폐기 순서가 올바르지 않습니다"));
    }
    Ok(seq)
}
pub(super) fn validate_snapshot(state: Validation<'_>) -> Result<(), StoreError> {
    let bad = || unavailable("폐기 원본·계산·업무·보유 근거가 일치하지 않습니다");
    let mut ids = HashSet::new();
    let mut confirmations = HashSet::new();
    for item in state.records {
        for id in [
            &item.id,
            &item.confirmation_id,
            &item.business_record_id,
            &item.effect.material_id,
            &item.effect.material_version_id,
        ] {
            identifier(id)?;
        }
        let seq = sequence(&item.seq)?;
        chrono::DateTime::parse_from_rfc3339(&item.recorded_at).map_err(unavailable)?;
        if seq > state.sequence
            || !ids.insert(item.id.as_str())
            || !confirmations.insert(item.confirmation_id.as_str())
        {
            return Err(bad());
        }
        let confirmation = state
            .confirmations
            .iter()
            .find(|c| c.id == item.confirmation_id)
            .ok_or_else(bad)?;
        identifier(&confirmation.request_id)?;
        if confirmation.record_id != item.business_record_id
            || confirmation.seq != item.seq
            || confirmation.recorded_at != item.recorded_at
            || state
                .mutations
                .iter()
                .filter(|(s, op, id)| {
                    *s == item.seq && *op == "business.confirm" && *id == confirmation.request_id
                })
                .count()
                != 1
        {
            return Err(bad());
        }
        let record = state
            .business_records
            .iter()
            .find(|r| r.id == item.business_record_id)
            .ok_or_else(bad)?;
        if record.kind != BusinessKind::Waste || record.corrects_confirmation_id.is_some() {
            return Err(bad());
        }
        let revision = state
            .revisions
            .iter()
            .find(|r| r.id == confirmation.revision_id)
            .ok_or_else(bad)?;
        if revision.record_id != record.id
            || revision.status != BusinessStatus::Confirmed
            || revision.seq != item.seq
            || revision.recorded_at != item.recorded_at
        {
            return Err(bad());
        }
        let BusinessPayload::Waste { version: 1, data } = &revision.payload else {
            return Err(bad());
        };
        let material = state
            .material_versions
            .iter()
            .filter(|v| {
                v.material_id == item.effect.material_id
                    && v.seq
                        .as_deref()
                        .map(|s| s.parse::<i64>().is_ok_and(|n| n < seq))
                        .unwrap_or(v.source == "legacy_migration")
            })
            .max_by_key(|v| {
                v.seq
                    .as_deref()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0)
            })
            .ok_or_else(bad)?;
        if material.id != item.effect.material_version_id || data.note != item.note {
            return Err(bad());
        }
        let before_id = item.effect.before.source_id.as_deref().ok_or_else(bad)?;
        let before = state
            .holdings
            .iter()
            .find(|h| h.id == before_id)
            .ok_or_else(bad)?;
        let after_id = item.effect.after.source_id.as_deref().ok_or_else(bad)?;
        let after = state
            .holdings
            .iter()
            .find(|h| h.id == after_id)
            .ok_or_else(bad)?;
        if before.material_id != material.material_id
            || sequence(&before.seq)? >= seq
            || after.material_id != material.material_id
            || after.material_version_id != material.id
            || after.waste_id.as_deref() != Some(item.id.as_str())
            || after.previous_version_id.as_deref() != Some(before_id)
            || after.seq != item.seq
            || after.recorded_at != item.recorded_at
        {
            return Err(bad());
        }
        let context = WasteMaterialContext {
            material_id: material.material_id.clone(),
            material_version_id: material.id.clone(),
            material_name: material.name.clone(),
            unit: material.canonical_unit.clone(),
            holding: holding::input_from_history(state.holdings, state.observations, Some(before))?,
        };
        let WasteCheck::Ready(mut expected) =
            calculate_waste(data, &context).map_err(unavailable)?
        else {
            return Err(bad());
        };
        expected.after.source_id = Some(after.id.clone());
        if *expected != item.effect
            || after.quantity != expected.after.quantity
            || Some(after.value.as_str()) != expected.after.value.as_deref()
        {
            return Err(bad());
        }
    }
    for c in state.confirmations {
        if state
            .business_records
            .iter()
            .any(|r| r.id == c.record_id && r.kind == BusinessKind::Waste)
            && !confirmations.contains(c.id.as_str())
        {
            return Err(bad());
        }
    }
    Ok(())
}

pub(super) fn validate_waste_schema(connection: &Connection) -> anyhow::Result<()> {
    let records = snapshot_rows(connection)?;
    let holdings = holding::versions(connection, None)?;
    let observations = stocktake::snapshot_rows(connection)?;
    let materials = catalog::versions(connection, None)?;
    let business = business::snapshot_rows(connection)?;
    let requests = connection
        .prepare("SELECT sequence,operation,request_id FROM mutation_requests ORDER BY sequence")?
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?.to_string(),
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let sequence = connection.query_row(
        "SELECT COALESCE(MAX(sequence),0) FROM snapshot_outbox",
        [],
        |r| r.get(0),
    )?;
    validate_snapshot(Validation {
        records: &records,
        holdings: &holdings,
        observations: &observations.versions,
        material_versions: &materials,
        business_records: &business.records,
        revisions: &business.revisions,
        confirmations: &business.confirmations,
        mutations: &requests
            .iter()
            .map(|(s, op, id)| (s.as_str(), op.as_str(), id.as_str()))
            .collect::<Vec<_>>(),
        sequence,
    })?;
    Ok(())
}
