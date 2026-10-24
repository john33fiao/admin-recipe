//! Immutable corrections and their resolved projections. Original domain rows
//! remain evidence; operational readers select a separately versioned head.
use super::{Store, StoreError, business, catalog, unavailable};
use crate::business::{BusinessEffect, BusinessIssue, BusinessResult};
use crate::correction::*;
use chrono::NaiveDate;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

mod history;
mod validation;
mod write;
pub(super) use history::from_snapshot as history_from_snapshot;
pub(super) use history::load as effective_history;
pub(super) use validation::validate_snapshot;
pub(super) use write::apply_correction;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EventVersionRow {
    pub id: String,
    pub sale_version_id: Option<String>,
    pub correction_id: String,
    pub previous_version_id: Option<String>,
    pub previous_original_version_id: Option<String>,
    pub reference: CorrectionEventRef,
    pub after: Option<CorrectionEventValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct EventHead {
    pub kind: CorrectionSubjectKind,
    pub original_id: String,
    pub current_version_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Projection {
    Material {
        change: CorrectionMaterialChange,
        holding_version_id: Option<String>,
        price_basis_id: Option<String>,
    },
    Observation {
        change: CorrectionObservationChange,
        observation_version_id: Option<String>,
        price_basis_id: Option<String>,
    },
    Allocation {
        change: CorrectionAllocationChange,
        consumption_version_id: Option<String>,
    },
    Inclusion {
        change: CorrectionInclusionChange,
        active_link_version_ids: Vec<String>,
        links: Vec<InclusionVersionRow>,
    },
    OrderClosure {
        change: CorrectionOrderClosureChange,
        order_version_id: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InclusionVersionRow {
    pub id: String,
    pub link_id: String,
    pub previous_version_id: Option<String>,
    pub reservation_id: String,
    pub observation_version_id: String,
    pub selection_position: usize,
    pub portion_position: usize,
    pub portion: crate::reconciliation::InclusionPortionEffect,
}

pub(super) fn ref_id(reference: &CorrectionPlanRef) -> &str {
    match reference {
        CorrectionPlanRef::Existing { id, .. } | CorrectionPlanRef::Proposed { id, .. } => id,
    }
}

impl Projection {
    pub(super) fn kind(&self) -> &'static str {
        match self {
            Self::Material { .. } => "material",
            Self::Observation { .. } => "observation",
            Self::Allocation { .. } => "allocation",
            Self::Inclusion { .. } => "inclusion",
            Self::OrderClosure { .. } => "order_closure",
        }
    }
    pub(super) fn subject_id(&self) -> &str {
        match self {
            Self::Material { change, .. } => &change.material_id,
            Self::Observation { change, .. } => ref_id(&change.reference),
            Self::Allocation { change, .. } => ref_id(&change.observation),
            Self::Inclusion { change, .. } => &change.receipt_id,
            Self::OrderClosure { change, .. } => &change.order_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProjectionVersionRow {
    pub id: String,
    pub correction_id: String,
    pub previous_version_id: Option<String>,
    pub recorded_at: String,
    pub seq: String,
    pub projection: Projection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ProjectionKind {
    Material,
    Observation,
    Allocation,
    Inclusion,
    OrderClosure,
}

impl ProjectionKind {
    pub(super) fn text(self) -> &'static str {
        match self {
            Self::Material => "material",
            Self::Observation => "observation",
            Self::Allocation => "allocation",
            Self::Inclusion => "inclusion",
            Self::OrderClosure => "order_closure",
        }
    }
    fn parse(value: &str) -> Result<Self, StoreError> {
        match value {
            "material" => Ok(Self::Material),
            "observation" => Ok(Self::Observation),
            "allocation" => Ok(Self::Allocation),
            "inclusion" => Ok(Self::Inclusion),
            "order_closure" => Ok(Self::OrderClosure),
            _ => Err(unavailable("지원하지 않는 정정 결과 종류입니다")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProjectionHead {
    pub kind: ProjectionKind,
    pub subject_id: String,
    pub current_version_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CorrectionPriceBasisRow {
    pub id: String,
    pub correction_id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub amount: String,
    pub quantity: String,
    pub source: CorrectionPlanRef,
    pub logical_seq: String,
    pub recorded_at: String,
    pub seq: String,
}

pub(super) struct SnapshotRows {
    pub records: Vec<CorrectionRecord>,
    pub event_versions: Vec<EventVersionRow>,
    pub event_heads: Vec<EventHead>,
    pub projection_versions: Vec<ProjectionVersionRow>,
    pub projection_heads: Vec<ProjectionHead>,
    pub price_bases: Vec<CorrectionPriceBasisRow>,
}

pub(super) struct Validation<'a> {
    pub records: &'a [CorrectionRecord],
    pub event_versions: &'a [EventVersionRow],
    pub event_heads: &'a [EventHead],
    pub projection_versions: &'a [ProjectionVersionRow],
    pub projection_heads: &'a [ProjectionHead],
    pub price_bases: &'a [CorrectionPriceBasisRow],
    pub business_records: &'a [super::business::Record],
    pub revisions: &'a [crate::business::BusinessRevision],
    pub confirmations: &'a [super::business::ConfirmationRow],
    pub materials: &'a [crate::catalog::MaterialVersion],
    pub prices: &'a [crate::catalog::MaterialPriceVersion],
    pub orders: &'a [crate::purchase_order::OrderVersion],
    pub closures: &'a [super::receipt::ClosureRow],
    pub receipts: &'a [crate::receipt::ReceiptRecord],
    pub purchase_returns: &'a [crate::purchase_return::PurchaseReturnRecord],
    pub productions: &'a [super::production::RecordRow],
    pub production_inputs: &'a [super::production::InputRow],
    pub production_outputs: &'a [super::production::OutputRow],
    pub production_effects: &'a [super::production::EffectRow],
    pub sales: &'a [super::sale::RecordRow],
    pub sale_versions: &'a [crate::sale::SaleVersion],
    pub sale_cost_rows: &'a [super::sale::CostRow],
    pub stocktake_batches: &'a [crate::stocktake::StocktakeRecord],
    pub observations: &'a [super::stocktake::ObservationRow],
    pub observation_versions: &'a [super::stocktake::VersionRow],
    pub consumption_versions: &'a [super::stocktake::ConsumptionVersion],
    pub consumption_rows: &'a [super::stocktake::ConsumptionRow],
    pub holdings: &'a [super::holding::HoldingVersion],
    pub wastes: &'a [crate::waste::WasteRecord],
    pub reconciliations: &'a [crate::reconciliation::ReceiptReconciliation],
    pub inclusion_links: &'a [super::reconciliation::LinkRow],
    pub inclusion_link_versions: &'a [super::reconciliation::LinkVersionRow],
    pub inclusion_reservations: &'a [super::reconciliation::ReservationRow],
    pub mutations: &'a [(&'a str, &'a str, &'a str)],
    pub sequence: i64,
}

pub(super) fn kind_text(kind: CorrectionSubjectKind) -> &'static str {
    match kind {
        CorrectionSubjectKind::Receipt => "receipt",
        CorrectionSubjectKind::PurchaseReturn => "purchase_return",
        CorrectionSubjectKind::Production => "production",
        CorrectionSubjectKind::SaleCount => "sale_count",
        CorrectionSubjectKind::Waste => "waste",
        CorrectionSubjectKind::Stocktake => "stocktake",
    }
}
fn parse_kind(value: &str) -> Result<CorrectionSubjectKind, StoreError> {
    match value {
        "receipt" => Ok(CorrectionSubjectKind::Receipt),
        "purchase_return" => Ok(CorrectionSubjectKind::PurchaseReturn),
        "production" => Ok(CorrectionSubjectKind::Production),
        "sale_count" => Ok(CorrectionSubjectKind::SaleCount),
        "waste" => Ok(CorrectionSubjectKind::Waste),
        "stocktake" => Ok(CorrectionSubjectKind::Stocktake),
        _ => Err(unavailable("지원하지 않는 정정 사건 종류입니다")),
    }
}

fn json_rows<T: DeserializeOwned>(
    connection: &Connection,
    sql: &str,
) -> Result<Vec<T>, StoreError> {
    connection
        .prepare(sql)
        .map_err(unavailable)?
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(unavailable)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(unavailable)?
        .into_iter()
        .map(|json| serde_json::from_str(&json).map_err(unavailable))
        .collect()
}

pub(super) fn snapshot_rows(connection: &Connection) -> Result<SnapshotRows, StoreError> {
    validate_physical_columns(connection)?;
    let event_heads = connection
        .prepare("SELECT kind,original_id,current_version_id FROM correction_event_heads ORDER BY kind,original_id")
        .map_err(unavailable)?
        .query_map([], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))
        .map_err(unavailable)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(unavailable)?
        .into_iter()
        .map(|(kind, original_id, current_version_id)| Ok(EventHead { kind:parse_kind(&kind)?, original_id, current_version_id }))
        .collect::<Result<_,StoreError>>()?;
    let projection_heads = connection
        .prepare("SELECT kind,subject_id,current_version_id FROM correction_projection_heads ORDER BY kind,subject_id")
        .map_err(unavailable)?
        .query_map([], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))
        .map_err(unavailable)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(unavailable)?
        .into_iter()
        .map(|(kind, subject_id, current_version_id)| Ok(ProjectionHead { kind:ProjectionKind::parse(&kind)?, subject_id, current_version_id }))
        .collect::<Result<_,StoreError>>()?;
    Ok(SnapshotRows {
        records: json_rows(
            connection,
            "SELECT record_json FROM correction_records ORDER BY sequence,id",
        )?,
        event_versions: json_rows(
            connection,
            "SELECT row_json FROM correction_event_versions ORDER BY sequence,logical_sequence,logical_position,id",
        )?,
        event_heads,
        projection_versions: json_rows(
            connection,
            "SELECT row_json FROM correction_projection_versions ORDER BY sequence,kind,subject_id,id",
        )?,
        projection_heads,
        price_bases: json_rows(
            connection,
            "SELECT row_json FROM correction_price_bases ORDER BY sequence,material_id,id",
        )?,
    })
}

pub(super) fn validate_physical_columns(connection: &Connection) -> Result<(), StoreError> {
    for sql in [
        "SELECT EXISTS(SELECT 1 FROM correction_records WHERE id IS NOT json_extract(record_json,'$.id') OR original_id IS NOT json_extract(record_json,'$.original_id') OR target_confirmation_id IS NOT json_extract(record_json,'$.target_confirmation_id') OR previous_version_id IS NOT json_extract(record_json,'$.previous_version_id') OR confirmation_id IS NOT json_extract(record_json,'$.confirmation_id') OR business_record_id IS NOT json_extract(record_json,'$.business_record_id') OR recorded_at IS NOT json_extract(record_json,'$.recorded_at') OR CAST(sequence AS TEXT) IS NOT json_extract(record_json,'$.seq'))",
        "SELECT EXISTS(SELECT 1 FROM correction_event_versions WHERE id IS NOT json_extract(row_json,'$.id') OR correction_id IS NOT json_extract(row_json,'$.correction_id') OR kind IS NOT json_extract(row_json,'$.reference.kind') OR original_id IS NOT json_extract(row_json,'$.reference.original_id') OR original_confirmation_id IS NOT json_extract(row_json,'$.reference.original_confirmation_id') OR previous_version_id IS NOT json_extract(row_json,'$.previous_version_id') OR previous_original_version_id IS NOT json_extract(row_json,'$.previous_original_version_id') OR CAST(logical_sequence AS TEXT) IS NOT json_extract(row_json,'$.reference.logical_seq') OR logical_position IS NOT json_extract(row_json,'$.reference.logical_position') OR recorded_at IS NOT json_extract(row_json,'$.reference.recorded_at') OR CAST(sequence AS TEXT) IS NOT json_extract(row_json,'$.reference.seq'))",
        "SELECT EXISTS(SELECT 1 FROM correction_projection_versions WHERE id IS NOT json_extract(row_json,'$.id') OR correction_id IS NOT json_extract(row_json,'$.correction_id') OR kind IS NOT json_extract(row_json,'$.projection.kind') OR previous_version_id IS NOT json_extract(row_json,'$.previous_version_id') OR recorded_at IS NOT json_extract(row_json,'$.recorded_at') OR CAST(sequence AS TEXT) IS NOT json_extract(row_json,'$.seq') OR subject_id IS NOT CASE kind WHEN 'material' THEN json_extract(row_json,'$.projection.change.material_id') WHEN 'observation' THEN json_extract(row_json,'$.projection.change.reference.id') WHEN 'allocation' THEN json_extract(row_json,'$.projection.change.observation.id') WHEN 'inclusion' THEN json_extract(row_json,'$.projection.change.receipt_id') WHEN 'order_closure' THEN json_extract(row_json,'$.projection.change.order_id') END)",
        "SELECT EXISTS(SELECT 1 FROM correction_price_bases WHERE id IS NOT json_extract(row_json,'$.id') OR correction_id IS NOT json_extract(row_json,'$.correction_id') OR material_id IS NOT json_extract(row_json,'$.material_id') OR material_version_id IS NOT json_extract(row_json,'$.material_version_id') OR CAST(logical_sequence AS TEXT) IS NOT json_extract(row_json,'$.logical_seq') OR recorded_at IS NOT json_extract(row_json,'$.recorded_at') OR CAST(sequence AS TEXT) IS NOT json_extract(row_json,'$.seq'))",
        "SELECT EXISTS(SELECT 1 FROM consumption_versions WHERE correction_projection_id IS NOT json_extract(allocation_json,'$.correction_projection_id') OR CAST(sequence AS TEXT) IS NOT json_extract(allocation_json,'$.seq'))",
    ] {
        if connection
            .query_row(sql, [], |r| r.get::<_, bool>(0))
            .map_err(unavailable)?
        {
            return Err(unavailable(
                "정정 관계의 원본 열과 기록 내용이 일치하지 않습니다",
            ));
        }
    }
    let rows = connection
        .prepare("SELECT id,material_id FROM correction_price_bases")
        .map_err(unavailable)?
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(unavailable)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(unavailable)?;
    for (id, material) in rows {
        price_basis(connection, &material, &id)?;
    }
    Ok(())
}

fn subject<'a>(history: &'a CorrectionHistory, target: &str) -> Option<&'a CorrectionHistoryEvent> {
    history
        .events
        .iter()
        .find(|event| event.reference.original_confirmation_id == target)
}
pub(super) fn check_target(
    connection: &Connection,
    target: Option<&str>,
    data: &DraftCorrectionV1,
) -> Result<(), StoreError> {
    let Some(target) = target else {
        return Ok(());
    };
    let history = history::load(connection, chrono::Local::now().date_naive())?;
    let event = subject(&history, target)
        .ok_or_else(|| StoreError::Validation("정정할 원확정 기록을 찾을 수 없습니다".into()))?;
    if data
        .expected_target_version_id
        .as_ref()
        .is_some_and(|id| id != &event.reference.version_id)
    {
        return Err(StoreError::StaleState(
            "정정 대상의 유효 버전이 바뀌었습니다. 입력을 유지하고 최신 대상을 확인하세요".into(),
        ));
    }
    if let CorrectionActionDraft::Replace { replacement } = &data.action
        && replacement.kind() != event.reference.kind
    {
        return Err(StoreError::Validation(
            "정정 대상의 업무 종류를 바꿀 수 없습니다".into(),
        ));
    }
    if let CorrectionActionDraft::Replace {
        replacement: CorrectionReplacementDraft::PurchaseReturn { data },
    } = &data.action
    {
        super::purchase_return::check_source(connection, data)?;
    }
    Ok(())
}

pub(super) fn inspect_correction(
    connection: &Connection,
    data: &DraftCorrectionV1,
    target: Option<&str>,
    today: NaiveDate,
) -> Result<(Vec<BusinessIssue>, Option<BusinessEffect>), StoreError> {
    let Some(target) = target else {
        return Ok((
            vec![BusinessIssue {
                field: "corrects_confirmation_id".into(),
                code: "missing_reference".into(),
                message: "정정할 원확정 기록을 선택하세요".into(),
            }],
            None,
        ));
    };
    let history = history::load(connection, today)?;
    match crate::correction::build_correction_plan(&history, target, data) {
        Ok(check) => Ok((
            check.issues,
            Some(BusinessEffect::Correction {
                correction: Box::new(check.plan),
            }),
        )),
        Err(CorrectionError::InvalidHistory(error)) => Err(unavailable(error)),
        Err(CorrectionError::OutOfRange(error)) => Ok((
            vec![BusinessIssue {
                field: "action".into(),
                code: "invalid_input".into(),
                message: error,
            }],
            None,
        )),
    }
}

pub(super) fn result(
    connection: &Connection,
    confirmation: &str,
) -> Result<BusinessResult, StoreError> {
    connection.query_row("SELECT id,original_id,target_confirmation_id FROM correction_records WHERE confirmation_id=?1",[confirmation],|r|Ok(BusinessResult::Correction{correction_id:r.get(0)?,original_id:r.get(1)?,target_confirmation_id:r.get(2)?})).map_err(unavailable)
}
pub(super) fn material_name(
    connection: &Connection,
    _data: &DraftCorrectionV1,
    target: Option<&str>,
) -> Result<Option<String>, StoreError> {
    let Some(target) = target else {
        return Ok(None);
    };
    let history = history::load(connection, chrono::Local::now().date_naive())?;
    let Some(event) = subject(&history, target) else {
        return Ok(None);
    };
    let Some(value) = event.effective_value.as_ref() else {
        return Ok(None);
    };
    Ok(match value {
        CorrectionEventValue::SaleCount { result, .. } => Some(result.material_name.clone()),
        CorrectionEventValue::Waste { result, .. } => Some(result.material_name.clone()),
        CorrectionEventValue::PurchaseReturn { result, .. } => Some(result.material_name.clone()),
        CorrectionEventValue::Production { result, .. } => {
            result.outputs.first().map(|r| r.material_name.clone())
        }
        CorrectionEventValue::Receipt { input, .. } => {
            super::receipt::material_name(connection, input)?
        }
        CorrectionEventValue::Stocktake { .. } => event
            .observations
            .first()
            .map(|o| o.effect.material_name.clone()),
    })
}

fn seed_version(event: &CorrectionHistoryEvent) -> CorrectionSeedVersion {
    CorrectionSeedVersion {
        reference: event.reference.clone(),
        input: event.effective_input.clone(),
        value: event.effective_value.clone(),
    }
}
impl Store {
    pub fn correction_seed(
        &self,
        confirmation_id: &str,
    ) -> Result<Option<CorrectionSeed>, StoreError> {
        let id = catalog::uuid(confirmation_id, "정정 원확정 ID")?;
        let history = history::load(&self.connection, (self.today)())?;
        let Some(current) = subject(&history, &id) else {
            return Ok(None);
        };
        let logical = current
            .reference
            .logical_seq
            .parse::<i64>()
            .map_err(unavailable)?;
        let old = history::load_at(&self.connection, (self.today)(), Some(logical))?;
        let original =
            subject(&old, &id).ok_or_else(|| unavailable("정정의 최초 사건이 없습니다"))?;
        let rows = snapshot_rows(&self.connection)?;
        let mut entries = vec![];
        for record in rows.records {
            if rows
                .event_versions
                .iter()
                .any(|v| v.correction_id == record.id && v.reference.original_confirmation_id == id)
            {
                let confirmation =
                    business::confirmation(&self.connection, &record.confirmation_id)?
                        .ok_or_else(|| unavailable("정정의 실제 확정이 없습니다"))?;
                entries.push(CorrectionHistoryEntry {
                    confirmation,
                    reason: record.reason,
                });
            }
        }
        let mut replacement_template = current
            .effective_input
            .clone()
            .unwrap_or_else(|| original.original_input.clone());
        if let CorrectionReplacementDraft::Stocktake { data } = &mut replacement_template {
            for row in &mut data.rows {
                let observation = current
                    .observations
                    .iter()
                    .find(|observation| row.observation_id.as_ref() == Some(&observation.id))
                    .ok_or_else(|| unavailable("복원할 실사의 현재 관측이 없습니다"))?;
                // A void observation uses its persisted projection as a tombstone.
                // The template never changes evidence or invents a future ID.
                row.expected_version_id = Some(observation.version_id.clone());
            }
        }
        if let CorrectionReplacementDraft::PurchaseReturn { data } = &mut replacement_template {
            data.expected_receipt_version_id = data
                .receipt_id
                .as_deref()
                .and_then(|id| {
                    history.events.iter().find(|e| {
                        e.reference.kind == CorrectionSubjectKind::Receipt
                            && e.reference.original_id == id
                            && e.effective_value.is_some()
                    })
                })
                .map(|e| e.reference.version_id.clone());
        }
        let draft = DraftCorrectionV1 {
            expected_target_version_id: Some(current.reference.version_id.clone()),
            reason: String::new(),
            action: match current.effective_input.clone() {
                Some(replacement) => CorrectionActionDraft::Replace { replacement },
                None => CorrectionActionDraft::Void {},
            },
            observation_reviews: vec![],
            linked_receipt_changes: vec![],
        };
        Ok(Some(CorrectionSeed {
            requested_confirmation_id: id,
            requested: seed_version(original),
            original: seed_version(original),
            current: seed_version(current),
            replacement_template,
            history: entries,
            draft,
            state: catalog::state(&self.connection)?,
        }))
    }
    pub fn correction_detail(
        &self,
        correction_id: &str,
    ) -> Result<Option<CorrectionDetail>, StoreError> {
        let id = catalog::uuid(correction_id, "정정 ID")?;
        let rows = snapshot_rows(&self.connection)?;
        let Some(item) = rows.records.iter().find(|r| r.id == id).cloned() else {
            return Ok(None);
        };
        let versions = rows
            .records
            .into_iter()
            .filter(|r| {
                r.target_confirmation_id == item.target_confirmation_id
                    || rows.event_versions.iter().any(|v| {
                        v.correction_id == r.id
                            && v.reference.original_confirmation_id == item.target_confirmation_id
                    })
            })
            .collect();
        Ok(Some(CorrectionDetail {
            item,
            versions,
            state: catalog::state(&self.connection)?,
        }))
    }
}

pub(super) fn validate_correction_schema(connection: &Connection) -> anyhow::Result<()> {
    for sql in [
        "SELECT id,original_id,target_confirmation_id,previous_version_id,confirmation_id,business_record_id,recorded_at,sequence,record_json FROM correction_records LIMIT 0",
        "SELECT id,correction_id,kind,original_id,original_confirmation_id,previous_version_id,previous_original_version_id,logical_sequence,logical_position,recorded_at,sequence,row_json FROM correction_event_versions LIMIT 0",
        "SELECT kind,original_id,current_version_id FROM correction_event_heads LIMIT 0",
        "SELECT id,correction_id,kind,subject_id,previous_version_id,recorded_at,sequence,row_json FROM correction_projection_versions LIMIT 0",
        "SELECT kind,subject_id,current_version_id FROM correction_projection_heads LIMIT 0",
        "SELECT id,correction_id,material_id,material_version_id,amount_micros,quantity_micros,logical_sequence,recorded_at,sequence,row_json FROM correction_price_bases LIMIT 0",
    ] {
        connection.prepare(sql)?;
    }
    validate_physical_columns(connection).map_err(|error| anyhow::anyhow!("{error}"))
}

pub(super) fn price_basis(
    connection: &Connection,
    material: &str,
    id: &str,
) -> Result<crate::domain::UnitPrice, StoreError> {
    let row:Option<(i64,i64,String)>=connection.query_row("SELECT amount_micros,quantity_micros,row_json FROM correction_price_bases WHERE id=?1 AND material_id=?2",rusqlite::params![id,material],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(unavailable)?;
    let (amount, quantity, json) =
        row.ok_or_else(|| unavailable("정정 단가의 실제 근거가 없습니다"))?;
    let saved: CorrectionPriceBasisRow = serde_json::from_str(&json).map_err(unavailable)?;
    let a = crate::domain::Money::from_micros(amount).map_err(unavailable)?;
    let q = crate::domain::Quantity::from_micros(quantity).map_err(unavailable)?;
    if saved.id != id
        || saved.material_id != material
        || saved.amount != a.decimal()
        || saved.quantity != q.decimal()
    {
        return Err(unavailable(
            "정정 단가의 값과 실제 근거가 일치하지 않습니다",
        ));
    }
    crate::domain::UnitPrice::new(a, q, id.into()).map_err(unavailable)
}

pub(super) fn sale_effective(
    connection: &Connection,
    sale_id: &str,
    at: i64,
) -> Result<Option<crate::sale::SaleEffectiveState>, StoreError> {
    let json:Option<String>=connection.query_row("SELECT row_json FROM correction_event_versions WHERE kind='sale_count' AND original_id=?1 AND sequence<=?2 ORDER BY sequence DESC LIMIT 1",rusqlite::params![sale_id,at],|r|r.get(0)).optional().map_err(unavailable)?;
    let Some(json) = json else { return Ok(None) };
    let row: EventVersionRow = serde_json::from_str(&json).map_err(unavailable)?;
    Ok(Some(match row.after {
        Some(CorrectionEventValue::SaleCount { result, .. }) => {
            crate::sale::SaleEffectiveState::Active {
                version_id: row
                    .sale_version_id
                    .ok_or_else(|| unavailable("판매 실제 개정이 없습니다"))?,
                count: result.count,
                amount: result.amount,
            }
        }
        None => crate::sale::SaleEffectiveState::Void { version_id: row.id },
        _ => return Err(unavailable("판매 정정 결과 종류가 다릅니다")),
    }))
}
