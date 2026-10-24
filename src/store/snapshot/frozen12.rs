//! Frozen schema12 wire types from the last pre-return commit.
//! Compatibility fields exist only in a validation view; old bytes remain intact.
use super::*;

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
        value["purchase_returns"] = serde_json::json!([]);
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
    material_heads: Vec<CatalogHead>,
    material_prices: Vec<MaterialPriceVersion>,
    catalog_state: CatalogState,
    creation_requests: Vec<LegacyRequest>,
    mutation_requests: Vec<MutationRecord>,
    business_records: Vec<Record>,
    business_revisions: Vec<Revision>,
    business_heads: Vec<BusinessHead>,
    business_confirmations: Vec<ConfirmationRow>,
    business_catalog_price_results: Vec<PriceResult>,
    business_corrections: Vec<BusinessCorrection>,
    purchase_orders: Vec<OrderRecord>,
    purchase_order_versions: Vec<OrderVersion>,
    purchase_order_heads: Vec<OrderHead>,
    receipts: Vec<ReceiptRecord>,
    holding_versions: Vec<HoldingVersion>,
    holding_heads: Vec<HoldingHead>,
    order_receipt_closures: Vec<ClosureRow>,
    recipe_versions: Vec<RecipeVersion>,
    recipe_lines: Vec<RecipeLineRow>,
    recipe_heads: Vec<RecipeHead>,
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
    correction_records: Vec<CorrectionRecord>,
    correction_event_versions: Vec<EventVersionRow>,
    correction_event_heads: Vec<EventHead>,
    correction_projection_versions: Vec<ProjectionVersionRow>,
    correction_projection_heads: Vec<ProjectionHead>,
    correction_price_bases: Vec<CorrectionPriceBasisRow>,
}

#[derive(Serialize, Deserialize)]
struct Material {
    id: String,
    name: String,
    unit: String,
    created_at: String,
}

