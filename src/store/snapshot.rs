use super::{
    Material, SCHEMA_VERSION, StoreError, business, catalog, holding, production, purchase_order,
    read_materials, receipt, recipe, reconciliation, sale, stocktake, unavailable, waste,
};
use crate::catalog::{CatalogState, MaterialPriceVersion, MaterialVersion};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
mod frozen10;
mod frozen6;
mod frozen7;
mod frozen8;
mod frozen9;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SnapshotV2Schema3 {
    pub format_version: u32,
    pub schema_version: i64,
    pub sequence: String,
    pub request_id: String,
    pub recorded_at: String,
    state: CatalogSnapshotState,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CatalogSnapshotState {
    materials: Vec<Material>,
    material_versions: Vec<MaterialVersion>,
    material_heads: Vec<Head>,
    material_prices: Vec<MaterialPriceVersion>,
    catalog_state: CatalogState,
    creation_requests: Vec<LegacyRequest>,
    mutation_requests: Vec<MutationRecord>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Head {
    material_id: String,
    current_version_id: String,
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

// Schema 4 predates purchase orders. Its reader must not inherit new live
// payload variants when the application learns another business kind.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FrozenPriceKindV4 {
    CatalogPrice,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenRecordV4 {
    id: String,
    kind: FrozenPriceKindV4,
    created_at: String,
    created_sequence: String,
    corrects_confirmation_id: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenPriceDataV4 {
    material_id: Option<String>,
    material_version_id: Option<String>,
    quantity: String,
    unit: String,
    amount: String,
    note: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenPayloadV4 {
    kind: FrozenPriceKindV4,
    version: u32,
    data: FrozenPriceDataV4,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenRevisionV4 {
    id: String,
    record_id: String,
    previous_revision_id: Option<String>,
    status: crate::business::BusinessStatus,
    payload: FrozenPayloadV4,
    recorded_at: String,
    seq: String,
}

pub(super) fn read_catalog_state(
    connection: &Connection,
) -> Result<CatalogSnapshotState, StoreError> {
    let heads = connection
        .prepare("SELECT material_id,current_version_id FROM material_heads ORDER BY material_id")
        .map_err(unavailable)?
        .query_map([], |r| {
            Ok(Head {
                material_id: r.get(0)?,
                current_version_id: r.get(1)?,
            })
        })
        .map_err(unavailable)?
        .collect::<rusqlite::Result<_>>()
        .map_err(unavailable)?;
    let legacy = connection
        .prepare(
            "SELECT request_id,name,unit,material_id FROM creation_requests ORDER BY request_id",
        )
        .map_err(unavailable)?
        .query_map([], |r| {
            Ok(LegacyRequest {
                request_id: r.get(0)?,
                name: r.get(1)?,
                unit: r.get(2)?,
                material_id: r.get(3)?,
            })
        })
        .map_err(unavailable)?
        .collect::<rusqlite::Result<_>>()
        .map_err(unavailable)?;
    let requests = connection
        .prepare(
            "SELECT request_id,operation,payload,response,epoch,sequence
                  FROM mutation_requests ORDER BY sequence,request_id",
        )
        .map_err(unavailable)?
        .query_map([], |row| {
            Ok(MutationRecord {
                request_id: row.get(0)?,
                operation: row.get(1)?,
                payload: row.get(2)?,
                response: row.get(3)?,
                epoch: row.get(4)?,
                sequence: row.get::<_, i64>(5)?.to_string(),
            })
        })
        .map_err(unavailable)?
        .collect::<rusqlite::Result<_>>()
        .map_err(unavailable)?;
    Ok(CatalogSnapshotState {
        materials: read_materials(connection).map_err(unavailable)?,
        material_versions: catalog::versions(connection, None)?,
        material_heads: heads,
        material_prices: catalog::prices(connection, None)?,
        catalog_state: catalog::state(connection)?,
        creation_requests: legacy,
        mutation_requests: requests,
    })
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotV2Schema4 {
    format_version: u32,
    schema_version: i64,
    sequence: String,
    request_id: String,
    recorded_at: String,
    state: SnapshotStateV4,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotStateV4 {
    materials: Vec<Material>,
    material_versions: Vec<MaterialVersion>,
    material_heads: Vec<Head>,
    material_prices: Vec<MaterialPriceVersion>,
    catalog_state: CatalogState,
    creation_requests: Vec<LegacyRequest>,
    mutation_requests: Vec<MutationRecord>,
    business_records: Vec<FrozenRecordV4>,
    business_revisions: Vec<FrozenRevisionV4>,
    business_heads: Vec<business::Head>,
    business_confirmations: Vec<business::ConfirmationRow>,
    business_catalog_price_results: Vec<business::PriceResult>,
    business_corrections: Vec<crate::business::BusinessCorrection>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotV2Schema5 {
    format_version: u32,
    schema_version: i64,
    sequence: String,
    request_id: String,
    recorded_at: String,
    state: SnapshotStateV5,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotStateV5 {
    materials: Vec<Material>,
    material_versions: Vec<MaterialVersion>,
    material_heads: Vec<Head>,
    material_prices: Vec<MaterialPriceVersion>,
    catalog_state: CatalogState,
    creation_requests: Vec<LegacyRequest>,
    mutation_requests: Vec<MutationRecord>,
    business_records: Vec<FrozenRecordV5>,
    business_revisions: Vec<FrozenRevisionV5>,
    business_heads: Vec<business::Head>,
    business_confirmations: Vec<business::ConfirmationRow>,
    business_catalog_price_results: Vec<business::PriceResult>,
    business_corrections: Vec<crate::business::BusinessCorrection>,
    purchase_orders: Vec<purchase_order::OrderRecord>,
    purchase_order_versions: Vec<FrozenOrderVersionV5>,
    purchase_order_heads: Vec<purchase_order::OrderHead>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotV2Schema11 {
    format_version: u32,
    schema_version: i64,
    sequence: String,
    request_id: String,
    recorded_at: String,
    state: SnapshotStateV11,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotStateV6 {
    materials: Vec<Material>,
    material_versions: Vec<MaterialVersion>,
    material_heads: Vec<Head>,
    material_prices: Vec<MaterialPriceVersion>,
    catalog_state: CatalogState,
    creation_requests: Vec<LegacyRequest>,
    mutation_requests: Vec<MutationRecord>,
    business_records: Vec<business::Record>,
    business_revisions: Vec<crate::business::BusinessRevision>,
    business_heads: Vec<business::Head>,
    business_confirmations: Vec<business::ConfirmationRow>,
    business_catalog_price_results: Vec<business::PriceResult>,
    business_corrections: Vec<crate::business::BusinessCorrection>,
    purchase_orders: Vec<purchase_order::OrderRecord>,
    purchase_order_versions: Vec<crate::purchase_order::OrderVersion>,
    purchase_order_heads: Vec<purchase_order::OrderHead>,
    receipts: Vec<crate::receipt::ReceiptRecord>,
    holding_versions: Vec<holding::LegacyHoldingVersion>,
    holding_heads: Vec<holding::HoldingHead>,
    order_receipt_closures: Vec<receipt::ClosureRow>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotStateV7 {
    materials: Vec<Material>,
    material_versions: Vec<MaterialVersion>,
    material_heads: Vec<Head>,
    material_prices: Vec<MaterialPriceVersion>,
    catalog_state: CatalogState,
    creation_requests: Vec<LegacyRequest>,
    mutation_requests: Vec<MutationRecord>,
    business_records: Vec<business::Record>,
    business_revisions: Vec<crate::business::BusinessRevision>,
    business_heads: Vec<business::Head>,
    business_confirmations: Vec<business::ConfirmationRow>,
    business_catalog_price_results: Vec<business::PriceResult>,
    business_corrections: Vec<crate::business::BusinessCorrection>,
    purchase_orders: Vec<purchase_order::OrderRecord>,
    purchase_order_versions: Vec<crate::purchase_order::OrderVersion>,
    purchase_order_heads: Vec<purchase_order::OrderHead>,
    receipts: Vec<crate::receipt::ReceiptRecord>,
    holding_versions: Vec<holding::LegacyHoldingVersion>,
    holding_heads: Vec<holding::HoldingHead>,
    order_receipt_closures: Vec<receipt::ClosureRow>,
    recipe_versions: Vec<recipe::VersionRow>,
    recipe_lines: Vec<recipe::LineRow>,
    recipe_heads: Vec<recipe::Head>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotStateV11 {
    materials: Vec<Material>,
    material_versions: Vec<MaterialVersion>,
    material_heads: Vec<Head>,
    material_prices: Vec<MaterialPriceVersion>,
    catalog_state: CatalogState,
    creation_requests: Vec<LegacyRequest>,
    mutation_requests: Vec<MutationRecord>,
    business_records: Vec<business::Record>,
    business_revisions: Vec<crate::business::BusinessRevision>,
    business_heads: Vec<business::Head>,
    business_confirmations: Vec<business::ConfirmationRow>,
    business_catalog_price_results: Vec<business::PriceResult>,
    business_corrections: Vec<crate::business::BusinessCorrection>,
    purchase_orders: Vec<purchase_order::OrderRecord>,
    purchase_order_versions: Vec<crate::purchase_order::OrderVersion>,
    purchase_order_heads: Vec<purchase_order::OrderHead>,
    receipts: Vec<crate::receipt::ReceiptRecord>,
    holding_versions: Vec<holding::HoldingVersion>,
    holding_heads: Vec<holding::HoldingHead>,
    order_receipt_closures: Vec<receipt::ClosureRow>,
    recipe_versions: Vec<recipe::VersionRow>,
    recipe_lines: Vec<recipe::LineRow>,
    recipe_heads: Vec<recipe::Head>,
    productions: Vec<production::RecordRow>,
    production_inputs: Vec<production::InputRow>,
    production_outputs: Vec<production::OutputRow>,
    production_effects: Vec<production::EffectRow>,
    sales: Vec<sale::RecordRow>,
    sale_versions: Vec<crate::sale::SaleVersion>,
    sale_heads: Vec<sale::Head>,
    sale_cost_rows: Vec<sale::CostRow>,
    stocktake_batches: Vec<crate::stocktake::StocktakeRecord>,
    stocktake_observations: Vec<stocktake::ObservationRow>,
    stocktake_versions: Vec<stocktake::VersionRow>,
    stocktake_heads: Vec<stocktake::Head>,
    consumption_versions: Vec<stocktake::ConsumptionVersion>,
    consumption_heads: Vec<stocktake::ConsumptionHead>,
    consumption_rows: Vec<stocktake::ConsumptionRow>,
    wastes: Vec<crate::waste::WasteRecord>,
    receipt_reconciliations: Vec<crate::reconciliation::ReceiptReconciliation>,
    inclusion_links: Vec<reconciliation::LinkRow>,
    inclusion_link_versions: Vec<reconciliation::LinkVersionRow>,
    inclusion_link_heads: Vec<reconciliation::LinkHead>,
    inclusion_reservations: Vec<reconciliation::ReservationRow>,
    reconciliation_material_effects: Vec<reconciliation::MaterialEffectRow>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FrozenKindV5 {
    CatalogPrice,
    PurchaseOrder,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenRecordV5 {
    id: String,
    kind: FrozenKindV5,
    created_at: String,
    created_sequence: String,
    corrects_confirmation_id: Option<String>,
}
#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum FrozenOrderDraftV5 {
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
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum FrozenPayloadV5 {
    CatalogPrice {
        version: u32,
        data: FrozenPriceDataV4,
    },
    PurchaseOrder {
        version: u32,
        data: FrozenOrderDraftV5,
    },
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenRevisionV5 {
    id: String,
    record_id: String,
    previous_revision_id: Option<String>,
    status: crate::business::BusinessStatus,
    payload: FrozenPayloadV5,
    recorded_at: String,
    seq: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum FrozenOrderActionV5 {
    Place,
    Revise,
    Cancel,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrozenOrderVersionV5 {
    id: String,
    order_id: String,
    previous_version_id: Option<String>,
    confirmation_id: String,
    business_record_id: String,
    action: FrozenOrderActionV5,
    status: crate::purchase_order::OrderStatus,
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

pub(super) fn encode(
    connection: &Connection,
    sequence: i64,
    request_id: &str,
    recorded_at: &str,
) -> Result<String, StoreError> {
    let old = read_catalog_state(connection)?;
    let rows = business::snapshot_rows(connection)?;
    let orders = purchase_order::order_snapshot_rows(connection)?;
    let recipes = recipe::snapshot_rows(connection)?;
    let productions = production::snapshot_rows(connection)?;
    let sales = sale::snapshot_rows(connection)?;
    let stocktakes = stocktake::snapshot_rows(connection)?;
    let reconciliations = reconciliation::snapshot_rows(connection)?;
    let state = SnapshotStateV11 {
        materials: old.materials,
        material_versions: old.material_versions,
        material_heads: old.material_heads,
        material_prices: old.material_prices,
        catalog_state: old.catalog_state,
        creation_requests: old.creation_requests,
        mutation_requests: old.mutation_requests,
        business_records: rows.records,
        business_revisions: rows.revisions,
        business_heads: rows.heads,
        business_confirmations: rows.confirmations,
        business_catalog_price_results: rows.results,
        business_corrections: rows.corrections,
        purchase_orders: orders.orders,
        purchase_order_versions: orders.versions,
        purchase_order_heads: orders.heads,
        receipts: receipt::records(connection, None, None)?,
        holding_versions: holding::versions(connection, None)?,
        holding_heads: holding::heads(connection)?,
        order_receipt_closures: receipt::closures(connection)?,
        recipe_versions: recipes.versions,
        recipe_lines: recipes.lines,
        recipe_heads: recipes.heads,
        productions: productions.records,
        production_inputs: productions.inputs,
        production_outputs: productions.outputs,
        production_effects: productions.effects,
        sales: sales.records,
        sale_versions: sales.versions,
        sale_heads: sales.heads,
        sale_cost_rows: sales.cost_rows,
        stocktake_batches: stocktakes.batches,
        stocktake_observations: stocktakes.observations,
        stocktake_versions: stocktakes.versions,
        stocktake_heads: stocktakes.heads,
        consumption_versions: stocktakes.consumption_versions,
        consumption_heads: stocktakes.consumption_heads,
        consumption_rows: stocktakes.consumption_rows,
        wastes: waste::snapshot_rows(connection)?,
        receipt_reconciliations: reconciliations.records,
        inclusion_links: reconciliations.links,
        inclusion_link_versions: reconciliations.versions,
        inclusion_link_heads: reconciliations.heads,
        inclusion_reservations: reconciliations.reservations,
        reconciliation_material_effects: reconciliations.effects,
    };
    let snapshot = SnapshotV2Schema11 {
        format_version: 2,
        schema_version: SCHEMA_VERSION,
        sequence: sequence.to_string(),
        request_id: request_id.into(),
        recorded_at: recorded_at.into(),
        state,
    };
    Ok(serde_json::to_string_pretty(&snapshot).map_err(unavailable)? + "\n")
}

pub(super) fn validate(payload: &str, sequence: i64) -> Result<(), StoreError> {
    validate_source(payload, sequence, SCHEMA_VERSION)
}

pub(super) fn validate_source(
    payload: &str,
    sequence: i64,
    source_schema: i64,
) -> Result<(), StoreError> {
    let header: serde_json::Value = serde_json::from_str(payload).map_err(unavailable)?;
    let format = header.get("format_version").and_then(|v| v.as_u64());
    let schema = header.get("schema_version").and_then(|v| v.as_i64());
    if schema.is_some_and(|value| value > source_schema) {
        return Err(unavailable(
            "현재 DB보다 새로운 JSON 스키마를 변환할 수 없습니다",
        ));
    }
    validate_catalog_fields(&header)?;
    let supported = match (format, schema) {
        (Some(1), Some(1..=2)) => {
            let old: super::Snapshot = serde_json::from_str(payload).map_err(unavailable)?;
            old.sequence == sequence
        }
        (Some(2), Some(3)) => {
            let old: SnapshotV2Schema3 = serde_json::from_str(payload).map_err(unavailable)?;
            old.sequence == sequence.to_string()
        }
        (Some(2), Some(4)) => {
            let current: SnapshotV2Schema4 = serde_json::from_str(payload).map_err(unavailable)?;
            for revision in &current.state.business_revisions {
                let old = serde_json::from_value(
                    serde_json::to_value(&revision.payload).map_err(unavailable)?,
                )
                .map_err(unavailable)?;
                crate::business::normalize_business_payload(old).map_err(unavailable)?;
            }
            current.sequence == sequence.to_string()
        }
        (Some(2), Some(5)) => {
            let current: SnapshotV2Schema5 = serde_json::from_str(payload).map_err(unavailable)?;
            for revision in &current.state.business_revisions {
                let old = serde_json::from_value(
                    serde_json::to_value(&revision.payload).map_err(unavailable)?,
                )
                .map_err(unavailable)?;
                crate::business::normalize_business_payload(old).map_err(unavailable)?;
            }
            for version in &current.state.purchase_order_versions {
                if version.status == crate::purchase_order::OrderStatus::Received {
                    return Err(unavailable("이전 주문 형식에는 실제 입고가 없습니다"));
                }
                let old =
                    serde_json::from_value(serde_json::to_value(version).map_err(unavailable)?)
                        .map_err(unavailable)?;
                purchase_order::validate_stored_version(&old)?;
            }
            current.sequence == sequence.to_string()
        }
        (Some(2), Some(6)) => {
            let frozen: frozen6::Snapshot = serde_json::from_str(payload).map_err(unavailable)?;
            let matching_sequence = frozen.sequence == sequence.to_string();
            let current = frozen.into_state()?;
            for revision in &current.business_revisions {
                crate::business::normalize_business_payload(revision.payload.clone())
                    .map_err(unavailable)?;
            }
            for version in &current.purchase_order_versions {
                purchase_order::validate_stored_version(version)?;
            }
            validate_receipt_references(ReceiptReferences {
                business_records: &current.business_records,
                business_revisions: &current.business_revisions,
                business_confirmations: &current.business_confirmations,
                business_catalog_price_results: &current.business_catalog_price_results,
                materials: &current.materials,
                material_versions: &current.material_versions,
                receipts: &current.receipts,
                reconciliations: &[],
                purchase_order_versions: &current.purchase_order_versions,
            })?;
            receipt::validate_snapshot(
                &current.receipts,
                &current.order_receipt_closures,
                &current.purchase_order_versions,
                &current.holding_versions,
                &current.holding_heads,
            )?;
            matching_sequence
        }
        (Some(2), Some(7)) => {
            let frozen: frozen7::Snapshot = serde_json::from_str(payload).map_err(unavailable)?;
            let matching_sequence = frozen.sequence == sequence.to_string();
            let current = frozen.into_state()?;
            for revision in &current.business_revisions {
                crate::business::normalize_business_payload(revision.payload.clone())
                    .map_err(unavailable)?;
            }
            for version in &current.purchase_order_versions {
                purchase_order::validate_stored_version(version)?;
            }
            validate_receipt_references(ReceiptReferences {
                business_records: &current.business_records,
                business_revisions: &current.business_revisions,
                business_confirmations: &current.business_confirmations,
                business_catalog_price_results: &current.business_catalog_price_results,
                materials: &current.materials,
                material_versions: &current.material_versions,
                receipts: &current.receipts,
                reconciliations: &[],
                purchase_order_versions: &current.purchase_order_versions,
            })?;
            receipt::validate_snapshot(
                &current.receipts,
                &current.order_receipt_closures,
                &current.purchase_order_versions,
                &current.holding_versions,
                &current.holding_heads,
            )?;
            recipe::validate_snapshot(
                &current.recipe_versions,
                &current.recipe_lines,
                &current.recipe_heads,
                &current.material_versions,
                &current.materials,
                &current
                    .mutation_requests
                    .iter()
                    .map(|r| (r.sequence.as_str(), r.operation.as_str()))
                    .collect::<Vec<_>>(),
                sequence,
            )?;
            matching_sequence
        }
        (Some(2), Some(8)) => {
            let frozen: frozen8::Snapshot = serde_json::from_str(payload).map_err(unavailable)?;
            let matching_sequence = frozen.sequence == sequence.to_string();
            validate_current_state(&frozen.into_state()?, sequence)?;
            matching_sequence
        }
        (Some(2), Some(9)) => {
            let frozen: frozen9::Snapshot = serde_json::from_str(payload).map_err(unavailable)?;
            let matching_sequence = frozen.sequence == sequence.to_string();
            validate_current_state(&frozen.into_state()?, sequence)?;
            matching_sequence
        }
        (Some(2), Some(10)) => {
            let frozen: frozen10::Snapshot = serde_json::from_str(payload).map_err(unavailable)?;
            let matching_sequence = frozen.sequence == sequence.to_string();
            validate_current_state(&frozen.into_state()?, sequence)?;
            matching_sequence
        }
        (Some(2), Some(11)) => {
            let current: SnapshotV2Schema11 = serde_json::from_str(payload).map_err(unavailable)?;
            validate_current_state(&current.state, sequence)?;
            current.sequence == sequence.to_string()
        }
        _ => false,
    };
    if !supported {
        return Err(unavailable("지원하지 않거나 손상된 JSON 변경 기록입니다"));
    }
    Ok(())
}

fn validate_current_state(state: &SnapshotStateV11, sequence: i64) -> Result<(), StoreError> {
    for revision in &state.business_revisions {
        crate::business::normalize_business_payload(revision.payload.clone())
            .map_err(unavailable)?;
    }
    for version in &state.purchase_order_versions {
        purchase_order::validate_stored_version(version)?;
    }
    validate_receipt_references(ReceiptReferences {
        business_records: &state.business_records,
        business_revisions: &state.business_revisions,
        business_confirmations: &state.business_confirmations,
        business_catalog_price_results: &state.business_catalog_price_results,
        materials: &state.materials,
        material_versions: &state.material_versions,
        receipts: &state.receipts,
        reconciliations: &state.receipt_reconciliations,
        purchase_order_versions: &state.purchase_order_versions,
    })?;
    receipt::validate_receipt_links(
        &state.receipts,
        &state.order_receipt_closures,
        &state.purchase_order_versions,
    )?;
    holding::validate_snapshot_reconciled(
        &state.receipts,
        &state.production_effects,
        &state.stocktake_versions,
        &state.wastes,
        &state.receipt_reconciliations,
        &state.reconciliation_material_effects,
        &state.holding_versions,
        &state.holding_heads,
    )?;
    let mutations: Vec<_> = state
        .mutation_requests
        .iter()
        .map(|r| (r.sequence.as_str(), r.operation.as_str()))
        .collect();
    recipe::validate_snapshot(
        &state.recipe_versions,
        &state.recipe_lines,
        &state.recipe_heads,
        &state.material_versions,
        &state.materials,
        &mutations,
        sequence,
    )?;
    let other_confirmations: Vec<_> = state
        .business_catalog_price_results
        .iter()
        .map(|r| r.confirmation_id.as_str())
        .chain(
            state
                .purchase_order_versions
                .iter()
                .map(|r| r.confirmation_id.as_str()),
        )
        .chain(state.receipts.iter().map(|r| r.confirmation_id.as_str()))
        .chain(state.wastes.iter().map(|r| r.confirmation_id.as_str()))
        .chain(
            state
                .stocktake_batches
                .iter()
                .map(|r| r.confirmation_id.as_str()),
        )
        .collect();
    production::validate_snapshot(production::Validation {
        records: &state.productions,
        inputs: &state.production_inputs,
        outputs: &state.production_outputs,
        effects: &state.production_effects,
        holdings: &state.holding_versions,
        observations: &state.stocktake_versions,
        business_records: &state.business_records,
        revisions: &state.business_revisions,
        confirmations: &state.business_confirmations,
        material_versions: &state.material_versions,
        materials: &state.materials,
        other_confirmation_ids: &other_confirmations,
        mutations: &mutations,
        sequence,
    })?;
    let sale_other_confirmations: Vec<_> = other_confirmations
        .into_iter()
        .chain(
            state
                .productions
                .iter()
                .map(|row| row.confirmation_id.as_str()),
        )
        .collect();
    let sale_mutations: Vec<_> = state
        .mutation_requests
        .iter()
        .map(|r| {
            (
                r.sequence.as_str(),
                r.operation.as_str(),
                r.request_id.as_str(),
            )
        })
        .collect();
    sale::validate_snapshot(sale::Validation {
        records: &state.sales,
        cost_rows: &state.sale_cost_rows,
        versions: &state.sale_versions,
        heads: &state.sale_heads,
        recipe_versions: &state.recipe_versions,
        recipe_lines: &state.recipe_lines,
        holdings: &state.holding_versions,
        prices: &state.material_prices,
        material_versions: &state.material_versions,
        business_records: &state.business_records,
        revisions: &state.business_revisions,
        confirmations: &state.business_confirmations,
        corrections: &state.business_corrections,
        other_confirmation_ids: &sale_other_confirmations,
        mutations: &sale_mutations,
        sequence,
    })?;
    let stocktake_others: Vec<_> = state
        .business_catalog_price_results
        .iter()
        .map(|r| r.confirmation_id.as_str())
        .chain(
            state
                .purchase_order_versions
                .iter()
                .map(|r| r.confirmation_id.as_str()),
        )
        .chain(state.receipts.iter().map(|r| r.confirmation_id.as_str()))
        .chain(state.productions.iter().map(|r| r.confirmation_id.as_str()))
        .chain(
            state
                .sale_versions
                .iter()
                .map(|r| r.confirmation_id.as_str()),
        )
        .chain(state.wastes.iter().map(|r| r.confirmation_id.as_str()))
        .collect();
    stocktake::validate_snapshot(stocktake::Validation {
        batches: &state.stocktake_batches,
        observations: &state.stocktake_observations,
        versions: &state.stocktake_versions,
        heads: &state.stocktake_heads,
        consumption_versions: &state.consumption_versions,
        consumption_heads: &state.consumption_heads,
        consumption_rows: &state.consumption_rows,
        holdings: &state.holding_versions,
        material_versions: &state.material_versions,
        prices: &state.material_prices,
        sales: &state.sales,
        sale_cost_rows: &state.sale_cost_rows,
        sale_versions: &state.sale_versions,
        business_records: &state.business_records,
        revisions: &state.business_revisions,
        confirmations: &state.business_confirmations,
        other_confirmation_ids: &stocktake_others,
        mutations: &sale_mutations,
        sequence,
    })?;
    reconciliation::validate_snapshot(reconciliation::Validation {
        records: &state.receipt_reconciliations,
        links: &state.inclusion_links,
        versions: &state.inclusion_link_versions,
        heads: &state.inclusion_link_heads,
        reservations: &state.inclusion_reservations,
        effects: &state.reconciliation_material_effects,
        receipts: &state.receipts,
        orders: &state.purchase_order_versions,
        holdings: &state.holding_versions,
        observations: &state.stocktake_versions,
        observation_rows: &state.stocktake_observations,
        observation_heads: &state.stocktake_heads,
        productions: &state.productions,
        production_inputs: &state.production_inputs,
        production_outputs: &state.production_outputs,
        wastes: &state.wastes,
        material_versions: &state.material_versions,
        business_records: &state.business_records,
        revisions: &state.business_revisions,
        confirmations: &state.business_confirmations,
        mutations: &sale_mutations,
        sequence,
    })?;
    waste::validate_snapshot(waste::Validation {
        records: &state.wastes,
        holdings: &state.holding_versions,
        observations: &state.stocktake_versions,
        material_versions: &state.material_versions,
        business_records: &state.business_records,
        revisions: &state.business_revisions,
        confirmations: &state.business_confirmations,
        mutations: &sale_mutations,
        sequence,
    })?;
    Ok(())
}

struct ReceiptReferences<'a> {
    business_records: &'a [business::Record],
    business_revisions: &'a [crate::business::BusinessRevision],
    business_confirmations: &'a [business::ConfirmationRow],
    business_catalog_price_results: &'a [business::PriceResult],
    materials: &'a [Material],
    material_versions: &'a [MaterialVersion],
    receipts: &'a [crate::receipt::ReceiptRecord],
    reconciliations: &'a [crate::reconciliation::ReceiptReconciliation],
    purchase_order_versions: &'a [crate::purchase_order::OrderVersion],
}

#[cfg(test)]
mod schema10_reader_tests {
    use super::validate_source;
    use serde_json::{Value, json};

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../tests/fixtures/snapshot-v2-schema10.json"
        ))
        .unwrap()
    }
    fn accepts(value: &Value) -> bool {
        let seq = value["sequence"].as_str().unwrap().parse().unwrap();
        validate_source(&value.to_string(), seq, 10).is_ok()
    }
    #[test]
    fn schema10_preserves_real_observation_waste_and_original_receipts() {
        let value = fixture();
        assert!(accepts(&value));
        assert!(
            !value["state"]["stocktake_versions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(!value["state"]["wastes"].as_array().unwrap().is_empty());
        assert!(value["state"].get("receipt_reconciliations").is_none());
    }
    #[test]
    fn schema10_rejects_future_receipt_tags_relations_and_holding_sources() {
        let original = fixture();
        let mut changed = original.clone();
        let receipt = changed["state"]["business_revisions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["payload"]["kind"] == "receipt")
            .unwrap();
        receipt["payload"]["data"]["stock_effect"] = json!({"kind":"reconciled","selections":[]});
        assert!(!accepts(&changed));
        for name in [
            "receipt_reconciliations",
            "inclusion_links",
            "inclusion_link_versions",
            "inclusion_link_heads",
            "inclusion_reservations",
            "reconciliation_material_effects",
        ] {
            let mut changed = original.clone();
            changed["state"][name] = json!([]);
            assert!(!accepts(&changed), "{name}");
        }
        for name in [
            "reconciliation_effect_id",
            "baseline_reconciliation_effect_id",
        ] {
            let mut changed = original.clone();
            changed["state"]["holding_versions"][0][name] = Value::Null;
            assert!(!accepts(&changed), "{name}");
        }
        let mut changed = original;
        changed["schema_version"] = json!(11);
        assert!(!accepts(&changed));
    }
    #[test]
    fn schema10_nested_observation_and_waste_basis_remain_closed() {
        let original = fixture();
        for path in ["stocktake_versions", "wastes"] {
            let mut changed = original.clone();
            changed["state"][path][0]["effect"]["after"]["value_delta"] = json!("1");
            assert!(!accepts(&changed));
        }
        let mut changed = original;
        changed["state"]["wastes"][0]["effect"]["basis_before"]["kind"] = json!("reconciliation");
        assert!(!accepts(&changed));
    }
}

#[cfg(test)]
mod schema9_reader_tests {
    use super::validate_source;
    use serde_json::{Value, json};

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../tests/fixtures/snapshot-v2-schema9.json"
        ))
        .unwrap()
    }
    fn accepts(value: &Value) -> bool {
        let seq = value["sequence"].as_str().unwrap().parse().unwrap();
        validate_source(&value.to_string(), seq, 9).is_ok()
    }
    #[test]
    fn original_sale_and_correction_snapshot_stays_valid_before_observation_conversion() {
        let original = fixture();
        assert!(accepts(&original));
        assert_eq!(
            original["state"]["sale_versions"].as_array().unwrap().len(),
            2
        );
        assert!(original["state"].get("stocktake_batches").is_none());
    }
    #[test]
    fn schema9_rejects_observation_waste_and_new_relations() {
        let original = fixture();
        for kind in ["stocktake", "waste"] {
            let mut value = original.clone();
            value["state"]["business_records"][0]["kind"] = json!(kind);
            assert!(!accepts(&value));
        }
        for key in [
            "stocktake_batches",
            "stocktake_observations",
            "stocktake_versions",
            "stocktake_heads",
            "consumption_versions",
            "consumption_heads",
            "consumption_rows",
            "wastes",
        ] {
            let mut value = original.clone();
            value["state"][key] = json!([]);
            assert!(!accepts(&value), "{key}");
        }
        let mut value = original;
        value["schema_version"] = json!(10);
        assert!(!accepts(&value));
    }
    #[test]
    fn schema9_keeps_holding_and_nested_sale_shapes_closed() {
        let original = fixture();
        for path in [
            "/state/holding_versions/0",
            "/state/holding_heads/0",
            "/state/sales/0",
            "/state/sale_versions/0",
            "/state/sale_cost_rows/0/row",
            "/state/sale_cost_rows/0/row/price",
        ] {
            let mut value = original.clone();
            value
                .pointer_mut(path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert(
                    "observation_version_id".into(),
                    json!("00000000-0000-4000-8000-000000000001"),
                );
            assert!(!accepts(&value), "{path}");
        }
        let mut value = original.clone();
        value["state"]["holding_heads"][0]["last_valid_version_id"] = Value::Null;
        assert!(!accepts(&value));
        let index = original["state"]["business_revisions"]
            .as_array()
            .unwrap()
            .iter()
            .position(|r| r["payload"]["kind"] == "sale")
            .unwrap();
        let mut value = original;
        value["state"]["business_revisions"][index]["payload"]["data"]["consumption"] = json!([]);
        assert!(!accepts(&value));
    }
}

#[cfg(test)]
mod schema8_reader_tests {
    use super::validate_source;
    use serde_json::{Value, json};

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../tests/fixtures/snapshot-v2-schema8.json"
        ))
        .unwrap()
    }
    fn accepts(value: &Value) -> bool {
        let sequence = value["sequence"].as_str().unwrap().parse().unwrap();
        validate_source(&value.to_string(), sequence, 8).is_ok()
    }

    #[test]
    fn original_production_snapshot_remains_valid_before_sale_conversion() {
        let original = include_str!("../../tests/fixtures/snapshot-v2-schema8.json");
        let value = fixture();
        assert!(accepts(&value));
        assert!(value["state"].get("sales").is_none());
        assert_eq!(serde_json::from_str::<Value>(original).unwrap(), value);
    }

    #[test]
    fn schema8_rejects_sale_kinds_collections_and_future_source_header() {
        let original = fixture();
        let mut changed = original.clone();
        changed["state"]["business_records"][0]["kind"] = json!("sale");
        assert!(!accepts(&changed));
        let mut changed = original.clone();
        changed["state"]["business_revisions"][0]["payload"] = json!({
            "kind":"sale","version":1,"data":{"material_id":null,
                "material_version_id":null,"recipe_version_id":null,"count":"0","note":""}
        });
        assert!(!accepts(&changed));
        for key in ["sales", "sale_versions", "sale_heads", "sale_cost_rows"] {
            let mut changed = original.clone();
            changed["state"][key] = json!([]);
            assert!(!accepts(&changed), "{key}");
        }
        let mut changed = original;
        changed["schema_version"] = json!(9);
        assert!(!accepts(&changed));
    }

    #[test]
    fn schema8_keeps_nested_production_and_holding_shapes_closed() {
        let original = fixture();
        for path in [
            "/state/productions/0",
            "/state/production_inputs/0",
            "/state/production_inputs/0/row",
            "/state/production_outputs/0/row",
            "/state/production_effects/0",
            "/state/holding_versions/0",
            "/state/holding_heads/0",
        ] {
            let mut changed = original.clone();
            changed
                .pointer_mut(path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert(
                    "sale_id".into(),
                    json!("00000000-0000-4000-8000-000000000001"),
                );
            assert!(!accepts(&changed), "{path}");
        }
        let index = original["state"]["business_revisions"]
            .as_array()
            .unwrap()
            .iter()
            .position(|row| row["payload"]["kind"] == "production")
            .unwrap();
        for key in ["inputs", "outputs"] {
            let mut changed = original.clone();
            changed["state"]["business_revisions"][index]["payload"]["data"][key][0]["sale_quantity"] =
                json!("0");
            assert!(!accepts(&changed), "{key}");
        }
    }
}

fn validate_receipt_references(state: ReceiptReferences<'_>) -> Result<(), StoreError> {
    use crate::{
        business::{BusinessKind, BusinessPayload, BusinessStatus},
        domain::{Money, Quantity, Unit},
        purchase_order::OrderAction,
        receipt::{ReceiptSource, ReceiptSourceDraft, ReceiptStockEffectDraft},
    };
    use std::collections::{HashMap, HashSet};

    let bad = || unavailable("JSON 입고의 확정 기록·품목 버전·결과가 일치하지 않습니다");
    let records: HashMap<_, _> = state.business_records.iter().map(|r| (&r.id, r)).collect();
    let confirmations: HashMap<_, _> = state
        .business_confirmations
        .iter()
        .map(|c| (&c.id, c))
        .collect();
    let revisions: HashMap<_, _> = state
        .business_revisions
        .iter()
        .map(|r| (&r.id, r))
        .collect();
    let materials: HashSet<_> = state.materials.iter().map(|m| &m.id).collect();
    let versions: HashMap<_, _> = state.material_versions.iter().map(|v| (&v.id, v)).collect();
    if records.len() != state.business_records.len()
        || confirmations.len() != state.business_confirmations.len()
        || revisions.len() != state.business_revisions.len()
        || materials.len() != state.materials.len()
        || versions.len() != state.material_versions.len()
    {
        return Err(bad());
    }
    let mut used = HashSet::new();
    for receipt in state.receipts {
        let confirmation = confirmations
            .get(&receipt.confirmation_id)
            .ok_or_else(bad)?;
        let record = records.get(&confirmation.record_id).ok_or_else(bad)?;
        let revision = revisions.get(&confirmation.revision_id).ok_or_else(bad)?;
        let material = versions.get(&receipt.material_version_id).ok_or_else(bad)?;
        if !used.insert(&receipt.confirmation_id)
            || record.kind != BusinessKind::Receipt
            || record.id != receipt.business_record_id
            || confirmation.seq != receipt.seq
            || confirmation.recorded_at != receipt.recorded_at
            || revision.record_id != record.id
            || revision.status != BusinessStatus::Confirmed
            || revision.seq != receipt.seq
            || revision.recorded_at != receipt.recorded_at
            || material.material_id != receipt.material_id
            || !materials.contains(&receipt.material_id)
            || material.name != receipt.material_name
            || material.canonical_unit != receipt.unit
            || !Unit::parse(&material.unit)
                .map_err(unavailable)?
                .same_dimension(&Unit::parse(&receipt.unit).map_err(unavailable)?)
            || state
                .business_catalog_price_results
                .iter()
                .any(|r| r.confirmation_id == confirmation.id)
            || state
                .purchase_order_versions
                .iter()
                .any(|v| v.confirmation_id == confirmation.id && v.action != OrderAction::Receive)
        {
            return Err(bad());
        }
        let BusinessPayload::Receipt { version: 1, data } = &revision.payload else {
            return Err(bad());
        };
        let source_matches = match (&data.source, &receipt.source) {
            (
                ReceiptSourceDraft::Direct {
                    material_id,
                    material_version_id,
                },
                ReceiptSource::Direct {},
            ) => {
                material_id.as_ref() == Some(&receipt.material_id)
                    && material_version_id.as_ref() == Some(&receipt.material_version_id)
            }
            (
                ReceiptSourceDraft::Order {
                    order_id,
                    expected_order_version_id,
                    material_version_id,
                },
                ReceiptSource::Order {
                    order_id: actual_order,
                    ordered_version_id,
                },
            ) => {
                order_id.as_ref() == Some(actual_order)
                    && expected_order_version_id.as_ref() == Some(ordered_version_id)
                    && material_version_id.as_ref() == Some(&receipt.material_version_id)
            }
            _ => false,
        };
        let reconciliations: Vec<_> = state
            .reconciliations
            .iter()
            .filter(|r| r.receipt_id == receipt.id)
            .collect();
        let effect_matches = match &data.stock_effect {
            ReceiptStockEffectDraft::Unreflected {} => reconciliations.is_empty(),
            ReceiptStockEffectDraft::Reconciled { .. } => {
                reconciliations.len() == 1
                    && reconciliations[0].confirmation_id == receipt.confirmation_id
            }
            ReceiptStockEffectDraft::NeedsReconciliation { .. } => false,
        };
        if !source_matches
            || !effect_matches
            || data.quantity.trim() != receipt.input_quantity
            || data.unit.trim() != receipt.input_unit
            || Unit::parse(&data.unit)
                .map_err(unavailable)?
                .to_canonical(Quantity::parse(&data.quantity).map_err(unavailable)?)
                .map_err(unavailable)?
                .decimal()
                != receipt.quantity
            || Money::parse(&data.amount).map_err(unavailable)?.decimal() != receipt.amount
            || data.receipt_date.trim() != receipt.receipt_date
            || data.note != receipt.note
        {
            return Err(bad());
        }
    }
    for confirmation in state.business_confirmations {
        if records
            .get(&confirmation.record_id)
            .is_some_and(|record| record.kind == BusinessKind::Receipt)
            && !used.contains(&confirmation.id)
        {
            return Err(bad());
        }
    }
    Ok(())
}

// Catalog response DTOs are intentionally reusable. Keep the frozen journal
// shapes strict without changing the legacy HTTP response deserializers.
fn validate_catalog_fields(snapshot: &serde_json::Value) -> Result<(), StoreError> {
    let state = snapshot.get("state").unwrap_or(snapshot);
    for (name, allowed) in [
        ("materials", &["id", "name", "unit", "created_at"][..]),
        (
            "material_versions",
            &[
                "id",
                "material_id",
                "name",
                "unit",
                "canonical_unit",
                "dimension",
                "kind",
                "description",
                "archived",
                "recorded_at",
                "seq",
                "source",
            ][..],
        ),
        (
            "material_prices",
            &[
                "id",
                "material_id",
                "material_version_id",
                "quantity",
                "amount",
                "unit",
                "input_quantity",
                "input_unit",
                "recorded_at",
                "seq",
            ][..],
        ),
    ] {
        if let Some(rows) = state.get(name).and_then(|v| v.as_array()) {
            for row in rows {
                reject_unknown(row, allowed)?;
            }
        }
    }
    if let Some(state) = state.get("catalog_state") {
        reject_unknown(state, &["epoch", "revision"])?;
    }
    Ok(())
}

fn reject_unknown(value: &serde_json::Value, allowed: &[&str]) -> Result<(), StoreError> {
    if value
        .as_object()
        .is_some_and(|object| object.keys().any(|key| !allowed.contains(&key.as_str())))
    {
        return Err(unavailable("JSON 기록에 지원하지 않는 필드가 있습니다"));
    }
    Ok(())
}

#[cfg(test)]
mod frozen_production_boundary_tests {
    use super::validate_source;
    use serde_json::{Value, json};

    fn fixtures() -> [(i64, Value); 2] {
        [
            (
                6,
                serde_json::from_str(include_str!(
                    "../../tests/fixtures/snapshot-v2-schema6.json"
                ))
                .unwrap(),
            ),
            (
                7,
                serde_json::from_str(include_str!(
                    "../../tests/fixtures/snapshot-v2-schema7.json"
                ))
                .unwrap(),
            ),
        ]
    }
    fn validate(schema: i64, value: &Value) -> bool {
        let sequence = value["sequence"].as_str().unwrap().parse().unwrap();
        validate_source(&value.to_string(), sequence, schema).is_ok()
    }

    #[test]
    fn old_receipt_only_snapshots_still_validate_without_rewriting_their_input() {
        for (schema, value) in fixtures() {
            let original = value.to_string();
            assert!(validate(schema, &value));
            assert_eq!(value.to_string(), original);
        }
    }

    #[test]
    fn old_readers_reject_production_business_and_new_holding_sources() {
        for (schema, original) in fixtures() {
            let mut changed = original.clone();
            changed["state"]["business_records"][0]["kind"] = json!("production");
            assert!(!validate(schema, &changed));
            let mut changed = original.clone();
            changed["state"]["business_revisions"][0]["payload"] = json!({
                "kind":"production","version":1,"data":{"inputs":[],"outputs":[],"note":""}
            });
            assert!(!validate(schema, &changed));
            for field in ["production_effect_id", "baseline_production_effect_id"] {
                let mut changed = original.clone();
                changed["state"]["holding_versions"][0][field] = Value::Null;
                assert!(!validate(schema, &changed), "{schema}/{field}");
            }
            let mut changed = original.clone();
            changed["state"]["holding_versions"][0]["receipt_id"] = Value::Null;
            assert!(!validate(schema, &changed));
            let mut changed = original;
            changed["state"]["holding_versions"][0]["quantity"] = json!("0");
            changed["state"]["holding_versions"][0]["value"] = json!("0");
            assert!(!validate(schema, &changed));
        }
    }

    #[test]
    fn old_shapes_reject_empty_future_collections_and_future_schema_before_conversion() {
        for (schema, original) in fixtures() {
            for field in [
                "productions",
                "production_inputs",
                "production_outputs",
                "production_effects",
            ] {
                let mut changed = original.clone();
                changed["state"][field] = json!([]);
                assert!(!validate(schema, &changed), "{schema}/{field}");
            }
            let mut future = original;
            future["schema_version"] = json!(8);
            assert!(!validate(schema, &future));
        }
    }

    #[test]
    fn schema7_recipe_shape_stays_closed_when_live_business_kinds_expand() {
        let (schema, original) = fixtures().into_iter().nth(1).unwrap();
        for path in ["/state/recipe_versions/0", "/state/recipe_lines/0/line"] {
            let mut changed = original.clone();
            changed
                .pointer_mut(path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("production_quantity".into(), json!("1"));
            assert!(!validate(schema, &changed), "{path}");
        }
    }
}
