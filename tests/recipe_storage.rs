use rusqlite::{Connection, types::Value as SqlValue};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};
use sunchang_hansang::{
    catalog::MaterialSummary,
    recipe::RecipeMutation,
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
    let request = json!({"request_id":Uuid::new_v4().to_string(),"epoch":store.catalog_state().unwrap().epoch,"expected_revision":store.catalog_state().unwrap().revision,"name":name,"unit":"g","kind":"raw","description":"가상 검증 자료"});
    store.catalog_create(decode(request)).unwrap().value
}
fn line(item: &MaterialSummary, quantity: &str) -> Value {
    json!({"material_id":item.id,"material_version_id":item.version_id,"quantity":quantity,"unit":"g","note":"적당량"})
}
fn data(lines: Vec<Value>) -> Value {
    json!({"basis_count":"1","reference_output":null,"selling_price":null,"cooking_description":"가상 조리 설명","process":"섞기\n가열","reference_time":"20분, 식힘 별도","lines":lines})
}
fn new_request(store: &Store, name: &str, data: Value) -> Value {
    json!({"meta":meta(store),"owner":{"mode":"new","name":name,"unit":"개","kind":"menu","description":"가상 메뉴"},"data":data})
}
fn create(store: &mut Store, name: &str, data: Value) -> RecipeMutation {
    let input = new_request(store, name, data);
    store.recipe_create(decode(input)).unwrap()
}
fn attach_request(store: &Store, owner: &MaterialSummary, data: Value) -> Value {
    json!({"meta":meta(store),"owner":{"mode":"existing","material_id":owner.id,"expected_material_version_id":owner.version_id},"data":data})
}
fn revise_request(store: &Store, old: &RecipeMutation, data: Value) -> Value {
    json!({"meta":meta(store),"expected_material_version_id":old.item.version_id,"expected_recipe_version_id":old.version.id,"data":data})
}
fn revise_item(
    store: &mut Store,
    item: &MaterialSummary,
    name: &str,
    description: &str,
) -> MaterialSummary {
    let request = catalog_request(
        store,
        json!({"expected_version_id":item.version_id,"name":name,"unit":item.unit,"kind":item.kind,"description":description}),
    );
    store
        .catalog_revise(&item.id, decode(request))
        .unwrap()
        .value
}
fn price(store: &mut Store, item: &MaterialSummary, quantity: &str, amount: &str) {
    let request = catalog_request(
        store,
        json!({"expected_version_id":item.version_id,"quantity":quantity,"unit":"g","amount":amount}),
    );
    store.catalog_add_price(&item.id, decode(request)).unwrap();
}
fn catalog_request(store: &Store, fields: Value) -> Value {
    let mut request = meta(store);
    request
        .as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    request
}
fn count(root: &Path, table: &str) -> i64 {
    Connection::open(root.join("app.sqlite3"))
        .unwrap()
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

fn legacy_v6(root: &Path) -> Vec<(i64, String)> {
    let mut database = Connection::open(root.join("app.sqlite3")).unwrap();
    let tx = database.transaction().unwrap();
    tx.execute_batch("PRAGMA defer_foreign_keys=ON;").unwrap();
    tx.execute_batch(include_str!("fixtures/schema-v6.sql"))
        .unwrap();
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

fn table_rows(database: &Connection, table: &str) -> Vec<Vec<SqlValue>> {
    let mut statement = database
        .prepare(&format!("SELECT * FROM \"{table}\" ORDER BY rowid"))
        .unwrap();
    let columns = statement.column_count();
    statement
        .query_map([], |row| {
            (0..columns).map(|column| row.get(column)).collect()
        })
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}

#[test]
fn schema_six_open_adds_empty_recipe_relations_without_rewriting_retained_data() {
    let root = TempDir::new().unwrap();
    let journals = legacy_v6(root.path());
    assert_eq!(journals.len(), 18);
    assert_eq!(
        journals.last().unwrap().1,
        include_str!("fixtures/snapshot-v2-schema6.json")
    );
    let frozen: Value = serde_json::from_str(&journals.last().unwrap().1).unwrap();
    assert_eq!(frozen["schema_version"], 6);
    assert_eq!(frozen["format_version"], 2);
    assert!(frozen["state"].get("recipe_versions").is_none());
    assert_eq!(frozen["state"]["receipts"].as_array().unwrap().len(), 3);
    assert_eq!(
        frozen["state"]["order_receipt_closures"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
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
            let rows = table_rows(&database, &name);
            (name, rows)
        })
        .collect();
    let old_schema: i64 = database
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(old_schema, 6);
    drop(database);

    let store = Store::open(root.path()).unwrap();
    let database = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let new_schema: i64 = database
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        new_schema, 8,
        "opening schema6 must install current storage"
    );
    for table in [
        "recipe_versions",
        "recipe_lines",
        "recipe_heads",
        "productions",
        "production_inputs",
        "production_outputs",
        "production_effects",
    ] {
        assert!(table_rows(&database, table).is_empty(), "{table}");
    }
    for (table, mut rows) in before {
        if table == "holding_versions" {
            // Schema8 appends two production references; every pre-existing
            // column remains identical and both new references remain absent.
            for row in &mut rows {
                row.extend([SqlValue::Null, SqlValue::Null]);
            }
        }
        assert_eq!(table_rows(&database, &table), rows, "{table}");
    }
    for (seq, payload) in journals {
        assert_eq!(
            fs::read_to_string(root.path().join(format!("snapshots/{seq:020}.json"))).unwrap(),
            payload
        );
    }
    assert_eq!(store.catalog_state().unwrap().revision, "18");
}

#[test]
fn exact_batch_zero_rows_and_current_cost_survive_restart_without_inventory_effects() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let ingredient = item(&mut store, "가상 곡물");
    price(&mut store, &ingredient, "1", "10");
    let mut content = data(vec![line(&ingredient, "100.00"), line(&ingredient, "0")]);
    content["basis_count"] = json!("3");
    content["selling_price"] = json!("1000");
    let saved = create(&mut store, "가상 정밀 메뉴", content.clone());
    assert_eq!(saved.version.lines[0].quantity, "100");
    assert_eq!(saved.version.lines[0].input_quantity, "100.00");
    assert_eq!(saved.version.lines[1].note, "적당량");
    assert_ne!(saved.version.lines[0].id, saved.version.lines[1].id);
    let version = value(&saved.version);
    let state = store.catalog_state().unwrap();
    let cost = store
        .recipe_cost(&saved.item.id, Some(&saved.version.id))
        .unwrap()
        .unwrap();
    assert_eq!(cost.total_cost.as_deref(), Some("333.333333"));
    assert_eq!(cost.rows[1].cost.as_deref(), Some("0"));
    assert_eq!(cost.rows[0].price.source, "preregistered");
    assert_eq!(store.catalog_state().unwrap(), state);
    assert_eq!(count(root.path(), "holding_versions"), 0);
    assert_eq!(
        store
            .recipe_uses(&ingredient.id)
            .unwrap()
            .unwrap()
            .items
            .len(),
        1
    );
    price(&mut store, &ingredient, "1", "20");
    assert_eq!(
        store
            .recipe_cost(&saved.item.id, Some(&saved.version.id))
            .unwrap()
            .unwrap()
            .total_cost
            .as_deref(),
        Some("666.666667")
    );
    content["basis_count"] = json!("6");
    content["lines"][0]["quantity"] = json!("250");
    let request = revise_request(&store, &saved, content);
    let revised = store
        .recipe_revise(&saved.item.id, decode(request))
        .unwrap();
    assert_eq!(
        revised.version.previous_version_id.as_deref(),
        Some(saved.version.id.as_str())
    );
    assert_eq!(
        value(
            store
                .recipe_version(&saved.item.id, &saved.version.id)
                .unwrap()
                .unwrap()
        ),
        version
    );
    drop(store);
    let store = Store::open(root.path()).unwrap();
    assert_eq!(
        store
            .recipe_detail(&saved.item.id)
            .unwrap()
            .unwrap()
            .head
            .id,
        revised.version.id
    );
    assert_eq!(
        value(
            store
                .recipe_version(&saved.item.id, &saved.version.id)
                .unwrap()
                .unwrap()
        ),
        version
    );
    assert_eq!(count(root.path(), "receipts"), 0);
    let list = store
        .recipe_list(decode(json!({"query":"정밀","archive":"active"})))
        .unwrap();
    assert_eq!(list.items.len(), 1);
}

