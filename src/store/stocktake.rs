use super::{Store, StoreError, business, catalog, holding, sale, snapshot, unavailable};
use crate::{
    business::{BusinessEffect, BusinessIssue, BusinessResult},
    domain::{Money, Quantity, UnitPrice},
    holding::{HoldingBasisRef, HoldingObservation},
    stocktake::*,
};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ObservationRow {
    pub id: String,
    pub stocktake_id: String,
    pub position: usize,
    pub material_id: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct VersionRow {
    pub id: String,
    pub observation_id: String,
    pub previous_version_id: Option<String>,
    pub previous_observation_version_id: Option<String>,
    pub effect: StocktakeRowEffect,
    pub recorded_at: String,
    pub seq: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Head {
    pub observation_id: String,
    pub current_version_id: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConsumptionVersion {
    pub id: String,
    pub observation_version_id: String,
    pub previous_version_id: Option<String>,
    pub basis: HoldingBasisRef,
    pub end_seq: String,
    pub as_of_revision: String,
    pub total_quantity: String,
    pub total_value: String,
    pub unallocated_quantity: String,
    pub unallocated_value: String,
    pub reason: Option<NoAllocationReason>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConsumptionHead {
    pub observation_id: String,
    pub current_version_id: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConsumptionRow {
    pub consumption_version_id: String,
    pub row: SaleAllocationProposal,
}
pub(super) struct SnapshotRows {
    pub batches: Vec<StocktakeRecord>,
    pub observations: Vec<ObservationRow>,
    pub versions: Vec<VersionRow>,
    pub heads: Vec<Head>,
    pub consumption_versions: Vec<ConsumptionVersion>,
    pub consumption_heads: Vec<ConsumptionHead>,
    pub consumption_rows: Vec<ConsumptionRow>,
}
pub(super) struct Validation<'a> {
    pub batches: &'a [StocktakeRecord],
    pub observations: &'a [ObservationRow],
    pub versions: &'a [VersionRow],
    pub heads: &'a [Head],
    pub consumption_versions: &'a [ConsumptionVersion],
    pub consumption_heads: &'a [ConsumptionHead],
    pub consumption_rows: &'a [ConsumptionRow],
    pub holdings: &'a [holding::HoldingVersion],
    pub material_versions: &'a [crate::catalog::MaterialVersion],
    pub prices: &'a [crate::catalog::MaterialPriceVersion],
    pub sales: &'a [sale::RecordRow],
    pub sale_cost_rows: &'a [sale::CostRow],
    pub sale_versions: &'a [crate::sale::SaleVersion],
    pub business_records: &'a [business::Record],
    pub revisions: &'a [crate::business::BusinessRevision],
    pub confirmations: &'a [business::ConfirmationRow],
    pub other_confirmation_ids: &'a [&'a str],
    pub mutations: &'a [(&'a str, &'a str, &'a str)],
    pub sequence: i64,
}

fn records(connection: &Connection) -> Result<Vec<StocktakeRecord>, StoreError> {
    connection.prepare("SELECT b.id,b.confirmation_id,c.record_id,b.note,b.recorded_at,b.sequence FROM stocktake_batches b JOIN business_confirmations c ON c.id=b.confirmation_id ORDER BY b.sequence,b.id").map_err(unavailable)?
        .query_map([], |r| Ok(StocktakeRecord {id:r.get(0)?,confirmation_id:r.get(1)?,business_record_id:r.get(2)?,note:r.get(3)?,recorded_at:r.get(4)?,seq:r.get::<_,i64>(5)?.to_string()})).map_err(unavailable)?
        .collect::<rusqlite::Result<_>>().map_err(unavailable)
}
pub(super) fn versions(connection: &Connection) -> Result<Vec<VersionRow>, StoreError> {
    let rows = connection.prepare("SELECT v.id,v.observation_id,v.previous_version_id,v.previous_observation_version_id,v.effect_json,v.recorded_at,v.sequence,v.before_holding_version_id,h.id FROM stocktake_versions v JOIN holding_versions h ON h.observation_version_id=v.id ORDER BY v.sequence,v.id").map_err(unavailable)?
        .query_map([], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,Option<String>>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,i64>(6)?,r.get::<_,Option<String>>(7)?,r.get::<_,String>(8)?))).map_err(unavailable)?
        .collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    rows.into_iter()
        .map(
            |(
                id,
                observation_id,
                previous_version_id,
                previous_observation_version_id,
                json,
                recorded_at,
                seq,
                before,
                after,
            )| {
                let mut effect: StocktakeRowEffect =
                    serde_json::from_str(&json).map_err(unavailable)?;
                if effect.after.source_id.is_some()
                    || effect.before.as_ref().and_then(|b| b.source_id.as_deref())
                        != before.as_deref()
                {
                    return Err(unavailable("관측 보유 참조가 일치하지 않습니다"));
                }
                effect.after.source_id = Some(after);
                Ok(VersionRow {
                    id,
                    observation_id,
                    previous_version_id,
                    previous_observation_version_id,
                    effect,
                    recorded_at,
                    seq: seq.to_string(),
                })
            },
        )
        .collect()
}
pub(super) fn snapshot_rows(connection: &Connection) -> Result<SnapshotRows, StoreError> {
    let observations=connection.prepare("SELECT id,stocktake_id,position,material_id FROM stocktake_observations ORDER BY stocktake_id,position").map_err(unavailable)?
        .query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,r.get::<_,String>(3)?))).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?
        .into_iter().map(|(id,stocktake_id,position,material_id)|Ok(ObservationRow{id,stocktake_id,position:usize::try_from(position).map_err(unavailable)?,material_id})).collect::<Result<Vec<_>,StoreError>>()?;
    let heads = connection
        .prepare(
            "SELECT observation_id,current_version_id FROM stocktake_heads ORDER BY observation_id",
        )
        .map_err(unavailable)?
        .query_map([], |r| {
            Ok(Head {
                observation_id: r.get(0)?,
                current_version_id: r.get(1)?,
            })
        })
        .map_err(unavailable)?
        .collect::<rusqlite::Result<_>>()
        .map_err(unavailable)?;
    let consumption_versions=connection.prepare("SELECT id,observation_version_id,previous_version_id,allocation_json FROM consumption_versions ORDER BY id").map_err(unavailable)?
        .query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,String>(3)?))).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?
        .into_iter().map(|(id,observation_version_id,previous_version_id,json)| {
            let row:ConsumptionVersion=serde_json::from_str(&json).map_err(unavailable)?;
            if row.id!=id || row.observation_version_id!=observation_version_id || row.previous_version_id!=previous_version_id {return Err(unavailable("정산 버전 참조가 다릅니다"));}
            Ok(row)
        }).collect::<Result<_,StoreError>>()?;
    let consumption_heads=connection.prepare("SELECT observation_id,current_version_id FROM consumption_heads ORDER BY observation_id").map_err(unavailable)?
        .query_map([],|r|Ok(ConsumptionHead{observation_id:r.get(0)?,current_version_id:r.get(1)?})).map_err(unavailable)?.collect::<rusqlite::Result<_>>().map_err(unavailable)?;
    let consumption_rows=connection.prepare("SELECT consumption_version_id,position,sale_id,sale_version_id,recipe_version_id,row_json FROM consumption_rows ORDER BY consumption_version_id,position").map_err(unavailable)?
        .query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?))).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?
        .into_iter().map(|(consumption_version_id,position,sale,version,recipe,json)| {
            let row:SaleAllocationProposal=serde_json::from_str(&json).map_err(unavailable)?;
            if i64::try_from(row.position).map_err(unavailable)?!=position || row.sale_id!=sale || row.sale_version_id!=version || row.recipe_version_id!=recipe {return Err(unavailable("정산 판매 참조가 다릅니다"));}
            Ok(ConsumptionRow{consumption_version_id,row})
        }).collect::<Result<_,StoreError>>()?;
    Ok(SnapshotRows {
        batches: records(connection)?,
        observations,
        versions: versions(connection)?,
        heads,
        consumption_versions,
        consumption_heads,
        consumption_rows,
    })
}

