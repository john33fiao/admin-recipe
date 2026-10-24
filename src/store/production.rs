use super::{Store, StoreError, business, catalog, holding, snapshot, unavailable};
use crate::{
    business::{BusinessEffect, BusinessIssue, BusinessResult},
    domain::{Money, Quantity},
    production::*,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RecordRow {
    pub id: String,
    pub confirmation_id: String,
    pub business_record_id: String,
    pub note: String,
    pub total_input_value: String,
    pub recorded_at: String,
    pub seq: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InputRow {
    pub production_id: String,
    pub effect_id: String,
    pub row: ProductionInputRow,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OutputRow {
    pub production_id: String,
    pub effect_id: String,
    pub row: ProductionOutputRow,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EffectRow {
    pub id: String,
    pub production_id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub input_quantity: String,
    pub input_value: String,
    pub output_quantity: String,
    pub output_value: String,
    pub recorded_at: String,
    pub seq: String,
}
pub(super) struct SnapshotRows {
    pub records: Vec<RecordRow>,
    pub inputs: Vec<InputRow>,
    pub outputs: Vec<OutputRow>,
    pub effects: Vec<EffectRow>,
}

pub(super) struct Validation<'a> {
    pub records: &'a [RecordRow],
    pub inputs: &'a [InputRow],
    pub outputs: &'a [OutputRow],
    pub effects: &'a [EffectRow],
    pub holdings: &'a [holding::HoldingVersion],
    pub business_records: &'a [business::Record],
    pub revisions: &'a [crate::business::BusinessRevision],
    pub confirmations: &'a [business::ConfirmationRow],
    pub material_versions: &'a [crate::catalog::MaterialVersion],
    pub materials: &'a [super::Material],
    pub other_confirmation_ids: &'a [&'a str],
    pub mutations: &'a [(&'a str, &'a str)],
    pub sequence: i64,
}

fn records(connection: &Connection, id: Option<&str>) -> Result<Vec<RecordRow>, StoreError> {
    let rows=connection.prepare("SELECT p.id,p.confirmation_id,c.record_id,p.note,p.total_input_value_micros,p.recorded_at,p.sequence FROM productions p JOIN business_confirmations c ON c.id=p.confirmation_id WHERE (?1 IS NULL OR p.id=?1) ORDER BY p.sequence,p.id").map_err(unavailable)?
        .query_map([id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,i64>(4)?,r.get::<_,String>(5)?,r.get::<_,i64>(6)?))).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    rows.into_iter()
        .map(
            |(id, confirmation_id, business_record_id, note, v, recorded_at, seq)| {
                Ok(RecordRow {
                    id,
                    confirmation_id,
                    business_record_id,
                    note,
                    total_input_value: Money::from_micros(v).map_err(unavailable)?.decimal(),
                    recorded_at,
                    seq: seq.to_string(),
                })
            },
        )
        .collect()
}
pub(super) fn effects(
    connection: &Connection,
    production: Option<&str>,
    material: Option<&str>,
) -> Result<Vec<EffectRow>, StoreError> {
    let rows=connection.prepare("SELECT e.id,e.production_id,e.material_id,e.material_version_id,e.input_quantity_micros,e.input_value_micros,e.output_quantity_micros,e.output_value_micros,p.recorded_at,p.sequence FROM production_effects e JOIN productions p ON p.id=e.production_id WHERE (?1 IS NULL OR e.production_id=?1) AND (?2 IS NULL OR e.material_id=?2) ORDER BY p.sequence,e.id").map_err(unavailable)?
        .query_map(params![production,material],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,i64>(4)?,r.get::<_,i64>(5)?,r.get::<_,i64>(6)?,r.get::<_,i64>(7)?,r.get::<_,String>(8)?,r.get::<_,i64>(9)?))).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    rows.into_iter()
        .map(
            |(
                id,
                production_id,
                material_id,
                material_version_id,
                iq,
                iv,
                oq,
                ov,
                recorded_at,
                seq,
            )| {
                Ok(EffectRow {
                    id,
                    production_id,
                    material_id,
                    material_version_id,
                    input_quantity: Quantity::from_micros(iq).map_err(unavailable)?.decimal(),
                    input_value: Money::from_micros(iv).map_err(unavailable)?.decimal(),
                    output_quantity: Quantity::from_micros(oq).map_err(unavailable)?.decimal(),
                    output_value: Money::from_micros(ov).map_err(unavailable)?.decimal(),
                    recorded_at,
                    seq: seq.to_string(),
                })
            },
        )
        .collect()
}
fn inputs(connection: &Connection, id: Option<&str>) -> Result<Vec<InputRow>, StoreError> {
    let rows=connection.prepare("SELECT r.production_id,r.effect_id,r.position,r.material_id,r.material_version_id,m.name,r.quantity_micros,r.unit,r.input_quantity,r.input_unit,r.value_micros FROM production_inputs r JOIN material_versions m ON m.id=r.material_version_id AND m.material_id=r.material_id WHERE (?1 IS NULL OR r.production_id=?1) ORDER BY r.production_id,r.position").map_err(unavailable)?
        .query_map([id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,i64>(6)?,r.get::<_,String>(7)?,r.get::<_,String>(8)?,r.get::<_,String>(9)?,r.get::<_,i64>(10)?))).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    rows.into_iter()
        .map(
            |(
                production_id,
                effect_id,
                position,
                material_id,
                material_version_id,
                material_name,
                q,
                unit,
                input_quantity,
                input_unit,
                v,
            )| {
                Ok(InputRow {
                    production_id,
                    effect_id,
                    row: ProductionInputRow {
                        position: position.to_string(),
                        material_id,
                        material_version_id,
                        material_name,
                        quantity: Quantity::from_micros(q).map_err(unavailable)?.decimal(),
                        unit,
                        input_quantity,
                        input_unit,
                        value: Money::from_micros(v).map_err(unavailable)?.decimal(),
                    },
                })
            },
        )
        .collect()
}
fn outputs(connection: &Connection, id: Option<&str>) -> Result<Vec<OutputRow>, StoreError> {
    let rows=connection.prepare("SELECT r.production_id,r.effect_id,r.position,r.material_id,r.material_version_id,m.name,r.quantity_micros,r.unit,r.input_quantity,r.input_unit,r.value_micros,r.allocation_micros FROM production_outputs r JOIN material_versions m ON m.id=r.material_version_id AND m.material_id=r.material_id WHERE (?1 IS NULL OR r.production_id=?1) ORDER BY r.production_id,r.position").map_err(unavailable)?
        .query_map([id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,r.get::<_,String>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,i64>(6)?,r.get::<_,String>(7)?,r.get::<_,String>(8)?,r.get::<_,String>(9)?,r.get::<_,i64>(10)?,r.get::<_,i64>(11)?))).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    rows.into_iter()
        .map(
            |(
                production_id,
                effect_id,
                position,
                material_id,
                material_version_id,
                material_name,
                q,
                unit,
                input_quantity,
                input_unit,
                v,
                a,
            )| {
                Ok(OutputRow {
                    production_id,
                    effect_id,
                    row: ProductionOutputRow {
                        position: position.to_string(),
                        material_id,
                        material_version_id,
                        material_name,
                        quantity: Quantity::from_micros(q).map_err(unavailable)?.decimal(),
                        unit,
                        input_quantity,
                        input_unit,
                        value: Money::from_micros(v).map_err(unavailable)?.decimal(),
                        allocation_percent: Quantity::from_micros(a)
                            .map_err(unavailable)?
                            .decimal(),
                    },
                })
            },
        )
        .collect()
}
pub(super) fn snapshot_rows(connection: &Connection) -> Result<SnapshotRows, StoreError> {
    Ok(SnapshotRows {
        records: records(connection, None)?,
        inputs: inputs(connection, None)?,
        outputs: outputs(connection, None)?,
        effects: effects(connection, None, None)?,
    })
}
pub(super) fn material_name(
    connection: &Connection,
    data: &DraftProductionV1,
) -> Result<Option<String>, StoreError> {
    let Some(row) = data.outputs.first() else {
        return Ok(None);
    };
    connection
        .query_row(
            "SELECT name FROM material_versions WHERE material_id=?1 AND id=?2",
            params![row.material_id, row.material_version_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(unavailable)
}
pub(super) fn inspect_production(
    connection: &Connection,
    data: &DraftProductionV1,
) -> Result<(Vec<BusinessIssue>, Option<BusinessEffect>), StoreError> {
    let mut issues = Vec::new();
    let mut contexts = Vec::new();
    let mut seen = HashSet::new();
    let refs = data
        .inputs
        .iter()
        .enumerate()
        .map(|(p, r)| {
            (
                format!("inputs.{p}"),
                &r.material_id,
                &r.material_version_id,
            )
        })
        .chain(data.outputs.iter().enumerate().map(|(p, r)| {
            (
                format!("outputs.{p}"),
                &r.material_id,
                &r.material_version_id,
            )
        }));
    for (field, id, version) in refs {
        let current = match id {
            Some(id) => catalog::current_version(connection, id)?,
            None => None,
        };
        let Some(current) = current else {
            issues.push(BusinessIssue {
                field: format!("{field}.material_id"),
                code: "missing_reference".into(),
                message: "생산의 모든 행에서 품목을 선택하세요".into(),
            });
            continue;
        };
        if version.as_deref() != Some(current.id.as_str()) {
            issues.push(BusinessIssue {
                field: format!("{field}.material_version_id"),
                code: "stale_reference".into(),
                message: "최신 품목 버전을 확인하세요".into(),
            });
        }
        if seen.insert(current.material_id.clone()) {
            contexts.push(ProductionMaterialContext {
                holding: holding::current_input(connection, &current.material_id)?,
                material_id: current.material_id,
                material_version_id: current.id,
                material_name: current.name,
                unit: current.canonical_unit,
            });
        }
    }
    if !issues.is_empty() {
        return Ok((issues, None));
    }
    match project_production(data, &contexts) {
        Ok(ProductionCheck::Ready(p)) => Ok((
            issues,
            Some(BusinessEffect::Production {
                inputs: p.inputs,
                outputs: p.outputs,
                materials: p.materials,
                total_input_value: p.total_input_value,
            }),
        )),
        Ok(ProductionCheck::NeedsReconciliation(conflicts)) => {
            issues.extend(conflicts.into_iter().map(|c| BusinessIssue {
                field: c.field.clone(),
                code: "needs_reconciliation".into(),
                message: c.message(),
            }));
            Ok((issues, None))
        }
        Err(e) => Ok((
            vec![BusinessIssue {
                field: "production".into(),
                code: "invalid_input".into(),
                message: e.to_string(),
            }],
            None,
        )),
    }
}
pub(super) fn apply_production(
    connection: &Connection,
    data: &DraftProductionV1,
    confirmation_id: &str,
    seq: i64,
    time: &str,
) -> Result<(), StoreError> {
    let (issues, effect) = inspect_production(connection, data)?;
    if !issues.is_empty() {
        return Err(StoreError::Validation(
            issues
                .iter()
                .map(|i| i.message.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        ));
    }
    let Some(BusinessEffect::Production {
        inputs,
        outputs,
        materials,
        total_input_value,
    }) = effect
    else {
        return Err(unavailable("생산 미리보기가 없습니다"));
    };
    let id = Uuid::new_v4().to_string();
    connection.execute("INSERT INTO productions(id,confirmation_id,note,total_input_value_micros,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6)",params![id,confirmation_id,data.note,Money::parse(&total_input_value).map_err(unavailable)?.micros(),time,seq]).map_err(unavailable)?;
    let mut effect_ids = HashMap::new();
    for material in materials {
        let effect_id = Uuid::new_v4().to_string();
        let before = holding::current_input(connection, &material.material_id)?;
        connection.execute("INSERT INTO production_effects(id,production_id,material_id,material_version_id,input_quantity_micros,input_value_micros,output_quantity_micros,output_value_micros) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![effect_id,id,material.material_id,material.material_version_id,Quantity::parse(&material.input_quantity).map_err(unavailable)?.micros(),Money::parse(&material.input_value).map_err(unavailable)?.micros(),Quantity::parse(&material.output_quantity).map_err(unavailable)?.micros(),Money::parse(&material.output_value).map_err(unavailable)?.micros()]).map_err(unavailable)?;
        holding::append(
            connection,
            holding::AppendHolding {
                material_id: &material.material_id,
                material_version_id: &material.material_version_id,
                receipt_id: None,
                production_effect_id: Some(&effect_id),
                before: &before,
                after: &material.after,
                seq,
                time,
            },
        )?;
        effect_ids.insert(material.material_id, effect_id);
    }
    for row in inputs {
        connection.execute("INSERT INTO production_inputs(production_id,position,effect_id,material_id,material_version_id,quantity_micros,unit,input_quantity,input_unit,value_micros) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![id,row.position.parse::<i64>().map_err(unavailable)?,effect_ids.get(&row.material_id),row.material_id,row.material_version_id,Quantity::parse(&row.quantity).map_err(unavailable)?.micros(),row.unit,row.input_quantity,row.input_unit,Money::parse(&row.value).map_err(unavailable)?.micros()]).map_err(unavailable)?;
    }
    for row in outputs {
        connection.execute("INSERT INTO production_outputs(production_id,position,effect_id,material_id,material_version_id,quantity_micros,unit,input_quantity,input_unit,value_micros,allocation_micros) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![id,row.position.parse::<i64>().map_err(unavailable)?,effect_ids.get(&row.material_id),row.material_id,row.material_version_id,Quantity::parse(&row.quantity).map_err(unavailable)?.micros(),row.unit,row.input_quantity,row.input_unit,Money::parse(&row.value).map_err(unavailable)?.micros(),AllocationPercent::parse(&row.allocation_percent).map_err(unavailable)?.micros()]).map_err(unavailable)?;
    }
    Ok(())
}
pub(super) fn result(
    connection: &Connection,
    confirmation_id: &str,
) -> Result<BusinessResult, StoreError> {
    connection
        .query_row(
            "SELECT id FROM productions WHERE confirmation_id=?1",
            [confirmation_id],
            |r| {
                Ok(BusinessResult::Production {
                    production_id: r.get(0)?,
                })
            },
        )
        .map_err(unavailable)
}
impl Store {
    pub fn production_list(&self, filter: ProductionFilter) -> Result<ProductionList, StoreError> {
        let material = filter
            .material_id
            .map(|id| catalog::uuid(&id, "재료 ID"))
            .transpose()?;
        let rows=self.connection.prepare("SELECT p.id,p.confirmation_id,c.record_id,m.name,(SELECT COUNT(*) FROM production_inputs i WHERE i.production_id=p.id),(SELECT COUNT(*) FROM production_outputs o WHERE o.production_id=p.id),p.total_input_value_micros,p.recorded_at,p.sequence FROM productions p JOIN business_confirmations c ON c.id=p.confirmation_id JOIN production_outputs first ON first.production_id=p.id AND first.position=0 JOIN material_versions m ON m.id=first.material_version_id WHERE (?1 IS NULL OR EXISTS(SELECT 1 FROM production_effects e WHERE e.production_id=p.id AND e.material_id=?1)) ORDER BY p.sequence DESC,p.id").map_err(unavailable)?
            .query_map([material],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?,r.get::<_,i64>(4)?,r.get::<_,i64>(5)?,r.get::<_,i64>(6)?,r.get::<_,String>(7)?,r.get::<_,i64>(8)?))).map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
        let items = rows
            .into_iter()
            .map(
                |(
                    id,
                    confirmation_id,
                    business_record_id,
                    first_output_name,
                    input_count,
                    output_count,
                    v,
                    recorded_at,
                    seq,
                )| {
                    Ok(ProductionSummary {
                        id,
                        confirmation_id,
                        business_record_id,
                        first_output_name,
                        input_count: usize::try_from(input_count).map_err(unavailable)?,
                        output_count: usize::try_from(output_count).map_err(unavailable)?,
                        total_input_value: Money::from_micros(v).map_err(unavailable)?.decimal(),
                        recorded_at,
                        seq: seq.to_string(),
                    })
                },
            )
            .collect::<Result<_, StoreError>>()?;
        Ok(ProductionList {
            items,
            state: catalog::state(&self.connection)?,
        })
    }
    pub fn production_detail(&self, id: &str) -> Result<Option<ProductionDetail>, StoreError> {
        let id = catalog::uuid(id, "생산 ID")?;
        let Some(row) = records(&self.connection, Some(&id))?.pop() else {
            return Ok(None);
        };
        let item = ProductionRecord {
            id: row.id,
            confirmation_id: row.confirmation_id,
            business_record_id: row.business_record_id,
            note: row.note,
            recorded_at: row.recorded_at,
            seq: row.seq,
            total_input_value: row.total_input_value,
            inputs: inputs(&self.connection, Some(&id))?
                .into_iter()
                .map(|r| r.row)
                .collect(),
            outputs: outputs(&self.connection, Some(&id))?
                .into_iter()
                .map(|r| r.row)
                .collect(),
        };
        let mut materials = Vec::new();
        for effect in effects(&self.connection, Some(&id), None)? {
            let (before, after) = holding::effect_holdings(&self.connection, &effect.id)?;
            let unit: String = self
                .connection
                .query_row(
                    "SELECT canonical_unit FROM material_versions WHERE material_id=?1 AND id=?2",
                    params![effect.material_id, effect.material_version_id],
                    |r| r.get(0),
                )
                .map_err(unavailable)?;
            materials.push(ProductionMaterialResult::new(
                effect.id,
                ProductionMaterialEffect {
                    material_id: effect.material_id,
                    material_version_id: effect.material_version_id,
                    unit,
                    before: before.as_ref().map(holding::amounts),
                    after: holding::amounts(&after),
                    basis_before: before.as_ref().map(holding::basis).transpose()?,
                    observation: None,
                    input_quantity: effect.input_quantity,
                    input_value: effect.input_value,
                    output_quantity: effect.output_quantity,
                    output_value: effect.output_value,
                },
            ));
        }
        Ok(Some(ProductionDetail {
            item,
            materials,
            state: catalog::state(&self.connection)?,
        }))
    }
}

pub(super) fn migrate_productions(connection: &Connection) -> anyhow::Result<()> {
    let retained = connection
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (seq, payload) in retained {
        snapshot::validate_source(&payload, seq, 7)?;
    }
    connection.execute_batch("PRAGMA defer_foreign_keys=ON;
        CREATE TABLE productions(
            id TEXT PRIMARY KEY NOT NULL,
            confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),
            note TEXT NOT NULL,
            total_input_value_micros INTEGER NOT NULL CHECK(total_input_value_micros>=0 AND total_input_value_micros<=9000000000000000000),
            recorded_at TEXT NOT NULL,
            sequence INTEGER NOT NULL UNIQUE REFERENCES snapshot_outbox(sequence));
        CREATE TABLE production_effects(
            id TEXT PRIMARY KEY NOT NULL,
            production_id TEXT NOT NULL REFERENCES productions(id),
            material_id TEXT NOT NULL,
            material_version_id TEXT NOT NULL,
            input_quantity_micros INTEGER NOT NULL CHECK(input_quantity_micros>=0 AND input_quantity_micros<=9000000000000000),
            input_value_micros INTEGER NOT NULL CHECK(input_value_micros>=0 AND input_value_micros<=9000000000000000000),
            output_quantity_micros INTEGER NOT NULL CHECK(output_quantity_micros>=0 AND output_quantity_micros<=9000000000000000),
            output_value_micros INTEGER NOT NULL CHECK(output_value_micros>=0 AND output_value_micros<=9000000000000000000),
            UNIQUE(production_id,material_id),
            UNIQUE(material_id,id),
            UNIQUE(production_id,material_id,material_version_id,id),
            FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),
            CHECK(input_quantity_micros>0 OR output_quantity_micros>0),
            CHECK(input_quantity_micros>0 OR input_value_micros=0),
            CHECK(output_quantity_micros>0 OR output_value_micros=0));
        CREATE TABLE production_inputs(
            production_id TEXT NOT NULL REFERENCES productions(id),
            position INTEGER NOT NULL CHECK(position>=0 AND position<100),
            effect_id TEXT NOT NULL,
            material_id TEXT NOT NULL,
            material_version_id TEXT NOT NULL,
            quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),
            unit TEXT NOT NULL,input_quantity TEXT NOT NULL,input_unit TEXT NOT NULL,
            value_micros INTEGER NOT NULL CHECK(value_micros>=0 AND value_micros<=9000000000000000000),
            PRIMARY KEY(production_id,position),
            FOREIGN KEY(production_id,material_id,material_version_id,effect_id) REFERENCES production_effects(production_id,material_id,material_version_id,id));
        CREATE TABLE production_outputs(
            production_id TEXT NOT NULL REFERENCES productions(id),
            position INTEGER NOT NULL CHECK(position>=0 AND position<100),
            effect_id TEXT NOT NULL,
            material_id TEXT NOT NULL,
            material_version_id TEXT NOT NULL,
            quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),
            unit TEXT NOT NULL,input_quantity TEXT NOT NULL,input_unit TEXT NOT NULL,
            value_micros INTEGER NOT NULL CHECK(value_micros>=0 AND value_micros<=9000000000000000000),
            allocation_micros INTEGER NOT NULL CHECK(allocation_micros>=0 AND allocation_micros<=100000000),
            PRIMARY KEY(production_id,position),
            FOREIGN KEY(production_id,material_id,material_version_id,effect_id) REFERENCES production_effects(production_id,material_id,material_version_id,id));
        CREATE TABLE holding_versions_next(
            id TEXT PRIMARY KEY NOT NULL,material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,previous_version_id TEXT,
            receipt_id TEXT UNIQUE REFERENCES receipts(id),
            baseline_receipt_id TEXT REFERENCES receipts(id),
            baseline_sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),
            quantity_micros INTEGER NOT NULL CHECK(quantity_micros>=0 AND quantity_micros<=9000000000000000),
            value_micros INTEGER NOT NULL CHECK(value_micros>=0 AND value_micros<=9000000000000000000),
            recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),
            production_effect_id TEXT UNIQUE,baseline_production_effect_id TEXT,
            UNIQUE(material_id,id),
            FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),
            FOREIGN KEY(material_id,previous_version_id) REFERENCES holding_versions_next(material_id,id),
            FOREIGN KEY(material_id,production_effect_id) REFERENCES production_effects(material_id,id),
            FOREIGN KEY(material_id,baseline_production_effect_id) REFERENCES production_effects(material_id,id),
            CHECK((receipt_id IS NULL)<>(production_effect_id IS NULL)),
            CHECK((baseline_receipt_id IS NULL)<>(baseline_production_effect_id IS NULL)),
            CHECK(quantity_micros>0 OR value_micros=0));
        INSERT INTO holding_versions_next SELECT *,NULL,NULL FROM holding_versions;
        CREATE TABLE holding_heads_next(
            material_id TEXT PRIMARY KEY NOT NULL REFERENCES materials(id),
            current_version_id TEXT NOT NULL,last_valid_version_id TEXT NOT NULL,
            FOREIGN KEY(material_id,current_version_id) REFERENCES holding_versions_next(material_id,id),
            FOREIGN KEY(material_id,last_valid_version_id) REFERENCES holding_versions_next(material_id,id));
        INSERT INTO holding_heads_next SELECT * FROM holding_heads;
        DROP TABLE holding_heads;
        DROP TABLE holding_versions;
        ALTER TABLE holding_versions_next RENAME TO holding_versions;
        ALTER TABLE holding_heads_next RENAME TO holding_heads;
        CREATE INDEX holding_versions_material ON holding_versions(material_id,sequence);
        CREATE INDEX production_effects_material ON production_effects(material_id,production_id);
        PRAGMA user_version=8;")?;
    Ok(())
}