#[test]
fn past_component_versions_and_reference_output_remain_explicit() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let ingredient = item(&mut store, "가상 이전 재료 이름");
    let owner = item(&mut store, "가상 비축 기준");
    let newer = revise_item(&mut store, &ingredient, "가상 현재 재료 이름", "변경 설명");
    let mut content = data(vec![line(&ingredient, "0")]);
    content["reference_output"] = json!({"quantity":"2.5","unit":"kg"});
    let request = attach_request(&store, &owner, content.clone());
    let saved = store.recipe_create(decode(request.clone())).unwrap();
    assert_eq!(
        saved.version.lines[0].material_version_id,
        ingredient.version_id
    );
    assert_eq!(saved.version.lines[0].material_name, "가상 이전 재료 이름");
    assert_eq!(
        saved.version.reference_output.as_ref().unwrap().quantity,
        "2500"
    );
    let cost = store.recipe_cost(&owner.id, None).unwrap().unwrap();
    assert_eq!(cost.total_cost.as_deref(), Some("0"));
    assert_eq!(cost.missing_price_line_ids.len(), 1);
    let mut duplicate = request;
    duplicate["meta"] = meta(&store);
    assert!(matches!(
        store.recipe_create(decode(duplicate)),
        Err(StoreError::StaleState(_))
    ));
    let state = store.catalog_state().unwrap();
    let mut bad = content.clone();
    bad["lines"][0]["material_id"] = json!(owner.id);
    let request = revise_request(&store, &saved, bad);
    assert!(matches!(
        store.recipe_revise(&owner.id, decode(request)),
        Err(StoreError::Validation(_))
    ));
    let mut bad = content.clone();
    bad["basis_count"] = json!("3");
    let request = revise_request(&store, &saved, bad);
    assert!(matches!(
        store.recipe_revise(&owner.id, decode(request)),
        Err(StoreError::Validation(_))
    ));
    assert_eq!(store.catalog_state().unwrap(), state);
    content["lines"][0]["material_version_id"] = json!(newer.version_id);
    let request = revise_request(&store, &saved, content);
    let revised = store.recipe_revise(&owner.id, decode(request)).unwrap();
    assert_eq!(
        revised.version.lines[0].material_name,
        "가상 현재 재료 이름"
    );
    assert_eq!(count(root.path(), "holding_versions"), 0);
}

