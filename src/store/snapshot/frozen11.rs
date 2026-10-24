//! Schema11 closes reconciliations and original observation/allocation rows before corrections.
//! New empty relations exist only in the validation view; old bytes stay intact.
use super::*;
use crate::recipe::{ExactRatio, RecipeRatioUnavailable};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Snapshot {
    format_version: u32,
    schema_version: i64,
    pub sequence: String,
    request_id: String,
    recorded_at: String,
    state: State,
}

impl Snapshot {
    pub(super) fn into_state(self) -> Result<SnapshotStateV14, StoreError> {
        let mut value = serde_json::to_value(self.state).map_err(unavailable)?;
        // Schema12 compatibility fields are added only to this validation view.
        for name in [
            "correction_records",
            "correction_event_versions",
            "correction_event_heads",
            "correction_projection_versions",
            "correction_projection_heads",
            "correction_price_bases",
            "purchase_returns",
        ] {
            value[name] = serde_json::json!([]);
        }
        value["photo_assets"] = serde_json::json!([]);
        value["material_profile_versions"] = serde_json::json!([]);
        value["material_profile_heads"] = serde_json::json!([]);
        value["material_profile_photos"] = serde_json::json!([]);
        value["material_profile_suppliers"] = serde_json::json!([]);
        serde_json::from_value(value).map_err(unavailable)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    materials: Vec<Material>,
    material_versions: Vec<MaterialVersion>,
    material_heads: Vec<Head>,
    material_prices: Vec<MaterialPriceVersion>,
    catalog_state: CatalogState,
    creation_requests: Vec<LegacyRequest>,
    mutation_requests: Vec<MutationRecord>,
    business_records: Vec<Record>,
    business_revisions: Vec<Revision>,
    business_heads: Vec<business::Head>,
    business_confirmations: Vec<business::ConfirmationRow>,
    business_catalog_price_results: Vec<business::PriceResult>,
    business_corrections: Vec<crate::business::BusinessCorrection>,
    purchase_orders: Vec<purchase_order::OrderRecord>,
    purchase_order_versions: Vec<frozen6::OrderVersion>,
    purchase_order_heads: Vec<purchase_order::OrderHead>,
    receipts: Vec<frozen6::Receipt>,
    holding_versions: Vec<HoldingVersion>,
    holding_heads: Vec<HoldingHead>,
    order_receipt_closures: Vec<receipt::ClosureRow>,
    recipe_versions: Vec<frozen7::RecipeVersion>,
    recipe_lines: Vec<frozen7::RecipeLineRow>,
    recipe_heads: Vec<recipe::Head>,
    productions: Vec<Production>,
    production_inputs: Vec<InputRow>,
    production_outputs: Vec<OutputRow>,
    production_effects: Vec<Effect>,
    sales: Vec<SaleRecord>,
    sale_versions: Vec<SaleVersion>,
    sale_heads: Vec<SaleHead>,
    sale_cost_rows: Vec<CostRow>,
    stocktake_batches: Vec<StocktakeRecord>,
    stocktake_observations: Vec<ObservationRow>,
    stocktake_versions: Vec<StocktakeVersion>,
    stocktake_heads: Vec<StocktakeHead>,
    consumption_versions: Vec<ConsumptionVersion>,
    consumption_heads: Vec<ConsumptionHead>,
    consumption_rows: Vec<ConsumptionRow>,
    wastes: Vec<WasteRecord>,
    receipt_reconciliations: Vec<ReceiptReconciliation>,
    inclusion_links: Vec<LinkRow>,
    inclusion_link_versions: Vec<LinkVersionRow>,
    inclusion_link_heads: Vec<LinkHead>,
    inclusion_reservations: Vec<ReservationRow>,
    reconciliation_material_effects: Vec<MaterialEffectRow>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    CatalogPrice,
    PurchaseOrder,
    Receipt,
    Production,
    Sale,
    Stocktake,
    Waste,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    id: String,
    kind: Kind,
    created_at: String,
    created_sequence: String,
    corrects_confirmation_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Draft,
    Confirmed,
    Archived,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Revision {
    id: String,
    record_id: String,
    previous_revision_id: Option<String>,
    status: Status,
    payload: Payload,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Payload {
    CatalogPrice {
        version: u32,
        data: FrozenPriceDataV4,
    },
    PurchaseOrder {
        version: u32,
        data: FrozenOrderDraftV5,
    },
    Receipt {
        version: u32,
        data: DraftReceiptV1,
    },
    Production {
        version: u32,
        data: ProductionDraft,
    },
    Sale {
        version: u32,
        data: SaleDraft,
    },
    Stocktake {
        version: u32,
        data: DraftStocktakeV1,
    },
    Waste {
        version: u32,
        data: DraftWasteV1,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductionDraft {
    inputs: Vec<DraftInput>,
    outputs: Vec<DraftOutput>,
    note: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DraftInput {
    material_id: Option<String>,
    material_version_id: Option<String>,
    quantity: String,
    unit: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DraftOutput {
    material_id: Option<String>,
    material_version_id: Option<String>,
    quantity: String,
    unit: String,
    allocation_percent: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HoldingVersion {
    reconciliation_effect_id: Option<String>,
    baseline_reconciliation_effect_id: Option<String>,
    id: String,
    material_id: String,
    material_version_id: String,
    previous_version_id: Option<String>,
    receipt_id: Option<String>,
    baseline_receipt_id: Option<String>,
    production_effect_id: Option<String>,
    baseline_production_effect_id: Option<String>,
    observation_version_id: Option<String>,
    baseline_observation_version_id: Option<String>,
    waste_id: Option<String>,
    baseline_seq: String,
    quantity: String,
    value: String,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HoldingHead {
    material_id: String,
    current_version_id: String,
    last_valid_version_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Production {
    id: String,
    confirmation_id: String,
    business_record_id: String,
    note: String,
    total_input_value: String,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputRow {
    production_id: String,
    effect_id: String,
    row: Input,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputRow {
    production_id: String,
    effect_id: String,
    row: Output,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    position: String,
    material_id: String,
    material_version_id: String,
    material_name: String,
    quantity: String,
    unit: String,
    input_quantity: String,
    input_unit: String,
    value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Output {
    position: String,
    material_id: String,
    material_version_id: String,
    material_name: String,
    quantity: String,
    unit: String,
    input_quantity: String,
    input_unit: String,
    value: String,
    allocation_percent: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Effect {
    id: String,
    production_id: String,
    material_id: String,
    material_version_id: String,
    input_quantity: String,
    input_value: String,
    output_quantity: String,
    output_value: String,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SaleDraft {
    pub material_id: Option<String>,
    pub material_version_id: Option<String>,
    pub recipe_version_id: Option<String>,
    pub count: String,
    pub note: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SaleRecord {
    pub id: String,
    pub original_confirmation_id: String,
    pub business_record_id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub recipe_version_id: String,
    pub basis_count: String,
    pub selling_price: Option<String>,
    pub known_subtotal: String,
    pub total_cost: Option<String>,
    pub cost_ratio: Option<ExactRatio>,
    pub ratio_unavailable_reason: Option<RecipeRatioUnavailable>,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CostRow {
    pub sale_id: String,
    pub position: String,
    pub row: SaleRecipeCostRow,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SaleHead {
    pub sale_id: String,
    pub current_version_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SaleVersion {
    pub id: String,
    pub sale_id: String,
    pub previous_version_id: Option<String>,
    pub confirmation_id: String,
    pub business_record_id: String,
    pub count: String,
    pub amount: Option<String>,
    pub note: String,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SaleRecipeCostRow {
    pub line_id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub numerator_quantity: String,
    pub basis_count: String,
    pub unit: String,
    pub price: SalePriceSnapshot,
    pub cost: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SalePriceSnapshot {
    pub source: SalePriceSource,
    pub source_id: Option<String>,
    pub amount: Option<String>,
    pub quantity: Option<String>,
    pub reason: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SalePriceSource {
    HoldingAverage,
    LastValid,
    Preregistered,
    Missing,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum HoldingBasisKind {
    Receipt,
    Production,
    Observation,
    Reconciliation,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HoldingBasisRef {
    kind: HoldingBasisKind,
    id: Option<String>,
    seq: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HoldingAmounts {
    quantity: String,
    value: Option<String>,
    source_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HoldingObservation {
    id: String,
    quantity: String,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DraftStocktakeV1 {
    rows: Vec<DraftStocktakeRowV1>,
    note: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DraftStocktakeRowV1 {
    material_id: Option<String>,
    material_version_id: Option<String>,
    quantity: String,
    unit: String,
    initial_value: Option<StocktakeValueDraft>,
    excess: Option<StocktakeExcessDraft>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum StocktakeValueDraft {
    ExistingPrice {},
    ManualAmount { amount: String },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StocktakeExcessDraft {
    reviewed: bool,
    reason: String,
    value: Option<StocktakeValueDraft>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StocktakeRowEffect {
    position: usize,
    material_id: String,
    material_version_id: String,
    material_name: String,
    unit: String,
    input_quantity: String,
    input_unit: String,
    measured_quantity: String,
    before: Option<HoldingAmounts>,
    after: HoldingAmounts,
    basis_before: Option<HoldingBasisRef>,
    observation_before: Option<HoldingObservation>,
    calculation: StocktakeCalculation,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
enum StocktakeCalculation {
    Initial {
        value: String,
        valuation: StocktakeValuation,
    },
    Settlement {
        consumption_quantity: String,
        consumption_value: String,
        excess_quantity: String,
        excess_value: String,
        valuation: Option<StocktakeValuation>,
        allocation: Box<ConsumptionAllocation>,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum StocktakeValuation {
    ExistingPrice {
        source: StocktakePriceSource,
        source_id: String,
        amount: String,
        quantity: String,
    },
    ManualAmount {
        amount: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StocktakePriceSource {
    HoldingAverage,
    LastValid,
    Preregistered,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConsumptionAllocation {
    basis: HoldingBasisRef,
    end_seq: String,
    as_of_revision: String,
    total_quantity: String,
    total_value: String,
    candidates: Vec<SaleAllocationProposal>,
    unallocated_quantity: String,
    unallocated_value: String,
    reason: Option<NoAllocationReason>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum NoAllocationReason {
    NoSales,
    ZeroWeight,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SaleAllocationProposal {
    sale_id: String,
    sale_version_id: String,
    recipe_version_id: String,
    original_seq: String,
    count: String,
    position: usize,
    quantity: Option<String>,
    value: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StocktakeRecord {
    id: String,
    confirmation_id: String,
    business_record_id: String,
    note: String,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationRow {
    id: String,
    stocktake_id: String,
    position: usize,
    material_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StocktakeVersion {
    id: String,
    observation_id: String,
    previous_version_id: Option<String>,
    previous_observation_version_id: Option<String>,
    effect: StocktakeRowEffect,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StocktakeHead {
    observation_id: String,
    current_version_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConsumptionVersion {
    id: String,
    observation_version_id: String,
    previous_version_id: Option<String>,
    basis: HoldingBasisRef,
    end_seq: String,
    as_of_revision: String,
    total_quantity: String,
    total_value: String,
    unallocated_quantity: String,
    unallocated_value: String,
    reason: Option<NoAllocationReason>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConsumptionHead {
    observation_id: String,
    current_version_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConsumptionRow {
    consumption_version_id: String,
    row: SaleAllocationProposal,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WasteReason {
    Discard,
    StaffMeal,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DraftWasteV1 {
    material_id: Option<String>,
    material_version_id: Option<String>,
    quantity: String,
    unit: String,
    reason: WasteReason,
    note: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WasteEffect {
    material_id: String,
    material_version_id: String,
    material_name: String,
    unit: String,
    input_quantity: String,
    input_unit: String,
    quantity: String,
    reason: WasteReason,
    before: HoldingAmounts,
    after: HoldingAmounts,
    basis_before: HoldingBasisRef,
    observation: Option<HoldingObservation>,
    loss_value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WasteRecord {
    id: String,
    confirmation_id: String,
    business_record_id: String,
    effect: WasteEffect,
    note: String,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ReceiptSourceDraft {
    Direct {
        material_id: Option<String>,
        material_version_id: Option<String>,
    },
    Order {
        order_id: Option<String>,
        expected_order_version_id: Option<String>,
        material_version_id: Option<String>,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ReceiptStockEffectDraft {
    Unreflected {},
    NeedsReconciliation {
        basis_id: Option<String>,
        included_quantity: String,
    },
    Reconciled {
        selections: Vec<ReceiptInclusionSelection>,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DraftReceiptV1 {
    source: ReceiptSourceDraft,
    quantity: String,
    unit: String,
    amount: String,
    receipt_date: String,
    note: String,
    stock_effect: ReceiptStockEffectDraft,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrderReceiptPreview {
    status: crate::purchase_order::OrderStatus,
    order_id: String,
    initial_order_version_id: String,
    initial_ordered_quantity: String,
    ordered_version_id: String,
    ordered_quantity: String,
    received_quantity: String,
    difference: String,
    unit: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SourceKind {
    ReceiptInflow,
    ObservationInitial,
    ObservationExcess,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceKey {
    kind: SourceKind,
    id: String,
    position: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum InclusionSourceSelection {
    AllAvailable {},
    Source { source_key: SourceKey },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptInclusionSelection {
    observation_version_id: Option<String>,
    source: Option<InclusionSourceSelection>,
    included_quantity: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceShare {
    numerator: String,
    denominator: String,
    unit: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReconciledReceiptEffect {
    material_id: String,
    material_version_id: String,
    material_name: String,
    quantity: String,
    amount: String,
    unit: String,
    input_quantity: String,
    input_unit: String,
    receipt_date: String,
    included_quantity: String,
    unreflected_quantity: String,
    included_amount: String,
    unreflected_amount: String,
    booked_value: String,
    value_difference: String,
    separate_adjustment: String,
    selections: Vec<ReceiptInclusionEffect>,
    materials: Vec<ReconciliationMaterialEffect>,
    trace: Vec<ReconciliationValueTrace>,
    order_before: Option<Box<frozen6::OrderVersion>>,
    order_after: Option<OrderReceiptPreview>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptInclusionEffect {
    position: usize,
    stocktake_id: String,
    observation_id: String,
    observation_version_id: String,
    source: InclusionSourceSelection,
    included_quantity: String,
    unit: String,
    booked_value: String,
    actual_included_value: String,
    value_difference: String,
    portions: Vec<InclusionPortionEffect>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InclusionPortionEffect {
    position: usize,
    source_key: SourceKey,
    source_version_id: String,
    included_share: SourceShare,
    reserved_source_share: SourceShare,
    booked_value: String,
    actual_included_value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReconciliationMaterialEffect {
    material_id: String,
    material_version_id: String,
    material_name: String,
    unit: String,
    before: Option<HoldingAmounts>,
    after: HoldingAmounts,
    basis_before: Option<HoldingBasisRef>,
    observation: Option<HoldingObservation>,
    value_delta: String,
    inflow_quantity: String,
    inflow_value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptReconciliation {
    id: String,
    receipt_id: String,
    confirmation_id: String,
    business_record_id: String,
    effect: ReconciledReceiptEffect,
    selection_results: Vec<ReceiptInclusionResult>,
    material_results: Vec<ReconciliationMaterialResult>,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptInclusionResult {
    selection_position: usize,
    portion_position: usize,
    link_id: String,
    link_version_id: String,
    reservation_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReconciliationMaterialResult {
    material_id: String,
    effect_id: String,
    holding_version_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReconciliationTraceKind {
    Inflow,
    Observation,
    Production,
    Waste,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReconciliationValueTrace {
    kind: ReconciliationTraceKind,
    event_id: String,
    seq: String,
    material_id: String,
    quantity_before: String,
    quantity_after: String,
    base_value_before: String,
    base_value_after: String,
    proposed_value_before: String,
    proposed_value_after: String,
    base_closed_value: String,
    proposed_closed_value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkRow {
    id: String,
    reconciliation_id: String,
    selection_position: usize,
    portion_position: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkVersionRow {
    id: String,
    link_id: String,
    previous_version_id: Option<String>,
    observation_version_id: String,
    portion: InclusionPortionEffect,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkHead {
    link_id: String,
    current_version_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReservationRow {
    id: String,
    link_version_id: String,
    source_key: SourceKey,
    source_version_id: String,
    reserved_source_share: SourceShare,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MaterialEffectRow {
    id: String,
    reconciliation_id: String,
    effect: ReconciliationMaterialEffect,
    recorded_at: String,
    seq: String,
}
