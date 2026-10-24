//! Schema7 has recipes and receipt-only holdings, with three business kinds.
//! Conversion is for validation; the original journal bytes are never rewritten.
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
    pub(super) fn into_state(self) -> Result<SnapshotStateV7, StoreError> {
        serde_json::from_value(serde_json::to_value(self.state).map_err(unavailable)?)
            .map_err(unavailable)
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
    business_records: Vec<frozen6::Record>,
    business_revisions: Vec<frozen6::Revision>,
    business_heads: Vec<business::Head>,
    business_confirmations: Vec<business::ConfirmationRow>,
    business_catalog_price_results: Vec<business::PriceResult>,
    business_corrections: Vec<crate::business::BusinessCorrection>,
    purchase_orders: Vec<purchase_order::OrderRecord>,
    purchase_order_versions: Vec<frozen6::OrderVersion>,
    purchase_order_heads: Vec<purchase_order::OrderHead>,
    receipts: Vec<frozen6::Receipt>,
    holding_versions: Vec<holding::LegacyHoldingVersion>,
    holding_heads: Vec<holding::HoldingHead>,
    order_receipt_closures: Vec<receipt::ClosureRow>,
    recipe_versions: Vec<RecipeVersion>,
    recipe_lines: Vec<RecipeLineRow>,
    recipe_heads: Vec<recipe::Head>,
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
    reference_output: Option<ReferenceOutput>,
    selling_price: Option<String>,
    cooking_description: String,
    process: String,
    reference_time: String,
    recorded_at: String,
    seq: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReferenceOutput {
    quantity: String,
    unit: String,
    input_quantity: String,
    input_unit: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecipeLineRow {
    recipe_version_id: String,
    line: RecipeLine,
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