#[test]
fn current_graph_rejects_zero_quantity_cycles_but_not_historical_unions() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let a = item(&mut store, "가상 순환 A");
    let b = item(&mut store, "가상 순환 B");
    let c = item(&mut store, "가상 순환 C");
    let request = attach_request(&store, &a, data(vec![line(&b, "0")]));
    let av = store.recipe_create(decode(request)).unwrap();
    let request = attach_request(&store, &b, data(vec![line(&c, "0"), line(&c, "1")]));
    store.recipe_create(decode(request)).unwrap();
    let state = store.catalog_state().unwrap();
    for target in [&a, &c] {
        let request = attach_request(&store, &c, data(vec![line(target, "0")]));
        assert!(matches!(
            store.recipe_create(decode(request)),
            Err(StoreError::Validation(_))
        ));
        assert_eq!(store.catalog_state().unwrap(), state);
    }
    let request = revise_request(&store, &av, data(vec![]));
    store.recipe_revise(&a.id, decode(request)).unwrap();
    let request = attach_request(&store, &c, data(vec![line(&a, "0")]));
    store.recipe_create(decode(request)).unwrap();
    assert_eq!(store.recipe_uses(&c.id).unwrap().unwrap().items.len(), 1);
    assert!(store.recipe_uses(&b.id).unwrap().unwrap().items.is_empty());
    drop(store);
    assert!(Store::open(root.path()).is_ok());
}