pub(super) fn validate_snapshot(state: Validation<'_>) -> Result<(), StoreError> {
    use crate::business::{BusinessKind, BusinessPayload, BusinessStatus};
    use crate::holding::HoldingInput;
    let bad = || unavailable("생산의 원행·확정·품목·보유·배분 근거가 일치하지 않습니다");
    let records = state
        .business_records
        .iter()
        .map(|r| (&r.id, r))
        .collect::<HashMap<_, _>>();
    let confirmations = state
        .confirmations
        .iter()
        .map(|c| (&c.id, c))
        .collect::<HashMap<_, _>>();
    let revisions = state
        .revisions
        .iter()
        .map(|r| (&r.id, r))
        .collect::<HashMap<_, _>>();
    let material_versions = state
        .material_versions
        .iter()
        .map(|m| (&m.id, m))
        .collect::<HashMap<_, _>>();
    let materials = state
        .materials
        .iter()
        .map(|m| &m.id)
        .collect::<HashSet<_>>();
    if records.len() != state.business_records.len()
        || confirmations.len() != state.confirmations.len()
        || revisions.len() != state.revisions.len()
        || material_versions.len() != state.material_versions.len()
        || materials.len() != state.materials.len()
    {
        return Err(bad());
    }
    let mut production_ids = HashSet::new();
    let mut confirmation_ids = HashSet::new();
    let mut effect_ids = HashSet::new();
    let (mut input_count, mut output_count) = (0, 0);
    let mut production_sequences = HashSet::new();
    for item in state.records {
        for id in [&item.id, &item.confirmation_id, &item.business_record_id] {
            Uuid::parse_str(id).map_err(unavailable)?;
        }
        chrono::DateTime::parse_from_rfc3339(&item.recorded_at).map_err(unavailable)?;
        let seq = item.seq.parse::<i64>().map_err(unavailable)?;
        let confirmation = confirmations.get(&item.confirmation_id).ok_or_else(bad)?;
        let record = records.get(&item.business_record_id).ok_or_else(bad)?;
        let revision = revisions.get(&confirmation.revision_id).ok_or_else(bad)?;
        if !production_ids.insert(&item.id)
            || !confirmation_ids.insert(&item.confirmation_id)
            || !production_sequences.insert(seq)
            || seq <= 0
            || seq.to_string() != item.seq
            || seq > state.sequence
            || state
                .mutations
                .iter()
                .filter(|(s, op)| *s == item.seq && *op == "business.confirm")
                .count()
                != 1
            || state
                .other_confirmation_ids
                .contains(&item.confirmation_id.as_str())
            || confirmation.record_id != item.business_record_id
            || confirmation.seq != item.seq
            || confirmation.recorded_at != item.recorded_at
            || record.kind != BusinessKind::Production
            || record.corrects_confirmation_id.is_some()
            || revision.record_id != record.id
            || revision.status != BusinessStatus::Confirmed
            || revision.seq != item.seq
            || revision.recorded_at != item.recorded_at
        {
            return Err(bad());
        }
        let BusinessPayload::Production { version: 1, data } = &revision.payload else {
            return Err(bad());
        };
        if item.note != data.note {
            return Err(bad());
        }
        let effects = state
            .effects
            .iter()
            .filter(|e| e.production_id == item.id)
            .collect::<Vec<_>>();
        let mut by_material = HashMap::new();
        let mut contexts = Vec::new();
        for effect in &effects {
            Uuid::parse_str(&effect.id).map_err(unavailable)?;
            let material = material_versions
                .get(&effect.material_version_id)
                .ok_or_else(bad)?;
            if !effect_ids.insert(&effect.id)
                || by_material.insert(&effect.material_id, *effect).is_some()
                || effect.seq != item.seq
                || effect.recorded_at != item.recorded_at
                || material.material_id != effect.material_id
                || !materials.contains(&effect.material_id)
                || !material_existed_before(material, seq)
            {
                return Err(bad());
            }
            let latest = state
                .material_versions
                .iter()
                .filter(|m| m.material_id == effect.material_id)
                .filter(|m| material_existed_before(m, seq))
                .max_by_key(|m| {
                    m.seq
                        .as_deref()
                        .and_then(|s| s.parse::<i64>().ok())
                        .unwrap_or(0)
                })
                .ok_or_else(bad)?;
            if latest.id != material.id {
                return Err(bad());
            }
            let actuals = state
                .holdings
                .iter()
                .filter(|h| h.production_effect_id.as_ref() == Some(&effect.id))
                .collect::<Vec<_>>();
            if actuals.len() != 1 {
                return Err(bad());
            }
            let after = actuals[0];
            let before = after
                .previous_version_id
                .as_ref()
                .map(|id| state.holdings.iter().find(|h| &h.id == id).ok_or_else(bad))
                .transpose()?;
            let holding = match before {
                Some(h) => HoldingInput {
                    basis: Some(holding::basis(h)?),
                    observation: None,
                    quantity: Some(Quantity::parse(&h.quantity).map_err(unavailable)?),
                    value: Some(Money::parse(&h.value).map_err(unavailable)?),
                    source_id: Some(h.id.clone()),
                    last_valid: None,
                },
                None => HoldingInput::default(),
            };
            contexts.push(ProductionMaterialContext {
                material_id: effect.material_id.clone(),
                material_version_id: effect.material_version_id.clone(),
                material_name: material.name.clone(),
                unit: material.canonical_unit.clone(),
                holding,
            });
        }
        let ProductionCheck::Ready(expected) =
            project_production(data, &contexts).map_err(unavailable)?
        else {
            return Err(bad());
        };
        if expected.total_input_value != item.total_input_value
            || expected.materials.len() != effects.len()
        {
            return Err(bad());
        }
        let mut actual_inputs = state
            .inputs
            .iter()
            .filter(|r| r.production_id == item.id)
            .collect::<Vec<_>>();
        let mut actual_outputs = state
            .outputs
            .iter()
            .filter(|r| r.production_id == item.id)
            .collect::<Vec<_>>();
        actual_inputs.sort_by_key(|r| r.row.position.parse::<usize>().unwrap_or(usize::MAX));
        actual_outputs.sort_by_key(|r| r.row.position.parse::<usize>().unwrap_or(usize::MAX));
        if actual_inputs.len() != expected.inputs.len()
            || actual_outputs.len() != expected.outputs.len()
        {
            return Err(bad());
        }
        for (row, expected) in actual_inputs.iter().zip(&expected.inputs) {
            if &row.row != expected
                || by_material.get(&row.row.material_id).map(|e| &e.id) != Some(&row.effect_id)
            {
                return Err(bad());
            }
        }
        for (row, expected) in actual_outputs.iter().zip(&expected.outputs) {
            if &row.row != expected
                || by_material.get(&row.row.material_id).map(|e| &e.id) != Some(&row.effect_id)
            {
                return Err(bad());
            }
        }
        input_count += actual_inputs.len();
        output_count += actual_outputs.len();
        for expected in expected.materials {
            let actual = by_material.get(&expected.material_id).ok_or_else(bad)?;
            let after = state
                .holdings
                .iter()
                .find(|h| h.production_effect_id.as_ref() == Some(&actual.id))
                .ok_or_else(bad)?;
            if actual.material_version_id != expected.material_version_id
                || actual.input_quantity != expected.input_quantity
                || actual.input_value != expected.input_value
                || actual.output_quantity != expected.output_quantity
                || actual.output_value != expected.output_value
                || after.quantity != expected.after.quantity
                || Some(after.value.as_str()) != expected.after.value.as_deref()
                || after.previous_version_id.as_ref()
                    != expected.before.as_ref().and_then(|b| b.source_id.as_ref())
            {
                return Err(bad());
            }
        }
    }
    if effect_ids.len() != state.effects.len()
        || input_count != state.inputs.len()
        || output_count != state.outputs.len()
    {
        return Err(bad());
    }
    for c in state.confirmations {
        if records
            .get(&c.record_id)
            .is_some_and(|r| r.kind == BusinessKind::Production)
            && !confirmation_ids.contains(&c.id)
        {
            return Err(bad());
        }
    }
    Ok(())
}

