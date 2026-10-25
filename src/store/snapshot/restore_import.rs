//! Explicit inverse of the snapshot readers. No business operation is replayed.
use super::{DecodedRestore, StoreError, unavailable};
use rusqlite::{Connection, params};

mod business;
mod catalog;
mod common;
mod correction;
mod inventory;
mod production;
mod profile;
mod recipe;
mod reconciliation;
mod sale;
mod stocktake;

const ACTIVE_TABLES: &[&str] = &[
    "materials",
    "material_versions",
    "material_heads",
    "material_prices",
    "catalog_state",
    "creation_requests",
    "mutation_requests",
    "business_records",
    "business_revisions",
    "business_heads",
    "business_confirmations",
    "business_catalog_price_results",
    "business_corrections",
    "purchase_orders",
    "purchase_order_versions",
    "purchase_order_heads",
    "receipts",
    "holding_versions",
    "holding_heads",
    "order_receipt_closures",
    "recipe_versions",
    "recipe_lines",
    "recipe_heads",
    "productions",
    "production_inputs",
    "production_outputs",
    "production_effects",
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
    "receipt_reconciliations",
    "inclusion_links",
    "inclusion_link_versions",
    "inclusion_link_heads",
    "inclusion_reservations",
    "reconciliation_material_effects",
    "correction_records",
    "correction_event_versions",
    "correction_event_heads",
    "correction_projection_versions",
    "correction_projection_heads",
    "correction_price_bases",
    "purchase_returns",
    "photo_assets",
    "material_profile_versions",
    "material_profile_heads",
    "material_profile_photos",
    "material_profile_suppliers",
];

// These are physical sequence references, not fabricated outbox records.
const SEQUENCE_COLUMNS: &[(&str, &str)] = &[
    ("material_versions", "sequence"),
    ("material_prices", "sequence"),
    ("mutation_requests", "sequence"),
    ("business_records", "created_sequence"),
    ("business_revisions", "sequence"),
    ("business_confirmations", "sequence"),
    ("purchase_orders", "sequence"),
    ("recipe_versions", "sequence"),
    ("productions", "sequence"),
    ("stocktake_batches", "sequence"),
    ("correction_records", "sequence"),
    ("correction_event_versions", "sequence"),
    ("correction_event_versions", "logical_sequence"),
    ("correction_projection_versions", "sequence"),
    ("correction_price_bases", "sequence"),
    ("correction_price_bases", "logical_sequence"),
    ("purchase_order_versions", "sequence"),
    ("receipts", "sequence"),
    ("receipt_reconciliations", "sequence"),
    ("purchase_returns", "sequence"),
    ("holding_versions", "sequence"),
    ("holding_versions", "baseline_sequence"),
    ("reconciliation_material_effects", "sequence"),
    ("stocktake_versions", "logical_sequence"),
    ("stocktake_versions", "sequence"),
    ("wastes", "sequence"),
    ("consumption_versions", "sequence"),
    ("inclusion_link_versions", "sequence"),
    ("photo_assets", "sequence"),
    ("material_profile_versions", "sequence"),
];

pub(super) fn replace_active(
    connection: &Connection,
    decoded: &DecodedRestore,
) -> Result<(), StoreError> {
    if connection.is_autocommit() {
        return Err(common::bad(
            "전체 복원은 트랜잭션 안에서만 실행할 수 있습니다",
        ));
    }
    connection
        .pragma_update(None, "defer_foreign_keys", true)
        .map_err(unavailable)?;
    for table in ACTIVE_TABLES {
        connection
            .execute(&format!("DELETE FROM {table}"), [])
            .map_err(unavailable)?;
    }
    let state = &decoded.active;
    catalog::insert(connection, state)?;
    business::insert(connection, state)?;
    recipe::insert(connection, state)?;
    inventory::insert(connection, state)?;
    production::insert(connection, state)?;
    sale::insert(connection, state)?;
    stocktake::insert(connection, state)?;
    reconciliation::insert(connection, state)?;
    correction::insert(connection, state)?;
    profile::insert(connection, state)?;
    connection
        .execute(
            "INSERT INTO catalog_state(singleton,epoch,revision) VALUES(1,?1,?2)",
            params![
                state.catalog_state.epoch,
                common::nonnegative(&state.catalog_state.revision)?
            ],
        )
        .map_err(unavailable)?;
    for (table, column) in SEQUENCE_COLUMNS {
        connection.execute(&format!("INSERT OR IGNORE INTO state_sequences(sequence) SELECT {column} FROM {table} WHERE {column} IS NOT NULL"), []).map_err(unavailable)?;
    }
    Ok(())
}