#[test]
fn copies_use_current_source_and_independent_ids_with_scalar_safe_names_and_retries() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let ingredient = item(&mut store, "가상 복제 구성");
    let name = format!("{}{}", "가".repeat(96), "😀".repeat(4));
    let original = create(&mut store, &name, data(vec![line(&ingredient, "0")]));
    item(&mut store, &format!("{} (2)", "가".repeat(96)));
    let archived = item(&mut store, &format!("{} (3)", "가".repeat(96)));
    let archive = catalog_request(&store, json!({"expected_version_id":archived.version_id}));
    store
        .catalog_archive(&archived.id, decode(archive))
        .unwrap();
    let request = json!({"meta":meta(&store),"expected_material_version_id":original.item.version_id,"source_recipe_version_id":original.version.id});
    let copied = store
        .recipe_copy(&original.item.id, decode(request.clone()))
        .unwrap();
    let original_response = value(&copied);
    assert_eq!(copied.item.name, format!("{} (4)", "가".repeat(96)));
    assert_eq!(copied.item.name.chars().count(), 100);
    assert_ne!(copied.item.id, original.item.id);
    assert_ne!(copied.version.lines[0].id, original.version.lines[0].id);
    assert_eq!(
        copied.version.copied_from_version_id.as_deref(),
        Some(original.version.id.as_str())
    );
    let copied_edit = revise_request(&store, &copied, data(vec![line(&ingredient, "7")]));
    store
        .recipe_revise(&copied.item.id, decode(copied_edit))
        .unwrap();
    assert_eq!(
        store
            .recipe_detail(&original.item.id)
            .unwrap()
            .unwrap()
            .head
            .lines[0]
            .quantity,
        "0"
    );
    let original_edit = revise_request(&store, &original, data(vec![line(&ingredient, "8")]));
    store
        .recipe_revise(&original.item.id, decode(original_edit))
        .unwrap();
    assert_eq!(
        value(
            store
                .recipe_copy(&original.item.id, decode(request.clone()))
                .unwrap()
        ),
        original_response
    );
    assert!(matches!(
        store.recipe_copy(&copied.item.id, decode(request.clone())),
        Err(StoreError::Conflict(_))
    ));
    let mut stale = request;
    stale["meta"] = meta(&store);
    assert!(matches!(
        store.recipe_copy(&original.item.id, decode(stale)),
        Err(StoreError::StaleState(_))
    ));
    drop(store);
    assert!(Store::open(root.path()).is_ok());
}

#[test]
fn recipe_and_new_owner_roll_back_at_every_write_boundary() {
    for table in [
        "materials",
        "material_versions",
        "recipe_versions",
        "recipe_lines",
        "recipe_heads",
        "mutation_requests",
        "snapshot_outbox",
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let ingredient = item(&mut store, "가상 원자 구성");
        let state = store.catalog_state().unwrap();
        let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
        let trigger = if table == "snapshot_outbox" {
            "CREATE TRIGGER injected BEFORE UPDATE OF payload ON snapshot_outbox BEGIN SELECT RAISE(FAIL,'injected'); END;".into()
        } else {
            format!(
                "CREATE TRIGGER injected BEFORE INSERT ON {table} BEGIN SELECT RAISE(FAIL,'injected'); END;"
            )
        };
        db.execute_batch(&trigger).unwrap();
        let request = new_request(&store, "가상 원자 메뉴", data(vec![line(&ingredient, "0")]));
        assert!(
            matches!(
                store.recipe_create(decode(request)),
                Err(StoreError::Unavailable(_))
            ),
            "{table}"
        );
        assert_eq!(store.catalog_state().unwrap(), state, "{table}");
        assert_eq!(count(root.path(), "materials"), 1, "{table}");
        for empty in ["recipe_versions", "recipe_lines", "recipe_heads"] {
            assert_eq!(count(root.path(), empty), 0, "{table}/{empty}");
        }
        assert_eq!(count(root.path(), "snapshot_outbox"), 1);
    }
}