pub(super) fn observation(row: &VersionRow) -> HoldingObservation {
    HoldingObservation {
        id: row.id.clone(),
        quantity: row.effect.measured_quantity.clone(),
        recorded_at: row.recorded_at.clone(),
        seq: row.seq.clone(),
    }
}
pub(super) fn material_name(
    connection: &Connection,
    data: &DraftStocktakeV1,
) -> Result<Option<String>, StoreError> {
    for row in &data.rows {
        if let (Some(id), Some(version)) = (&row.material_id, &row.material_version_id)
            && let Some(v) = catalog::versions(connection, Some(id))?
                .into_iter()
                .find(|v| &v.id == version)
        {
            return Ok(Some(v.name));
        }
    }
    Ok(None)
}
fn inspect_at(
    connection: &Connection,
    data: &DraftStocktakeV1,
    end: i64,
    as_of: i64,
) -> Result<(Vec<BusinessIssue>, Option<BusinessEffect>), StoreError> {
    let sources = sale::sources_at(connection, as_of)?;
    let mut contexts = vec![];
    let mut seen = HashSet::new();
    for row in &data.rows {
        if row.quantity.trim().is_empty() {
            continue;
        }
        if let Some(id) = &row.material_id
            && seen.insert(id.clone())
            && let Some(m) = catalog::current_version(connection, id)?
        {
            contexts.push(StocktakeMaterialContext {
                material_id: m.material_id.clone(),
                material_version_id: m.id,
                material_name: m.name,
                unit: m.canonical_unit,
                holding: holding::current_input(connection, id)?,
                price: holding::price(connection, id, catalog::registered_price(connection, id)?)?,
                sales: sources.clone(),
            });
        }
    }
    match calculate_stocktake(data, &contexts, end, as_of) {
        Ok(StocktakeCheck::Ready(p)) => Ok((
            vec![],
            Some(BusinessEffect::Stocktake {
                rows: p.rows,
                unchecked_positions: p.unchecked_positions,
            }),
        )),
        Ok(StocktakeCheck::NeedsConfirmation(issues)) => Ok((
            issues
                .into_iter()
                .map(|i| BusinessIssue {
                    field: i.field,
                    code: i.code,
                    message: i.message,
                })
                .collect(),
            None,
        )),
        Err(e) => Ok((
            vec![BusinessIssue {
                field: "rows".into(),
                code: "invalid_input".into(),
                message: e.to_string(),
            }],
            None,
        )),
    }
}
pub(super) fn inspect_stocktake(
    connection: &Connection,
    data: &DraftStocktakeV1,
) -> Result<(Vec<BusinessIssue>, Option<BusinessEffect>), StoreError> {
    let revision = catalog::state(connection)?
        .revision
        .parse::<i64>()
        .map_err(unavailable)?;
    inspect_at(
        connection,
        data,
        revision
            .checked_add(1)
            .ok_or_else(|| unavailable("실사 순번 범위 초과"))?,
        revision,
    )
}
pub(super) fn apply_stocktake(
    connection: &Connection,
    data: &DraftStocktakeV1,
    confirmation: &str,
    seq: i64,
    time: &str,
) -> Result<(), StoreError> {
    let as_of = seq
        .checked_sub(1)
        .ok_or_else(|| unavailable("실사 순번 범위 초과"))?;
    if catalog::state(connection)?.revision != as_of.to_string() {
        return Err(StoreError::StaleState(
            "실사 미리보기 순번이 변경되었습니다".into(),
        ));
    }
    let (issues, effect) = inspect_at(connection, data, seq, as_of)?;
    let Some(BusinessEffect::Stocktake { rows, .. }) = effect else {
        return Err(StoreError::Validation(
            issues
                .iter()
                .map(|i| i.message.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        ));
    };
    let batch = Uuid::new_v4().to_string();
    connection.execute("INSERT INTO stocktake_batches(id,confirmation_id,note,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5)",params![batch,confirmation,data.note,time,seq]).map_err(unavailable)?;
    for effect in rows {
        let id = Uuid::new_v4().to_string();
        let version = Uuid::new_v4().to_string();
        let before = holding::current_input(connection, &effect.material_id)?;
        connection.execute("INSERT INTO stocktake_observations(id,stocktake_id,position,material_id) VALUES(?1,?2,?3,?4)",params![id,batch,i64::try_from(effect.position).map_err(unavailable)?,effect.material_id]).map_err(unavailable)?;
        connection.execute("INSERT INTO stocktake_versions(id,observation_id,previous_version_id,previous_observation_version_id,before_holding_version_id,effect_json,recorded_at,sequence) VALUES(?1,?2,NULL,?3,?4,?5,?6,?7)",params![version,id,before.observation.as_ref().map(|o|&o.id),before.source_id,serde_json::to_string(&effect).map_err(unavailable)?,time,seq]).map_err(unavailable)?;
        connection
            .execute(
                "INSERT INTO stocktake_heads(observation_id,current_version_id) VALUES(?1,?2)",
                params![id, version],
            )
            .map_err(unavailable)?;
        if let StocktakeCalculation::Settlement { allocation, .. } = &effect.calculation {
            let cv = ConsumptionVersion {
                id: Uuid::new_v4().to_string(),
                observation_version_id: version.clone(),
                previous_version_id: None,
                basis: allocation.basis.clone(),
                end_seq: allocation.end_seq.clone(),
                as_of_revision: allocation.as_of_revision.clone(),
                total_quantity: allocation.total_quantity.clone(),
                total_value: allocation.total_value.clone(),
                unallocated_quantity: allocation.unallocated_quantity.clone(),
                unallocated_value: allocation.unallocated_value.clone(),
                reason: allocation.reason,
            };
            connection.execute("INSERT INTO consumption_versions(id,observation_version_id,previous_version_id,allocation_json) VALUES(?1,?2,NULL,?3)",params![cv.id,version,serde_json::to_string(&cv).map_err(unavailable)?]).map_err(unavailable)?;
            connection.execute("INSERT INTO consumption_heads(observation_id,current_version_id) VALUES(?1,?2)",params![id,cv.id]).map_err(unavailable)?;
            for row in &allocation.candidates {
                connection.execute("INSERT INTO consumption_rows(consumption_version_id,position,sale_id,sale_version_id,recipe_version_id,row_json) VALUES(?1,?2,?3,?4,?5,?6)",params![cv.id,i64::try_from(row.position).map_err(unavailable)?,row.sale_id,row.sale_version_id,row.recipe_version_id,serde_json::to_string(row).map_err(unavailable)?]).map_err(unavailable)?;
            }
        }
        holding::append(
            connection,
            holding::AppendHolding {
                material_id: &effect.material_id,
                material_version_id: &effect.material_version_id,
                receipt_id: None,
                production_effect_id: None,
                waste_id: None,
                observation_version_id: Some(&version),
                before: &before,
                after: &effect.after,
                seq,
                time,
            },
        )?;
    }
    Ok(())
}
pub(super) fn result(
    connection: &Connection,
    confirmation: &str,
) -> Result<BusinessResult, StoreError> {
    connection
        .query_row(
            "SELECT id FROM stocktake_batches WHERE confirmation_id=?1",
            [confirmation],
            |r| {
                Ok(BusinessResult::Stocktake {
                    stocktake_id: r.get(0)?,
                })
            },
        )
        .map_err(unavailable)
}

impl Store {
    pub fn stocktake_list(&self, filter: StocktakeFilter) -> Result<StocktakeList, StoreError> {
        let material = filter
            .material_id
            .as_deref()
            .map(|id| catalog::uuid(id, "실사 품목 ID"))
            .transpose()?;
        let rows = snapshot_rows(&self.connection)?;
        let mut items = vec![];
        for batch in rows.batches {
            let mut observations = rows
                .observations
                .iter()
                .filter(|o| o.stocktake_id == batch.id)
                .collect::<Vec<_>>();
            observations.sort_by_key(|o| o.position);
            if material
                .as_ref()
                .is_some_and(|id| !observations.iter().any(|o| &o.material_id == id))
            {
                continue;
            }
            let first = observations
                .first()
                .and_then(|o| rows.versions.iter().find(|v| v.observation_id == o.id))
                .ok_or_else(|| unavailable("실사 관측이 없습니다"))?;
            items.push(StocktakeSummary {
                id: batch.id,
                confirmation_id: batch.confirmation_id,
                business_record_id: batch.business_record_id,
                first_material_name: first.effect.material_name.clone(),
                confirmed_count: observations.len(),
                recorded_at: batch.recorded_at,
                seq: batch.seq,
            });
        }
        items.reverse();
        Ok(StocktakeList {
            items,
            state: catalog::state(&self.connection)?,
        })
    }
    pub fn stocktake_detail(&self, id: &str) -> Result<Option<StocktakeDetail>, StoreError> {
        let id = catalog::uuid(id, "실사 ID")?;
        let rows = snapshot_rows(&self.connection)?;
        let Some(item) = rows.batches.into_iter().find(|b| b.id == id) else {
            return Ok(None);
        };
        let mut observations = vec![];
        for original in rows.observations.iter().filter(|o| o.stocktake_id == id) {
            for v in rows
                .versions
                .iter()
                .filter(|v| v.observation_id == original.id)
            {
                observations.push(StocktakeObservationRecord {
                    id: original.id.clone(),
                    version_id: v.id.clone(),
                    previous_version_id: v.previous_version_id.clone(),
                    stocktake_id: id.clone(),
                    previous_observation_version_id: v.previous_observation_version_id.clone(),
                    consumption_version_id: rows
                        .consumption_versions
                        .iter()
                        .find(|c| c.observation_version_id == v.id)
                        .map(|c| c.id.clone()),
                    effect: v.effect.clone(),
                    recorded_at: v.recorded_at.clone(),
                    seq: v.seq.clone(),
                });
            }
        }
        observations.sort_by_key(|v| v.effect.position);
        Ok(Some(StocktakeDetail {
            item,
            observations,
            state: catalog::state(&self.connection)?,
        }))
    }
}

pub(super) fn migrate_stocktakes(connection: &Connection) -> anyhow::Result<()> {
    let payloads = connection
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (seq, payload) in payloads {
        snapshot::validate_source(&payload, seq, 9)?;
    }
    connection.execute_batch("CREATE TABLE stocktake_batches(id TEXT PRIMARY KEY NOT NULL,confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),note TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL UNIQUE REFERENCES snapshot_outbox(sequence));
        CREATE TABLE stocktake_observations(id TEXT PRIMARY KEY NOT NULL,stocktake_id TEXT NOT NULL REFERENCES stocktake_batches(id),position INTEGER NOT NULL CHECK(position>=0 AND position<100),material_id TEXT NOT NULL REFERENCES materials(id),UNIQUE(stocktake_id,position),UNIQUE(stocktake_id,material_id));
        CREATE TABLE stocktake_versions(id TEXT PRIMARY KEY NOT NULL,observation_id TEXT NOT NULL REFERENCES stocktake_observations(id),previous_version_id TEXT,previous_observation_version_id TEXT REFERENCES stocktake_versions(id),before_holding_version_id TEXT REFERENCES holding_versions_next(id),effect_json TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),UNIQUE(observation_id,id),FOREIGN KEY(observation_id,previous_version_id) REFERENCES stocktake_versions(observation_id,id));
        CREATE TABLE stocktake_heads(observation_id TEXT PRIMARY KEY NOT NULL REFERENCES stocktake_observations(id),current_version_id TEXT NOT NULL,FOREIGN KEY(observation_id,current_version_id) REFERENCES stocktake_versions(observation_id,id));
        CREATE TABLE consumption_versions(id TEXT PRIMARY KEY NOT NULL,observation_version_id TEXT NOT NULL UNIQUE REFERENCES stocktake_versions(id),previous_version_id TEXT REFERENCES consumption_versions(id),allocation_json TEXT NOT NULL);
        CREATE TABLE consumption_heads(observation_id TEXT PRIMARY KEY NOT NULL REFERENCES stocktake_observations(id),current_version_id TEXT NOT NULL UNIQUE REFERENCES consumption_versions(id));
        CREATE TABLE consumption_rows(consumption_version_id TEXT NOT NULL REFERENCES consumption_versions(id),position INTEGER NOT NULL CHECK(position>=0),sale_id TEXT NOT NULL REFERENCES sales(id),sale_version_id TEXT NOT NULL,recipe_version_id TEXT NOT NULL REFERENCES recipe_versions(id),row_json TEXT NOT NULL,PRIMARY KEY(consumption_version_id,position),UNIQUE(consumption_version_id,sale_id),FOREIGN KEY(sale_id,sale_version_id) REFERENCES sale_versions(sale_id,id));
        CREATE INDEX consumption_rows_sale ON consumption_rows(sale_id);")?;
    Ok(())
}

fn identifier(value: &str) -> Result<(), StoreError> {
    if Uuid::parse_str(value).map_err(unavailable)?.to_string() != value {
        return Err(unavailable("실사 참조 ID가 올바르지 않습니다"));
    }
    Ok(())
}
fn sequence(value: &str) -> Result<i64, StoreError> {
    let seq = value.parse::<i64>().map_err(unavailable)?;
    if seq <= 0 || seq.to_string() != value {
        return Err(unavailable("실사 순번이 올바르지 않습니다"));
    }
    Ok(seq)
}
fn equal<T: Serialize>(left: &T, right: &T) -> Result<bool, StoreError> {
    Ok(serde_json::to_value(left).map_err(unavailable)?
        == serde_json::to_value(right).map_err(unavailable)?)
}

pub(super) fn validate_snapshot(state: Validation<'_>) -> Result<(), StoreError> {
    use crate::business::{BusinessKind, BusinessPayload, BusinessStatus};
    let bad = || unavailable("실사·관측·정산의 원본과 확정 근거가 일치하지 않습니다");
    if state.observations.len() != state.versions.len()
        || state.heads.len() != state.versions.len()
        || state.consumption_versions.len() != state.consumption_heads.len()
    {
        return Err(bad());
    }
    let mut batch_ids = HashSet::new();
    let mut confirmation_ids = HashSet::new();
    let mut used_observations = HashSet::new();
    let mut used_versions = HashSet::new();
    let mut used_consumptions = HashSet::new();
    let mut used_rows = HashSet::new();
    for batch in state.batches {
        identifier(&batch.id)?;
        identifier(&batch.confirmation_id)?;
        identifier(&batch.business_record_id)?;
        let seq = sequence(&batch.seq)?;
        if seq > state.sequence
            || chrono::DateTime::parse_from_rfc3339(&batch.recorded_at).is_err()
            || !batch_ids.insert(&batch.id)
            || !confirmation_ids.insert(&batch.confirmation_id)
            || state
                .other_confirmation_ids
                .contains(&batch.confirmation_id.as_str())
        {
            return Err(bad());
        }
        let confirmation = state
            .confirmations
            .iter()
            .find(|c| c.id == batch.confirmation_id)
            .ok_or_else(bad)?;
        identifier(&confirmation.request_id)?;
        let record = state
            .business_records
            .iter()
            .find(|r| r.id == batch.business_record_id)
            .ok_or_else(bad)?;
        let revision = state
            .revisions
            .iter()
            .find(|r| r.id == confirmation.revision_id)
            .ok_or_else(bad)?;
        if confirmation.record_id != record.id
            || record.kind != BusinessKind::Stocktake
            || record.corrects_confirmation_id.is_some()
            || confirmation.seq != batch.seq
            || confirmation.recorded_at != batch.recorded_at
            || revision.record_id != record.id
            || revision.status != BusinessStatus::Confirmed
            || revision.seq != batch.seq
            || revision.recorded_at != batch.recorded_at
            || state
                .mutations
                .iter()
                .filter(|(s, op, id)| {
                    *s == batch.seq && *op == "business.confirm" && *id == confirmation.request_id
                })
                .count()
                != 1
        {
            return Err(bad());
        }
        let BusinessPayload::Stocktake { version: 1, data } = &revision.payload else {
            return Err(bad());
        };
        if data.note != batch.note {
            return Err(bad());
        }
        let sources = sale::consumption_sources(
            state.sales,
            state.sale_cost_rows,
            state.sale_versions,
            seq - 1,
        )?;
        let mut contexts = vec![];
        let mut seen = HashSet::new();
        for row in &data.rows {
            if row.quantity.trim().is_empty() {
                continue;
            }
            let id = row.material_id.as_deref().ok_or_else(bad)?;
            if !seen.insert(id) {
                return Err(bad());
            }
            let material = state
                .material_versions
                .iter()
                .filter(|m| {
                    m.material_id == id
                        && match m.seq.as_deref() {
                            Some(s) => sequence(s).is_ok_and(|s| s < seq),
                            None => m.source == "legacy_migration",
                        }
                })
                .max_by_key(|m| {
                    m.seq
                        .as_deref()
                        .and_then(|s| s.parse::<i64>().ok())
                        .unwrap_or(0)
                })
                .ok_or_else(bad)?;
            if row.material_version_id.as_deref() != Some(&material.id) {
                return Err(bad());
            }
            let before = state
                .holdings
                .iter()
                .filter(|h| h.material_id == id && sequence(&h.seq).is_ok_and(|s| s < seq))
                .max_by_key(|h| sequence(&h.seq).unwrap_or(0));
            let holding = holding::input_from_history(state.holdings, state.versions, before)?;
            let price = state
                .prices
                .iter()
                .filter(|p| p.material_id == id && sequence(&p.seq).is_ok_and(|s| s < seq))
                .max_by_key(|p| sequence(&p.seq).unwrap_or(0))
                .map(|p| {
                    UnitPrice::new(
                        Money::parse(&p.amount)?,
                        Quantity::parse(&p.quantity)?,
                        p.id.clone(),
                    )
                })
                .transpose()
                .map_err(unavailable)?;
            let price =
                crate::holding::resolve_holding_price(&holding, price).map_err(unavailable)?;
            contexts.push(StocktakeMaterialContext {
                material_id: material.material_id.clone(),
                material_version_id: material.id.clone(),
                material_name: material.name.clone(),
                unit: material.canonical_unit.clone(),
                holding,
                price,
                sales: sources.clone(),
            });
        }
        let StocktakeCheck::Ready(expected) =
            calculate_stocktake(data, &contexts, seq, seq - 1).map_err(unavailable)?
        else {
            return Err(bad());
        };
        let originals = state
            .observations
            .iter()
            .filter(|o| o.stocktake_id == batch.id)
            .collect::<Vec<_>>();
        if originals.len() != expected.rows.len() {
            return Err(bad());
        }
        for effect in expected.rows {
            let matching = originals
                .iter()
                .filter(|o| o.position == effect.position)
                .collect::<Vec<_>>();
            if matching.len() != 1 {
                return Err(bad());
            }
            let original = *matching[0];
            identifier(&original.id)?;
            if original.material_id != effect.material_id || !used_observations.insert(&original.id)
            {
                return Err(bad());
            }
            let matching = state
                .versions
                .iter()
                .filter(|v| v.observation_id == original.id)
                .collect::<Vec<_>>();
            if matching.len() != 1 {
                return Err(bad());
            }
            let version = matching[0];
            identifier(&version.id)?;
            if !used_versions.insert(&version.id)
                || version.previous_version_id.is_some()
                || version.previous_observation_version_id.as_deref()
                    != effect.observation_before.as_ref().map(|o| o.id.as_str())
                || version.seq != batch.seq
                || version.recorded_at != batch.recorded_at
            {
                return Err(bad());
            }
            let afters = state
                .holdings
                .iter()
                .filter(|h| h.observation_version_id.as_ref() == Some(&version.id))
                .collect::<Vec<_>>();
            if afters.len() != 1 || version.effect.after.source_id.as_deref() != Some(&afters[0].id)
            {
                return Err(bad());
            }
            let mut actual = version.effect.clone();
            actual.after.source_id = None;
            if actual != effect
                || state
                    .heads
                    .iter()
                    .filter(|h| {
                        h.observation_id == original.id && h.current_version_id == version.id
                    })
                    .count()
                    != 1
            {
                return Err(bad());
            }
            let consumption = state
                .consumption_versions
                .iter()
                .filter(|c| c.observation_version_id == version.id)
                .collect::<Vec<_>>();
            match &effect.calculation {
                StocktakeCalculation::Initial { .. } => {
                    if !consumption.is_empty()
                        || state
                            .consumption_heads
                            .iter()
                            .any(|h| h.observation_id == original.id)
                    {
                        return Err(bad());
                    }
                }
                StocktakeCalculation::Settlement { allocation, .. } => {
                    if consumption.len() != 1 {
                        return Err(bad());
                    }
                    let c = consumption[0];
                    identifier(&c.id)?;
                    if !used_consumptions.insert(&c.id)
                        || c.previous_version_id.is_some()
                        || c.basis != allocation.basis
                        || c.end_seq != allocation.end_seq
                        || c.as_of_revision != allocation.as_of_revision
                        || c.total_quantity != allocation.total_quantity
                        || c.total_value != allocation.total_value
                        || c.unallocated_quantity != allocation.unallocated_quantity
                        || c.unallocated_value != allocation.unallocated_value
                        || c.reason != allocation.reason
                        || state
                            .consumption_heads
                            .iter()
                            .filter(|h| {
                                h.observation_id == original.id && h.current_version_id == c.id
                            })
                            .count()
                            != 1
                    {
                        return Err(bad());
                    }
                    let rows = state
                        .consumption_rows
                        .iter()
                        .filter(|r| r.consumption_version_id == c.id)
                        .collect::<Vec<_>>();
                    if rows.len() != allocation.candidates.len() {
                        return Err(bad());
                    }
                    for expected in &allocation.candidates {
                        let matching = rows
                            .iter()
                            .filter(|r| r.row.position == expected.position)
                            .collect::<Vec<_>>();
                        if matching.len() != 1
                            || !equal(&matching[0].row, expected)?
                            || !used_rows.insert((&c.id, expected.position))
                        {
                            return Err(bad());
                        }
                        // Until correction replay is introduced, any candidate relation freezes the sale count.
                        if state.sale_versions.iter().any(|v| {
                            v.sale_id == expected.sale_id && sequence(&v.seq).is_ok_and(|s| s > seq)
                        }) {
                            return Err(bad());
                        }
                    }
                }
            }
        }
    }
    if used_observations.len() != state.observations.len()
        || used_versions.len() != state.versions.len()
        || used_consumptions.len() != state.consumption_versions.len()
        || used_rows.len() != state.consumption_rows.len()
        || state.confirmations.iter().any(|c| {
            state
                .business_records
                .iter()
                .any(|r| r.id == c.record_id && r.kind == BusinessKind::Stocktake)
                && !confirmation_ids.contains(&c.id)
        })
    {
        return Err(bad());
    }
    Ok(())
}

pub(super) fn validate_stocktake_schema(connection: &Connection) -> anyhow::Result<()> {
    let rows = snapshot_rows(connection)?;
    let business = business::snapshot_rows(connection)?;
    let sales = sale::snapshot_rows(connection)?;
    let mutations = connection
        .prepare("SELECT sequence,operation,request_id FROM mutation_requests ORDER BY sequence")?
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?.to_string(),
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let other=connection.prepare("SELECT confirmation_id FROM business_catalog_price_results UNION ALL SELECT confirmation_id FROM purchase_order_versions UNION ALL SELECT confirmation_id FROM receipts UNION ALL SELECT confirmation_id FROM productions UNION ALL SELECT confirmation_id FROM sale_versions UNION ALL SELECT confirmation_id FROM wastes")?.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let sequence = connection.query_row(
        "SELECT COALESCE(MAX(sequence),0) FROM snapshot_outbox",
        [],
        |r| r.get(0),
    )?;
    validate_snapshot(Validation {
        batches: &rows.batches,
        observations: &rows.observations,
        versions: &rows.versions,
        heads: &rows.heads,
        consumption_versions: &rows.consumption_versions,
        consumption_heads: &rows.consumption_heads,
        consumption_rows: &rows.consumption_rows,
        holdings: &holding::versions(connection, None)?,
        material_versions: &catalog::versions(connection, None)?,
        prices: &catalog::prices(connection, None)?,
        sales: &sales.records,
        sale_cost_rows: &sales.cost_rows,
        sale_versions: &sales.versions,
        business_records: &business.records,
        revisions: &business.revisions,
        confirmations: &business.confirmations,
        other_confirmation_ids: &other.iter().map(String::as_str).collect::<Vec<_>>(),
        mutations: &mutations
            .iter()
            .map(|(s, o, i)| (s.as_str(), o.as_str(), i.as_str()))
            .collect::<Vec<_>>(),
        sequence,
    })?;
    Ok(())
}
