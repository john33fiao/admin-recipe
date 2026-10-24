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
    let mut input = meta(store);
    input["name"] = json!(name);
    input["unit"] = json!("g");
    input["kind"] = json!("raw");
    input["description"] = json!("가상 판매 시험");
    store.catalog_create(decode(input)).unwrap().value
}
fn price(store: &mut Store, item: &MaterialSummary, amount: &str) {
    let mut input = meta(store);
    input["expected_version_id"] = json!(item.version_id);
    input["quantity"] = json!("1");
    input["unit"] = json!("g");
    input["amount"] = json!(amount);
    store.catalog_add_price(&item.id, decode(input)).unwrap();
}
fn line(item: &MaterialSummary, q: &str) -> Value {
    json!({"material_id":item.id,"material_version_id":item.version_id,"quantity":q,"unit":"g","note":"가상 직접 참조"})
}
fn recipe(store: &mut Store, name: &str, lines: Vec<Value>, selling: Option<&str>) -> Value {
    let input = json!({"meta":meta(store),"owner":{"mode":"new","name":name,"unit":"개","kind":"menu","description":"가상 판매 메뉴"},"data":{"basis_count":"3","reference_output":null,"selling_price":selling,"cooking_description":"가상 설명","process":"","reference_time":"","lines":lines}});
    value(store.recipe_create(decode(input)).unwrap())
}
fn payload(recipe: &Value, count: &str, note: &str) -> Value {
    json!({"kind":"sale","version":1,"data":{"material_id":recipe["item"]["id"],"material_version_id":recipe["item"]["version_id"],"recipe_version_id":recipe["version"]["id"],"count":count,"note":note}})
}
fn save(store: &mut Store, payload: Value, target: Option<&str>) -> Value {
    let input = json!({"meta":meta(store),"payload":payload,"corrects_confirmation_id":target});
    value(store.business_create(decode(input)).unwrap())
}
fn business_id(saved: &Value) -> &str {
    saved["result"]["item"]["id"].as_str().unwrap()
}
fn confirm_input(store: &Store, saved: &Value) -> Value {
    json!({"meta":meta(store),"expected_head_id":saved["result"]["head"]["id"]})
}
fn confirm(store: &mut Store, saved: &Value) -> Value {
    let input = confirm_input(store, saved);
    value(
        store
            .business_confirm(business_id(saved), decode(input))
            .unwrap(),
    )
}
fn sale_id(saved: &Value) -> &str {
    saved["result"]["confirmation"]["result"]["sale_id"]
        .as_str()
        .unwrap()
}
fn confirmation_id(saved: &Value) -> &str {
    saved["result"]["confirmation"]["id"].as_str().unwrap()
}
fn sell(store: &mut Store, recipe: &Value, count: &str) -> Value {
    let draft = save(store, payload(recipe, count, "가상 판매"), None);
    confirm(store, &draft)
}
fn preview(store: &Store, saved: &Value) -> Value {
    value(
        store
            .business_preview(
                business_id(saved),
                decode(json!({"expected_head_id":saved["result"]["head"]["id"]})),
            )
            .unwrap(),
    )
}
fn receipt(store: &mut Store, item: &MaterialSummary, q: &str, amount: &str) {
    let draft = save(
        store,
        json!({"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":item.id,"material_version_id":item.version_id},"quantity":q,"unit":"g","amount":amount,"receipt_date":"2026-01-01","note":"가상 입고","stock_effect":{"kind":"unreflected"}}}),
        None,
    );
    confirm(store, &draft);
}
fn inventory(root: &Path) -> BTreeMap<String, Vec<Vec<SqlValue>>> {
    let db = Connection::open(root.join("app.sqlite3")).unwrap();
    [
        "holding_versions",
        "holding_heads",
        "receipts",
        "productions",
        "production_inputs",
        "production_outputs",
        "production_effects",
        "material_prices",
    ]
    .into_iter()
    .map(|t| (t.to_owned(), table_rows(&db, t, &columns(&db, t))))
    .collect()
}
fn table_count(root: &Path, table: &str) -> i64 {
    Connection::open(root.join("app.sqlite3"))
        .unwrap()
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
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
fn schema_eight_open_adds_sale_relations_without_rewriting_retained_data() {
    let root = TempDir::new().unwrap();
    let journals = install_fixture(root.path(), include_str!("fixtures/schema-v8.sql"));
    assert_eq!(journals.len(), 13);
    assert_eq!(
        journals.last().unwrap().1,
        include_str!("fixtures/snapshot-v2-schema8.json")
    );
    let snapshot: Value = serde_json::from_str(&journals.last().unwrap().1).unwrap();
    assert_eq!(snapshot["format_version"], 2);
    assert_eq!(snapshot["schema_version"], 8);
    assert!(snapshot["state"].get("sales").is_none());
    for (key, expected) in [
        ("receipts", 1),
        ("holding_versions", 3),
        ("order_receipt_closures", 1),
        ("recipe_versions", 2),
        ("recipe_heads", 2),
        ("productions", 1),
        ("production_inputs", 1),
        ("production_outputs", 1),
        ("production_effects", 2),
    ] {
        assert_eq!(snapshot["state"][key].as_array().unwrap().len(), expected);
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
    assert_eq!(old_schema, 8);
    drop(database);

    let store = Store::open(root.path()).unwrap();
    let database = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let new_schema: i64 = database
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        new_schema, 12,
        "opening schema8 must install current storage"
    );
    for table in ["sales", "sale_cost_rows", "sale_versions", "sale_heads"] {
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
    for (seq, payload) in journals {
        assert_eq!(
            fs::read_to_string(root.path().join(format!("snapshots/{seq:020}.json"))).unwrap(),
            payload
        );
    }
    let broken_foreign_keys: i64 = database
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(broken_foreign_keys, 0);
    assert_eq!(store.catalog_state().unwrap().revision, "13");
}

#[test]
fn incomplete_sale_survives_restart_edit_confirmation_and_never_changes_inventory() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = item(&mut store, "가상 미입고 재료");
    let menu = recipe(&mut store, "가상 영분량 메뉴", vec![line(&raw, "0")], None);
    let draft = save(
        &mut store,
        json!({"kind":"sale","version":1,"data":{"material_id":null,"material_version_id":null,"recipe_version_id":null,"count":"","note":"가상\n미완성"}}),
        None,
    );
    assert!(!preview(&store, &draft)["can_confirm"].as_bool().unwrap());
    let saved_payload = draft["result"]["head"]["payload"].clone();
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    assert_eq!(
        value(
            store
                .business_detail(business_id(&draft))
                .unwrap()
                .unwrap()
                .head
                .payload
        ),
        saved_payload
    );
    let update = json!({"meta":meta(&store),"expected_head_id":draft["result"]["head"]["id"],"payload":payload(&menu,"0","가상 수정")});
    let edited = value(
        store
            .business_revise(business_id(&draft), decode(update))
            .unwrap(),
    );
    let before = inventory(root.path());
    let state = store.catalog_state().unwrap();
    let p = preview(&store, &edited);
    assert_eq!(p["can_confirm"], true);
    assert_eq!(p["effect"]["sale"]["amount"], Value::Null);
    assert_eq!(p["effect"]["sale"]["recipe_cost"]["total_cost"], "0");
    assert_eq!(
        p["effect"]["sale"]["recipe_cost"]["rows"][0]["price"]["source"],
        "missing"
    );
    assert_eq!(store.catalog_state().unwrap(), state);
    let result = confirm(&mut store, &edited);
    let detail = store.sale_detail(sale_id(&result)).unwrap().unwrap();
    assert_eq!(detail.head.count, "0");
    assert!(detail.head.amount.is_none());
    assert_eq!(detail.item.seq, detail.head.seq);
    assert_eq!(detail.item.recorded_at, detail.head.recorded_at);
    assert_eq!(value(detail.consumption_status), "unconfirmed");
    chrono::DateTime::parse_from_rfc3339(&detail.item.recorded_at).unwrap();
    assert_eq!(inventory(root.path()), before);
    drop(store);
    let store = Store::open(root.path()).unwrap();
    assert_eq!(
        value(store.sale_detail(sale_id(&result)).unwrap().unwrap().item),
        value(detail.item)
    );
}

#[test]
fn sale_cost_keeps_exact_recipe_and_each_historical_price_source_after_changes() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = item(&mut store, "가상 원가 재료");
    let output = item(&mut store, "가상 비축 결과");
    let menu = recipe(
        &mut store,
        "가상 분수 메뉴",
        vec![line(&raw, "100"), line(&raw, "0")],
        Some("1000"),
    );
    let mut originals = vec![];
    for (source, cost) in [
        ("missing", None),
        ("preregistered", Some("300")),
        ("holding_average", Some("333.333333")),
        ("last_valid", Some("333.333333")),
        ("holding_average", Some("0")),
    ] {
        match source {
            "preregistered" => price(&mut store, &raw, "9"),
            "holding_average" if cost == Some("333.333333") => {
                receipt(&mut store, &raw, "3000", "30000")
            }
            "last_valid" => {
                let p = save(
                    &mut store,
                    json!({"kind":"production","version":1,"data":{"inputs":[{"material_id":raw.id,"material_version_id":raw.version_id,"quantity":"3000","unit":"g"}],"outputs":[{"material_id":output.id,"material_version_id":output.version_id,"quantity":"2000","unit":"g","allocation_percent":"100"}],"note":"가상 전량 생산"}}),
                    None,
                );
                confirm(&mut store, &p);
            }
            "holding_average" => receipt(&mut store, &raw, "100", "0"),
            _ => {}
        }
        let before = inventory(root.path());
        let result = sell(&mut store, &menu, "3");
        let original = store.sale_detail(sale_id(&result)).unwrap().unwrap().item;
        assert_eq!(value(original.recipe_cost.rows[0].price.source), source);
        assert_eq!(original.recipe_cost.total_cost.as_deref(), cost);
        assert_eq!(original.recipe_cost.rows[0].numerator_quantity, "100");
        assert_eq!(original.recipe_cost.basis_count, "3");
        assert_eq!(original.recipe_cost.rows[1].cost.as_deref(), Some("0"));
        assert_eq!(inventory(root.path()), before);
        originals.push(original);
    }
    let mut edited = meta(&store);
    edited["expected_version_id"] = json!(raw.version_id);
    edited["name"] = json!("가상 바뀐 재료명");
    edited["unit"] = json!("g");
    edited["kind"] = json!("raw");
    edited["description"] = json!("가상 이름 변경");
    store.catalog_revise(&raw.id, decode(edited)).unwrap();
    let changed = json!({"meta":meta(&store),"expected_material_version_id":menu["item"]["version_id"],"expected_recipe_version_id":menu["version"]["id"],"data":{"basis_count":"1","reference_output":null,"selling_price":"2000","cooking_description":"변경","process":"","reference_time":"","lines":[]}});
    store
        .recipe_revise(menu["item"]["id"].as_str().unwrap(), decode(changed))
        .unwrap();
    drop(store);
    let store = Store::open(root.path()).unwrap();
    for original in originals {
        assert_eq!(
            store.sale_detail(&original.id).unwrap().unwrap().item,
            original
        );
    }
}