#[test]
fn publication_recovery_precedes_same_request_response_and_new_recipe_writes() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let request = new_request(&store, "가상 발행 복구", data(vec![]));
    let file = root.path().join("snapshots/00000000000000000001.json");
    fs::create_dir(&file).unwrap();
    assert!(matches!(
        store.recipe_create(decode(request.clone())),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(count(root.path(), "recipe_versions"), 1);
    assert!(matches!(
        store.recipe_create(decode(request.clone())),
        Err(StoreError::Unavailable(_))
    ));
    let other = new_request(&store, "가상 다음 요청", data(vec![]));
    assert!(matches!(
        store.recipe_create(decode(other)),
        Err(StoreError::Unavailable(_))
    ));
    fs::remove_dir(&file).unwrap();
    let response = store.recipe_create(decode(request.clone())).unwrap();
    let bytes = fs::read(&file).unwrap();
    fs::remove_file(&file).unwrap();
    let renamed = revise_item(&mut store, &response.item, "가상 바뀐 이름", "수정");
    assert_eq!(renamed.id, response.item.id);
    assert_eq!(fs::read(&file).unwrap(), bytes);
    assert_eq!(
        value(store.recipe_create(decode(request)).unwrap()),
        value(response)
    );
    assert_eq!(count(root.path(), "recipe_versions"), 1);
}

#[test]
fn missing_and_overflow_cost_do_not_block_composition_and_direct_stock_is_not_expanded() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = item(&mut store, "가상 원가 원물");
    price(&mut store, &raw, "1", "10");
    let stock = item(&mut store, "가상 원가 비축");
    let request = attach_request(&store, &stock, data(vec![line(&raw, "10")]));
    store.recipe_create(decode(request)).unwrap();
    let menu = create(&mut store, "가상 직접 원가", data(vec![line(&stock, "1")]));
    assert!(
        store
            .recipe_cost(&menu.item.id, None)
            .unwrap()
            .unwrap()
            .total_cost
            .is_none()
    );
    price(&mut store, &stock, "1", "0");
    assert_eq!(
        store
            .recipe_cost(&menu.item.id, None)
            .unwrap()
            .unwrap()
            .total_cost
            .as_deref(),
        Some("0")
    );
    price(&mut store, &stock, "0.000001", "9000000000000");
    assert!(matches!(
        store.recipe_cost(&menu.item.id, None),
        Err(StoreError::Validation(_))
    ));
    assert_eq!(
        store.recipe_detail(&menu.item.id).unwrap().unwrap().head.id,
        menu.version.id
    );
    let edited = revise_request(&store, &menu, data(vec![line(&stock, "0")]));
    store.recipe_revise(&menu.item.id, decode(edited)).unwrap();
    assert_eq!(
        store
            .recipe_cost(&menu.item.id, None)
            .unwrap()
            .unwrap()
            .total_cost
            .as_deref(),
        Some("0")
    );
}

