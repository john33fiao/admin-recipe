use super::{
    Material, SCHEMA_VERSION, StoreError, business, catalog, holding, purchase_order,
    read_materials, receipt, unavailable,
};
use crate::catalog::{CatalogState, MaterialPriceVersion, MaterialVersion};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

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
struct SnapshotV2Schema6 {
    format_version: u32,
    schema_version: i64,
    sequence: String,
    request_id: String,
    recorded_at: String,
    state: SnapshotStateV6,
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
    holding_versions: Vec<holding::HoldingVersion>,
    holding_heads: Vec<holding::HoldingHead>,
    order_receipt_closures: Vec<receipt::ClosureRow>,
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
    let state = SnapshotStateV6 {
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
    };
    let snapshot = SnapshotV2Schema6 {
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
            let current: SnapshotV2Schema6 = serde_json::from_str(payload).map_err(unavailable)?;
            for revision in &current.state.business_revisions {
                crate::business::normalize_business_payload(revision.payload.clone())
                    .map_err(unavailable)?;
            }
            for version in &current.state.purchase_order_versions {
                purchase_order::validate_stored_version(version)?;
            }
            validate_receipt_references(&current.state)?;
            receipt::validate_snapshot(
                &current.state.receipts,
                &current.state.order_receipt_closures,
                &current.state.purchase_order_versions,
                &current.state.holding_versions,
                &current.state.holding_heads,
            )?;
            current.sequence == sequence.to_string()
        }
        _ => false,
    };
    if !supported {
        return Err(unavailable("지원하지 않거나 손상된 JSON 변경 기록입니다"));
    }
    Ok(())
}

fn validate_receipt_references(state: &SnapshotStateV6) -> Result<(), StoreError> {
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
    for receipt in &state.receipts {
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
        if !source_matches
            || !matches!(data.stock_effect, ReceiptStockEffectDraft::Unreflected {})
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
    for confirmation in &state.business_confirmations {
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