#[test]
fn sale_revenue_distinguishes_null_zero_bounds_and_wide_list_totals() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let missing = recipe(&mut store, "가상 미정 판매가", vec![], None);
    let zero = recipe(&mut store, "가상 영원 메뉴", vec![], Some("0"));
    let maximum = recipe(
        &mut store,
        "가상 최대금액 메뉴",
        vec![],
        Some("9000000000000"),
    );
    sell(&mut store, &missing, "0");
    sell(&mut store, &zero, "1000000000");
    sell(&mut store, &maximum, "1");
    sell(&mut store, &maximum, "1");
    let list = store.sale_list(Default::default()).unwrap();
    assert_eq!(list.known_revenue, "18000000000000");
    assert_eq!(list.missing_revenue_count, 1);
    assert!(
        list.items
            .iter()
            .any(|s| s.count == "0" && s.amount.is_none())
    );
    assert!(
        list.items
            .iter()
            .any(|s| s.count == "1000000000" && s.amount.as_deref() == Some("0"))
    );
    let draft = save(&mut store, payload(&maximum, "2", "가상 상한 초과"), None);
    assert_eq!(preview(&store, &draft)["can_confirm"], false);
    let state = store.catalog_state().unwrap();
    let before = inventory(root.path());
    assert!(matches!(
        store.business_confirm(business_id(&draft), decode(confirm_input(&store, &draft))),
        Err(StoreError::Validation(_))
    ));
    assert_eq!(store.catalog_state().unwrap(), state);
    assert_eq!(inventory(root.path()), before);
    assert_eq!(table_count(root.path(), "sales"), 4);
}

