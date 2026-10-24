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

fn decode<T: DeserializeOwned>(v: Value) -> T {
    serde_json::from_value(v).unwrap()
}
fn value(v: impl Serialize) -> Value {
    serde_json::to_value(v).unwrap()
}
fn meta(store: &Store) -> Value {
    let s = store.catalog_state().unwrap();
    json!({"request_id":Uuid::new_v4().to_string(),"epoch":s.epoch,"expected_revision":s.revision})
}
fn item(store: &mut Store, name: &str) -> MaterialSummary {
    let mut request = meta(store);
    request["name"] = json!(name);
    request["unit"] = json!("g");
    request["kind"] = json!("raw");
    request["description"] = json!("가상 실사 시험");
    store.catalog_create(decode(request)).unwrap().value
}
fn save(store: &mut Store, payload: Value, target: Option<&str>) -> Value {
    value(
        store
            .business_create(decode(
                json!({"meta":meta(store),"payload":payload,"corrects_confirmation_id":target}),
            ))
            .unwrap(),
    )
}
fn bid(v: &Value) -> &str {
    v["result"]["item"]["id"].as_str().unwrap()
}
fn confirm_request(store: &Store, saved: &Value) -> Value {
    json!({"meta":meta(store),"expected_head_id":saved["result"]["head"]["id"]})
}
fn confirm(store: &mut Store, saved: &Value) -> Value {
    let r = confirm_request(store, saved);
    value(store.business_confirm(bid(saved), decode(r)).unwrap())
}
fn preview(store: &Store, saved: &Value) -> Value {
    value(
        store
            .business_preview(
                bid(saved),
                decode(json!({"expected_head_id":saved["result"]["head"]["id"]})),
            )
            .unwrap(),
    )
}
fn row(m: &MaterialSummary, q: &str) -> Value {
    json!({"material_id":m.id,"material_version_id":m.version_id,"quantity":q,"unit":"g","initial_value":null,"excess":null})
}
fn initial(m: &MaterialSummary, q: &str, v: &str) -> Value {
    let mut r = row(m, q);
    r["initial_value"] = json!({"kind":"manual_amount","amount":v});
    r
}
fn stocktake(rows: Vec<Value>) -> Value {
    json!({"kind":"stocktake","version":1,"data":{"rows":rows,"note":"가상 실사"}})
}
fn observe(store: &mut Store, rows: Vec<Value>) -> Value {
    let s = save(store, stocktake(rows), None);
    confirm(store, &s)
}
fn stocktake_detail(store: &Store, confirmed: &Value) -> Value {
    value(
        store
            .stocktake_detail(
                confirmed["result"]["confirmation"]["result"]["stocktake_id"]
                    .as_str()
                    .unwrap(),
            )
            .unwrap()
            .unwrap(),
    )
}
fn receipt(store: &mut Store, m: &MaterialSummary, q: &str, amount: &str) {
    let s = save(
        store,
        json!({"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":m.id,"material_version_id":m.version_id},"quantity":q,"unit":"g","amount":amount,"receipt_date":"2026-01-01","note":"가상 입고","stock_effect":{"kind":"unreflected"}}}),
        None,
    );
    confirm(store, &s);
}
fn recipe(store: &mut Store, name: &str, lines: Vec<Value>) -> Value {
    value(store.recipe_create(decode(json!({"meta":meta(store),"owner":{"mode":"new","name":name,"unit":"개","kind":"menu","description":"가상"},"data":{"basis_count":"1","reference_output":null,"selling_price":"1000","cooking_description":"","process":"","reference_time":"","lines":lines}}))).unwrap())
}
fn line(m: &MaterialSummary, q: &str) -> Value {
    json!({"material_id":m.id,"material_version_id":m.version_id,"quantity":q,"unit":"g","note":""})
}
fn sale_payload(r: &Value, count: &str, note: &str) -> Value {
    json!({"kind":"sale","version":1,"data":{"material_id":r["item"]["id"],"material_version_id":r["item"]["version_id"],"recipe_version_id":r["version"]["id"],"count":count,"note":note}})
}
fn sell(store: &mut Store, r: &Value, count: &str) -> Value {
    let s = save(store, sale_payload(r, count, "가상 판매"), None);
    confirm(store, &s)
}
fn sale_id(s: &Value) -> &str {
    s["result"]["confirmation"]["result"]["sale_id"]
        .as_str()
        .unwrap()
}
fn count(root: &Path, table: &str) -> i64 {
    Connection::open(root.join("app.sqlite3"))
        .unwrap()
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}
fn latest_change(root: &Path, change: impl FnOnce(&mut Value)) -> (std::path::PathBuf, String) {
    let db = Connection::open(root.join("app.sqlite3")).unwrap();
    let (seq, text): (i64, String) = db
        .query_row(
            "SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    let mut v: Value = serde_json::from_str(&text).unwrap();
    change(&mut v);
    let text = serde_json::to_string_pretty(&v).unwrap() + "\n";
    db.execute(
        "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=?2",
        rusqlite::params![text, seq],
    )
    .unwrap();
    let path = root.join(format!("snapshots/{seq:020}.json"));
    fs::write(&path, &text).unwrap();
    (path, text)
}

fn install_fixture(root: &Path, sql: &str) -> Vec<(i64, String)> {
    let mut database = Connection::open(root.join("app.sqlite3")).unwrap();
    database.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    let transaction = database.transaction().unwrap();
    transaction
        .execute_batch("PRAGMA defer_foreign_keys=ON;")
        .unwrap();
    transaction.execute_batch(sql).unwrap();
    transaction.commit().unwrap();
    fs::create_dir(root.join("snapshots")).unwrap();
    let journals = database
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")
        .unwrap()
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    for (sequence, payload) in &journals {
        fs::write(root.join(format!("snapshots/{sequence:020}.json")), payload).unwrap();
    }
    journals
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
    database
        .prepare(&format!(
            "SELECT {selected} FROM \"{table}\" ORDER BY rowid"
        ))
        .unwrap()
        .query_map([], |row| {
            (0..columns.len()).map(|column| row.get(column)).collect()
        })
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

#[test]
fn schema_nine_open_adds_observations_without_rewriting_sales_or_holding_references() {
    let root = TempDir::new().unwrap();
    let journals = install_fixture(root.path(), include_str!("fixtures/schema-v9.sql"));
    assert_eq!(journals.len(), 17);
    assert_eq!(
        journals.last().unwrap().1,
        include_str!("fixtures/snapshot-v2-schema9.json")
    );
    let snapshot: Value = serde_json::from_str(&journals.last().unwrap().1).unwrap();
    assert_eq!(snapshot["format_version"], 2);
    assert_eq!(snapshot["schema_version"], 9);
    assert!(snapshot["state"].get("stocktake_batches").is_none());
    for (key, expected) in [
        ("sales", 1),
        ("sale_cost_rows", 2),
        ("sale_versions", 2),
        ("sale_heads", 1),
        ("business_corrections", 1),
        ("holding_versions", 3),
        ("productions", 1),
        ("receipts", 1),
    ] {
        assert_eq!(snapshot["state"][key].as_array().unwrap().len(), expected);
    }
    let database = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let names = database.prepare("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap()
        .query_map([], |row| row.get::<_,String>(0)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
    let before: BTreeMap<_, _> = names
        .into_iter()
        .map(|name| {
            let columns = columns(&database, &name);
            let rows = table_rows(&database, &name, &columns);
            (name, (columns, rows))
        })
        .collect();
    let holding_links: i64 = database
        .query_row(
            "SELECT COUNT(*) FROM sale_cost_rows WHERE holding_version_id IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(holding_links, 2);
    let old_schema: i64 = database
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(old_schema, 9);
    drop(database);

    let store = Store::open(root.path()).unwrap();
    let database = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let new_schema: i64 = database
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        new_schema, 10,
        "opening schema9 must install stocktake storage"
    );
    for table in [
        "stocktake_batches",
        "stocktake_observations",
        "stocktake_versions",
        "stocktake_heads",
        "consumption_versions",
        "consumption_heads",
        "consumption_rows",
        "wastes",
    ] {
        let count: i64 = database
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "{table}");
    }
    for (table, (columns, rows)) in before {
        assert_eq!(table_rows(&database, &table, &columns), rows, "{table}");
    }
    let last_valid_required: i64 = database.query_row("SELECT [notnull] FROM pragma_table_info('holding_heads') WHERE name='last_valid_version_id'", [], |row| row.get(0)).unwrap();
    assert_eq!(
        last_valid_required, 0,
        "first confirmed zero needs an optional price source"
    );
    let referenced_tables = database
        .prepare("SELECT [table] FROM pragma_foreign_key_list('sale_cost_rows')")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert!(
        referenced_tables
            .iter()
            .any(|table| table == "holding_versions")
    );
    assert!(
        !referenced_tables
            .iter()
            .any(|table| table.ends_with("_next"))
    );
    let broken_foreign_keys: i64 = database
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(broken_foreign_keys, 0);
    for (sequence, payload) in journals {
        assert_eq!(
            fs::read_to_string(root.path().join(format!("snapshots/{sequence:020}.json"))).unwrap(),
            payload
        );
    }
    assert_eq!(store.catalog_state().unwrap().revision, "17");
}

#[test]
fn mixed_batch_preserves_blank_rows_and_distinguishes_initial_zero_from_settlement() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let known = item(&mut store, "가상 기존 기준");
    let unknown = item(&mut store, "가상 미확인");
    let zero = item(&mut store, "가상 최초 영");
    receipt(&mut store, &known, "1000", "9000");
    let saved = save(
        &mut store,
        stocktake(vec![
            row(&unknown, "  "),
            row(&known, "600"),
            initial(&zero, "0", "0"),
        ]),
        None,
    );
    let p = preview(&store, &saved);
    assert!(p["issues"].as_array().unwrap().is_empty());
    assert_eq!(p["effect"]["unchecked_positions"], json!([0]));
    let confirmed = confirm(&mut store, &saved);
    let detail = stocktake_detail(&store, &confirmed);
    assert_eq!(detail["observations"].as_array().unwrap().len(), 2);
    let settlement = &detail["observations"][0]["effect"];
    assert_eq!(settlement["calculation"]["consumption_quantity"], "400");
    assert_eq!(settlement["calculation"]["consumption_value"], "3600");
    assert_eq!(settlement["after"]["value"], "5400");
    let first = &detail["observations"][1];
    assert_eq!(first["effect"]["calculation"]["mode"], "initial");
    assert!(first["consumption_version_id"].is_null());
    let z = value(store.holding_detail(&zero.id).unwrap().unwrap());
    assert_eq!(z["observation"]["id"], first["version_id"]);
    assert_eq!(z["basis"]["id"], first["version_id"]);
    assert!(z["last_valid_price"].is_null());
    assert!(
        store
            .holding_detail(&unknown.id)
            .unwrap()
            .unwrap()
            .basis
            .is_none()
    );
    let again = observe(&mut store, vec![row(&zero, "0")]);
    assert_eq!(
        stocktake_detail(&store, &again)["observations"][0]["effect"]["calculation"]["consumption_value"],
        "0"
    );
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn direct_distinct_references_allocate_counts_and_preserve_unsettled_materials() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let m = item(&mut store, "가상 배분 재료");
    let other = item(&mut store, "가상 다음 실사");
    receipt(&mut store, &m, "1000", "10000");
    receipt(&mut store, &other, "100", "200");
    let a = recipe(
        &mut store,
        "가상 메뉴 갑",
        vec![line(&m, "0"), line(&m, "8"), line(&other, "1")],
    );
    let b = recipe(&mut store, "가상 메뉴 을", vec![line(&m, "999")]);
    let nested = recipe(
        &mut store,
        "가상 하위만 참조",
        vec![
            json!({"material_id":a["item"]["id"],"material_version_id":a["item"]["version_id"],"quantity":"1","unit":"개","note":""}),
        ],
    );
    let sa = sell(&mut store, &a, "3");
    let sb = sell(&mut store, &b, "1");
    let sn = sell(&mut store, &nested, "7");
    let done = observe(&mut store, vec![row(&m, "600")]);
    let detail = stocktake_detail(&store, &done);
    let allocation = &detail["observations"][0]["effect"]["calculation"]["allocation"];
    assert_eq!(allocation["candidates"].as_array().unwrap().len(), 2);
    assert_eq!(allocation["candidates"][0]["quantity"], "300");
    assert_eq!(allocation["candidates"][0]["value"], "3000");
    assert_eq!(allocation["candidates"][1]["quantity"], "100");
    let a_detail = value(store.sale_detail(sale_id(&sa)).unwrap().unwrap());
    assert_eq!(a_detail["consumption_status"], "partially_settled");
    assert_eq!(a_detail["consumption"].as_array().unwrap().len(), 2);
    assert_eq!(
        value(store.sale_detail(sale_id(&sb)).unwrap().unwrap())["consumption_status"],
        "settled"
    );
    assert_eq!(
        value(store.sale_detail(sale_id(&sn)).unwrap().unwrap())["consumption_status"],
        "unconfirmed"
    );
    let correction = save(
        &mut store,
        sale_payload(&a, "4", "가상 수정 이유"),
        sa["result"]["confirmation"]["id"].as_str(),
    );
    assert!(
        preview(&store, &correction)["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["code"] == "requires_allocation_correction")
    );
    let before = value(store.catalog_state().unwrap());
    let request = confirm_request(&store, &correction);
    assert!(matches!(
        store.business_confirm(bid(&correction), decode(request)),
        Err(StoreError::Validation(_))
    ));
    assert_eq!(value(store.catalog_state().unwrap()), before);
    observe(&mut store, vec![row(&other, "90")]);
    assert_eq!(
        value(store.sale_detail(sale_id(&sa)).unwrap().unwrap())["consumption_status"],
        "settled"
    );
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn corrections_before_observation_use_effective_count_but_keep_original_interval() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let m = item(&mut store, "가상 순번 재료");
    let r = recipe(&mut store, "가상 순번 메뉴", vec![line(&m, "1")]);
    let old = sell(&mut store, &r, "5");
    receipt(&mut store, &m, "100", "1000");
    let newer = sell(&mut store, &r, "3");
    let revise = save(
        &mut store,
        sale_payload(&r, "1", "가상 개수 정정"),
        newer["result"]["confirmation"]["id"].as_str(),
    );
    let changed = confirm(&mut store, &revise);
    let old_revise = save(
        &mut store,
        sale_payload(&r, "8", "가상 과거 개수 정정"),
        old["result"]["confirmation"]["id"].as_str(),
    );
    confirm(&mut store, &old_revise);
    let done = observe(&mut store, vec![row(&m, "60")]);
    let detail = stocktake_detail(&store, &done);
    let candidates =
        &detail["observations"][0]["effect"]["calculation"]["allocation"]["candidates"];
    assert_eq!(candidates.as_array().unwrap().len(), 1);
    assert_eq!(candidates[0]["sale_id"], sale_id(&newer));
    assert_eq!(
        candidates[0]["sale_version_id"],
        changed["result"]["confirmation"]["result"]["sale_version_id"]
    );
    assert_eq!(candidates[0]["count"], "1");
    assert_eq!(candidates[0]["quantity"], "40");
    assert_eq!(
        value(store.sale_detail(sale_id(&old)).unwrap().unwrap())["consumption_status"],
        "unconfirmed"
    );
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn zero_weight_and_zero_consumption_candidates_remain_explicit_and_block_count_correction() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let m = item(&mut store, "가상 영 개수");
    receipt(&mut store, &m, "100", "1000");
    let r = recipe(&mut store, "가상 영 판매", vec![line(&m, "0")]);
    let s = sell(&mut store, &r, "0");
    let done = observe(&mut store, vec![row(&m, "100")]);
    let detail = stocktake_detail(&store, &done);
    let a = &detail["observations"][0]["effect"]["calculation"]["allocation"];
    assert_eq!(a["reason"], "zero_weight");
    assert!(a["candidates"][0]["quantity"].is_null());
    assert_eq!(a["unallocated_quantity"], "0");
    let sale = value(store.sale_detail(sale_id(&s)).unwrap().unwrap());
    assert_eq!(sale["consumption"][0]["status"], "unallocated");
    assert_eq!(sale["consumption_status"], "settled");
    let correction = save(
        &mut store,
        sale_payload(&r, "1", "가상 수정"),
        s["result"]["confirmation"]["id"].as_str(),
    );
    assert!(
        preview(&store, &correction)["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["code"] == "requires_allocation_correction")
    );
    let next = observe(&mut store, vec![row(&m, "50")]);
    assert_eq!(
        stocktake_detail(&store, &next)["observations"][0]["effect"]["calculation"]["allocation"]["reason"],
        "no_sales"
    );
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn first_unknown_observation_does_not_allocate_prior_sales_and_later_receipts_are_not_readded() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let m = item(&mut store, "가상 첫 관측");
    let r = recipe(&mut store, "가상 첫 메뉴", vec![line(&m, "1")]);
    let s = sell(&mut store, &r, "2");
    observe(&mut store, vec![initial(&m, "100", "500")]);
    let sale = value(store.sale_detail(sale_id(&s)).unwrap().unwrap());
    assert_eq!(sale["consumption"][0]["reason"], "initial_basis_missing");
    assert_eq!(count(root.path(), "consumption_rows"), 0);
    receipt(&mut store, &m, "50", "250");
    let next = observe(&mut store, vec![row(&m, "120")]);
    assert_eq!(
        stocktake_detail(&store, &next)["observations"][0]["effect"]["calculation"]["consumption_quantity"],
        "30"
    );
    let holding = value(store.holding_detail(&m.id).unwrap().unwrap());
    assert_eq!(holding["settlement_basis"]["quantity"], "120");
    assert_eq!(holding["settlement_basis"]["value"], "600");
    assert_eq!(holding["confirmed_changes"], json!([]));
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn excess_requires_review_and_uses_existing_or_explicit_missing_price_value() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let m = item(&mut store, "가상 초과");
    receipt(&mut store, &m, "10", "100");
    let saved = save(&mut store, stocktake(vec![row(&m, "12")]), None);
    assert!(
        !preview(&store, &saved)["issues"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let mut excess = row(&m, "12");
    excess["excess"] =
        json!({"reviewed":true,"reason":"가상 확인 완료","value":{"kind":"existing_price"}});
    let done = observe(&mut store, vec![excess]);
    let detail = stocktake_detail(&store, &done);
    assert_eq!(detail["observations"][0]["effect"]["after"]["value"], "120");
    assert_eq!(
        detail["observations"][0]["effect"]["calculation"]["consumption_quantity"],
        "0"
    );
    let zero = item(&mut store, "가상 가격 없음");
    observe(&mut store, vec![initial(&zero, "0", "0")]);
    let mut excess = row(&zero, "2");
    excess["excess"] = json!({"reviewed":true,"reason":"가상 확인 완료","value":{"kind":"manual_amount","amount":"0"}});
    observe(&mut store, vec![excess]);
    assert_eq!(
        store
            .holding_detail(&zero.id)
            .unwrap()
            .unwrap()
            .price
            .amount
            .as_deref(),
        Some("0")
    );
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn invalid_rows_and_last_row_sql_failure_leave_the_entire_batch_unchanged() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let a = item(&mut store, "가상 원자 갑");
    let b = item(&mut store, "가상 원자 을");
    receipt(&mut store, &a, "10", "100");
    receipt(&mut store, &b, "20", "200");
    for rows in [
        vec![row(&a, "9"), row(&b, "1.")],
        vec![row(&a, "9"), row(&a, "8")],
        vec![row(&a, "")],
    ] {
        let saved = save(&mut store, stocktake(rows), None);
        let state = value(store.catalog_state().unwrap());
        let request = confirm_request(&store, &saved);
        assert!(matches!(
            store.business_confirm(bid(&saved), decode(request)),
            Err(StoreError::Validation(_))
        ));
        assert_eq!(value(store.catalog_state().unwrap()), state);
        assert_eq!(count(root.path(), "stocktake_batches"), 0);
    }
    let saved = save(
        &mut store,
        stocktake(vec![row(&a, "9"), row(&b, "18")]),
        None,
    );
    let before = value(store.catalog_state().unwrap());
    let holdings = count(root.path(), "holding_versions");
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    db.execute_batch("CREATE TRIGGER fail_second_observation BEFORE INSERT ON stocktake_observations WHEN NEW.position=1 BEGIN SELECT RAISE(ABORT,'injected second row'); END;").unwrap();
    let request = confirm_request(&store, &saved);
    assert!(matches!(
        store.business_confirm(bid(&saved), decode(request.clone())),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(value(store.catalog_state().unwrap()), before);
    assert_eq!(count(root.path(), "stocktake_versions"), 0);
    assert_eq!(count(root.path(), "holding_versions"), holdings);
    db.execute_batch("DROP TRIGGER fail_second_observation")
        .unwrap();
    store
        .business_confirm(bid(&saved), decode(request))
        .unwrap();
    assert_eq!(count(root.path(), "stocktake_versions"), 2);
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn same_request_recovers_missing_json_and_replays_original_response_after_later_changes() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let m = item(&mut store, "가상 재생");
    receipt(&mut store, &m, "10", "100");
    let saved = save(&mut store, stocktake(vec![row(&m, "8")]), None);
    let request = confirm_request(&store, &saved);
    let response = value(
        store
            .business_confirm(bid(&saved), decode(request.clone()))
            .unwrap(),
    );
    let seq = response["state"]["revision"]
        .as_str()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    let path = root.path().join(format!("snapshots/{seq:020}.json"));
    let bytes = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    assert_eq!(
        value(
            store
                .business_confirm(bid(&saved), decode(request.clone()))
                .unwrap()
        ),
        response
    );
    assert_eq!(fs::read(&path).unwrap(), bytes);
    receipt(&mut store, &m, "1", "5");
    let current = value(store.catalog_state().unwrap());
    assert_eq!(
        value(
            store
                .business_confirm(bid(&saved), decode(request.clone()))
                .unwrap()
        ),
        response
    );
    assert_eq!(value(store.catalog_state().unwrap()), current);
    let mut different = request;
    different["expected_head_id"] = json!(Uuid::new_v4().to_string());
    assert!(matches!(
        store.business_confirm(bid(&saved), decode(different)),
        Err(StoreError::Conflict(_))
    ));
    assert_eq!(count(root.path(), "stocktake_batches"), 1);
}

#[test]
fn production_shortage_uses_actual_observation_and_stale_preview_cannot_be_confirmed() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = item(&mut store, "가상 실측 투입");
    let output = item(&mut store, "가상 실측 결과");
    let production = json!({"kind":"production","version":1,"data":{"inputs":[{"material_id":raw.id,"material_version_id":raw.version_id,"quantity":"10","unit":"g"}],"outputs":[{"material_id":output.id,"material_version_id":output.version_id,"quantity":"5","unit":"g","allocation_percent":"100"}],"note":"가상 생산"}});
    let saved = save(&mut store, production, None);
    assert!(
        !preview(&store, &saved)["issues"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let stale = confirm_request(&store, &saved);
    let observation = observe(&mut store, vec![initial(&raw, "12", "120")]);
    assert!(matches!(
        store.business_confirm(bid(&saved), decode(stale)),
        Err(StoreError::StaleState(_))
    ));
    let p = preview(&store, &saved);
    assert!(p["issues"].as_array().unwrap().is_empty());
    let done = confirm(&mut store, &saved);
    let pid = done["result"]["confirmation"]["result"]["production_id"]
        .as_str()
        .unwrap();
    let details = value(store.production_detail(pid).unwrap().unwrap());
    let input = details["materials"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["material_id"] == raw.id)
        .unwrap();
    assert_eq!(
        input["observation"]["id"],
        stocktake_detail(&store, &observation)["observations"][0]["version_id"]
    );
    let holding = value(store.holding_detail(&raw.id).unwrap().unwrap());
    assert_eq!(holding["settlement_basis"]["quantity"], "2");
    assert_eq!(holding["confirmed_changes"].as_array().unwrap().len(), 1);
    let next = observe(&mut store, vec![row(&raw, "1")]);
    assert_eq!(
        stocktake_detail(&store, &next)["observations"][0]["effect"]["calculation"]["consumption_quantity"],
        "1"
    );
    assert_eq!(count(root.path(), "productions"), 1);
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn snapshot_rejects_forged_observation_allocation_time_request_and_holding_relations() {
    for mode in [
        "unknown_confirmation",
        "other_request",
        "future_sequence",
        "invalid_time",
        "wrong_observation",
        "allocation_value",
        "candidate_count",
        "candidate_version",
        "wrong_after",
        "null_last_valid",
        "missing_collection",
        "unknown_field",
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let m = item(&mut store, "가상 검증");
        receipt(&mut store, &m, "10", "100");
        let r = recipe(&mut store, "가상 검증 메뉴", vec![line(&m, "1")]);
        sell(&mut store, &r, "1");
        observe(&mut store, vec![row(&m, "8")]);
        drop(store);
        let (path, text) = latest_change(root.path(), |s| {
            let state = &mut s["state"];
            match mode {
                "unknown_confirmation" => {
                    state["stocktake_batches"][0]["confirmation_id"] =
                        json!(Uuid::new_v4().to_string())
                }
                "other_request" => {
                    let id = state["stocktake_batches"][0]["confirmation_id"].clone();
                    let request = state["business_confirmations"][0]["request_id"].clone();
                    state["business_confirmations"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|c| c["id"] == id)
                        .unwrap()["request_id"] = request;
                }
                "future_sequence" => state["stocktake_batches"][0]["seq"] = json!("999999"),
                "invalid_time" => state["stocktake_versions"][0]["recorded_at"] = json!("invalid"),
                "wrong_observation" => {
                    state["stocktake_versions"][0]["observation_id"] =
                        json!(Uuid::new_v4().to_string())
                }
                "allocation_value" => {
                    state["consumption_versions"][0]["unallocated_value"] = json!("1")
                }
                "candidate_count" => state["consumption_rows"][0]["row"]["count"] = json!("2"),
                "candidate_version" => {
                    state["consumption_rows"][0]["row"]["sale_version_id"] =
                        json!(Uuid::new_v4().to_string())
                }
                "wrong_after" => {
                    state["stocktake_versions"][0]["effect"]["after"]["source_id"] =
                        state["holding_versions"][0]["id"].clone()
                }
                "null_last_valid" => {
                    state["holding_heads"][0]["last_valid_version_id"] = Value::Null
                }
                "missing_collection" => {
                    state.as_object_mut().unwrap().remove("consumption_rows");
                }
                "unknown_field" => {
                    state["stocktake_versions"][0]["effect"]["calculation"]["extra"] = json!(true)
                }
                _ => unreachable!(),
            }
        });
        let database = root.path().join("app.sqlite3");
        let before = fs::read(&database).unwrap();
        assert!(Store::open(root.path()).is_err(), "{mode}");
        assert_eq!(fs::read(&database).unwrap(), before, "{mode}");
        assert_eq!(fs::read_to_string(path).unwrap(), text, "{mode}");
    }
}

#[test]
fn schema_nine_future_or_new_kind_is_rejected_before_migration_with_original_bytes() {
    for mode in ["future_source", "stocktake_payload"] {
        let root = TempDir::new().unwrap();
        install_fixture(root.path(), include_str!("fixtures/schema-v9.sql"));
        let (path, text) = latest_change(root.path(), |s| {
            if mode == "future_source" {
                s["schema_version"] = json!(10);
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
                    s["state"][key] = json!([]);
                }
            } else {
                s["state"]["business_revisions"][0]["payload"] = stocktake(vec![]);
            }
        });
        let database = root.path().join("app.sqlite3");
        let bytes = fs::read(&database).unwrap();
        assert!(Store::open(root.path()).is_err(), "{mode}");
        assert_eq!(fs::read(&database).unwrap(), bytes);
        assert_eq!(fs::read_to_string(path).unwrap(), text);
    }
}

#[test]
fn candidate_count_is_not_limited_by_the_stocktake_input_row_limit() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let m = item(&mut store, "가상 전체 후보");
    receipt(&mut store, &m, "101", "101");
    let r = recipe(&mut store, "가상 후보 메뉴", vec![line(&m, "0")]);
    for _ in 0..101 {
        sell(&mut store, &r, "1");
    }
    let done = observe(&mut store, vec![row(&m, "0")]);
    let detail = stocktake_detail(&store, &done);
    let allocation = &detail["observations"][0]["effect"]["calculation"]["allocation"];
    assert_eq!(allocation["candidates"].as_array().unwrap().len(), 101);
    assert!(
        allocation["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["quantity"] == "1" && r["value"] == "1")
    );
    assert_eq!(allocation["unallocated_quantity"], "0");
}

#[test]
fn publication_failure_blocks_other_mutations_and_retry_recovers_exact_commit() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let m = item(&mut store, "가상 발행 복구");
    receipt(&mut store, &m, "10", "100");
    let saved = save(&mut store, stocktake(vec![row(&m, "8")]), None);
    let request = confirm_request(&store, &saved);
    let sequence = store
        .catalog_state()
        .unwrap()
        .revision
        .parse::<i64>()
        .unwrap()
        + 1;
    let blocked = root.path().join(format!("snapshots/{sequence:020}.json"));
    fs::create_dir(&blocked).unwrap();
    assert!(matches!(
        store.business_confirm(bid(&saved), decode(request.clone())),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(count(root.path(), "stocktake_batches"), 1);
    assert!(matches!(
        store.business_confirm(bid(&saved), decode(request.clone())),
        Err(StoreError::Unavailable(_))
    ));
    let other = json!({"meta":meta(&store),"payload":stocktake(vec![row(&m,"7")]),"corrects_confirmation_id":null});
    assert!(matches!(
        store.business_create(decode(other)),
        Err(StoreError::Unavailable(_))
    ));
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let original: String = db
        .query_row(
            "SELECT payload FROM snapshot_outbox WHERE sequence=?1",
            [sequence],
            |r| r.get(0),
        )
        .unwrap();
    fs::remove_dir(&blocked).unwrap();
    let replay = value(
        store
            .business_confirm(bid(&saved), decode(request.clone()))
            .unwrap(),
    );
    assert_eq!(fs::read_to_string(&blocked).unwrap(), original);
    assert_eq!(replay["state"]["revision"], sequence.to_string());
    assert_eq!(
        value(
            store
                .business_confirm(bid(&saved), decode(request))
                .unwrap()
        ),
        replay
    );
    assert_eq!(count(root.path(), "stocktake_batches"), 1);
}
