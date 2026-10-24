use super::{Material, SCHEMA_VERSION, StoreError, business, catalog, read_materials, unavailable};
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
    business_records: Vec<business::Record>,
    business_revisions: Vec<crate::business::BusinessRevision>,
    business_heads: Vec<business::Head>,
    business_confirmations: Vec<business::ConfirmationRow>,
    business_catalog_price_results: Vec<business::PriceResult>,
    business_corrections: Vec<crate::business::BusinessCorrection>,
}
pub(super) fn encode(
    connection: &Connection,
    sequence: i64,
    request_id: &str,
    recorded_at: &str,
) -> Result<String, StoreError> {
    let old = read_catalog_state(connection)?;
    let rows = business::snapshot_rows(connection)?;
    let state = SnapshotStateV4 {
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
    };
    let snapshot = SnapshotV2Schema4 {
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
                crate::business::normalize_business_payload(revision.payload.clone())
                    .map_err(unavailable)?;
            }
            current.sequence == sequence.to_string()
        }
        _ => false,
    };
    if !supported {
        return Err(unavailable("지원하지 않거나 손상된 JSON 변경 기록입니다"));
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