#[test]
fn invalid_and_stale_sale_drafts_preserve_user_input_and_have_no_effect() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = item(&mut store, "가상 검증 재료");
    let mut menu = recipe(
        &mut store,
        "가상 검증 메뉴",
        vec![line(&raw, "5000")],
        Some("1"),
    );
    for count in ["", "-1", "1.", "1.1", "1e1", "1000000001"] {
        let draft = save(&mut store, payload(&menu, count, "가상 미완성"), None);
        assert_eq!(draft["result"]["head"]["payload"]["data"]["count"], count);
        assert_eq!(preview(&store, &draft)["can_confirm"], false);
        let state = store.catalog_state().unwrap();
        assert!(
            store
                .business_confirm(business_id(&draft), decode(confirm_input(&store, &draft)))
                .is_err()
        );
        assert_eq!(store.catalog_state().unwrap(), state);
    }
    let owner = menu["item"]["id"].as_str().unwrap().to_owned();
    let mut revise = meta(&store);
    revise["expected_version_id"] = menu["item"]["version_id"].clone();
    revise["name"] = json!("가상 새 메뉴명");
    revise["unit"] = json!("개");
    revise["kind"] = json!("menu");
    revise["description"] = json!("레시피와 독립 수정");
    let updated = store.catalog_revise(&owner, decode(revise)).unwrap().value;
    let old = save(&mut store, payload(&menu, "1", "가상 옛 선택"), None);
    assert_eq!(preview(&store, &old)["can_confirm"], false);
    menu["item"]["version_id"] = json!(updated.version_id);
    let selected = save(&mut store, payload(&menu, "1", "가상 현재 선택"), None);
    assert_eq!(preview(&store, &selected)["can_confirm"], true);
    let stale = confirm_input(&store, &selected);
    price(&mut store, &raw, "1");
    assert!(matches!(
        store.business_confirm(business_id(&selected), decode(stale)),
        Err(StoreError::StaleState(_))
    ));
    let result = confirm(&mut store, &selected);
    assert_eq!(
        store
            .sale_detail(sale_id(&result))
            .unwrap()
            .unwrap()
            .item
            .material_name,
        "가상 새 메뉴명"
    );
}

