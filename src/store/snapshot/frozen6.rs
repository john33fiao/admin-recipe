//! The schema6 journal shape stays closed when live business DTOs evolve.
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
    pub(super) fn into_state(self) -> Result<SnapshotStateV6, StoreError> {
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
    business_records: Vec<Record>,
    business_revisions: Vec<Revision>,
    business_heads: Vec<business::Head>,
    business_confirmations: Vec<business::ConfirmationRow>,
    business_catalog_price_results: Vec<business::PriceResult>,
    business_corrections: Vec<crate::business::BusinessCorrection>,
    purchase_orders: Vec<purchase_order::OrderRecord>,
    purchase_order_versions: Vec<OrderVersion>,
    purchase_order_heads: Vec<purchase_order::OrderHead>,
    receipts: Vec<Receipt>,
    holding_versions: Vec<holding::LegacyHoldingVersion>,
    holding_heads: Vec<holding::HoldingHead>,
    order_receipt_closures: Vec<receipt::ClosureRow>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Kind {
    CatalogPrice,
    PurchaseOrder,
    Receipt,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Record {
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
pub(super) struct Revision {
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
        data: ReceiptDraft,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptDraft {
    source: ReceiptSourceDraft,
    quantity: String,
    unit: String,
    amount: String,
    receipt_date: String,
    note: String,
    stock_effect: StockEffect,
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
enum StockEffect {
    Unreflected {},
    NeedsReconciliation {
        basis_id: Option<String>,
        included_quantity: String,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OrderAction {
    Place,
    Revise,
    Cancel,
    Receive,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OrderStatus {
    Open,
    Cancelled,
    Received,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OrderVersion {
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
pub(super) struct Receipt {
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