#[test]
fn schema_six_requests_replay_exactly_and_holding_prices_feed_new_recipe_snapshots() {
    let root = TempDir::new().unwrap();
    let originals = legacy_v6(root.path());
    let frozen: Value =
        serde_json::from_str(include_str!("fixtures/snapshot-v2-schema6.json")).unwrap();
    let mut store = Store::open(root.path()).unwrap();
    for request in frozen["state"]["mutation_requests"].as_array().unwrap() {
        let payload: Value = serde_json::from_str(request["payload"].as_str().unwrap()).unwrap();
        let expected: Value = serde_json::from_str(request["response"].as_str().unwrap()).unwrap();
        let actual = match request["operation"].as_str().unwrap() {
            "material.create" => value(store.catalog_create(decode(payload)).unwrap()),
            "business.create" => value(store.business_create(decode(payload)).unwrap()),
            "business.confirm" => value(
                store
                    .business_confirm(payload[0].as_str().unwrap(), decode(payload[1].clone()))
                    .unwrap(),
            ),
            op => panic!("unexpected frozen operation {op}"),
        };
        assert_eq!(actual, expected);
    }
    assert_eq!(store.catalog_state().unwrap().revision, "18");
    let ingredient = store
        .catalog_list(Default::default())
        .unwrap()
        .into_iter()
        .find(|i| i.unit == "g")
        .unwrap();
    let mut content = data(vec![line(&ingredient, "100")]);
    content["basis_count"] = json!("3");
    let saved = create(&mut store, "가상 이전 보유 원가", content);
    let cost = store.recipe_cost(&saved.item.id, None).unwrap().unwrap();
    assert_eq!(cost.rows[0].price.source, "holding_average");
    assert_eq!(cost.total_cost.as_deref(), Some("341.666667"));
    let newest: Value = serde_json::from_slice(
        &fs::read(root.path().join("snapshots/00000000000000000019.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(newest["format_version"], 2);
    assert_eq!(newest["schema_version"], 8);
    for key in ["recipe_versions", "recipe_lines", "recipe_heads"] {
        assert_eq!(newest["state"][key].as_array().unwrap().len(), 1);
    }
    for (seq, payload) in originals {
        assert_eq!(
            fs::read_to_string(root.path().join(format!("snapshots/{seq:020}.json"))).unwrap(),
            payload
        );
    }
    assert_eq!(count(root.path(), "receipts"), 3);
    assert_eq!(count(root.path(), "holding_versions"), 3);
    drop(store);
    assert!(Store::open(root.path()).is_ok());
}

#[test]
fn frozen_six_and_migration_failures_leave_database_and_journals_unchanged() {
    for mode in [
        "future_seven",
        "old_new_collection",
        "old_new_receipt_source",
        "old_nested_extra",
        "old_orphan_confirmation",
        "ddl_failure",
    ] {
        let root = TempDir::new().unwrap();
        legacy_v6(root.path());
        let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
        let (seq, payload): (i64, String) = db
            .query_row(
                "SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let mut snapshot: Value = serde_json::from_str(&payload).unwrap();
        match mode {
            "future_seven" => {
                snapshot["schema_version"] = json!(7);
                for key in ["recipe_versions", "recipe_lines", "recipe_heads"] {
                    snapshot["state"][key] = json!([]);
                }
            }
            "old_new_collection" => snapshot["state"]["recipe_heads"] = json!([]),
            "old_new_receipt_source" => {
                snapshot["state"]["receipts"][0]["source"] = json!({"kind":"future_source"})
            }
            "old_nested_extra" => {
                let revision = snapshot["state"]["business_revisions"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| {
                        r["payload"]["kind"] == "receipt"
                            && r["payload"]["data"]["stock_effect"]["kind"] == "unreflected"
                    })
                    .unwrap();
                revision["payload"]["data"]["stock_effect"]["extra"] = json!(true);
            }
            "old_orphan_confirmation" => {
                snapshot["state"]["receipts"][0]["confirmation_id"] =
                    json!(Uuid::new_v4().to_string())
            }
            "ddl_failure" => {
                db.execute_batch("CREATE TABLE recipe_lines (keep TEXT); INSERT INTO recipe_lines VALUES ('keep');").unwrap();
            }
            _ => unreachable!(),
        }
        let changed = if mode == "ddl_failure" {
            payload
        } else {
            snapshot.to_string()
        };
        db.execute(
            "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=?2",
            rusqlite::params![changed, seq],
        )
        .unwrap();
        drop(db);
        let file = root.path().join(format!("snapshots/{seq:020}.json"));
        fs::write(&file, &changed).unwrap();
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
fn recipe_snapshot_rejects_broken_graph_history_copy_and_future_material_references() {
    for mode in [
        "missing_lines",
        "line_position",
        "line_quantity",
        "line_id",
        "owner_version",
        "component_version",
        "old_copy_source",
        "other_copy_source",
        "head",
        "cycle",
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let ingredient = item(&mut store, "가상 불변 구성");
        let owner = item(&mut store, "가상 불변 소유");
        let request = attach_request(&store, &owner, data(vec![line(&ingredient, "0")]));
        let first = store.recipe_create(decode(request)).unwrap();
        let other = create(
            &mut store,
            "가상 다른 원본",
            data(vec![line(&ingredient, "2")]),
        );
        let request = revise_request(&store, &first, data(vec![line(&ingredient, "0")]));
        let second = store.recipe_revise(&owner.id, decode(request)).unwrap();
        let request = json!({"meta":meta(&store),"expected_material_version_id":owner.version_id,"source_recipe_version_id":second.version.id});
        let copied = store.recipe_copy(&owner.id, decode(request)).unwrap();
        let future_owner = revise_item(&mut store, &owner, &owner.name, "나중 소유 설명");
        let future_component =
            revise_item(&mut store, &ingredient, &ingredient.name, "나중 구성 설명");
        let mut request = revise_request(&store, &second, data(vec![line(&ingredient, "0")]));
        request["expected_material_version_id"] = json!(future_owner.version_id);
        let third = store.recipe_revise(&owner.id, decode(request)).unwrap();
        drop(store);
        let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
        let (seq, payload): (i64, String) = db
            .query_row(
                "SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let mut snapshot: Value = serde_json::from_str(&payload).unwrap();
        match mode {
            "missing_lines" => {
                snapshot["state"]
                    .as_object_mut()
                    .unwrap()
                    .remove("recipe_lines");
            }
            "line_position" | "line_quantity" | "line_id" | "component_version" => {
                let row = snapshot["state"]["recipe_lines"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["recipe_version_id"] == first.version.id)
                    .unwrap();
                match mode {
                    "line_position" => row["line"]["position"] = json!("1"),
                    "line_quantity" => row["line"]["quantity"] = json!("1"),
                    "line_id" => row["line"]["id"] = json!("not-uuid"),
                    _ => row["line"]["material_version_id"] = json!(future_component.version_id),
                }
            }
            "owner_version" => {
                let row = snapshot["state"]["recipe_versions"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["id"] == first.version.id)
                    .unwrap();
                row["material_version_id"] = json!(future_owner.version_id);
            }
            "old_copy_source" | "other_copy_source" => {
                let row = snapshot["state"]["recipe_versions"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["id"] == copied.version.id)
                    .unwrap();
                row["copied_from_version_id"] = if mode == "old_copy_source" {
                    json!(first.version.id)
                } else {
                    json!(other.version.id)
                };
            }
            "head" => {
                let row = snapshot["state"]["recipe_heads"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["material_id"] == owner.id)
                    .unwrap();
                row["current_version_id"] = json!(first.version.id);
            }
            "cycle" => {
                let row = snapshot["state"]["recipe_lines"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["recipe_version_id"] == third.version.id)
                    .unwrap();
                row["line"]["material_id"] = json!(future_owner.id);
                row["line"]["material_version_id"] = json!(future_owner.version_id);
                row["line"]["material_name"] = json!(future_owner.name);
            }
            _ => unreachable!(),
        }
        let changed = snapshot.to_string();
        db.execute(
            "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=?2",
            rusqlite::params![changed, seq],
        )
        .unwrap();
        drop(db);
        let file = root.path().join(format!("snapshots/{seq:020}.json"));
        fs::write(&file, &changed).unwrap();
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
fn database_recipe_semantics_reject_valid_foreign_keys_with_wrong_values() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let ingredient = item(&mut store, "가상 DB 구성");
    create(
        &mut store,
        "가상 DB 레시피",
        data(vec![line(&ingredient, "1")]),
    );
    drop(store);
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    db.execute(
        "UPDATE recipe_lines SET quantity_micros=quantity_micros+1",
        [],
    )
    .unwrap();
    drop(db);
    let before = fs::read(root.path().join("app.sqlite3")).unwrap();
    assert!(Store::open(root.path()).is_err());
    assert_eq!(fs::read(root.path().join("app.sqlite3")).unwrap(), before);
}

#[test]
fn recipe_journal_sequence_and_command_references_cannot_be_fabricated() {
    let mut accepted = vec![];
    for mode in [
        "future_sequence",
        "wrong_operation",
        "missing_request",
        "duplicate_request",
        "missing_material",
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let saved = create(&mut store, "가상 변경 근거", data(vec![]));
        drop(store);
        let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
        let (seq, payload): (i64, String) = db
            .query_row(
                "SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let mut snapshot: Value = serde_json::from_str(&payload).unwrap();
        match mode {
            "future_sequence" => snapshot["state"]["recipe_versions"][0]["seq"] = json!("999999"),
            "wrong_operation" => {
                snapshot["state"]["mutation_requests"][0]["operation"] = json!("material.create")
            }
            "missing_request" => snapshot["state"]["mutation_requests"] = json!([]),
            "duplicate_request" => {
                let mut request = snapshot["state"]["mutation_requests"][0].clone();
                request["request_id"] = json!(Uuid::new_v4().to_string());
                snapshot["state"]["mutation_requests"]
                    .as_array_mut()
                    .unwrap()
                    .push(request);
            }
            "missing_material" => snapshot["state"]["materials"]
                .as_array_mut()
                .unwrap()
                .retain(|m| m["id"] != saved.item.id),
            _ => unreachable!(),
        }
        let changed = snapshot.to_string();
        db.execute(
            "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=?2",
            rusqlite::params![changed, seq],
        )
        .unwrap();
        drop(db);
        let file = root.path().join(format!("snapshots/{seq:020}.json"));
        fs::write(&file, &changed).unwrap();
        let before = fs::read(root.path().join("app.sqlite3")).unwrap();
        if Store::open(root.path()).is_ok() {
            accepted.push(mode);
        }
        assert_eq!(
            fs::read(root.path().join("app.sqlite3")).unwrap(),
            before,
            "{mode}"
        );
        assert_eq!(fs::read_to_string(file).unwrap(), changed, "{mode}");
    }
    assert!(
        accepted.is_empty(),
        "accepted fabricated journal references: {accepted:?}"
    );
}