#[test]
fn count_corrections_keep_original_basis_and_replay_responses_after_later_versions() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = item(&mut store, "가상 정정 원재료");
    price(&mut store, &raw, "10");
    let menu = recipe(
        &mut store,
        "가상 정정 메뉴",
        vec![line(&raw, "100")],
        Some("1000"),
    );
    let draft = save(&mut store, payload(&menu, "3", "가상 원판매"), None);
    let request = confirm_input(&store, &draft);
    let original = value(
        store
            .business_confirm(business_id(&draft), decode(request.clone()))
            .unwrap(),
    );
    let frozen = store.sale_detail(sale_id(&original)).unwrap().unwrap().item;
    let losing = save(
        &mut store,
        payload(&menu, "2", "가상 경쟁 정정"),
        Some(confirmation_id(&original)),
    );
    let empty = save(
        &mut store,
        payload(&menu, "1", ""),
        Some(confirmation_id(&original)),
    );
    assert_eq!(preview(&store, &empty)["can_confirm"], false);
    price(&mut store, &raw, "20");
    let revised_recipe = json!({"meta":meta(&store),"expected_material_version_id":menu["item"]["version_id"],"expected_recipe_version_id":menu["version"]["id"],"data":{"basis_count":"1","reference_output":null,"selling_price":"2000","cooking_description":"가상 새 조리","process":"","reference_time":"","lines":[]}});
    store
        .recipe_revise(menu["item"]["id"].as_str().unwrap(), decode(revised_recipe))
        .unwrap();
    let corrected = save(
        &mut store,
        payload(&menu, "1", "가상 개수 오입력 정정"),
        Some(confirmation_id(&original)),
    );
    let p = preview(&store, &corrected);
    assert_eq!(p["effect"]["previous"]["count"], "3");
    assert_eq!(p["effect"]["sale"]["amount"], "1000");
    assert_eq!(
        p["effect"]["sale"]["recipe_cost"],
        value(&frozen.recipe_cost)
    );
    let inventory_before = inventory(root.path());
    let replacement = confirm(&mut store, &corrected);
    let losing_request = confirm_input(&store, &losing);
    assert!(matches!(
        store.business_confirm(business_id(&losing), decode(losing_request)),
        Err(StoreError::StaleState(_))
    ));
    assert_eq!(preview(&store, &losing)["can_confirm"], false);
    assert!(
        !store
            .business_list(Default::default())
            .unwrap()
            .items
            .is_empty()
    );
    let cancelled = save(
        &mut store,
        payload(&menu, "0", "가상 재정정"),
        Some(confirmation_id(&replacement)),
    );
    let cancel_result = confirm(&mut store, &cancelled);
    assert_eq!(sale_id(&cancel_result), sale_id(&original));
    let detail = store.sale_detail(sale_id(&original)).unwrap().unwrap();
    assert_eq!(detail.item, frozen);
    assert_eq!(detail.versions.len(), 3);
    assert_eq!(detail.head.count, "0");
    assert_eq!(detail.head.amount.as_deref(), Some("0"));
    assert_eq!(
        detail
            .versions
            .iter()
            .map(|v| v.count.as_str())
            .collect::<Vec<_>>(),
        vec!["3", "1", "0"]
    );
    assert_eq!(inventory(root.path()), inventory_before);
    assert_eq!(table_count(root.path(), "sales"), 1);
    assert_eq!(table_count(root.path(), "sale_cost_rows"), 1);
    let old_response = value(
        store
            .business_confirm(business_id(&draft), decode(request.clone()))
            .unwrap(),
    );
    assert_eq!(old_response, original);
    let mut changed = request;
    changed["expected_head_id"] = cancel_result["result"]["head"]["id"].clone();
    assert!(matches!(
        store.business_confirm(business_id(&draft), decode(changed)),
        Err(StoreError::Conflict(_))
    ));
    let chain = store.business_detail(business_id(&draft)).unwrap().unwrap();
    assert_eq!(chain.confirmations.len(), 3);
    assert_eq!(chain.corrections.len(), 2);
    let mut wrong = payload(&menu, "2", "가상 다른 참조");
    wrong["data"]["recipe_version_id"] = json!(Uuid::new_v4().to_string());
    let wrong_request = json!({"meta":meta(&store),"payload":wrong,"corrects_confirmation_id":confirmation_id(&cancel_result)});
    assert!(matches!(
        store.business_create(decode(wrong_request)),
        Err(StoreError::Validation(_))
    ));
    assert_eq!(
        store.sale_list(Default::default()).unwrap().known_revenue,
        "0"
    );
    drop(store);
    let store = Store::open(root.path()).unwrap();
    assert_eq!(
        store.sale_detail(sale_id(&original)).unwrap().unwrap().item,
        frozen
    );
}