#[derive(Serialize, Deserialize)]
struct MaterialVersion {
    id: String,
    material_id: String,
    name: String,
    unit: String,
    canonical_unit: String,
    dimension: String,
    kind: String,
    description: String,
    archived: bool,
    recorded_at: String,
    seq: Option<String>,
    source: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogHead {
    material_id: String,
    current_version_id: String,
}

#[derive(Serialize, Deserialize)]
struct MaterialPriceVersion {
    id: String,
    material_id: String,
    material_version_id: String,
    quantity: String,
    amount: String,
    unit: String,
    input_quantity: String,
    input_unit: String,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
struct CatalogState {
    epoch: String,
    revision: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyRequest {
    request_id: String,
    name: String,
    unit: String,
    material_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MutationRecord {
    request_id: String,
    operation: String,
    payload: String,
    response: String,
    epoch: String,
    sequence: String,
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
#[serde(deny_unknown_fields)]
struct BusinessHead {
    record_id: String,
    current_revision_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfirmationRow {
    id: String,
    record_id: String,
    revision_id: String,
    request_id: String,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PriceResult {
    confirmation_id: String,
    price_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BusinessCorrection {
    target_confirmation_id: String,
    replacement_confirmation_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrderRecord {
    id: String,
    first_confirmation_id: String,
    created_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrderVersion {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    correction_projection_id: Option<String>,
    id: String,
    order_id: String,
    previous_version_id: Option<String>,
    confirmation_id: String,
    business_record_id: String,
    action: OrderAction,
    status: OrderStatus,
    material_id: String,
    material_version_id: String,
    material_name: String,
    quantity: String,
    unit: String,
    input_quantity: String,
    input_unit: String,
    supplier: Option<String>,
    expected_date: Option<String>,
    estimated_amount: Option<String>,
    note: String,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrderHead {
    order_id: String,
    current_version_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptRecord {
    id: String,
    confirmation_id: String,
    business_record_id: String,
    material_id: String,
    material_version_id: String,
    material_name: String,
    quantity: String,
    unit: String,
    input_quantity: String,
    input_unit: String,
    amount: String,
    receipt_date: String,
    note: String,
    recorded_at: String,
    seq: String,
    source: ReceiptSource,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct HoldingVersion {
    correction_projection_id: Option<String>,
    baseline_correction_projection_id: Option<String>,
    last_valid_correction_price_id: Option<String>,
    correction_last_valid_price: Option<PriceBasis>,
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
    current_version_id: Option<String>,
    last_valid_version_id: Option<String>,
    unknown_correction_projection_id: Option<String>,
    last_valid_correction_price_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClosureRow {
    order_id: String,
    receipt_id: String,
    confirmation_id: String,
    initial_version_id: String,
    ordered_version_id: String,
    received_version_id: String,
    received_quantity: String,
    difference: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecipeVersion {
    id: String,
    material_id: String,
    material_version_id: String,
    material_name: String,
    material_unit: String,
    previous_version_id: Option<String>,
    copied_from_version_id: Option<String>,
    basis_count: String,
    reference_output: Option<RecipeReferenceOutput>,
    selling_price: Option<String>,
    cooking_description: String,
    process: String,
    reference_time: String,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecipeLineRow {
    recipe_version_id: String,
    line: RecipeLine,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecipeHead {
    material_id: String,
    current_version_id: String,
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
struct SaleRecord {
    id: String,
    original_confirmation_id: String,
    business_record_id: String,
    material_id: String,
    material_version_id: String,
    material_name: String,
    recipe_version_id: String,
    basis_count: String,
    selling_price: Option<String>,
    known_subtotal: String,
    total_cost: Option<String>,
    cost_ratio: Option<ExactRatio>,
    ratio_unavailable_reason: Option<RecipeRatioUnavailable>,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SaleVersion {
    id: String,
    sale_id: String,
    previous_version_id: Option<String>,
    confirmation_id: String,
    business_record_id: String,
    count: String,
    amount: Option<String>,
    note: String,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SaleHead {
    sale_id: String,
    current_version_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CostRow {
    sale_id: String,
    position: String,
    row: SaleRecipeCostRow,
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
    correction_projection_id: Option<String>,
    logical_seq: Option<String>,
    observed_at: Option<String>,
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
    correction_projection_id: Option<String>,
    seq: Option<String>,
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

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionRecord {
    id: String,
    original_id: String,
    target_confirmation_id: String,
    previous_version_id: Option<String>,
    confirmation_id: String,
    business_record_id: String,
    reason: String,
    input: DraftCorrectionV1,
    plan: CorrectionPlan,
    resolved_refs: Vec<CorrectionResolvedRef>,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EventVersionRow {
    id: String,
    sale_version_id: Option<String>,
    correction_id: String,
    previous_version_id: Option<String>,
    previous_original_version_id: Option<String>,
    reference: CorrectionEventRef,
    after: Option<CorrectionEventValue>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EventHead {
    kind: CorrectionSubjectKind,
    original_id: String,
    current_version_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectionVersionRow {
    id: String,
    correction_id: String,
    previous_version_id: Option<String>,
    recorded_at: String,
    seq: String,
    projection: Projection,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectionHead {
    kind: ProjectionKind,
    subject_id: String,
    current_version_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionPriceBasisRow {
    id: String,
    correction_id: String,
    material_id: String,
    material_version_id: String,
    amount: String,
    quantity: String,
    source: CorrectionPlanRef,
    logical_seq: String,
    recorded_at: String,
    seq: String,
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
    Correction,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Status {
    Draft,
    Confirmed,
    Archived,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Payload {
    CatalogPrice {
        version: u32,
        data: DraftPriceV1,
    },
    PurchaseOrder {
        version: u32,
        data: DraftOrderActionV1,
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
    Correction {
        version: u32,
        data: DraftCorrectionV1,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OrderAction {
    Place,
    Revise,
    Cancel,
    Receive,
    Correct,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OrderStatus {
    Open,
    Received,
    Cancelled,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ReceiptSource {
    Direct {},
    Order {
        order_id: String,
        ordered_version_id: String,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PriceBasis {
    amount: String,
    quantity: String,
    source_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecipeReferenceOutput {
    quantity: String,
    unit: String,
    input_quantity: String,
    input_unit: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecipeLine {
    id: String,
    position: String,
    material_id: String,
    material_version_id: String,
    material_name: String,
    quantity: String,
    unit: String,
    input_quantity: String,
    input_unit: String,
    note: String,
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
struct ExactRatio {
    numerator: String,
    denominator: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RecipeRatioUnavailable {
    MissingSellingPrice,
    ZeroSellingPrice,
    MissingCost,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SaleRecipeCostRow {
    line_id: String,
    material_id: String,
    material_version_id: String,
    material_name: String,
    numerator_quantity: String,
    basis_count: String,
    unit: String,
    price: SalePriceSnapshot,
    cost: Option<String>,
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
#[serde(deny_unknown_fields)]
struct HoldingBasisRef {
    kind: HoldingBasisKind,
    id: Option<String>,
    seq: Option<String>,
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
    order_before: Option<Box<OrderVersion>>,
    order_after: Option<OrderReceiptPreview>,
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
struct SourceKey {
    kind: SourceKind,
    id: String,
    position: usize,
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
struct DraftCorrectionV1 {
    expected_target_version_id: Option<String>,
    reason: String,
    action: CorrectionActionDraft,
    observation_reviews: Vec<ObservationReviewDraft>,
    linked_receipt_changes: Vec<LinkedReceiptChangeDraft>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionPlan {
    target: CorrectionEventRef,
    events: Vec<CorrectionEventChange>,
    materials: Vec<CorrectionMaterialChange>,
    observations: Vec<CorrectionObservationChange>,
    allocations: Vec<CorrectionAllocationChange>,
    inclusions: Vec<CorrectionInclusionChange>,
    separate_adjustments: Vec<CorrectionSeparateAdjustmentChange>,
    order_closures: Vec<CorrectionOrderClosureChange>,
    review_requests: Vec<CorrectionReviewRequest>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionResolvedRef {
    reference: CorrectionPlanRef,
    version_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionEventRef {
    kind: CorrectionSubjectKind,
    original_id: String,
    original_confirmation_id: String,
    confirmation_id: String,
    version_id: String,
    business_record_id: String,
    logical_seq: String,
    logical_position: usize,
    original_recorded_at: String,
    recorded_at: String,
    seq: String,
}

#[allow(
    clippy::large_enum_variant,
    reason = "Replay and persistence share these owned event snapshots; the enclosing business correction plan is boxed"
)]
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum CorrectionEventValue {
    Receipt {
        input: DraftReceiptV1,
        reconciliation: Option<ReconciledReceiptEffect>,
    },
    Production {
        input: ProductionDraft,
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CorrectionSubjectKind {
    Receipt,
    Production,
    SaleCount,
    Waste,
    Stocktake,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Projection {
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ProjectionKind {
    Material,
    Observation,
    Allocation,
    Inclusion,
    OrderClosure,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum CorrectionPlanRef {
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

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DraftPriceV1 {
    material_id: Option<String>,
    material_version_id: Option<String>,
    quantity: String,
    unit: String,
    amount: String,
    note: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum DraftOrderActionV1 {
    Place {
        material_id: Option<String>,
        material_version_id: Option<String>,
        quantity: String,
        unit: String,
        supplier: String,
        expected_date: String,
        estimated_amount: String,
        note: String,
    },
    Revise {
        order_id: String,
        expected_order_version_id: String,
        material_version_id: Option<String>,
        quantity: String,
        unit: String,
        supplier: String,
        expected_date: String,
        estimated_amount: String,
        note: String,
    },
    Cancel {
        order_id: String,
        expected_order_version_id: String,
        note: String,
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
struct ProductionDraft {
    inputs: Vec<DraftInput>,
    outputs: Vec<DraftOutput>,
    note: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SaleDraft {
    material_id: Option<String>,
    material_version_id: Option<String>,
    recipe_version_id: Option<String>,
    count: String,
    note: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DraftStocktakeV1 {
    rows: Vec<DraftStocktakeRowV1>,
    note: String,
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
struct SalePriceSnapshot {
    source: SalePriceSource,
    source_id: Option<String>,
    amount: Option<String>,
    quantity: Option<String>,
    reason: Option<String>,
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
#[serde(rename_all = "snake_case")]
enum HoldingBasisKind {
    Receipt,
    Production,
    Observation,
    Reconciliation,
    Correction,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WasteReason {
    Discard,
    StaffMeal,
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
struct OrderReceiptPreview {
    status: OrderStatus,
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

#[allow(
    clippy::large_enum_variant,
    reason = "Owned draft payloads stay inline across normalization and replay without an extra allocation per replacement"
)]
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum CorrectionActionDraft {
    Void {},
    Replace {
        replacement: CorrectionReplacementDraft,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservationReviewDraft {
    observation_id: Option<String>,
    expected_version_id: Option<String>,
    initial_value: Option<StocktakeValueDraft>,
    excess: Option<StocktakeExcessDraft>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LinkedReceiptChangeDraft {
    receipt_id: Option<String>,
    expected_version_id: Option<String>,
    action: LinkedReceiptActionDraft,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionEventChange {
    reference: CorrectionEventRef,
    before: Option<CorrectionEventValue>,
    after: Option<CorrectionEventValue>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionMaterialChange {
    material_id: String,
    material_version_id: String,
    material_name: String,
    unit: String,
    before: CorrectionHoldingState,
    after: CorrectionHoldingState,
    quantity_delta: String,
    value_delta: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionObservationChange {
    reference: CorrectionPlanRef,
    stocktake_id: String,
    logical_seq: String,
    before: Option<StocktakeRowEffect>,
    after: Option<StocktakeRowEffect>,
    /// Derived price evidence for the proposed observation. The store resolves
    /// this reference into a real price-basis row without inventing a holding.
    valuation_basis: Option<CorrectionPriceBasis>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionAllocationChange {
    observation: CorrectionPlanRef,
    expected_consumption_version_id: Option<String>,
    before: Option<ConsumptionAllocation>,
    after: Option<ConsumptionAllocation>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionInclusionChange {
    receipt_id: String,
    expected_version_id: String,
    before: Option<ReconciledReceiptEffect>,
    after: Option<ReconciledReceiptEffect>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionSeparateAdjustmentChange {
    receipt_id: String,
    before: String,
    after: String,
    delta: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionOrderClosureChange {
    order_id: String,
    expected_order_version_id: String,
    before: Option<OrderReceiptPreview>,
    after: Option<OrderReceiptPreview>,
}

#[allow(
    clippy::large_enum_variant,
    reason = "Review DTOs carry the complete proposed context by value and live inside the boxed correction plan"
)]
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum CorrectionReviewRequest {
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

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductionPreview {
    inputs: Vec<Input>,
    outputs: Vec<Output>,
    materials: Vec<ProductionMaterialEffect>,
    total_input_value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DraftSaleCountCorrectionV1 {
    count: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SaleProposal {
    material_id: String,
    material_version_id: String,
    material_name: String,
    recipe_version_id: String,
    count: String,
    amount: Option<String>,
    recipe_cost: SaleRecipeCost,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DraftStocktakeCorrectionV1 {
    rows: Vec<StocktakeCorrectionRowDraft>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InclusionVersionRow {
    id: String,
    link_id: String,
    previous_version_id: Option<String>,
    reservation_id: String,
    observation_version_id: String,
    selection_position: usize,
    portion_position: usize,
    portion: InclusionPortionEffect,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CorrectionRefKind {
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
struct DraftStocktakeRowV1 {
    material_id: Option<String>,
    material_version_id: Option<String>,
    quantity: String,
    unit: String,
    initial_value: Option<StocktakeValueDraft>,
    excess: Option<StocktakeExcessDraft>,
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
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum InclusionSourceSelection {
    AllAvailable {},
    Source { source_key: SourceKey },
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
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum CorrectionReplacementDraft {
    Receipt { data: DraftReceiptV1 },
    Production { data: ProductionDraft },
    SaleCount { data: DraftSaleCountCorrectionV1 },
    Waste { data: DraftWasteV1 },
    Stocktake { data: DraftStocktakeCorrectionV1 },
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

#[allow(
    clippy::large_enum_variant,
    reason = "Linked replacements reuse the owned receipt draft used by normalization and replay"
)]
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum LinkedReceiptActionDraft {
    Void {},
    Replace { data: DraftReceiptV1 },
}

#[allow(
    clippy::large_enum_variant,
    reason = "Known holding evidence is one cohesive state; keep its optional boundaries inline for replay and comparison"
)]
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum CorrectionHoldingState {
    NotEstablished {},
    Known {
        amounts: HoldingAmounts,
        projection_seq: Option<String>,
        last_valid_price: Option<CorrectionPriceBasis>,
        observation_boundary: Option<CorrectionObservationBoundary>,
        initial_boundary: Option<CorrectionInitialBoundary>,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionPriceBasis {
    amount: String,
    quantity: String,
    source: CorrectionPlanRef,
    logical_seq: String,
}

#[derive(Serialize, Deserialize)]
struct ResolvedPrice {
    source: String,
    source_id: Option<String>,
    amount: Option<String>,
    quantity: Option<String>,
    reason: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum CorrectionValuationMode {
    Initial,
    Excess,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionInclusionCandidate {
    observation: CorrectionPlanRef,
    stocktake_id: String,
    material_id: String,
    material_version_id: String,
    material_name: String,
    unit: String,
    quantity: String,
    value: String,
    available_share: SourceShare,
    available_quantity: String,
    logical_seq: String,
    sources: Vec<CorrectionInclusionSourceCandidate>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProductionMaterialEffect {
    material_id: String,
    material_version_id: String,
    unit: String,
    before: Option<HoldingAmounts>,
    after: HoldingAmounts,
    basis_before: Option<HoldingBasisRef>,
    observation: Option<HoldingObservation>,
    input_quantity: String,
    input_value: String,
    output_quantity: String,
    output_value: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SaleRecipeCost {
    recipe_version_id: String,
    basis_count: String,
    rows: Vec<SaleRecipeCostRow>,
    missing_price_line_ids: Vec<String>,
    known_subtotal: String,
    total_cost: Option<String>,
    selling_price: Option<String>,
    cost_ratio: Option<ExactRatio>,
    ratio_unavailable_reason: Option<RecipeRatioUnavailable>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StocktakeCorrectionRowDraft {
    observation_id: Option<String>,
    expected_version_id: Option<String>,
    action: ObservationCorrectionActionDraft,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptInclusionSelection {
    observation_version_id: Option<String>,
    source: Option<InclusionSourceSelection>,
    included_quantity: String,
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
struct CorrectionObservationBoundary {
    reference: CorrectionPlanRef,
    stocktake_id: String,
    logical_seq: String,
    observed_at: String,
    quantity: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionInitialBoundary {
    reference: CorrectionPlanRef,
    logical_seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorrectionInclusionSourceCandidate {
    source_key: SourceKey,
    source_ref: CorrectionPlanRef,
    source_material_id: String,
    source_material_name: String,
    source_unit: String,
    source_quantity: String,
    reserved_source_share: SourceShare,
    remaining_source_share: SourceShare,
    available_share: SourceShare,
    available_quantity: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ObservationCorrectionActionDraft {
    Void {},
    Replace {
        quantity: String,
        unit: String,
        initial_value: Option<StocktakeValueDraft>,
        excess: Option<StocktakeExcessDraft>,
    },
}