fn material_existed_before(material: &crate::catalog::MaterialVersion, sequence: i64) -> bool {
    match material.seq.as_deref() {
        Some(text) => text
            .parse::<i64>()
            .is_ok_and(|seq| seq > 0 && seq < sequence && seq.to_string() == text),
        None => material.source == "legacy_migration",
    }
}

pub(super) fn validate_production_schema(connection: &Connection) -> anyhow::Result<()> {
    let rows = snapshot_rows(connection)?;
    let business = business::snapshot_rows(connection)?;
    let holdings = holding::versions(connection, None)?;
    let versions = catalog::versions(connection, None)?;
    let materials = super::read_materials(connection)?;
    let others=connection.prepare("SELECT confirmation_id FROM business_catalog_price_results UNION ALL SELECT confirmation_id FROM purchase_order_versions UNION ALL SELECT confirmation_id FROM receipts")?
        .query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    let mutations = connection
        .prepare("SELECT sequence,operation FROM mutation_requests ORDER BY sequence")?
        .query_map([], |r| {
            Ok((r.get::<_, i64>(0)?.to_string(), r.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let sequence: i64 = connection.query_row(
        "SELECT COALESCE(MAX(sequence),0) FROM snapshot_outbox",
        [],
        |r| r.get(0),
    )?;
    validate_snapshot(Validation {
        records: &rows.records,
        inputs: &rows.inputs,
        outputs: &rows.outputs,
        effects: &rows.effects,
        holdings: &holdings,
        business_records: &business.records,
        revisions: &business.revisions,
        confirmations: &business.confirmations,
        material_versions: &versions,
        materials: &materials,
        other_confirmation_ids: &others.iter().map(String::as_str).collect::<Vec<_>>(),
        mutations: &mutations
            .iter()
            .map(|(seq, op)| (seq.as_str(), op.as_str()))
            .collect::<Vec<_>>(),
        sequence,
    })?;
    Ok(())
}
