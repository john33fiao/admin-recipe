//! Explicit receipt inclusion and comparison-only value replay contracts.
use crate::{
    business::{normalize_draft_uuid, validate_draft_text},
    catalog::CatalogState,
    domain::DomainError,
    holding::{HoldingAmounts, HoldingBasisRef, HoldingObservation},
    purchase_order::OrderVersion,
    receipt::OrderReceiptPreview,
};
use serde::{Deserialize, Serialize};

pub mod provenance;
pub mod value;
pub use value::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    ReceiptInflow,
    ObservationInitial,
    ObservationExcess,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceKey {
    pub kind: SourceKind,
    pub id: String,
    pub position: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InclusionSourceSelection {
    AllAvailable {},
    Source { source_key: SourceKey },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptInclusionSelection {
    pub observation_version_id: Option<String>,
    pub source: Option<InclusionSourceSelection>,
    pub included_quantity: String,
}

pub fn normalize_receipt_inclusions(
    selections: &mut [ReceiptInclusionSelection],
) -> Result<(), DomainError> {
    if selections.len() > 20 {
        return Err(DomainError(
            "입고 포함 근거는 20행 이내로 입력하세요".into(),
        ));
    }
    for row in selections {
        if let Some(id) = &mut row.observation_version_id {
            normalize_draft_uuid(id, "포함 관측 버전")?;
        }
        if let Some(InclusionSourceSelection::Source { source_key }) = &mut row.source {
            normalize_draft_uuid(&mut source_key.id, "포함 출처 ID")?;
            if source_key.position != 0 {
                return Err(DomainError("지원하지 않는 포함 출처 위치입니다".into()));
            }
        }
        validate_draft_text(&row.included_quantity, "이미 포함된 수량", 80, false)?;
    }
    Ok(())
}

/// Exact quantity in `unit`, not in micro-units. Fractions are server output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceShare {
    pub numerator: String,
    pub denominator: String,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptInclusionCandidates {
    pub items: Vec<ReceiptInclusionObservation>,
    pub state: CatalogState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptInclusionObservation {
    pub stocktake_id: String,
    pub observation_id: String,
    pub observation_version_id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub unit: String,
    pub quantity: String,
    pub value: String,
    pub available_share: SourceShare,
    pub available_quantity: String,
    pub recorded_at: String,
    pub seq: String,
    pub sources: Vec<ReceiptInclusionSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptInclusionSource {
    pub source_key: SourceKey,
    pub source_version_id: String,
    pub source_material_id: String,
    pub source_material_name: String,
    pub source_unit: String,
    pub source_quantity: String,
    pub reserved_source_share: SourceShare,
    pub remaining_source_share: SourceShare,
    pub available_share: SourceShare,
    pub available_quantity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconciledReceiptEffect {
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub quantity: String,
    pub amount: String,
    pub unit: String,
    pub input_quantity: String,
    pub input_unit: String,
    pub receipt_date: String,
    pub included_quantity: String,
    pub unreflected_quantity: String,
    pub included_amount: String,
    pub unreflected_amount: String,
    pub booked_value: String,
    pub value_difference: String,
    pub separate_adjustment: String,
    pub selections: Vec<ReceiptInclusionEffect>,
    pub materials: Vec<ReconciliationMaterialEffect>,
    pub trace: Vec<ReconciliationValueTrace>,
    pub order_before: Option<Box<OrderVersion>>,
    pub order_after: Option<OrderReceiptPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptInclusionEffect {
    pub position: usize,
    pub stocktake_id: String,
    pub observation_id: String,
    pub observation_version_id: String,
    pub source: InclusionSourceSelection,
    pub included_quantity: String,
    pub unit: String,
    pub booked_value: String,
    pub actual_included_value: String,
    pub value_difference: String,
    pub portions: Vec<InclusionPortionEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InclusionPortionEffect {
    pub position: usize,
    pub source_key: SourceKey,
    pub source_version_id: String,
    pub included_share: SourceShare,
    pub reserved_source_share: SourceShare,
    pub booked_value: String,
    pub actual_included_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationMaterialEffect {
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub unit: String,
    pub before: Option<HoldingAmounts>,
    pub after: HoldingAmounts,
    pub basis_before: Option<HoldingBasisRef>,
    pub observation: Option<HoldingObservation>,
    pub value_delta: String,
    pub inflow_quantity: String,
    pub inflow_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptReconciliation {
    pub id: String,
    pub receipt_id: String,
    pub confirmation_id: String,
    pub business_record_id: String,
    pub effect: ReconciledReceiptEffect,
    pub selection_results: Vec<ReceiptInclusionResult>,
    pub material_results: Vec<ReconciliationMaterialResult>,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptInclusionResult {
    pub selection_position: usize,
    pub portion_position: usize,
    pub link_id: String,
    pub link_version_id: String,
    pub reservation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationMaterialResult {
    pub material_id: String,
    pub effect_id: String,
    pub holding_version_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationTraceKind {
    Inflow,
    Observation,
    Production,
    Waste,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationValueTrace {
    pub kind: ReconciliationTraceKind,
    pub event_id: String,
    pub seq: String,
    pub material_id: String,
    pub quantity_before: String,
    pub quantity_after: String,
    pub base_value_before: String,
    pub base_value_after: String,
    pub proposed_value_before: String,
    pub proposed_value_after: String,
    pub base_closed_value: String,
    pub proposed_closed_value: String,
}
