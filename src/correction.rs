//! Explicit corrections retain original event positions and materialize a new
//! effective projection. Preview references never allocate future identifiers.
use crate::{
    business::{BusinessConfirmation, BusinessIssue, normalize_draft_uuid, validate_draft_text},
    catalog::{CatalogState, MaterialPriceVersion, MaterialVersion},
    domain::{DomainError, ResolvedPrice},
    holding::HoldingAmounts,
    production::{DraftProductionV1, ProductionPreview, normalize_production_draft},
    purchase_order::{OrderReceiptClosure, OrderVersion},
    receipt::{DraftReceiptV1, OrderReceiptPreview, normalize_receipt_draft},
    reconciliation::{ReceiptReconciliation, ReconciledReceiptEffect, SourceKey, SourceShare},
    sale::{SaleOriginal, SaleProposal, SaleVersion},
    stocktake::{
        ConsumptionAllocation, DraftStocktakeRowV1, DraftStocktakeV1, StocktakeExcessDraft,
        StocktakeObservationRecord, StocktakeRowEffect, StocktakeValueDraft,
        normalize_stocktake_draft,
    },
    waste::{DraftWasteV1, WasteEffect, normalize_waste_draft},
};
use serde::{Deserialize, Serialize};

pub mod inclusion;
mod replay;
pub use replay::{build_correction_plan, effective_correction_replay};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftCorrectionV1 {
    pub expected_target_version_id: Option<String>,
    pub reason: String,
    pub action: CorrectionActionDraft,
    pub observation_reviews: Vec<ObservationReviewDraft>,
    pub linked_receipt_changes: Vec<LinkedReceiptChangeDraft>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[allow(
    clippy::large_enum_variant,
    reason = "Owned draft payloads stay inline across normalization and replay without an extra allocation per replacement"
)]
pub enum CorrectionActionDraft {
    Void {},
    Replace {
        replacement: CorrectionReplacementDraft,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CorrectionReplacementDraft {
    Receipt { data: DraftReceiptV1 },
    Production { data: DraftProductionV1 },
    SaleCount { data: DraftSaleCountCorrectionV1 },
    Waste { data: DraftWasteV1 },
    Stocktake { data: DraftStocktakeCorrectionV1 },
}

impl CorrectionReplacementDraft {
    pub fn kind(&self) -> CorrectionSubjectKind {
        match self {
            Self::Receipt { .. } => CorrectionSubjectKind::Receipt,
            Self::Production { .. } => CorrectionSubjectKind::Production,
            Self::SaleCount { .. } => CorrectionSubjectKind::SaleCount,
            Self::Waste { .. } => CorrectionSubjectKind::Waste,
            Self::Stocktake { .. } => CorrectionSubjectKind::Stocktake,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftSaleCountCorrectionV1 {
    pub count: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftStocktakeCorrectionV1 {
    pub rows: Vec<StocktakeCorrectionRowDraft>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StocktakeCorrectionRowDraft {
    pub observation_id: Option<String>,
    pub expected_version_id: Option<String>,
    pub action: ObservationCorrectionActionDraft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObservationCorrectionActionDraft {
    Void {},
    Replace {
        quantity: String,
        unit: String,
        initial_value: Option<StocktakeValueDraft>,
        excess: Option<StocktakeExcessDraft>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationReviewDraft {
    pub observation_id: Option<String>,
    pub expected_version_id: Option<String>,
    pub initial_value: Option<StocktakeValueDraft>,
    pub excess: Option<StocktakeExcessDraft>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkedReceiptChangeDraft {
    pub receipt_id: Option<String>,
    pub expected_version_id: Option<String>,
    pub action: LinkedReceiptActionDraft,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[allow(
    clippy::large_enum_variant,
    reason = "Linked replacements reuse the owned receipt draft used by normalization and replay"
)]
pub enum LinkedReceiptActionDraft {
    Void {},
    Replace { data: DraftReceiptV1 },
}

fn optional_uuid(value: &mut Option<String>, label: &str) -> Result<(), DomainError> {
    if let Some(id) = value {
        normalize_draft_uuid(id, label)?;
    }
    Ok(())
}

fn valuation_text(
    initial: &mut Option<StocktakeValueDraft>,
    excess: &mut Option<StocktakeExcessDraft>,
) -> Result<(), DomainError> {
    let mut holder = DraftStocktakeV1 {
        rows: vec![DraftStocktakeRowV1 {
            material_id: None,
            material_version_id: None,
            quantity: String::new(),
            unit: String::new(),
            initial_value: initial.take(),
            excess: excess.take(),
        }],
        note: String::new(),
    };
    let result = normalize_stocktake_draft(&mut holder);
    *initial = holder.rows[0].initial_value.take();
    *excess = holder.rows[0].excess.take();
    result
}

/// Structure/length checks preserve incomplete input. Review lists are bounded
/// by the HTTP body, never truncated or capped independently of actual history.
pub fn normalize_correction_draft(data: &mut DraftCorrectionV1) -> Result<(), DomainError> {
    optional_uuid(&mut data.expected_target_version_id, "정정 대상 버전")?;
    data.reason = data.reason.replace("\r\n", "\n").replace('\r', "\n");
    validate_draft_text(&data.reason, "정정 이유", 2000, true)?;
    if let CorrectionActionDraft::Replace { replacement } = &mut data.action {
        match replacement {
            CorrectionReplacementDraft::Receipt { data } => normalize_receipt_draft(data)?,
            CorrectionReplacementDraft::Production { data } => normalize_production_draft(data)?,
            CorrectionReplacementDraft::SaleCount { data } => {
                validate_draft_text(&data.count, "판매 개수", 80, false)?;
            }
            CorrectionReplacementDraft::Waste { data } => normalize_waste_draft(data)?,
            CorrectionReplacementDraft::Stocktake { data } => {
                if data.rows.len() > 100 {
                    return Err(DomainError("실사 정정은 100행 이내로 입력하세요".into()));
                }
                for row in &mut data.rows {
                    optional_uuid(&mut row.observation_id, "정정 관측")?;
                    optional_uuid(&mut row.expected_version_id, "정정 관측 버전")?;
                    if let ObservationCorrectionActionDraft::Replace {
                        quantity,
                        unit,
                        initial_value,
                        excess,
                    } = &mut row.action
                    {
                        validate_draft_text(quantity, "실측 수량", 80, false)?;
                        validate_draft_text(unit, "입력 단위", 20, false)?;
                        valuation_text(initial_value, excess)?;
                    }
                }
            }
        }
    }
    for row in &mut data.observation_reviews {
        optional_uuid(&mut row.observation_id, "검토 관측")?;
        optional_uuid(&mut row.expected_version_id, "검토 관측 버전")?;
        valuation_text(&mut row.initial_value, &mut row.excess)?;
    }
    for row in &mut data.linked_receipt_changes {
        optional_uuid(&mut row.receipt_id, "관련 입고")?;
        optional_uuid(&mut row.expected_version_id, "관련 입고 버전")?;
        if let LinkedReceiptActionDraft::Replace { data } = &mut row.action {
            normalize_receipt_draft(data)?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionSubjectKind {
    Receipt,
    Production,
    SaleCount,
    Waste,
    Stocktake,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionEventRef {
    pub kind: CorrectionSubjectKind,
    pub original_id: String,
    pub original_confirmation_id: String,
    pub confirmation_id: String,
    pub version_id: String,
    pub business_record_id: String,
    pub logical_seq: String,
    pub logical_position: usize,
    pub original_recorded_at: String,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionRefKind {
    Receipt,
    Production,
    Sale,
    Waste,
    Stocktake,
    Observation,
    InclusionLink,
    Material,
    PriceBasis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CorrectionPlanRef {
    Existing {
        entity: CorrectionRefKind,
        id: String,
        version_id: String,
    },
    Proposed {
        entity: CorrectionRefKind,
        id: String,
        expected_version_id: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionInclusionCandidate {
    pub observation: CorrectionPlanRef,
    pub stocktake_id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub unit: String,
    pub quantity: String,
    pub value: String,
    pub available_share: SourceShare,
    pub available_quantity: String,
    pub logical_seq: String,
    pub sources: Vec<CorrectionInclusionSourceCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionInclusionSourceCandidate {
    pub source_key: SourceKey,
    pub source_ref: CorrectionPlanRef,
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
pub struct CorrectionPlan {
    pub target: CorrectionEventRef,
    pub events: Vec<CorrectionEventChange>,
    pub materials: Vec<CorrectionMaterialChange>,
    pub observations: Vec<CorrectionObservationChange>,
    pub allocations: Vec<CorrectionAllocationChange>,
    pub inclusions: Vec<CorrectionInclusionChange>,
    pub separate_adjustments: Vec<CorrectionSeparateAdjustmentChange>,
    pub order_closures: Vec<CorrectionOrderClosureChange>,
    pub review_requests: Vec<CorrectionReviewRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionEventChange {
    pub reference: CorrectionEventRef,
    pub before: Option<CorrectionEventValue>,
    pub after: Option<CorrectionEventValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[allow(
    clippy::large_enum_variant,
    reason = "Replay and persistence share these owned event snapshots; the enclosing business correction plan is boxed"
)]
pub enum CorrectionEventValue {
    Receipt {
        input: DraftReceiptV1,
        reconciliation: Option<ReconciledReceiptEffect>,
    },
    Production {
        input: DraftProductionV1,
        result: ProductionPreview,
    },
    SaleCount {
        input: DraftSaleCountCorrectionV1,
        result: SaleProposal,
    },
    Waste {
        input: DraftWasteV1,
        result: WasteEffect,
    },
    Stocktake {
        input: DraftStocktakeCorrectionV1,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionObservationBoundary {
    pub reference: CorrectionPlanRef,
    pub stocktake_id: String,
    pub logical_seq: String,
    pub observed_at: String,
    pub quantity: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionInitialBoundary {
    pub reference: CorrectionPlanRef,
    pub logical_seq: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionPriceBasis {
    pub amount: String,
    pub quantity: String,
    pub source: CorrectionPlanRef,
    pub logical_seq: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[allow(
    clippy::large_enum_variant,
    reason = "Known holding evidence is one cohesive state; keep its optional boundaries inline for replay and comparison"
)]
pub enum CorrectionHoldingState {
    NotEstablished {},
    Known {
        amounts: HoldingAmounts,
        projection_seq: Option<String>,
        last_valid_price: Option<CorrectionPriceBasis>,
        observation_boundary: Option<CorrectionObservationBoundary>,
        initial_boundary: Option<CorrectionInitialBoundary>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionMaterialChange {
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub unit: String,
    pub before: CorrectionHoldingState,
    pub after: CorrectionHoldingState,
    pub quantity_delta: String,
    pub value_delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionObservationChange {
    pub reference: CorrectionPlanRef,
    pub stocktake_id: String,
    pub logical_seq: String,
    pub before: Option<StocktakeRowEffect>,
    pub after: Option<StocktakeRowEffect>,
    /// Derived price evidence for the proposed observation. The store resolves
    /// this reference into a real price-basis row without inventing a holding.
    pub valuation_basis: Option<CorrectionPriceBasis>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionAllocationChange {
    pub observation: CorrectionPlanRef,
    pub expected_consumption_version_id: Option<String>,
    pub before: Option<ConsumptionAllocation>,
    pub after: Option<ConsumptionAllocation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionInclusionChange {
    pub receipt_id: String,
    pub expected_version_id: String,
    pub before: Option<ReconciledReceiptEffect>,
    pub after: Option<ReconciledReceiptEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionSeparateAdjustmentChange {
    pub receipt_id: String,
    pub before: String,
    pub after: String,
    pub delta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionOrderClosureChange {
    pub order_id: String,
    pub expected_order_version_id: String,
    pub before: Option<OrderReceiptPreview>,
    pub after: Option<OrderReceiptPreview>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CorrectionValuationMode {
    Initial,
    Excess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[allow(
    clippy::large_enum_variant,
    reason = "Review DTOs carry the complete proposed context by value and live inside the boxed correction plan"
)]
pub enum CorrectionReviewRequest {
    ObservationValuation {
        field: String,
        observation: CorrectionPlanRef,
        material_id: String,
        material_name: String,
        unit: String,
        measured_quantity: String,
        proposed_before: CorrectionHoldingState,
        proposed_price: ResolvedPrice,
        mode: CorrectionValuationMode,
        excess_quantity: Option<String>,
        message: String,
    },
    ReceiptInclusions {
        field: String,
        receipt_id: String,
        expected_version_id: String,
        candidates: Vec<CorrectionInclusionCandidate>,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionSeedVersion {
    pub reference: CorrectionEventRef,
    pub input: Option<CorrectionReplacementDraft>,
    pub value: Option<CorrectionEventValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionHistoryEntry {
    pub confirmation: BusinessConfirmation,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionSeed {
    pub requested_confirmation_id: String,
    pub requested: CorrectionSeedVersion,
    pub original: CorrectionSeedVersion,
    pub current: CorrectionSeedVersion,
    /// Editable values with current existing observation versions. Original and
    /// effective evidence remain separate, including a void effective event.
    pub replacement_template: CorrectionReplacementDraft,
    pub history: Vec<CorrectionHistoryEntry>,
    pub draft: DraftCorrectionV1,
    pub state: CatalogState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionResolvedRef {
    pub reference: CorrectionPlanRef,
    pub version_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionRecord {
    pub id: String,
    pub original_id: String,
    pub target_confirmation_id: String,
    pub previous_version_id: Option<String>,
    pub confirmation_id: String,
    pub business_record_id: String,
    pub reason: String,
    pub input: DraftCorrectionV1,
    pub plan: CorrectionPlan,
    pub resolved_refs: Vec<CorrectionResolvedRef>,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CorrectionDetail {
    pub item: CorrectionRecord,
    pub versions: Vec<CorrectionRecord>,
    pub state: CatalogState,
}

/// Store adapters select effective versions as of this revision. Materialized
/// correction deltas must not be supplied as additional original events.
#[derive(Debug, Clone)]
pub struct CorrectionHistory {
    pub as_of_revision: i64,
    pub today: chrono::NaiveDate,
    pub events: Vec<CorrectionHistoryEvent>,
    pub materials: Vec<MaterialVersion>,
    pub prices: Vec<MaterialPriceVersion>,
    pub orders: Vec<OrderVersion>,
    pub closures: Vec<OrderReceiptClosure>,
    pub sales: Vec<SaleOriginal>,
    pub sale_versions: Vec<SaleVersion>,
    pub existing_inclusions: Vec<CorrectionEffectiveInclusion>,
    pub actual_materials: Vec<CorrectionMaterialSnapshot>,
    pub actual_allocations: Vec<CorrectionAllocationSnapshot>,
}

#[derive(Debug, Clone)]
pub struct CorrectionHistoryEvent {
    pub reference: CorrectionEventRef,
    pub original_input: CorrectionReplacementDraft,
    pub effective_input: Option<CorrectionReplacementDraft>,
    pub effective_value: Option<CorrectionEventValue>,
    pub observations: Vec<StocktakeObservationRecord>,
}

#[derive(Debug, Clone)]
pub struct CorrectionEffectiveInclusion {
    pub receipt_id: String,
    pub effective_confirmation_id: String,
    pub reconciliation: ReceiptReconciliation,
}

#[derive(Debug, Clone)]
pub struct CorrectionMaterialSnapshot {
    pub material_id: String,
    pub state: CorrectionHoldingState,
}

#[derive(Debug, Clone)]
pub struct CorrectionAllocationSnapshot {
    pub observation_id: String,
    pub consumption_version_id: String,
    pub allocation: ConsumptionAllocation,
}

#[derive(Debug, Clone)]
pub struct CorrectionCheck {
    pub plan: CorrectionPlan,
    pub issues: Vec<BusinessIssue>,
}

/// Algorithm-only source version normalization. Persisted/public portions use
/// actual_version_id; a receipt root uses its stable ID inside T011's solver.
#[derive(Debug, Clone)]
pub struct CorrectionSourceVersion {
    pub source_key: SourceKey,
    pub calculation_version_id: String,
    pub actual_version_id: String,
}

#[derive(Debug, Clone)]
pub struct CorrectionEffectiveReplay {
    pub events: Vec<crate::reconciliation::ValueReplayEvent>,
    pub replacements: Vec<crate::reconciliation::ValueReplacement>,
    pub reservations: Vec<crate::reconciliation::provenance::ReservedInclusion>,
    pub observations: Vec<inclusion::CorrectionReplayObservation>,
    pub source_refs: Vec<CorrectionSourceVersion>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorrectionError {
    InvalidHistory(String),
    OutOfRange(String),
}

impl std::fmt::Display for CorrectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidHistory(message) => write!(f, "정정 이력이 올바르지 않습니다: {message}"),
            Self::OutOfRange(message) => write!(f, "정정 계산 범위를 벗어났습니다: {message}"),
        }
    }
}
impl std::error::Error for CorrectionError {}
