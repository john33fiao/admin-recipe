use rusqlite::{Connection, types::Value as SqlValue};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};
use sunchang_hansang::{
    catalog::MaterialSummary,
    store::{Store, StoreError},
};
use tempfile::TempDir;
use uuid::Uuid;

fn decode<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}
fn value(input: impl Serialize) -> Value {
    serde_json::to_value(input).unwrap()
}
fn meta(store: &Store) -> Value {
    let state = store.catalog_state().unwrap();
    json!({"request_id":Uuid::new_v4().to_string(),"epoch":state.epoch,"expected_revision":state.revision})
}
fn item(store: &mut Store, name: &str) -> MaterialSummary {
    let input = json!({"request_id":Uuid::new_v4().to_string(),"epoch":store.catalog_state().unwrap().epoch,"expected_revision":store.catalog_state().unwrap().revision,"name":name,"unit":"g","kind":"raw","description":"가상 생산 자료"});
    store.catalog_create(decode(input)).unwrap().value
}
fn save(store: &mut Store, payload: Value) -> Value {
    let input = json!({"meta":meta(store),"payload":payload,"corrects_confirmation_id":null});
    value(store.business_create(decode(input)).unwrap())
}
fn id(saved: &Value) -> &str {
    saved["result"]["item"]["id"].as_str().unwrap()
}
fn confirmation_input(store: &Store, saved: &Value) -> Value {
    json!({"meta":meta(store),"expected_head_id":saved["result"]["head"]["id"]})
}
fn confirm(store: &mut Store, saved: &Value) -> Value {
    let input = confirmation_input(store, saved);
    value(store.business_confirm(id(saved), decode(input)).unwrap())
}
fn preview(store: &Store, saved: &Value) -> Value {
    value(
        store
            .business_preview(
                id(saved),
                decode(json!({"expected_head_id":saved["result"]["head"]["id"]})),
            )
            .unwrap(),
    )
}
fn production_id(result: &Value) -> &str {
    result["result"]["confirmation"]["result"]["production_id"]
        .as_str()
        .unwrap()
}
fn input(item: &MaterialSummary, quantity: &str) -> Value {
    json!({"material_id":item.id,"material_version_id":item.version_id,"quantity":quantity,"unit":"g"})
}
fn output(item: &MaterialSummary, quantity: &str, percent: &str) -> Value {
    let mut row = input(item, quantity);
    row["allocation_percent"] = json!(percent);
    row
}
fn production(inputs: Vec<Value>, outputs: Vec<Value>) -> Value {
    json!({"kind":"production","version":1,"data":{"inputs":inputs,"outputs":outputs,"note":"가상 실제 생산"}})
}
fn receipt(store: &mut Store, item: &MaterialSummary, q: &str, amount: &str) -> Value {
    let saved = save(
        store,
        json!({"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":item.id,"material_version_id":item.version_id},"quantity":q,"unit":"g","amount":amount,"receipt_date":"2026-01-01","note":"가상 보유 근거","stock_effect":{"kind":"unreflected"}}}),
    );
    confirm(store, &saved)
}
fn count(root: &Path, table: &str) -> i64 {
    Connection::open(root.join("app.sqlite3"))
        .unwrap()
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

fn change_latest(root: &Path, edit: impl FnOnce(&mut Value)) -> (std::path::PathBuf, String) {
    let database = Connection::open(root.join("app.sqlite3")).unwrap();
    let (seq, payload): (i64, String) = database
        .query_row(
            "SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    let mut snapshot: Value = serde_json::from_str(&payload).unwrap();
    edit(&mut snapshot);
    let changed = serde_json::to_string_pretty(&snapshot).unwrap() + "\n";
    database
        .execute(
            "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=?2",
            rusqlite::params![changed, seq],
        )
        .unwrap();
    let file = root.join(format!("snapshots/{seq:020}.json"));
    fs::write(&file, &changed).unwrap();
    (file, changed)
}

#[test]
fn production_snapshot_rejects_nonprior_material_versions_and_invalid_event_metadata() {
    let mut accepted = Vec::new();
    for mode in [
        "same_sequence_material",
        "missing_sequence_material",
        "invalid_time",
        "invalid_confirmation_uuid",
        "noncanonical_sequence",
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let raw = item(&mut store, "가상 투입");
        let prepared = item(&mut store, "가상 결과");
        receipt(&mut store, &raw, "2", "20");
        let saved = save(
            &mut store,
            production(vec![input(&raw, "2")], vec![output(&prepared, "1", "100")]),
        );
        confirm(&mut store, &saved);
        drop(store);
        let (file, changed) = change_latest(root.path(), |snapshot| {
            let state = &mut snapshot["state"];
            let seq = state["productions"][0]["seq"].clone();
            let confirmation = state["productions"][0]["confirmation_id"].clone();
            if mode.ends_with("material") {
                let version = state["material_versions"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|v| v["id"] == prepared.version_id)
                    .unwrap();
                version["seq"] = if mode == "same_sequence_material" {
                    seq
                } else {
                    Value::Null
                };
            } else if mode == "invalid_confirmation_uuid" {
                state["productions"][0]["confirmation_id"] = json!("invalid-confirmation");
                state["business_confirmations"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|c| c["id"] == confirmation)
                    .unwrap()["id"] = json!("invalid-confirmation");
            } else {
                let key = if mode == "invalid_time" {
                    "recorded_at"
                } else {
                    "seq"
                };
                let replacement = if mode == "invalid_time" {
                    json!("invalid-time")
                } else {
                    json!(format!("0{}", seq.as_str().unwrap()))
                };
                for collection in [
                    "productions",
                    "business_confirmations",
                    "business_revisions",
                    "production_effects",
                    "holding_versions",
                ] {
                    for row in state[collection].as_array_mut().unwrap() {
                        if row["seq"] == seq {
                            row[key] = replacement.clone();
                        }
                        if mode == "noncanonical_sequence"
                            && collection == "holding_versions"
                            && row["baseline_seq"] == seq
                        {
                            row["baseline_seq"] = replacement.clone();
                        }
                    }
                }
                if mode == "noncanonical_sequence" {
                    state["mutation_requests"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|r| r["sequence"] == seq)
                        .unwrap()["sequence"] = replacement;
                }
            }
        });
        let bytes = fs::read(root.path().join("app.sqlite3")).unwrap();
        if let Ok(opened) = Store::open(root.path()) {
            accepted.push(mode);
            drop(opened);
        }
        assert_eq!(
            fs::read(root.path().join("app.sqlite3")).unwrap(),
            bytes,
            "{mode}"
        );
        assert_eq!(fs::read_to_string(file).unwrap(), changed, "{mode}");
    }
    assert!(
        accepted.is_empty(),
        "accepted invalid production metadata: {accepted:?}"
    );
}

fn legacy_v7(root: &Path) -> Vec<(i64, String)> {
    install_fixture(root, include_str!("fixtures/schema-v7.sql"))
}

fn install_fixture(root: &Path, sql: &str) -> Vec<(i64, String)> {
    let mut database = Connection::open(root.join("app.sqlite3")).unwrap();
    database.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    let tx = database.transaction().unwrap();
    tx.execute_batch("PRAGMA defer_foreign_keys=ON;").unwrap();
    tx.execute_batch(sql).unwrap();
    tx.commit().unwrap();
    fs::create_dir(root.join("snapshots")).unwrap();
    let rows = database
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")
        .unwrap()
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    for (seq, payload) in &rows {
        fs::write(root.join(format!("snapshots/{seq:020}.json")), payload).unwrap();
    }
    rows
}

fn columns(database: &Connection, table: &str) -> Vec<String> {
    database
        .prepare(&format!("PRAGMA table_info(\"{table}\")"))
        .unwrap()
        .query_map([], |row| row.get(1))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

fn table_rows(database: &Connection, table: &str, columns: &[String]) -> Vec<Vec<SqlValue>> {
    let selected = columns
        .iter()
        .map(|column| format!("\"{column}\""))
        .collect::<Vec<_>>()
        .join(",");
    let mut statement = database
        .prepare(&format!(
            "SELECT {selected} FROM \"{table}\" ORDER BY rowid"
        ))
        .unwrap();
    statement
        .query_map([], |row| {
            (0..columns.len()).map(|column| row.get(column)).collect()
        })
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

#[test]
fn schema_seven_open_adds_production_relations_without_rewriting_retained_data() {
    let root = TempDir::new().unwrap();
    let journals = legacy_v7(root.path());
    assert_eq!(journals.len(), 12);
    assert_eq!(
        journals.last().unwrap().1,
        include_str!("fixtures/snapshot-v2-schema7.json")
    );
    let snapshot: Value = serde_json::from_str(&journals.last().unwrap().1).unwrap();
    assert_eq!(snapshot["format_version"], 2);
    assert_eq!(snapshot["schema_version"], 7);
    assert!(snapshot["state"].get("productions").is_none());
    for (key, count) in [
        ("receipts", 1),
        ("holding_versions", 1),
        ("order_receipt_closures", 1),
        ("recipe_versions", 4),
        ("recipe_heads", 2),
    ] {
        assert_eq!(snapshot["state"][key].as_array().unwrap().len(), count);
    }
    let database = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let names = database
        .prepare("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    let before: BTreeMap<_, _> = names
        .into_iter()
        .map(|name| {
            let columns = columns(&database, &name);
            let rows = table_rows(&database, &name, &columns);
            (name, (columns, rows))
        })
        .collect();
    let old_schema: i64 = database
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(old_schema, 7);
    drop(database);

    let store = Store::open(root.path()).unwrap();
    let database = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let new_schema: i64 = database
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        new_schema, 11,
        "opening schema7 must install current storage"
    );
    for table in [
        "productions",
        "production_inputs",
        "production_outputs",
        "production_effects",
    ] {
        let count: i64 = database
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "{table}");
    }
    for (table, (columns, rows)) in before {
        assert_eq!(table_rows(&database, &table, &columns), rows, "{table}");
    }
    for (seq, payload) in journals {
        assert_eq!(
            fs::read_to_string(root.path().join(format!("snapshots/{seq:020}.json"))).unwrap(),
            payload
        );
    }
    let broken_foreign_keys: i64 = database
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(broken_foreign_keys, 0);
    assert_eq!(store.catalog_state().unwrap().revision, "12");
}

#[test]
fn actual_production_survives_restart_and_transfers_all_value_with_last_valid_cost() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = item(&mut store, "가상 원재료");
    let prepared = item(&mut store, "가상 손질분");
    receipt(&mut store, &raw, "3000", "30000");
    let before = store.holding_detail(&raw.id).unwrap().unwrap();
    let before_source = before.settlement_basis.unwrap().source_id.unwrap();
    let payload = production(
        vec![input(&raw, "3000")],
        vec![output(&prepared, "2000", "100")],
    );
    let saved = save(&mut store, payload.clone());
    let state = store.catalog_state().unwrap();
    let p = preview(&store, &saved);
    assert_eq!(p["can_confirm"], true);
    assert_eq!(p["effect"]["total_input_value"], "30000");
    assert!(
        p["effect"]["materials"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["after"]["source_id"].is_null())
    );
    assert_eq!(store.catalog_state().unwrap(), state);
    assert_eq!(count(root.path(), "holding_versions"), 1);
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    assert_eq!(
        value(
            store
                .business_detail(id(&saved))
                .unwrap()
                .unwrap()
                .head
                .payload
        ),
        payload
    );
    let request = confirmation_input(&store, &saved);
    let result = value(
        store
            .business_confirm(id(&saved), decode(request.clone()))
            .unwrap(),
    );
    let detail = store
        .production_detail(production_id(&result))
        .unwrap()
        .unwrap();
    assert_eq!(detail.item.inputs[0].quantity, "3000");
    assert_eq!(detail.item.outputs[0].quantity, "2000");
    assert_eq!(detail.item.outputs[0].value, "30000");
    let source_effect = detail
        .materials
        .iter()
        .find(|m| m.material_id == raw.id)
        .unwrap();
    assert_eq!(
        source_effect.before.as_ref().unwrap().source_id.as_deref(),
        Some(before_source.as_str())
    );
    let output_effect = detail
        .materials
        .iter()
        .find(|m| m.material_id == prepared.id)
        .unwrap();
    let current = store.holding_detail(&raw.id).unwrap().unwrap();
    assert_eq!(current.settlement_basis.unwrap().quantity, "0");
    assert_eq!(current.price.source, "last_valid");
    assert_eq!(
        current.price.source_id.as_deref(),
        Some(before_source.as_str())
    );
    assert_eq!(current.price.amount.as_deref(), Some("30000"));
    let produced = store.holding_detail(&prepared.id).unwrap().unwrap();
    assert!(produced.observation.is_none());
    assert_eq!(value(produced.basis.unwrap())["kind"], "production");
    assert_eq!(
        store
            .holding_detail(&prepared.id)
            .unwrap()
            .unwrap()
            .basis
            .unwrap()
            .id,
        Some(output_effect.id.clone())
    );
    assert_ne!(output_effect.id, detail.item.id);
    assert!(produced.confirmed_changes.is_empty());
    assert_eq!(produced.price.quantity.as_deref(), Some("2000"));
    assert_eq!(produced.price.amount.as_deref(), Some("30000"));
    let recipe_request = json!({"meta":meta(&store),"owner":{"mode":"new","name":"가상 원가 메뉴","unit":"개","kind":"menu","description":""},"data":{"basis_count":"1","reference_output":null,"selling_price":null,"cooking_description":"","process":"","reference_time":"","lines":[{"material_id":raw.id,"material_version_id":raw.version_id,"quantity":"100","unit":"g","note":""}]}});
    let menu = store.recipe_create(decode(recipe_request)).unwrap();
    assert_eq!(
        store
            .recipe_cost(&menu.item.id, None)
            .unwrap()
            .unwrap()
            .total_cost
            .as_deref(),
        Some("1000")
    );
    receipt(&mut store, &raw, "100", "0");
    assert_eq!(
        store
            .recipe_cost(&menu.item.id, None)
            .unwrap()
            .unwrap()
            .total_cost
            .as_deref(),
        Some("0")
    );
    assert_eq!(
        value(store.business_confirm(id(&saved), decode(request)).unwrap()),
        result
    );
    assert_eq!(
        value(
            store
                .production_detail(production_id(&result))
                .unwrap()
                .unwrap()
                .item
        ),
        value(detail.item)
    );
    assert_eq!(count(root.path(), "productions"), 1);
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn multiple_outputs_preserve_allocation_zero_and_smallest_money_remainders() {
    for (amount, a, b, expected_a, expected_b) in [
        ("30000", "100", "0", "30000", "0"),
        ("30000", "70", "30", "21000", "9000"),
        ("0.000001", "50", "50", "0.000001", "0"),
        ("0", "70", "30", "0", "0"),
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let raw = item(&mut store, "가상 투입");
        let main = item(&mut store, "가상 주결과");
        let side = item(&mut store, "가상 부결과");
        receipt(&mut store, &raw, "3", amount);
        let saved = save(
            &mut store,
            production(
                vec![input(&raw, "3")],
                vec![output(&main, "2", a), output(&side, "1", b)],
            ),
        );
        let result = confirm(&mut store, &saved);
        let detail = store
            .production_detail(production_id(&result))
            .unwrap()
            .unwrap();
        assert_eq!(detail.item.total_input_value, amount);
        assert_eq!(detail.item.outputs[0].value, expected_a);
        assert_eq!(detail.item.outputs[1].value, expected_b);
        assert_eq!(
            store
                .catalog_detail(&side.id)
                .unwrap()
                .unwrap()
                .item
                .price
                .amount
                .as_deref(),
            Some(expected_b)
        );
        drop(store);
        Store::open(root.path()).unwrap();
    }
}

#[test]
fn duplicate_input_output_materials_use_pre_event_state_and_keep_one_net_zero_effect() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = item(&mut store, "가상 재손질");
    receipt(&mut store, &raw, "3", "0.000001");
    let saved = save(
        &mut store,
        production(
            vec![input(&raw, "1"), input(&raw, "2")],
            vec![output(&raw, "1", "50"), output(&raw, "2", "50")],
        ),
    );
    let result = confirm(&mut store, &saved);
    let detail = store
        .production_detail(production_id(&result))
        .unwrap()
        .unwrap();
    assert_eq!(detail.materials.len(), 1);
    assert_eq!(detail.item.inputs[0].value, "0");
    assert_eq!(detail.item.inputs[1].value, "0.000001");
    assert_eq!(detail.item.outputs[0].value, "0.000001");
    assert_eq!(detail.item.outputs[1].value, "0");
    assert_eq!(detail.materials[0].before.as_ref().unwrap().quantity, "3");
    assert_eq!(detail.materials[0].after.quantity, "3");
    assert_eq!(detail.materials[0].after.value.as_deref(), Some("0.000001"));
    assert_eq!(count(root.path(), "holding_versions"), 2);
    assert_eq!(count(root.path(), "production_effects"), 1);
    assert_eq!(
        store
            .production_list(decode(json!({"material_id":raw.id})))
            .unwrap()
            .items
            .len(),
        1
    );
    let held = store.holding_detail(&raw.id).unwrap().unwrap();
    assert_eq!(held.confirmed_changes.len(), 1);
    let change = value(&held.confirmed_changes[0]);
    assert_eq!(change["kind"], "production");
    assert_eq!(change["input_quantity"], "3");
    assert_eq!(change["output_quantity"], "3");
    assert_eq!(change["production_id"], production_id(&result));
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn production_basis_receipt_and_later_production_replay_each_effect_once() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = item(&mut store, "가상 재료");
    let stock = item(&mut store, "가상 비축");
    let result_item = item(&mut store, "가상 최종결과");
    receipt(&mut store, &raw, "5", "50");
    let saved = save(
        &mut store,
        production(vec![input(&raw, "5")], vec![output(&stock, "2", "100")]),
    );
    confirm(&mut store, &saved);
    let basis = store.holding_detail(&stock.id).unwrap().unwrap().basis;
    receipt(&mut store, &stock, "1", "10");
    let saved = save(
        &mut store,
        production(
            vec![input(&stock, "3")],
            vec![output(&result_item, "6", "100")],
        ),
    );
    confirm(&mut store, &saved);
    let current = store.holding_detail(&stock.id).unwrap().unwrap();
    assert_eq!(current.basis, basis);
    assert!(current.observation.is_none());
    assert_eq!(current.settlement_basis.unwrap().quantity, "0");
    assert_eq!(current.confirmed_changes.len(), 2);
    assert_eq!(value(&current.confirmed_changes[0])["kind"], "receipt");
    assert_eq!(value(&current.confirmed_changes[1])["kind"], "production");
    assert_eq!(current.price.quantity.as_deref(), Some("3"));
    assert_eq!(current.price.amount.as_deref(), Some("60"));
    assert_eq!(
        store
            .holding_detail(&result_item.id)
            .unwrap()
            .unwrap()
            .settlement_basis
            .unwrap()
            .value
            .as_deref(),
        Some("60")
    );
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn schema_seven_requests_replay_original_responses_after_new_production() {
    let root = TempDir::new().unwrap();
    let originals = legacy_v7(root.path());
    let frozen: Value =
        serde_json::from_str(include_str!("fixtures/snapshot-v2-schema7.json")).unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = store
        .catalog_list(Default::default())
        .unwrap()
        .into_iter()
        .find(|m| m.unit == "g")
        .unwrap();
    let prepared = item(&mut store, "가상 새 생산 결과");
    let saved = save(
        &mut store,
        production(
            vec![input(&raw, "3000")],
            vec![output(&prepared, "2000", "100")],
        ),
    );
    confirm(&mut store, &saved);
    let before = store.catalog_state().unwrap();
    for request in frozen["state"]["mutation_requests"].as_array().unwrap() {
        let payload: Value = serde_json::from_str(request["payload"].as_str().unwrap()).unwrap();
        let expected: Value = serde_json::from_str(request["response"].as_str().unwrap()).unwrap();
        let actual = match request["operation"].as_str().unwrap() {
            "material.create" => value(store.catalog_create(decode(payload)).unwrap()),
            "material.revise" => value(
                store
                    .catalog_revise(payload[0].as_str().unwrap(), decode(payload[1].clone()))
                    .unwrap(),
            ),
            "material.price" => value(
                store
                    .catalog_add_price(payload[0].as_str().unwrap(), decode(payload[1].clone()))
                    .unwrap(),
            ),
            "business.create" => value(store.business_create(decode(payload)).unwrap()),
            "business.confirm" => value(
                store
                    .business_confirm(payload[0].as_str().unwrap(), decode(payload[1].clone()))
                    .unwrap(),
            ),
            "recipe.create" => value(store.recipe_create(decode(payload)).unwrap()),
            "recipe.revise" => value(
                store
                    .recipe_revise(payload[0].as_str().unwrap(), decode(payload[1].clone()))
                    .unwrap(),
            ),
            "recipe.copy" => value(
                store
                    .recipe_copy(payload[0].as_str().unwrap(), decode(payload[1].clone()))
                    .unwrap(),
            ),
            op => panic!("unexpected fixture operation {op}"),
        };
        assert_eq!(actual, expected);
    }
    assert_eq!(store.catalog_state().unwrap(), before);
    for (seq, payload) in &originals {
        let file = root.path().join(format!("snapshots/{seq:020}.json"));
        assert_eq!(fs::read_to_string(&file).unwrap(), *payload);
        fs::remove_file(file).unwrap();
    }
    store.recover().unwrap();
    for (seq, payload) in originals {
        assert_eq!(
            fs::read_to_string(root.path().join(format!("snapshots/{seq:020}.json"))).unwrap(),
            payload
        );
    }
}

#[test]
fn frozen_six_seven_and_failed_migrations_reject_expansion_without_rewriting() {
    for mode in [
        "schema6_kind",
        "schema6_source",
        "schema7_kind",
        "schema7_source",
        "schema7_zero",
        "source7_future8",
        "migration_ddl",
    ] {
        let root = TempDir::new().unwrap();
        if mode.starts_with("schema6") {
            install_fixture(root.path(), include_str!("fixtures/schema-v6.sql"));
        } else {
            legacy_v7(root.path());
        }
        if mode == "migration_ddl" {
            Connection::open(root.path().join("app.sqlite3"))
                .unwrap()
                .execute_batch("CREATE TABLE productions(injected TEXT);")
                .unwrap();
        }
        let (file, changed) = change_latest(root.path(), |snapshot| match mode {
            "schema6_kind" | "schema7_kind" => {
                snapshot["state"]["business_records"][0]["kind"] = json!("production")
            }
            "schema6_source" | "schema7_source" => {
                snapshot["state"]["holding_versions"][0]["production_effect_id"] =
                    json!(Uuid::new_v4().to_string())
            }
            "schema7_zero" => {
                snapshot["state"]["holding_versions"][0]["quantity"] = json!("0");
                snapshot["state"]["holding_versions"][0]["value"] = json!("0");
            }
            "source7_future8" => snapshot["schema_version"] = json!(8),
            _ => {}
        });
        let before = fs::read(root.path().join("app.sqlite3")).unwrap();
        assert!(Store::open(root.path()).is_err(), "{mode}");
        assert_eq!(
            fs::read(root.path().join("app.sqlite3")).unwrap(),
            before,
            "{mode}"
        );
        assert_eq!(fs::read_to_string(file).unwrap(), changed, "{mode}");
        let database = Connection::open(root.path().join("app.sqlite3")).unwrap();
        let version: i64 = database
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, if mode.starts_with("schema6") { 6 } else { 7 });
    }
}

#[test]
fn schema_eight_rejects_broken_production_allocation_source_and_holding_relationships() {
    for mode in [
        "missing_productions",
        "orphan_confirmation",
        "wrong_input_effect",
        "input_value",
        "output_value",
        "allocation",
        "duplicate_effect",
        "effect_material",
        "missing_holding",
        "wrong_previous",
        "old_last_valid",
        "baseline_twice",
        "future_seq",
        "non_uuid_holding",
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let raw = item(&mut store, "가상 변조 투입");
        let prepared = item(&mut store, "가상 변조 결과");
        receipt(&mut store, &raw, "1", "10");
        let first_holding = store
            .holding_detail(&raw.id)
            .unwrap()
            .unwrap()
            .settlement_basis
            .unwrap()
            .source_id
            .unwrap();
        receipt(&mut store, &raw, "1", "10");
        let saved = save(
            &mut store,
            production(vec![input(&raw, "2")], vec![output(&prepared, "1", "100")]),
        );
        confirm(&mut store, &saved);
        drop(store);
        let (file, changed) = change_latest(root.path(), |snapshot| {
            let state = &mut snapshot["state"];
            match mode {
                "missing_productions" => {
                    state.as_object_mut().unwrap().remove("productions");
                }
                "orphan_confirmation" => {
                    state["productions"][0]["confirmation_id"] = json!(Uuid::new_v4().to_string())
                }
                "wrong_input_effect" => {
                    state["production_inputs"][0]["effect_id"] = json!(Uuid::new_v4().to_string())
                }
                "input_value" => state["production_inputs"][0]["row"]["value"] = json!("19.999999"),
                "output_value" => {
                    state["production_outputs"][0]["row"]["value"] = json!("19.999999")
                }
                "allocation" => {
                    state["production_outputs"][0]["row"]["allocation_percent"] = json!("99.999999")
                }
                "duplicate_effect" => {
                    let copy = state["production_effects"][0].clone();
                    state["production_effects"]
                        .as_array_mut()
                        .unwrap()
                        .push(copy);
                }
                "effect_material" => {
                    state["production_effects"][0]["material_version_id"] =
                        json!(Uuid::new_v4().to_string())
                }
                "missing_holding" => {
                    state["holding_versions"].as_array_mut().unwrap().pop();
                }
                "wrong_previous" => {
                    state["holding_versions"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|h| {
                            h["material_id"] == raw.id && h["production_effect_id"].is_string()
                        })
                        .unwrap()["previous_version_id"] = json!(first_holding)
                }
                "old_last_valid" => {
                    state["holding_heads"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|h| h["material_id"] == raw.id)
                        .unwrap()["last_valid_version_id"] = json!(first_holding)
                }
                "baseline_twice" => {
                    state["holding_versions"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|h| h["material_id"] == prepared.id)
                        .unwrap()["baseline_receipt_id"] = state["receipts"][0]["id"].clone()
                }
                "future_seq" => state["productions"][0]["seq"] = json!("999999"),
                _ => {
                    let h = state["holding_versions"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|h| h["material_id"] == prepared.id)
                        .unwrap();
                    h["id"] = json!("invalid-holding");
                    let head = state["holding_heads"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|h| h["material_id"] == prepared.id)
                        .unwrap();
                    head["current_version_id"] = json!("invalid-holding");
                    head["last_valid_version_id"] = json!("invalid-holding");
                }
            }
        });
        let before = fs::read(root.path().join("app.sqlite3")).unwrap();
        assert!(Store::open(root.path()).is_err(), "{mode}");
        assert_eq!(
            fs::read(root.path().join("app.sqlite3")).unwrap(),
            before,
            "{mode}"
        );
        assert_eq!(fs::read_to_string(file).unwrap(), changed, "{mode}");
    }
}

#[test]
fn shortage_keeps_actual_inputs_until_real_missing_receipt_and_fresh_confirmation() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = item(&mut store, "가상 부족 재료");
    receipt(&mut store, &raw, "1000", "10000");
    let payload = production(vec![input(&raw, "1200")], vec![output(&raw, "2000", "100")]);
    let saved = save(&mut store, payload.clone());
    let stale = confirmation_input(&store, &saved);
    let before = store.catalog_state().unwrap();
    let p = preview(&store, &saved);
    assert_eq!(p["can_confirm"], false);
    assert!(
        p["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["code"] == "needs_reconciliation")
    );
    assert!(matches!(
        store.business_confirm(id(&saved), decode(stale.clone())),
        Err(StoreError::Validation(_))
    ));
    assert_eq!(store.catalog_state().unwrap(), before);
    assert_eq!(count(root.path(), "productions"), 0);
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    assert_eq!(
        value(
            store
                .business_detail(id(&saved))
                .unwrap()
                .unwrap()
                .head
                .payload
        ),
        payload
    );
    let actual_receipt = receipt(&mut store, &raw, "500", "5000");
    let new_source = actual_receipt["result"]["confirmation"]["result"]["holding_version_id"]
        .as_str()
        .unwrap();
    assert!(matches!(
        store.business_confirm(id(&saved), decode(stale)),
        Err(StoreError::StaleState(_))
    ));
    let p = preview(&store, &saved);
    assert_eq!(p["can_confirm"], true);
    assert_eq!(
        p["effect"]["materials"][0]["before"]["source_id"],
        new_source
    );
    let result = confirm(&mut store, &saved);
    let detail = store
        .production_detail(production_id(&result))
        .unwrap()
        .unwrap();
    assert_eq!(
        detail.materials[0]
            .before
            .as_ref()
            .unwrap()
            .source_id
            .as_deref(),
        Some(new_source)
    );
    assert_eq!(detail.materials[0].after.quantity, "2300");
    assert_eq!(detail.materials[0].after.value.as_deref(), Some("15000"));
    assert_eq!(count(root.path(), "production_effects"), 1);
}

#[test]
fn incomplete_and_invalid_rows_remain_drafts_without_partial_effects() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = item(&mut store, "가상 검증 투입");
    let prepared = item(&mut store, "가상 검증 결과");
    receipt(&mut store, &raw, "100", "100");
    for mode in [
        "blank",
        "partial",
        "zero",
        "negative",
        "dimension",
        "percentage",
        "missing",
        "range",
    ] {
        let mut payload = production(vec![input(&raw, "1")], vec![output(&prepared, "1", "100")]);
        match mode {
            "blank" => payload["data"]["inputs"].as_array_mut().unwrap().push(
                json!({"material_id":null,"material_version_id":null,"quantity":"","unit":""}),
            ),
            "partial" => payload["data"]["inputs"][0]["quantity"] = json!("1."),
            "zero" => payload["data"]["outputs"][0]["quantity"] = json!("0"),
            "negative" => payload["data"]["inputs"][0]["quantity"] = json!("-1"),
            "dimension" => payload["data"]["inputs"][0]["unit"] = json!("mL"),
            "percentage" => payload["data"]["outputs"][0]["allocation_percent"] = json!("99"),
            "missing" => {
                payload["data"]["outputs"][0]["material_id"] = json!(Uuid::new_v4().to_string())
            }
            _ => payload["data"]["outputs"][0]["quantity"] = json!("9000000000.000001"),
        }
        let saved = save(&mut store, payload.clone());
        let state = store.catalog_state().unwrap();
        assert_eq!(preview(&store, &saved)["can_confirm"], false, "{mode}");
        let request = confirmation_input(&store, &saved);
        assert!(
            store.business_confirm(id(&saved), decode(request)).is_err(),
            "{mode}"
        );
        assert_eq!(store.catalog_state().unwrap(), state);
        assert_eq!(
            value(
                store
                    .business_detail(id(&saved))
                    .unwrap()
                    .unwrap()
                    .head
                    .payload
            ),
            payload
        );
        assert_eq!(count(root.path(), "holding_versions"), 1);
        assert_eq!(count(root.path(), "productions"), 0);
    }
    let payload = production(vec![input(&raw, "1")], vec![output(&prepared, "1", "100")]);
    let saved = save(&mut store, payload);
    let changed = json!({"request_id":Uuid::new_v4().to_string(),"epoch":store.catalog_state().unwrap().epoch,"expected_revision":store.catalog_state().unwrap().revision,"expected_version_id":raw.version_id,"name":raw.name,"unit":raw.unit,"kind":raw.kind,"description":"가상 변경"});
    store.catalog_revise(&raw.id, decode(changed)).unwrap();
    let p = preview(&store, &saved);
    assert!(
        p["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["code"] == "stale_reference")
    );
    let request = confirmation_input(&store, &saved);
    assert!(matches!(
        store.business_confirm(id(&saved), decode(request)),
        Err(StoreError::StaleState(_))
    ));
}

#[test]
fn production_each_write_failure_rolls_back_all_rows_and_preserves_retry_identity() {
    for table in [
        "business_confirmations",
        "productions",
        "production_effects",
        "production_inputs",
        "production_outputs",
        "holding_versions",
        "holding_heads",
        "snapshot_outbox",
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let raw = item(&mut store, "가상 투입");
        let result_item = item(&mut store, "가상 결과");
        receipt(&mut store, &raw, "2", "20");
        let saved = save(
            &mut store,
            production(
                vec![input(&raw, "2")],
                vec![output(&result_item, "1", "100")],
            ),
        );
        let request = confirmation_input(&store, &saved);
        let before = store.catalog_state().unwrap();
        let old_holding = value(store.holding_detail(&raw.id).unwrap().unwrap());
        let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
        let trigger = if table == "snapshot_outbox" {
            "CREATE TRIGGER injected BEFORE UPDATE OF payload ON snapshot_outbox BEGIN SELECT RAISE(FAIL,'injected'); END;".into()
        } else {
            format!(
                "CREATE TRIGGER injected BEFORE INSERT ON {table} BEGIN SELECT RAISE(FAIL,'injected'); END;"
            )
        };
        db.execute_batch(&trigger).unwrap();
        assert!(
            store
                .business_confirm(id(&saved), decode(request.clone()))
                .is_err(),
            "{table}"
        );
        assert_eq!(store.catalog_state().unwrap(), before, "{table}");
        for new_table in [
            "productions",
            "production_effects",
            "production_inputs",
            "production_outputs",
        ] {
            assert_eq!(count(root.path(), new_table), 0, "{table}/{new_table}");
        }
        assert_eq!(count(root.path(), "holding_versions"), 1);
        assert_eq!(
            value(store.holding_detail(&raw.id).unwrap().unwrap()),
            old_holding
        );
        db.execute_batch("DROP TRIGGER injected;").unwrap();
        let first = value(
            store
                .business_confirm(id(&saved), decode(request.clone()))
                .unwrap(),
        );
        assert_eq!(
            value(
                store
                    .business_confirm(id(&saved), decode(request.clone()))
                    .unwrap()
            ),
            first
        );
        let mut different = request;
        different["expected_head_id"] = json!(Uuid::new_v4().to_string());
        assert!(matches!(
            store.business_confirm(id(&saved), decode(different)),
            Err(StoreError::Conflict(_))
        ));
        assert_eq!(count(root.path(), "productions"), 1);
        assert_eq!(count(root.path(), "holding_versions"), 3);
    }
}

#[test]
fn publication_failure_recovers_same_production_before_new_commands() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = item(&mut store, "가상 투입");
    let prepared = item(&mut store, "가상 결과");
    receipt(&mut store, &raw, "2", "20");
    let saved = save(
        &mut store,
        production(vec![input(&raw, "2")], vec![output(&prepared, "1", "100")]),
    );
    let request = confirmation_input(&store, &saved);
    let seq = store
        .catalog_state()
        .unwrap()
        .revision
        .parse::<i64>()
        .unwrap()
        + 1;
    let file = root.path().join(format!("snapshots/{seq:020}.json"));
    fs::create_dir(&file).unwrap();
    assert!(matches!(
        store.business_confirm(id(&saved), decode(request.clone())),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(count(root.path(), "productions"), 1);
    assert!(matches!(
        store.business_confirm(id(&saved), decode(request.clone())),
        Err(StoreError::Unavailable(_))
    ));
    let new_request = json!({"meta":meta(&store),"payload":production(vec![],vec![]),"corrects_confirmation_id":null});
    assert!(matches!(
        store.business_create(decode(new_request)),
        Err(StoreError::Unavailable(_))
    ));
    fs::remove_dir(&file).unwrap();
    let response = value(
        store
            .business_confirm(id(&saved), decode(request.clone()))
            .unwrap(),
    );
    let bytes = fs::read(&file).unwrap();
    fs::remove_file(&file).unwrap();
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    assert_eq!(fs::read(file).unwrap(), bytes);
    assert_eq!(
        value(store.business_confirm(id(&saved), decode(request)).unwrap()),
        response
    );
    assert_eq!(count(root.path(), "holding_versions"), 3);
}
