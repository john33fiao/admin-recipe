//! Schema9 closes sale inputs and receipt/production holdings before observations.
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
    pub(super) fn into_state(self) -> Result<SnapshotStateV12, StoreError> {
        let mut value = serde_json::to_value(self.state).map_err(unavailable)?;
        for name in [
            "stocktake_batches",
            "stocktake_observations",
            "stocktake_versions",
            "stocktake_heads",
            "consumption_versions",
            "consumption_heads",
            "consumption_rows",
            "wastes",
        ] {
            value[name] = serde_json::json!([]);
        }
        for name in [
            "receipt_reconciliations",
            "inclusion_links",
            "inclusion_link_versions",
            "inclusion_link_heads",
            "inclusion_reservations",
            "reconciliation_material_effects",
        ] {
            value[name] = serde_json::json!([]);
        }
        for name in [
            "correction_records",
            "correction_event_versions",
            "correction_event_heads",
            "correction_projection_versions",
            "correction_projection_heads",
            "correction_price_bases",
        ] {
            value[name] = serde_json::json!([]);
        }
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
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    CatalogPrice,
    PurchaseOrder,
    Receipt,
    Production,
    Sale,
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
        data: frozen6::ReceiptDraft,
    },
    Production {
        version: u32,
        data: ProductionDraft,
    },
    Sale {
        version: u32,
        data: SaleDraft,
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
    id: String,
    material_id: String,
    material_version_id: String,
    previous_version_id: Option<String>,
    receipt_id: Option<String>,
    baseline_receipt_id: Option<String>,
    production_effect_id: Option<String>,
    baseline_production_effect_id: Option<String>,
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
    last_valid_version_id: String,
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