#[test]
fn allocation_uses_direct_references_original_intervals_and_as_of_counts_once() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = item(&mut store, "가상 하위 원물");
    let stocked_recipe = recipe(
        &mut store,
        "가상 비축 레시피",
        vec![line(&raw, "100")],
        Some("1"),
    );
    let stocked = store
        .catalog_detail(stocked_recipe["item"]["id"].as_str().unwrap())
        .unwrap()
        .unwrap()
        .item;
    let stock_line = json!({"material_id":stocked.id,"material_version_id":stocked.version_id,"quantity":"0","unit":"개","note":"가상 직접 비축"});
    let a = recipe(
        &mut store,
        "가상 배분 A",
        vec![stock_line.clone(), stock_line.clone()],
        Some("1"),
    );
    let b = recipe(&mut store, "가상 배분 B", vec![stock_line], Some("1"));
    let first = sell(&mut store, &a, "3");
    let second = sell(&mut store, &b, "1");
    let first_seq = first["result"]["confirmation"]["seq"]
        .as_str()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    let second_seq = second["result"]["confirmation"]["seq"]
        .as_str()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    let old_as_of = store
        .catalog_state()
        .unwrap()
        .revision
        .parse::<i64>()
        .unwrap();
    let old = store
        .allocation_inputs(&stocked.id, 0, second_seq + 1, old_as_of)
        .unwrap();
    assert_eq!(old.len(), 2);
    assert_eq!(
        old.iter().map(|s| s.count.value()).collect::<Vec<_>>(),
        vec![3, 1]
    );
    assert_eq!(old[0].direct_material_ids, vec![stocked.id.clone()]);
    assert!(
        store
            .allocation_inputs(&raw.id, 0, second_seq + 1, old_as_of)
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        store
            .allocation_inputs(&stocked.id, first_seq, second_seq + 1, old_as_of)
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        store
            .allocation_inputs(&stocked.id, 0, second_seq, old_as_of)
            .unwrap()
            .len(),
        1
    );
    let inventory_before = inventory(root.path());
    let corrected = save(
        &mut store,
        payload(&a, "1", "가상 배분 가중치 정정"),
        Some(confirmation_id(&first)),
    );
    confirm(&mut store, &corrected);
    let current = store
        .catalog_state()
        .unwrap()
        .revision
        .parse::<i64>()
        .unwrap();
    let fresh = store
        .allocation_inputs(&stocked.id, 0, second_seq + 1, current)
        .unwrap();
    assert_eq!(
        fresh.iter().map(|s| s.count.value()).collect::<Vec<_>>(),
        vec![1, 1]
    );
    assert_eq!(fresh[0].original_seq, first_seq);
    assert_ne!(fresh[0].sale_version_id, old[0].sale_version_id);
    assert_eq!(
        store
            .allocation_inputs(&stocked.id, 0, second_seq + 1, old_as_of)
            .unwrap()[0]
            .count
            .value(),
        3
    );
    let weights = old
        .iter()
        .map(|s| i64::from(s.count.value()))
        .collect::<Vec<_>>();
    assert_eq!(
        sunchang_hansang::domain::allocate_largest_remainder(400, &weights).unwrap(),
        vec![300, 100]
    );
    assert_eq!(
        sunchang_hansang::domain::allocate_largest_remainder(400, &[1, 1]).unwrap(),
        vec![200, 200]
    );
    assert_eq!(inventory(root.path()), inventory_before);
    assert!(
        store
            .allocation_inputs(&stocked.id, 0, second_seq + 1, current + 1)
            .is_err()
    );
}

#[test]
fn sale_each_write_stage_rolls_back_and_reuses_the_same_request_identity() {
    for table in [
        "sales",
        "sale_cost_rows",
        "sale_versions",
        "sale_heads",
        "business_confirmations",
        "snapshot_outbox",
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let raw = item(&mut store, "가상 원자 재료");
        let menu = recipe(
            &mut store,
            "가상 원자 판매",
            vec![line(&raw, "1")],
            Some("1"),
        );
        let draft = save(&mut store, payload(&menu, "1", "가상 원자성"), None);
        let request = confirm_input(&store, &draft);
        let state = store.catalog_state().unwrap();
        let before = inventory(root.path());
        let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
        db.execute_batch(&format!("CREATE TRIGGER reject_sale BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT,'synthetic sale failure'); END;")).unwrap();
        assert!(
            store
                .business_confirm(business_id(&draft), decode(request.clone()))
                .is_err(),
            "{table}"
        );
        assert_eq!(store.catalog_state().unwrap(), state);
        assert_eq!(table_count(root.path(), "sales"), 0);
        assert_eq!(table_count(root.path(), "sale_versions"), 0);
        assert_eq!(inventory(root.path()), before);
        db.execute_batch("DROP TRIGGER reject_sale;").unwrap();
        let result = value(
            store
                .business_confirm(business_id(&draft), decode(request.clone()))
                .unwrap(),
        );
        assert_eq!(
            value(
                store
                    .business_confirm(business_id(&draft), decode(request))
                    .unwrap()
            ),
            result
        );
        assert_eq!(table_count(root.path(), "sales"), 1);
    }
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let menu = recipe(&mut store, "가상 정정 원자성", vec![], Some("1"));
    let original = sell(&mut store, &menu, "3");
    let draft = save(
        &mut store,
        payload(&menu, "1", "가상 수정 원자성"),
        Some(confirmation_id(&original)),
    );
    let request = confirm_input(&store, &draft);
    let state = store.catalog_state().unwrap();
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    db.execute_batch("CREATE TRIGGER reject_correction BEFORE INSERT ON business_corrections BEGIN SELECT RAISE(ABORT,'synthetic correction failure'); END;").unwrap();
    assert!(
        store
            .business_confirm(business_id(&draft), decode(request.clone()))
            .is_err()
    );
    assert_eq!(store.catalog_state().unwrap(), state);
    assert_eq!(
        store
            .sale_detail(sale_id(&original))
            .unwrap()
            .unwrap()
            .head
            .count,
        "3"
    );
    assert_eq!(table_count(root.path(), "sale_versions"), 1);
    db.execute_batch("DROP TRIGGER reject_correction;").unwrap();
    store
        .business_confirm(business_id(&draft), decode(request))
        .unwrap();
    assert_eq!(
        store
            .sale_detail(sale_id(&original))
            .unwrap()
            .unwrap()
            .head
            .count,
        "1"
    );
}

