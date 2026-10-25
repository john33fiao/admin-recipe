//! Bounded, read-only operational evidence; no predicted stock quantities.
use crate::{
    business::BusinessKind,
    catalog::CatalogState,
    correction::CorrectionEventRef,
    purchase_order::{OrderReceiptClosure, OrderSummary},
    statistics::{StatisticsChangeRef, StatisticsOrigin, StatisticsReceiptBasis},
    stocktake::NoAllocationReason,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug)]
pub enum OverviewError {
    Validation(String),
    StaleState(String),
    Unavailable(String),
}
impl std::fmt::Display for OverviewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(s) | Self::StaleState(s) | Self::Unavailable(s) => f.write_str(s),
        }
    }
}
impl std::error::Error for OverviewError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverviewSection {
    Materials,
    MaterialChanges,
    OpenOrders,
    ReceivedOrders,
    Receipts,
    Sales,
    Observations,
    Unallocated,
    Unconfirmed,
    Drafts,
}
impl OverviewSection {
    pub fn page_size(self) -> usize {
        if self == Self::Materials { 25 } else { 20 }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewListQuery {
    pub section: OverviewSection,
    pub offset: String,
    pub epoch: String,
    pub revision: String,
    pub material_id: Option<String>,
}
impl OverviewListQuery {
    pub fn validate(&self) -> Result<usize, OverviewError> {
        fn uuid(value: &str) -> bool {
            Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
        }
        let offset = self
            .offset
            .parse::<usize>()
            .ok()
            .filter(|n| n.to_string() == self.offset)
            .ok_or_else(|| OverviewError::Validation("목록 위치가 올바르지 않습니다".into()))?;
        if !uuid(&self.epoch)
            || !self
                .revision
                .parse::<i64>()
                .is_ok_and(|n| n >= 0 && n.to_string() == self.revision)
        {
            return Err(OverviewError::Validation(
                "조회 기준이 올바르지 않습니다".into(),
            ));
        }
        match (self.section, self.material_id.as_deref()) {
            (OverviewSection::MaterialChanges, Some(id)) if uuid(id) => {}
            (OverviewSection::MaterialChanges, _) | (_, Some(_)) => {
                return Err(OverviewError::Validation(
                    "목록 종류와 품목 조건이 다릅니다".into(),
                ));
            }
            _ => {}
        }
        Ok(offset)
    }
    pub fn scope(&self) -> OverviewScope {
        OverviewScope {
            section: self.section,
            material_id: self.material_id.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewScope {
    pub section: OverviewSection,
    pub material_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewPage<T> {
    pub total_count: usize,
    pub returned_count: usize,
    pub offset: usize,
    pub page_size: usize,
    pub next_offset: Option<usize>,
    pub rows: Vec<T>,
}
pub fn overview_page<T: Clone>(
    rows: &[T],
    offset: usize,
    section: OverviewSection,
) -> OverviewPage<T> {
    let start = offset.min(rows.len());
    let end = start
        .checked_add(section.page_size())
        .unwrap_or(rows.len())
        .min(rows.len());
    OverviewPage {
        total_count: rows.len(),
        returned_count: end - start,
        offset,
        page_size: section.page_size(),
        next_offset: (end < rows.len()).then_some(end),
        rows: rows[start..end].to_vec(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewMaterialRef {
    pub id: String,
    pub version_id: String,
    pub name: String,
    pub unit: String,
    pub archived: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewEffectMaterial {
    pub id: String,
    pub version_id: String,
    pub name: String,
    pub unit: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverviewObservationMode {
    Initial,
    Settlement,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewConsumption {
    pub consumption_version_id: String,
    pub quantity: String,
    pub value: String,
    pub excess_quantity: String,
    pub excess_value: String,
    pub unallocated_quantity: String,
    pub unallocated_value: String,
    pub reason: Option<NoAllocationReason>,
    pub candidate_count: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewObservation {
    pub origin: StatisticsOrigin,
    pub observation_id: String,
    pub observation_version_id: String,
    pub position: usize,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub unit: String,
    pub quantity: String,
    pub observed_at: String,
    pub logical_seq: String,
    pub mode: OverviewObservationMode,
    pub consumption: Option<OverviewConsumption>,
    pub changes: Vec<StatisticsChangeRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OverviewChangeBoundary {
    Observation {
        observation_id: String,
        observation_version_id: String,
        stocktake_id: String,
        logical_seq: String,
        observed_at: String,
    },
    NotObserved {},
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewMaterial {
    pub material: OverviewMaterialRef,
    pub last_observation: Option<OverviewObservation>,
    pub changes_boundary: OverviewChangeBoundary,
    pub changes_after_observation: OverviewPage<OverviewMaterialChange>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum OverviewMaterialEffect {
    Receipt {
        quantity: String,
        amount: String,
        receipt_date: String,
    },
    ReconciledReceipt {
        receipt: Box<StatisticsReceiptBasis>,
        included_quantity: String,
        included_amount: String,
        inflow_quantity: String,
        inflow_value: String,
        value_delta: String,
        separate_adjustment: String,
    },
    Production {
        input_quantity: String,
        input_value: String,
        output_quantity: String,
        output_value: String,
    },
    PurchaseReturn {
        quantity: String,
        deducted_value: String,
        source_receipt_id: String,
    },
    Waste {
        quantity: String,
        loss_value: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewMaterialChange {
    pub origin: StatisticsOrigin,
    pub effective_ref: CorrectionEventRef,
    pub changes: Vec<StatisticsChangeRef>,
    pub material: OverviewEffectMaterial,
    pub effect: OverviewMaterialEffect,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewReconciliation {
    pub included_quantity: String,
    pub included_amount: String,
    pub unreflected_quantity: String,
    pub unreflected_amount: String,
    pub value_difference: String,
    pub separate_adjustment: String,
    pub affected_material_count: usize,
    pub selection_count: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewReceipt {
    pub origin: StatisticsOrigin,
    pub effective_ref: CorrectionEventRef,
    pub changes: Vec<StatisticsChangeRef>,
    pub original: StatisticsReceiptBasis,
    pub effective: StatisticsReceiptBasis,
    pub reconciliation: Option<OverviewReconciliation>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewConsumptionCounts {
    pub allocated: usize,
    pub unallocated: usize,
    pub unconfirmed: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewSale {
    pub origin: StatisticsOrigin,
    pub effective_ref: CorrectionEventRef,
    pub changes: Vec<StatisticsChangeRef>,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub recipe_version_id: String,
    pub count: String,
    pub amount: Option<String>,
    pub consumption_counts: OverviewConsumptionCounts,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewReceivedOrder {
    pub order: OrderSummary,
    pub closure: OrderReceiptClosure,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewUnallocated {
    pub observation: OverviewObservation,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverviewUnconfirmedReason {
    NoEffectiveAllocation,
    InitialBasisMissing,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewUnconfirmedSale {
    pub origin: StatisticsOrigin,
    pub effective_ref: CorrectionEventRef,
    pub material_id: String,
    pub material_name: String,
    pub count: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewInitialObservation {
    pub stocktake_id: String,
    pub observation_id: String,
    pub observation_version_id: String,
    pub observed_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewUnconfirmed {
    pub sale: OverviewUnconfirmedSale,
    pub material: OverviewEffectMaterial,
    pub reason: OverviewUnconfirmedReason,
    pub initial_observation: Option<OverviewInitialObservation>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverviewDraftValidation {
    NotEvaluated,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewDraft {
    pub id: String,
    pub kind: BusinessKind,
    pub head_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub corrects_confirmation_id: Option<String>,
    pub material_name: Option<String>,
    pub validation: OverviewDraftValidation,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewOrderCounts {
    pub open: usize,
    pub received: usize,
    pub cancelled: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationsOverview {
    pub state: CatalogState,
    pub materials: OverviewPage<OverviewMaterial>,
    pub order_counts: OverviewOrderCounts,
    pub open_orders: OverviewPage<OrderSummary>,
    pub received_orders: OverviewPage<OverviewReceivedOrder>,
    pub receipts: OverviewPage<OverviewReceipt>,
    pub sales: OverviewPage<OverviewSale>,
    pub observations: OverviewPage<OverviewObservation>,
    pub unallocated: OverviewPage<OverviewUnallocated>,
    pub unconfirmed: OverviewPage<OverviewUnconfirmed>,
    pub drafts: OverviewPage<OverviewDraft>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "section", rename_all = "snake_case", deny_unknown_fields)]
pub enum OverviewList {
    Materials {
        page: OverviewPage<OverviewMaterial>,
    },
    MaterialChanges {
        material: OverviewMaterialRef,
        boundary: Box<OverviewChangeBoundary>,
        page: OverviewPage<OverviewMaterialChange>,
    },
    OpenOrders {
        page: OverviewPage<OrderSummary>,
    },
    ReceivedOrders {
        page: OverviewPage<OverviewReceivedOrder>,
    },
    Receipts {
        page: OverviewPage<OverviewReceipt>,
    },
    Sales {
        page: OverviewPage<OverviewSale>,
    },
    Observations {
        page: OverviewPage<OverviewObservation>,
    },
    Unallocated {
        page: OverviewPage<OverviewUnallocated>,
    },
    Unconfirmed {
        page: OverviewPage<OverviewUnconfirmed>,
    },
    Drafts {
        page: OverviewPage<OverviewDraft>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OverviewListResponse {
    pub state: CatalogState,
    pub scope: OverviewScope,
    pub list: OverviewList,
}
