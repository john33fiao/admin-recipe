//! Schema8 closes the four original business kinds and production holdings.
//! Empty sale relations are a validation view, never a rewrite of old bytes.
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
    pub(super) fn into_state(self) -> Result<SnapshotStateV10, StoreError> {
        let mut value = serde_json::to_value(self.state).map_err(unavailable)?;
        for name in [
            "sales",
            "sale_versions",
            "sale_heads",
            "sale_cost_rows",
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
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    CatalogPrice,
    PurchaseOrder,
    Receipt,
    Production,
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