#[test]
fn sale_publication_failure_recovers_original_bytes_before_any_new_write() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let menu = recipe(&mut store, "가상 복구 판매", vec![], Some("1"));
    let draft = save(&mut store, payload(&menu, "2", "가상 발행 실패"), None);
    let request = confirm_input(&store, &draft);
    let sequence = store
        .catalog_state()
        .unwrap()
        .revision
        .parse::<i64>()
        .unwrap()
        + 1;
    let path = root.path().join(format!("snapshots/{sequence:020}.json"));
    fs::create_dir(&path).unwrap();
    assert!(matches!(
        store.business_confirm(business_id(&draft), decode(request.clone())),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(table_count(root.path(), "sales"), 1);
    assert!(matches!(
        store.business_confirm(business_id(&draft), decode(request.clone())),
        Err(StoreError::Unavailable(_))
    ));
    let new_input = json!({"meta":meta(&store),"payload":payload(&menu,"1","가상 후속"),"corrects_confirmation_id":null});
    assert!(matches!(
        store.business_create(decode(new_input)),
        Err(StoreError::Unavailable(_))
    ));
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let expected: String = db
        .query_row(
            "SELECT payload FROM snapshot_outbox WHERE sequence=?1",
            [sequence],
            |r| r.get(0),
        )
        .unwrap();
    let response: String = db
        .query_row(
            "SELECT response FROM mutation_requests WHERE sequence=?1",
            [sequence],
            |r| r.get(0),
        )
        .unwrap();
    drop(db);
    fs::remove_dir(&path).unwrap();
    let replay = value(
        store
            .business_confirm(business_id(&draft), decode(request.clone()))
            .unwrap(),
    );
    assert_eq!(replay, serde_json::from_str::<Value>(&response).unwrap());
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
    fs::remove_file(&path).unwrap();
    assert_eq!(
        value(
            store
                .business_confirm(business_id(&draft), decode(request))
                .unwrap()
        ),
        replay
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), expected);
    assert_eq!(table_count(root.path(), "sales"), 1);
}

#[test]
fn explicitly_selected_archived_owner_can_be_sold_with_its_current_version() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let mut menu = recipe(&mut store, "가상 보관 메뉴", vec![], Some("1"));
    let id = menu["item"]["id"].as_str().unwrap().to_owned();
    let mut archive = meta(&store);
    archive["expected_version_id"] = menu["item"]["version_id"].clone();
    let archived = store.catalog_archive(&id, decode(archive)).unwrap().value;
    assert!(archived.archived);
    menu["item"]["version_id"] = json!(archived.version_id);
    let result = sell(&mut store, &menu, "1");
    let original = store.sale_detail(sale_id(&result)).unwrap().unwrap().item;
    drop(store);
    let store = Store::open(root.path()).unwrap();
    assert_eq!(
        store.sale_detail(&original.id).unwrap().unwrap().item,
        original
    );
}

#[test]
fn recipe_cost_overflow_remains_a_saved_sale_draft_issue() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = item(&mut store, "가상 원가 한도 재료");
    price(&mut store, &raw, "9000000000000");
    let menu = recipe(
        &mut store,
        "가상 원가 합계 초과",
        vec![line(&raw, "3"), line(&raw, "3")],
        Some("1"),
    );
    let draft = save(&mut store, payload(&menu, "1", "가상 원가 확인 필요"), None);
    let p = preview(&store, &draft);
    assert_eq!(p["can_confirm"], false);
    assert!(
        p["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["code"] == "invalid_cost")
    );
    let state = store.catalog_state().unwrap();
    assert!(matches!(
        store.business_confirm(business_id(&draft), decode(confirm_input(&store, &draft))),
        Err(StoreError::Validation(_))
    ));
    assert_eq!(store.catalog_state().unwrap(), state);
    assert_eq!(table_count(root.path(), "sales"), 0);
    drop(store);
    let store = Store::open(root.path()).unwrap();
    assert_eq!(
        value(
            store
                .business_detail(business_id(&draft))
                .unwrap()
                .unwrap()
                .head
                .payload
        ),
        payload(&menu, "1", "가상 원가 확인 필요")
    );
}

