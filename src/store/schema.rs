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

pub(super) fn migrate_purchase_returns(connection: &Connection) -> anyhow::Result<()> {
    let journals = connection
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (seq, json) in journals {
        snapshot::validate_source(&json, seq, 12)?;
    }
    connection.execute_batch("PRAGMA defer_foreign_keys=ON;
        CREATE TABLE purchase_returns(
        id TEXT PRIMARY KEY NOT NULL,confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),
        receipt_id TEXT NOT NULL REFERENCES receipts(id),receipt_version_id TEXT NOT NULL,
        material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,
        before_holding_version_id TEXT NOT NULL REFERENCES holding_versions_next(id),
        quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),
        deducted_value_micros INTEGER NOT NULL CHECK(deducted_value_micros>=0 AND deducted_value_micros<=9000000000000000000),
        note TEXT NOT NULL,effect_json TEXT NOT NULL,recorded_at TEXT NOT NULL,
        sequence INTEGER NOT NULL UNIQUE REFERENCES snapshot_outbox(sequence),
        FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id));
        CREATE INDEX purchase_returns_receipt ON purchase_returns(receipt_id);
        CREATE INDEX purchase_returns_material ON purchase_returns(material_id);")?;
    let tables = connection.prepare("SELECT name,sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name!='purchase_returns' ORDER BY name")?
        .query_map([], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut names = vec!["holding_versions".to_string()];
    loop {
        let old = names.len();
        for (name, _) in &tables {
            if names.contains(name) {
                continue;
            }
            let parents = connection
                .prepare(&format!("PRAGMA foreign_key_list({name})"))?
                .query_map([], |r| r.get::<_, String>(2))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            if parents.iter().any(|p| names.contains(p)) {
                names.push(name.clone());
            }
        }
        if old == names.len() {
            break;
        }
    }
    let identifiers = names.iter().map(String::as_str).collect::<Vec<_>>();
    let mut indexes = vec![];
    for name in &names {
        indexes.extend(connection.prepare("SELECT sql FROM sqlite_master WHERE type='index' AND tbl_name=?1 AND sql IS NOT NULL ORDER BY name")?
            .query_map([name],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?);
        let original = &tables
            .iter()
            .find(|(n, _)| n == name)
            .ok_or_else(|| anyhow::anyhow!("원테이블이 없습니다"))?
            .1;
        let changed = if name == "holding_versions" {
            let sql = original.replace(
                "(correction_projection_id IS NOT NULL)=1",
                "(correction_projection_id IS NOT NULL)+(purchase_return_id IS NOT NULL)=1",
            );
            let open = sql
                .find('(')
                .ok_or_else(|| anyhow::anyhow!("테이블 정의 오류"))?;
            format!(
                "{}purchase_return_id TEXT UNIQUE REFERENCES purchase_returns(id),{}",
                &sql[..open + 1],
                &sql[open + 1..]
            )
        } else {
            original.clone()
        };
        connection.execute_batch(&renamed_identifiers(&changed, &identifiers))?;
    }
    for name in &names {
        let columns = connection
            .prepare(&format!("PRAGMA table_info({name})"))?
            .query_map([], |r| r.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let list = columns
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(",");
        connection.execute_batch(&format!(
            "INSERT INTO {name}_next({list}) SELECT {list} FROM {name}"
        ))?;
    }
    for name in names.iter().rev() {
        connection.execute_batch(&format!("DROP TABLE {name}"))?;
    }
    for name in &names {
        connection.execute_batch(&format!("ALTER TABLE {name}_next RENAME TO {name}"))?;
    }
    for sql in indexes {
        connection.execute_batch(&sql)?;
    }
    connection.execute_batch("PRAGMA user_version=13")?;
    Ok(())
}

/// Preserve all incoming edges while widening only the derived-state tables.
pub(super) fn migrate_corrections(connection: &Connection) -> anyhow::Result<()> {
    let journals = connection
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (seq, json) in journals {
        snapshot::validate_source(&json, seq, 11)?;
    }
    connection.execute_batch("PRAGMA defer_foreign_keys=ON;
        CREATE TABLE correction_records(id TEXT PRIMARY KEY NOT NULL,original_id TEXT NOT NULL,target_confirmation_id TEXT NOT NULL REFERENCES business_confirmations(id),previous_version_id TEXT REFERENCES correction_records(id),confirmation_id TEXT NOT NULL UNIQUE REFERENCES business_confirmations(id),business_record_id TEXT NOT NULL REFERENCES business_records(id),recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL UNIQUE REFERENCES snapshot_outbox(sequence),record_json TEXT NOT NULL);
        CREATE TABLE correction_event_versions(id TEXT PRIMARY KEY NOT NULL,correction_id TEXT NOT NULL REFERENCES correction_records(id),kind TEXT NOT NULL,original_id TEXT NOT NULL,original_confirmation_id TEXT NOT NULL REFERENCES business_confirmations(id),previous_version_id TEXT,previous_original_version_id TEXT,logical_sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),logical_position INTEGER NOT NULL CHECK(logical_position>=0),recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),row_json TEXT NOT NULL,UNIQUE(kind,original_id,id),UNIQUE(correction_id,kind,original_id),FOREIGN KEY(kind,original_id,previous_version_id) REFERENCES correction_event_versions(kind,original_id,id),CHECK((previous_version_id IS NOT NULL)+(previous_original_version_id IS NOT NULL)=1));
        CREATE TABLE correction_event_heads(kind TEXT NOT NULL,original_id TEXT NOT NULL,current_version_id TEXT NOT NULL,PRIMARY KEY(kind,original_id),FOREIGN KEY(kind,original_id,current_version_id) REFERENCES correction_event_versions(kind,original_id,id));
        CREATE TABLE correction_projection_versions(id TEXT PRIMARY KEY NOT NULL,correction_id TEXT NOT NULL REFERENCES correction_records(id),kind TEXT NOT NULL,subject_id TEXT NOT NULL,previous_version_id TEXT,recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),row_json TEXT NOT NULL,UNIQUE(kind,subject_id,id),UNIQUE(correction_id,kind,subject_id),FOREIGN KEY(kind,subject_id,previous_version_id) REFERENCES correction_projection_versions(kind,subject_id,id));
        CREATE TABLE correction_projection_heads(kind TEXT NOT NULL,subject_id TEXT NOT NULL,current_version_id TEXT NOT NULL,PRIMARY KEY(kind,subject_id),FOREIGN KEY(kind,subject_id,current_version_id) REFERENCES correction_projection_versions(kind,subject_id,id));
        CREATE TABLE correction_price_bases(id TEXT PRIMARY KEY NOT NULL,correction_id TEXT NOT NULL REFERENCES correction_records(id),material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,amount_micros INTEGER NOT NULL CHECK(amount_micros>=0 AND amount_micros<=9000000000000000000),quantity_micros INTEGER NOT NULL CHECK(quantity_micros>0 AND quantity_micros<=9000000000000000),logical_sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),row_json TEXT NOT NULL,UNIQUE(material_id,id),FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id));")?;
    let tables = connection.prepare("SELECT name,sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'correction_%' ORDER BY name")?
        .query_map([], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut names = vec![
        "holding_versions".to_string(),
        "holding_heads".into(),
        "sale_cost_rows".into(),
        "stocktake_versions".into(),
        "consumption_versions".into(),
        "purchase_order_versions".into(),
        "order_receipt_closures".into(),
    ];
    loop {
        let old_len = names.len();
        for (name, _) in &tables {
            if names.contains(name) {
                continue;
            }
            let refs = connection
                .prepare(&format!("PRAGMA foreign_key_list({name})"))?
                .query_map([], |r| r.get::<_, String>(2))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            if refs.iter().any(|parent| names.contains(parent)) {
                names.push(name.clone());
            }
        }
        if names.len() == old_len {
            break;
        }
    }
    let identifiers = names.iter().map(String::as_str).collect::<Vec<_>>();
    let mut indexes = vec![];
    for name in &names {
        indexes.extend(connection.prepare("SELECT sql FROM sqlite_master WHERE type='index' AND tbl_name=?1 AND sql IS NOT NULL ORDER BY name")?
            .query_map([name], |r| r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?);
        let original = &tables
            .iter()
            .find(|(n, _)| n == name)
            .ok_or_else(|| anyhow::anyhow!("마이그레이션 원본 테이블 없음"))?
            .1;
        let changed = correction_table_definition(name, original)?;
        connection.execute_batch(&renamed_identifiers(&changed, &identifiers))?;
    }
    for name in &names {
        let columns = connection
            .prepare(&format!("PRAGMA table_info({name})"))?
            .query_map([], |r| r.get::<_, String>(1))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let columns = columns
            .iter()
            .map(|v| format!("\"{v}\""))
            .collect::<Vec<_>>()
            .join(",");
        connection.execute_batch(&format!(
            "INSERT INTO {name}_next({columns}) SELECT {columns} FROM {name}"
        ))?;
    }
    for name in names.iter().rev() {
        connection.execute_batch(&format!("DROP TABLE {name}"))?;
    }
    for name in &names {
        connection.execute_batch(&format!("ALTER TABLE {name}_next RENAME TO {name}"))?;
    }
    for index in indexes {
        connection.execute_batch(&index)?;
    }
    connection.execute_batch("PRAGMA user_version=12")?;
    Ok(())
}

fn correction_table_definition(name: &str, original: &str) -> anyhow::Result<String> {
    let mut sql = original.to_owned();
    let added = match name {
        "holding_versions" => {
            sql = sql.replace("(reconciliation_effect_id IS NOT NULL)=1", "(reconciliation_effect_id IS NOT NULL)+(correction_projection_id IS NOT NULL)=1")
                .replace("(baseline_reconciliation_effect_id IS NOT NULL)=1", "(baseline_reconciliation_effect_id IS NOT NULL)+(baseline_correction_projection_id IS NOT NULL)=1");
            "correction_projection_id TEXT UNIQUE REFERENCES correction_projection_versions(id),baseline_correction_projection_id TEXT REFERENCES correction_projection_versions(id),last_valid_correction_price_id TEXT REFERENCES correction_price_bases(id)"
        }
        "holding_heads" => {
            sql = sql.replace(
                "current_version_id TEXT NOT NULL",
                "current_version_id TEXT",
            );
            "unknown_correction_projection_id TEXT REFERENCES correction_projection_versions(id),last_valid_correction_price_id TEXT,FOREIGN KEY(material_id,last_valid_correction_price_id) REFERENCES correction_price_bases(material_id,id),CHECK((current_version_id IS NOT NULL)+(unknown_correction_projection_id IS NOT NULL)=1),CHECK((last_valid_version_id IS NOT NULL)+(last_valid_correction_price_id IS NOT NULL)<=1),CHECK(current_version_id IS NOT NULL OR (last_valid_version_id IS NULL AND last_valid_correction_price_id IS NULL))"
        }
        "stocktake_versions" => {
            "correction_projection_id TEXT UNIQUE REFERENCES correction_projection_versions(id),logical_sequence INTEGER REFERENCES snapshot_outbox(sequence),observed_at TEXT"
        }
        "consumption_versions" => {
            sql = sql.replace(
                "observation_version_id TEXT NOT NULL UNIQUE",
                "observation_version_id TEXT NOT NULL",
            );
            "correction_projection_id TEXT UNIQUE REFERENCES correction_projection_versions(id),sequence INTEGER REFERENCES snapshot_outbox(sequence)"
        }
        "purchase_order_versions" => {
            sql = sql.replace("confirmation_id TEXT NOT NULL UNIQUE", "confirmation_id TEXT NOT NULL")
                .replace("(action='receive' AND status='received')", "(action='receive' AND status='received') OR (action='correct' AND status IN('open','received'))");
            "correction_projection_id TEXT UNIQUE REFERENCES correction_projection_versions(id),CHECK((action='correct')=(correction_projection_id IS NOT NULL))"
        }
        "order_receipt_closures" => {
            sql = sql
                .replace(
                    "order_id TEXT PRIMARY KEY NOT NULL",
                    "order_id TEXT NOT NULL",
                )
                .replace(
                    "receipt_id TEXT NOT NULL UNIQUE",
                    "receipt_id TEXT PRIMARY KEY NOT NULL",
                );
            return Ok(sql);
        }
        "sale_cost_rows" => {
            sql = sql.replace("source_kind IN('holding_average','last_valid') AND holding_version_id IS NOT NULL", "((source_kind='holding_average' AND holding_version_id IS NOT NULL AND correction_price_basis_id IS NULL) OR (source_kind='last_valid' AND ((holding_version_id IS NOT NULL)+(correction_price_basis_id IS NOT NULL)=1)))");
            "correction_price_basis_id TEXT REFERENCES correction_price_bases(id),CHECK(correction_price_basis_id IS NULL OR (source_kind='last_valid' AND holding_version_id IS NULL AND material_price_id IS NULL))"
        }
        _ => return Ok(sql),
    };
    let open = sql
        .find('(')
        .ok_or_else(|| anyhow::anyhow!("테이블 정의의 시작이 없습니다"))?;
    let close = sql
        .rfind(')')
        .ok_or_else(|| anyhow::anyhow!("테이블 정의의 끝이 없습니다"))?;
    let constraint = |s: &str| {
        [
            "CHECK",
            "UNIQUE",
            "FOREIGN KEY",
            "PRIMARY KEY",
            "CONSTRAINT",
        ]
        .iter()
        .any(|prefix| s.trim_start().starts_with(prefix))
    };
    let mut entries = sql_definition_parts(&sql[open + 1..close]);
    let additions = sql_definition_parts(added);
    let position = entries
        .iter()
        .position(|e| constraint(e))
        .unwrap_or(entries.len());
    entries.splice(
        position..position,
        additions.iter().filter(|e| !constraint(e)).cloned(),
    );
    entries.extend(additions.into_iter().filter(|e| constraint(e)));
    Ok(format!(
        "{}{}{}",
        &sql[..open + 1],
        entries.join(","),
        &sql[close..]
    ))
}

fn sql_definition_parts(body: &str) -> Vec<String> {
    let mut result = vec![];
    let mut depth = 0;
    let mut start = 0;
    let mut quote = None;
    for (position, ch) in body.char_indices() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                result.push(body[start..position].trim().to_owned());
                start = position + 1;
            }
            _ => {}
        }
    }
    result.push(body[start..].trim().to_owned());
    result
}
