//! Schema 11 preserves every inbound holding FK while adding reconciliation.
use super::snapshot;
use rusqlite::Connection;

pub(super) fn migrate_reconciliation(connection: &Connection) -> anyhow::Result<()> {
    let rows = connection
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (seq, payload) in rows {
        snapshot::validate_source(&payload, seq, 10)?;
    }
    connection.execute_batch("PRAGMA defer_foreign_keys=ON;
        CREATE TABLE receipt_reconciliations(id TEXT PRIMARY KEY NOT NULL,receipt_id TEXT NOT NULL UNIQUE REFERENCES receipts(id),confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),result_json TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL UNIQUE REFERENCES snapshot_outbox(sequence));
        CREATE TABLE inclusion_links(id TEXT PRIMARY KEY NOT NULL,reconciliation_id TEXT NOT NULL REFERENCES receipt_reconciliations(id),selection_position INTEGER NOT NULL CHECK(selection_position>=0 AND selection_position<20),portion_position INTEGER NOT NULL CHECK(portion_position>=0),row_json TEXT NOT NULL,UNIQUE(reconciliation_id,selection_position,portion_position));
        CREATE TABLE inclusion_link_versions(id TEXT PRIMARY KEY NOT NULL,link_id TEXT NOT NULL REFERENCES inclusion_links(id),previous_version_id TEXT,observation_version_id TEXT NOT NULL REFERENCES stocktake_versions_next(id),row_json TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),UNIQUE(link_id,id),FOREIGN KEY(link_id,previous_version_id) REFERENCES inclusion_link_versions(link_id,id));
        CREATE TABLE inclusion_link_heads(link_id TEXT PRIMARY KEY NOT NULL REFERENCES inclusion_links(id),current_version_id TEXT NOT NULL,FOREIGN KEY(link_id,current_version_id) REFERENCES inclusion_link_versions(link_id,id));
        CREATE TABLE inclusion_reservations(id TEXT PRIMARY KEY NOT NULL,link_version_id TEXT NOT NULL UNIQUE REFERENCES inclusion_link_versions(id),row_json TEXT NOT NULL);
        CREATE TABLE reconciliation_material_effects(id TEXT PRIMARY KEY NOT NULL,reconciliation_id TEXT NOT NULL REFERENCES receipt_reconciliations(id),material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,before_holding_version_id TEXT REFERENCES holding_versions_next(id),row_json TEXT NOT NULL,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),UNIQUE(reconciliation_id,material_id),UNIQUE(material_id,id),FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id));
        CREATE TABLE holding_versions_next(
        id TEXT PRIMARY KEY NOT NULL,material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,previous_version_id TEXT,
        receipt_id TEXT UNIQUE REFERENCES receipts(id),baseline_receipt_id TEXT REFERENCES receipts(id),
        baseline_sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),
        quantity_micros INTEGER NOT NULL CHECK(quantity_micros>=0 AND quantity_micros<=9000000000000000),
        value_micros INTEGER NOT NULL CHECK(value_micros>=0 AND value_micros<=9000000000000000000),
        recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),
        production_effect_id TEXT UNIQUE,baseline_production_effect_id TEXT,
        observation_version_id TEXT UNIQUE REFERENCES stocktake_versions_next(id),baseline_observation_version_id TEXT REFERENCES stocktake_versions_next(id),waste_id TEXT UNIQUE REFERENCES wastes_next(id),
        reconciliation_effect_id TEXT UNIQUE,baseline_reconciliation_effect_id TEXT,
        UNIQUE(material_id,id),
        FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),
        FOREIGN KEY(material_id,previous_version_id) REFERENCES holding_versions_next(material_id,id),
        FOREIGN KEY(material_id,production_effect_id) REFERENCES production_effects(material_id,id),
        FOREIGN KEY(material_id,baseline_production_effect_id) REFERENCES production_effects(material_id,id),
        FOREIGN KEY(material_id,reconciliation_effect_id) REFERENCES reconciliation_material_effects(material_id,id),
        FOREIGN KEY(material_id,baseline_reconciliation_effect_id) REFERENCES reconciliation_material_effects(material_id,id),
        CHECK((receipt_id IS NOT NULL)+(production_effect_id IS NOT NULL)+(observation_version_id IS NOT NULL)+(waste_id IS NOT NULL)+(reconciliation_effect_id IS NOT NULL)=1),
        CHECK((baseline_receipt_id IS NOT NULL)+(baseline_production_effect_id IS NOT NULL)+(baseline_observation_version_id IS NOT NULL)+(baseline_reconciliation_effect_id IS NOT NULL)=1),
        CHECK(quantity_micros>0 OR value_micros=0));")?;
    // Include the incoming stocktake/consumption edges as well as direct holding
    // references. No foreign-key disabling, in-place SQL rewriting, or ID remap.
    let related = [
        "holding_heads",
        "sale_cost_rows",
        "stocktake_versions",
        "stocktake_heads",
        "consumption_versions",
        "consumption_heads",
        "consumption_rows",
        "wastes",
    ];
    let mut names = related.to_vec();
    names.push("holding_versions");
    let mut indexes = Vec::new();
    for name in &names {
        indexes.extend(connection.prepare("SELECT sql FROM sqlite_master WHERE type='index' AND tbl_name=?1 AND sql IS NOT NULL ORDER BY name")?
            .query_map([name], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?);
    }
    for name in related {
        let sql: String = connection.query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name=?1",
            [name],
            |r| r.get(0),
        )?;
        connection.execute_batch(&renamed_identifiers(&sql, &names))?;
    }
    connection.execute_batch(
        "INSERT INTO holding_versions_next SELECT *,NULL,NULL FROM holding_versions",
    )?;
    for name in related {
        connection.execute_batch(&format!("INSERT INTO {name}_next SELECT * FROM {name}"))?;
    }
    for name in [
        "consumption_rows",
        "consumption_heads",
        "consumption_versions",
        "stocktake_heads",
        "sale_cost_rows",
        "holding_heads",
        "wastes",
        "stocktake_versions",
        "holding_versions",
    ] {
        connection.execute_batch(&format!("DROP TABLE {name}"))?;
    }
    for name in names {
        connection.execute_batch(&format!("ALTER TABLE {name}_next RENAME TO {name}"))?;
    }
    for sql in indexes {
        connection.execute_batch(&sql)?;
    }
    connection.execute_batch("PRAGMA user_version=11")?;
    Ok(())
}

fn renamed_identifiers(sql: &str, names: &[&str]) -> String {
    let mut result = String::new();
    let mut token = String::new();
    let flush = |token: &mut String, result: &mut String| {
        result.push_str(token);
        if names.contains(&token.as_str()) {
            result.push_str("_next");
        }
        token.clear();
    };
    for ch in sql.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            token.push(ch);
        } else {
            flush(&mut token, &mut result);
            result.push(ch);
        }
    }
    flush(&mut token, &mut result);
    result
}