#[test]
fn schema_eight_requests_replay_original_responses_after_new_sale_and_missing_files() {
    let root = TempDir::new().unwrap();
    let originals = install_fixture(root.path(), include_str!("fixtures/schema-v8.sql"));
    let frozen: Value =
        serde_json::from_str(include_str!("fixtures/snapshot-v2-schema8.json")).unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let menu = recipe(&mut store, "가상 새 판매 메뉴", vec![], Some("1"));
    sell(&mut store, &menu, "1");
    let before = store.catalog_state().unwrap();
    for request in frozen["state"]["mutation_requests"].as_array().unwrap() {
        let payload: Value = serde_json::from_str(request["payload"].as_str().unwrap()).unwrap();
        let expected: Value = serde_json::from_str(request["response"].as_str().unwrap()).unwrap();
        let actual = match request["operation"].as_str().unwrap() {
            "material.create" => value(store.catalog_create(decode(payload)).unwrap()),
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

fn change_latest(root: &Path, edit: impl FnOnce(&mut Value)) -> (std::path::PathBuf, String) {
    let db = Connection::open(root.join("app.sqlite3")).unwrap();
    let (seq, payload): (i64, String) = db
        .query_row(
            "SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    let mut snapshot: Value = serde_json::from_str(&payload).unwrap();
    edit(&mut snapshot);
    let changed = serde_json::to_string_pretty(&snapshot).unwrap() + "\n";
    db.execute(
        "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=?2",
        rusqlite::params![changed, seq],
    )
    .unwrap();
    let path = root.join(format!("snapshots/{seq:020}.json"));
    fs::write(&path, &changed).unwrap();
    (path, changed)
}
#[test]
fn frozen_eight_and_failed_migration_preserve_database_and_original_journal_bytes() {
    for mode in [
        "frozen_sale",
        "frozen_unknown_nested",
        "frozen_missing_production",
        "source8_future9",
        "migration_ddl",
    ] {
        let root = TempDir::new().unwrap();
        install_fixture(root.path(), include_str!("fixtures/schema-v8.sql"));
        if mode.starts_with("frozen") {
            drop(Store::open(root.path()).unwrap());
        }
        let (path, changed) = if mode == "migration_ddl" {
            let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
            db.execute_batch("CREATE TABLE sales(synthetic TEXT);")
                .unwrap();
            let path = root.path().join("snapshots/00000000000000000013.json");
            let text = fs::read_to_string(&path).unwrap();
            (path, text)
        } else {
            change_latest(root.path(), |s| match mode {
                "frozen_sale" => {
                    s["state"]["business_revisions"][0]["payload"] = json!({"kind":"sale","version":1,"data":{"material_id":null,"material_version_id":null,"recipe_version_id":null,"count":"","note":""}})
                }
                "frozen_unknown_nested" => {
                    s["state"]["production_inputs"][0]["row"]["future"] = json!(true)
                }
                "frozen_missing_production" => {
                    s["state"].as_object_mut().unwrap().remove("productions");
                }
                "source8_future9" => {
                    s["schema_version"] = json!(9);
                    for key in ["sales", "sale_cost_rows", "sale_versions", "sale_heads"] {
                        s["state"][key] = json!([]);
                    }
                }
                _ => unreachable!(),
            })
        };
        let before = fs::read(root.path().join("app.sqlite3")).unwrap();
        assert!(Store::open(root.path()).is_err(), "{mode}");
        assert_eq!(
            fs::read(root.path().join("app.sqlite3")).unwrap(),
            before,
            "{mode}"
        );
        assert_eq!(fs::read_to_string(path).unwrap(), changed, "{mode}");
    }
}

#[test]
fn schema_nine_rejects_sale_reference_price_arithmetic_chain_and_metadata_tampering() {
    for mode in [
        "missing_original",
        "orphan_confirmation",
        "wrong_recipe",
        "wrong_component",
        "wrong_price_priority",
        "future_holding",
        "wrong_cost",
        "wrong_count",
        "wrong_amount",
        "wrong_head",
        "wrong_previous",
        "future_seq",
        "noncanonical_seq",
        "invalid_uuid",
        "invalid_time",
        "unknown_price_field",
        "unknown_price_source",
        "missing_cost_row",
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let raw = item(&mut store, "가상 무결성 재료");
        price(&mut store, &raw, "10");
        receipt(&mut store, &raw, "1000", "10000");
        let menu = recipe(
            &mut store,
            "가상 무결성 메뉴",
            vec![line(&raw, "100")],
            Some("1000"),
        );
        let original = sell(&mut store, &menu, "3");
        let correction = save(
            &mut store,
            payload(&menu, "1", "가상 정정"),
            Some(confirmation_id(&original)),
        );
        confirm(&mut store, &correction);
        receipt(&mut store, &raw, "1000", "10000");
        drop(store);
        let (path, changed) = change_latest(root.path(), |s| {
            let state = &mut s["state"];
            match mode {
                "missing_original" => state["sales"] = json!([]),
                "orphan_confirmation" => {
                    state["sales"][0]["original_confirmation_id"] =
                        json!(Uuid::new_v4().to_string())
                }
                "wrong_recipe" => {
                    state["sales"][0]["recipe_version_id"] = json!(Uuid::new_v4().to_string())
                }
                "wrong_component" => {
                    state["sale_cost_rows"][0]["row"]["material_version_id"] =
                        menu["item"]["version_id"].clone()
                }
                "wrong_price_priority" => {
                    let price = state["material_prices"][0].clone();
                    state["sale_cost_rows"][0]["row"]["price"] = json!({"source":"preregistered","source_id":price["id"],"amount":price["amount"],"quantity":price["quantity"],"reason":null});
                }
                "future_holding" => {
                    let holding = state["holding_versions"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .max_by_key(|h| h["seq"].as_str().unwrap().parse::<i64>().unwrap())
                        .unwrap()
                        .clone();
                    state["sale_cost_rows"][0]["row"]["price"] = json!({"source":"holding_average","source_id":holding["id"],"amount":holding["value"],"quantity":holding["quantity"],"reason":null});
                }
                "wrong_cost" => state["sale_cost_rows"][0]["row"]["cost"] = json!("333.333334"),
                "wrong_count" => state["sale_versions"][0]["count"] = json!("4"),
                "wrong_amount" => state["sale_versions"][0]["amount"] = json!("4000"),
                "wrong_head" => {
                    state["sale_heads"][0]["current_version_id"] =
                        state["sale_versions"][0]["id"].clone()
                }
                "wrong_previous" => state["sale_versions"][1]["previous_version_id"] = Value::Null,
                "future_seq" => state["sale_versions"][0]["seq"] = json!("999999"),
                "noncanonical_seq" => {
                    state["sales"][0]["seq"] =
                        json!(format!("0{}", state["sales"][0]["seq"].as_str().unwrap()))
                }
                "invalid_uuid" => state["sale_versions"][0]["id"] = json!("invalid-sale-version"),
                "invalid_time" => state["sales"][0]["recorded_at"] = json!("invalid-time"),
                "unknown_price_field" => {
                    state["sale_cost_rows"][0]["row"]["price"]["future"] = json!(true)
                }
                "unknown_price_source" => {
                    state["sale_cost_rows"][0]["row"]["price"]["source"] = json!("future_source")
                }
                "missing_cost_row" => state["sale_cost_rows"] = json!([]),
                _ => unreachable!(),
            }
        });
        let before = fs::read(root.path().join("app.sqlite3")).unwrap();
        assert!(Store::open(root.path()).is_err(), "{mode}");
        assert_eq!(
            fs::read(root.path().join("app.sqlite3")).unwrap(),
            before,
            "{mode}"
        );
        assert_eq!(fs::read_to_string(path).unwrap(), changed, "{mode}");
    }
}

#[test]
fn sale_confirmation_request_must_match_its_original_mutation() {
    let mut accepted = vec![];
    for mode in [
        "unknown_request",
        "other_sequence_request",
        "noncanonical_request",
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let menu = recipe(&mut store, "가상 요청 근거 메뉴", vec![], Some("1"));
        let result = sell(&mut store, &menu, "1");
        let confirmation = confirmation_id(&result).to_owned();
        drop(store);
        let (path, changed) = change_latest(root.path(), |s| {
            let replacement = match mode {
                "unknown_request" => Uuid::new_v4().to_string(),
                "other_sequence_request" => s["state"]["mutation_requests"][0]["request_id"]
                    .as_str()
                    .unwrap()
                    .to_owned(),
                "noncanonical_request" => s["state"]["business_confirmations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|c| c["id"] == confirmation)
                    .unwrap()["request_id"]
                    .as_str()
                    .unwrap()
                    .replace('-', ""),
                _ => unreachable!(),
            };
            s["state"]["business_confirmations"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|c| c["id"] == confirmation)
                .unwrap()["request_id"] = json!(replacement);
        });
        let before = fs::read(root.path().join("app.sqlite3")).unwrap();
        match Store::open(root.path()) {
            Ok(store) => {
                accepted.push(mode);
                drop(store);
            }
            Err(_) => {
                assert_eq!(fs::read(root.path().join("app.sqlite3")).unwrap(), before);
                assert_eq!(fs::read_to_string(path).unwrap(), changed);
            }
        }
    }
    assert!(
        accepted.is_empty(),
        "accepted broken sale request references: {accepted:?}"
    );
}
