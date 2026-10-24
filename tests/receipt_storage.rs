use rusqlite::Connection;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{fs, path::Path};
use sunchang_hansang::{
    business::CreateBusinessDraft,
    catalog::{CreateCatalogMaterial, MaterialSummary},
    store::{Store, StoreError},
};
use tempfile::TempDir;
use uuid::Uuid;

fn decode<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}

fn meta(store: &Store) -> Value {
    let state = store.catalog_state().unwrap();
    json!({"request_id":Uuid::new_v4().to_string(),"epoch":state.epoch,"expected_revision":state.revision})
}

fn item(store: &mut Store) -> MaterialSummary {
    let input = CreateCatalogMaterial {
        meta: decode(meta(store)),
        name: "가상 입고 재료".into(),
        unit: "g".into(),
        kind: "raw".into(),
        description: String::new(),
    };
    store.catalog_create(input).unwrap().value
}

fn direct(item: &MaterialSummary, quantity: &str, amount: &str) -> Value {
    json!({"kind":"receipt","version":1,"data":{
        "source":{"kind":"direct","material_id":item.id,"material_version_id":item.version_id},
        "quantity":quantity,"unit":"kg","amount":amount,"receipt_date":"2026-01-01",
        "note":"","stock_effect":{"kind":"unreflected"}}})
}

#[test]
fn direct_receipt_draft_survives_restart_and_confirmation_establishes_holding_price() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = item(&mut store);
    assert_eq!(item.price.source, "missing");
    let input = decode::<CreateBusinessDraft>(json!({
        "meta":meta(&store),"payload":direct(&item,"1","10000"),
        "corrects_confirmation_id":null
    }));
    let saved = serde_json::to_value(store.business_create(input).unwrap()).unwrap();
    let id = saved["result"]["item"]["id"].as_str().unwrap().to_owned();
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    let draft = serde_json::to_value(store.business_detail(&id).unwrap().unwrap()).unwrap();
    assert_eq!(draft["head"]["payload"]["data"]["quantity"], "1");
    let preview = serde_json::to_value(
        store
            .business_preview(
                &id,
                decode(json!({
                    "expected_head_id":draft["head"]["id"]
                })),
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(preview["can_confirm"], true);
    assert_eq!(preview["effect"]["before"], Value::Null);
    assert_eq!(preview["effect"]["after"]["quantity"], "1000");
    assert_eq!(preview["effect"]["after"]["value"], "10000");
    let result = serde_json::to_value(
        store
            .business_confirm(
                &id,
                decode(json!({
                    "meta":meta(&store),"expected_head_id":draft["head"]["id"]
                })),
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        result["result"]["confirmation"]["result"]["kind"],
        "receipt"
    );
    assert!(result["result"]["confirmation"]["result"]["receipt_id"].is_string());
    let detail = store.catalog_detail(&item.id).unwrap().unwrap();
    assert_eq!(detail.item.price.source, "holding_average");
    assert_eq!(detail.item.price.quantity.as_deref(), Some("1000"));
    assert_eq!(detail.item.price.amount.as_deref(), Some("10000"));
    assert!(detail.prices.is_empty());
    let holding = store.holding_detail(&item.id).unwrap().unwrap();
    assert!(holding.observation.is_none());
    assert_eq!(holding.settlement_basis.unwrap().quantity, "1000");
}

fn save(store: &mut Store, payload: Value) -> Value {
    let input =
        decode(json!({"meta":meta(store),"payload":payload,"corrects_confirmation_id":null}));
    serde_json::to_value(store.business_create(input).unwrap()).unwrap()
}
fn id(saved: &Value) -> &str {
    saved["result"]["item"]["id"].as_str().unwrap()
}
fn confirmation_input(store: &Store, saved: &Value) -> Value {
    json!({"meta":meta(store),"expected_head_id":saved["result"]["head"]["id"]})
}
fn confirm(store: &mut Store, saved: &Value) -> Value {
    let input = decode(confirmation_input(store, saved));
    serde_json::to_value(store.business_confirm(id(saved), input).unwrap()).unwrap()
}
fn receipt_id(result: &Value) -> &str {
    result["result"]["confirmation"]["result"]["receipt_id"]
        .as_str()
        .unwrap()
}
fn count(root: &Path, table: &str) -> i64 {
    Connection::open(root.join("app.sqlite3"))
        .unwrap()
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}
fn order_payload(item: &MaterialSummary) -> Value {
    json!({"kind":"purchase_order","version":1,"data":{"action":"place","material_id":item.id,"material_version_id":item.version_id,"quantity":"10","unit":"kg","supplier":"가상 거래처","expected_date":"2026-01-01","estimated_amount":"999","note":"가상 원주문"}})
}
fn ordered(store: &mut Store, item: &MaterialSummary) -> Value {
    let draft = save(store, order_payload(item));
    let placed = confirm(store, &draft);
    let order_id = placed["result"]["confirmation"]["result"]["order_id"]
        .as_str()
        .unwrap();
    serde_json::to_value(store.order_detail(order_id).unwrap().unwrap()).unwrap()
}
fn linked(item: &MaterialSummary, order: &Value, q: &str, a: &str) -> Value {
    let mut payload = direct(item, q, a);
    payload["data"]["source"] = json!({"kind":"order","order_id":order["item"]["id"],"expected_order_version_id":order["head"]["id"],"material_version_id":item.version_id});
    payload
}

#[test]
fn receipts_preserve_unknown_observation_and_exact_weighted_value_including_zero() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = item(&mut store);
    let unknown = store.holding_detail(&item.id).unwrap().unwrap();
    assert!(unknown.basis.is_none());
    assert!(unknown.settlement_basis.is_none());
    assert_eq!(count(root.path(), "holding_versions"), 0);
    let draft = save(&mut store, direct(&item, "1", "10000"));
    let first = confirm(&mut store, &draft);
    let baseline = store
        .holding_detail(&item.id)
        .unwrap()
        .unwrap()
        .basis
        .unwrap();
    assert_eq!(baseline.id.as_deref(), Some(receipt_id(&first)));
    let draft = save(&mut store, direct(&item, "2", "40000"));
    confirm(&mut store, &draft);
    let held = store.holding_detail(&item.id).unwrap().unwrap();
    assert_eq!(held.basis, Some(baseline.clone()));
    assert!(held.observation.is_none());
    assert_eq!(held.confirmed_changes.len(), 1);
    let amounts = held.settlement_basis.unwrap();
    assert_eq!(amounts.quantity, "3000");
    assert_eq!(amounts.value.as_deref(), Some("50000"));
    assert_eq!(held.price.amount.as_deref(), Some("50000"));
    assert_eq!(held.price.quantity.as_deref(), Some("3000"));
    let draft = save(&mut store, direct(&item, "1", "0"));
    confirm(&mut store, &draft);
    let held = store.holding_detail(&item.id).unwrap().unwrap();
    assert_eq!(held.price.amount.as_deref(), Some("50000"));
    assert_eq!(held.price.quantity.as_deref(), Some("4000"));
    assert_eq!(held.confirmed_changes.len(), 2);
    assert_eq!(count(root.path(), "material_prices"), 0);
    drop(store);
    let store = Store::open(root.path()).unwrap();
    let restored = store.holding_detail(&item.id).unwrap().unwrap();
    assert_eq!(restored.basis, Some(baseline));
    assert_eq!(restored.price, held.price);
    assert_eq!(
        store.receipt_list(Default::default()).unwrap().items.len(),
        3
    );
}

#[test]
fn linked_receipt_closes_real_order_and_keeps_original_current_and_actual_quantities() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = item(&mut store);
    let original = ordered(&mut store, &item);
    let order_id = original["item"]["id"].as_str().unwrap();
    let revised = save(
        &mut store,
        json!({"kind":"purchase_order","version":1,"data":{"action":"revise","order_id":order_id,"expected_order_version_id":original["head"]["id"],"material_version_id":item.version_id,"quantity":"12","unit":"kg","supplier":"가상 거래처","expected_date":"2026-01-02","estimated_amount":"1200","note":"가상 수정 메모"}}),
    );
    confirm(&mut store, &revised);
    let current = serde_json::to_value(store.order_detail(order_id).unwrap().unwrap()).unwrap();
    let saved = save(&mut store, linked(&item, &current, "11.8", "59000"));
    let confirmed = confirm(&mut store, &saved);
    let receipt = store
        .receipt_detail(receipt_id(&confirmed))
        .unwrap()
        .unwrap();
    let closure = receipt.order_closure.unwrap();
    assert_eq!(closure.initial_ordered_quantity, "10000");
    assert_eq!(closure.ordered_quantity, "12000");
    assert_eq!(closure.received_quantity, "11800");
    assert_eq!(closure.difference, "-200");
    let actual = serde_json::to_value(store.order_detail(order_id).unwrap().unwrap()).unwrap();
    assert_eq!(actual["head"]["action"], "receive");
    assert_eq!(actual["head"]["status"], "received");
    assert_eq!(actual["head"]["quantity"], "12000");
    assert_eq!(actual["head"]["note"], "가상 수정 메모");
    assert_eq!(actual["head"]["business_record_id"], id(&saved));
    assert_eq!(actual["versions"].as_array().unwrap().len(), 3);
    assert_eq!(
        store
            .catalog_detail(&item.id)
            .unwrap()
            .unwrap()
            .item
            .price
            .quantity
            .as_deref(),
        Some("11800")
    );
    assert_eq!(count(root.path(), "order_receipt_closures"), 1);
    assert_eq!(count(root.path(), "receipts"), 1);
    drop(store);
    let store = Store::open(root.path()).unwrap();
    assert_eq!(
        store
            .order_list(decode(json!({"status":"received"})))
            .unwrap()
            .items
            .len(),
        1
    );
}

#[test]
fn included_receipt_is_durable_draft_and_never_adds_quantity_or_closes_order() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = item(&mut store);
    let order = ordered(&mut store, &item);
    let mut payload = linked(&item, &order, "1", "1000");
    payload["data"]["stock_effect"] =
        json!({"kind":"needs_reconciliation","basis_id":null,"included_quantity":""});
    let saved = save(&mut store, payload.clone());
    assert!(
        saved["result"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["code"] == "needs_reconciliation")
    );
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    let detail = serde_json::to_value(store.business_detail(id(&saved)).unwrap().unwrap()).unwrap();
    assert_eq!(detail["head"]["payload"], payload);
    let state = store.catalog_state().unwrap();
    let preview = store
        .business_preview(
            id(&saved),
            decode(json!({"expected_head_id":detail["head"]["id"]})),
        )
        .unwrap();
    assert!(!preview.can_confirm);
    assert!(preview.effect.is_none());
    assert!(matches!(
        store.business_confirm(id(&saved), decode(confirmation_input(&store, &saved))),
        Err(StoreError::Validation(_))
    ));
    assert_eq!(store.catalog_state().unwrap(), state);
    assert_eq!(count(root.path(), "receipts"), 0);
    assert_eq!(count(root.path(), "holding_versions"), 0);
    assert_eq!(count(root.path(), "order_receipt_closures"), 0);
    payload["data"]["note"] = json!("가상 확인 메모");
    let edited=serde_json::to_value(store.business_revise(id(&saved),decode(json!({"meta":meta(&store),"expected_head_id":detail["head"]["id"],"payload":payload}))).unwrap()).unwrap();
    assert_eq!(
        edited["result"]["head"]["payload"]["data"]["note"],
        "가상 확인 메모"
    );
}

#[test]
fn stale_order_and_request_retries_keep_one_receipt_and_original_response() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = item(&mut store);
    let order = ordered(&mut store, &item);
    let first = save(&mut store, linked(&item, &order, "1", "1000"));
    let other = save(&mut store, linked(&item, &order, "2", "2000"));
    let request = confirmation_input(&store, &first);
    let response = serde_json::to_value(
        store
            .business_confirm(id(&first), decode(request.clone()))
            .unwrap(),
    )
    .unwrap();
    let state = store.catalog_state().unwrap();
    assert!(matches!(
        store.business_confirm(id(&other), decode(confirmation_input(&store, &other))),
        Err(StoreError::StaleState(_))
    ));
    assert_eq!(store.catalog_state().unwrap(), state);
    assert!(matches!(
        store.business_confirm(id(&other), decode(request.clone())),
        Err(StoreError::Conflict(_))
    ));
    let add = save(&mut store, direct(&item, "1", "0"));
    confirm(&mut store, &add);
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    assert_eq!(
        serde_json::to_value(store.business_confirm(id(&first), decode(request)).unwrap()).unwrap(),
        response
    );
    assert_eq!(count(root.path(), "receipts"), 2);
    assert_eq!(count(root.path(), "order_receipt_closures"), 1);
}

#[test]
fn local_clock_and_invalid_inputs_block_confirmation_without_effects() {
    fn today() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
    }
    let root = TempDir::new().unwrap();
    let mut store = Store::open_with_today(root.path(), today).unwrap();
    let item = item(&mut store);
    for (q, u, a, date) in [
        ("0", "kg", "1", "2026-01-01"),
        ("1", "L", "1", "2026-01-01"),
        ("1", "kg", "", "2026-01-01"),
        ("1", "kg", "0", "2026-01-02"),
        ("1", "kg", "1", "2026-02-29"),
    ] {
        let mut payload = direct(&item, q, a);
        payload["data"]["unit"] = json!(u);
        payload["data"]["receipt_date"] = json!(date);
        let saved = save(&mut store, payload);
        assert!(!saved["result"]["issues"].as_array().unwrap().is_empty());
        assert!(matches!(
            store.business_confirm(id(&saved), decode(confirmation_input(&store, &saved))),
            Err(StoreError::Validation(_))
        ));
    }
    assert_eq!(count(root.path(), "receipts"), 0);
    let saved = save(&mut store, direct(&item, "1", "0"));
    confirm(&mut store, &saved);
    assert_eq!(
        store
            .catalog_detail(&item.id)
            .unwrap()
            .unwrap()
            .item
            .price
            .amount
            .as_deref(),
        Some("0")
    );
}

#[test]
fn receipt_holding_closure_and_journal_failures_roll_back_together() {
    for table in [
        "receipts",
        "holding_versions",
        "order_receipt_closures",
        "snapshot_outbox",
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let item = item(&mut store);
        let order = ordered(&mut store, &item);
        let saved = save(&mut store, linked(&item, &order, "1", "1000"));
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
        assert!(
            matches!(
                store.business_confirm(id(&saved), decode(confirmation_input(&store, &saved))),
                Err(StoreError::Unavailable(_))
            ),
            "{table}"
        );
        assert_eq!(store.catalog_state().unwrap(), state);
        assert_eq!(count(root.path(), "receipts"), 0);
        assert_eq!(count(root.path(), "holding_versions"), 0);
        assert_eq!(count(root.path(), "order_receipt_closures"), 0);
        assert_eq!(count(root.path(), "purchase_order_versions"), 1);
    }
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = item(&mut store);
    let mut payload = direct(&item, "9000000000", "9000000000000");
    payload["data"]["unit"] = json!("g");
    let saved = save(&mut store, payload);
    confirm(&mut store, &saved);
    let bad = save(&mut store, direct(&item, "1", "1"));
    let before = store.catalog_state().unwrap();
    assert!(matches!(
        store.business_confirm(id(&bad), decode(confirmation_input(&store, &bad))),
        Err(StoreError::Validation(_))
    ));
    assert_eq!(store.catalog_state().unwrap(), before);
    assert_eq!(count(root.path(), "receipts"), 1);
}

#[test]
fn publication_failure_recovers_same_receipt_before_new_writes() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = item(&mut store);
    let saved = save(&mut store, direct(&item, "1", "1000"));
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
    assert_eq!(count(root.path(), "receipts"), 1);
    assert!(matches!(
        store.business_confirm(id(&saved), decode(request.clone())),
        Err(StoreError::Unavailable(_))
    ));
    let input = decode(
        json!({"meta":meta(&store),"payload":direct(&item,"2","2"),"corrects_confirmation_id":null}),
    );
    assert!(matches!(
        store.business_create(input),
        Err(StoreError::Unavailable(_))
    ));
    fs::remove_dir(&file).unwrap();
    let response = serde_json::to_value(
        store
            .business_confirm(id(&saved), decode(request.clone()))
            .unwrap(),
    )
    .unwrap();
    let bytes = fs::read(&file).unwrap();
    fs::remove_file(&file).unwrap();
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    assert_eq!(fs::read(file).unwrap(), bytes);
    assert_eq!(
        serde_json::to_value(store.business_confirm(id(&saved), decode(request)).unwrap()).unwrap(),
        response
    );
    assert_eq!(count(root.path(), "holding_versions"), 1);
}

fn legacy_v5(root: &Path) -> Vec<(i64, String)> {
    let mut db = Connection::open(root.join("app.sqlite3")).unwrap();
    let tx = db.transaction().unwrap();
    tx.execute_batch("PRAGMA defer_foreign_keys=ON;").unwrap();
    tx.execute_batch(include_str!("fixtures/schema-v5.sql"))
        .unwrap();
    tx.commit().unwrap();
    fs::create_dir(root.join("snapshots")).unwrap();
    let rows = db
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")
        .unwrap()
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    for (seq, payload) in &rows {
        fs::write(root.join(format!("snapshots/{seq:020}.json")), payload).unwrap();
    }
    rows
}

#[test]
fn schema_five_migration_preserves_all_ids_requests_and_json_bytes() {
    let root = TempDir::new().unwrap();
    let originals = legacy_v5(root.path());
    let frozen: Value =
        serde_json::from_str(include_str!("fixtures/snapshot-v2-schema5.json")).unwrap();
    let mut store = Store::open(root.path()).unwrap();
    for request in frozen["state"]["mutation_requests"].as_array().unwrap() {
        let payload: Value = serde_json::from_str(request["payload"].as_str().unwrap()).unwrap();
        let expected: Value = serde_json::from_str(request["response"].as_str().unwrap()).unwrap();
        let actual = match request["operation"].as_str().unwrap() {
            "material.create" => {
                serde_json::to_value(store.catalog_create(decode(payload)).unwrap()).unwrap()
            }
            "business.create" => {
                serde_json::to_value(store.business_create(decode(payload)).unwrap()).unwrap()
            }
            "business.confirm" => serde_json::to_value(
                store
                    .business_confirm(payload[0].as_str().unwrap(), decode(payload[1].clone()))
                    .unwrap(),
            )
            .unwrap(),
            op => panic!("unexpected {op}"),
        };
        assert_eq!(actual, expected);
    }
    assert_eq!(count(root.path(), "purchase_order_versions"), 3);
    assert_eq!(count(root.path(), "holding_versions"), 0);
    let item = store
        .catalog_list(Default::default())
        .unwrap()
        .pop()
        .unwrap();
    let draft = save(&mut store, direct(&item, "1", "1000"));
    confirm(&mut store, &draft);
    for (seq, bytes) in &originals {
        let file = root.path().join(format!("snapshots/{seq:020}.json"));
        assert_eq!(fs::read_to_string(&file).unwrap(), *bytes);
        fs::remove_file(file).unwrap();
    }
    store.recover().unwrap();
    for (seq, bytes) in originals {
        assert_eq!(
            fs::read_to_string(root.path().join(format!("snapshots/{seq:020}.json"))).unwrap(),
            bytes
        );
    }
}

#[test]
fn corrupt_cross_schema_and_receipt_relations_are_rejected_without_rewriting() {
    for mode in [
        "future_six",
        "old_receipt_kind",
        "old_receive",
        "missing_receipts",
        "missing_closure",
        "bad_closure",
        "bad_holding",
    ] {
        let root = TempDir::new().unwrap();
        if mode.starts_with("future") || mode.starts_with("old") {
            legacy_v5(root.path());
        } else {
            let mut store = Store::open(root.path()).unwrap();
            let item = item(&mut store);
            let order = ordered(&mut store, &item);
            let saved = save(&mut store, linked(&item, &order, "1", "1000"));
            confirm(&mut store, &saved);
        }
        let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
        let (seq, payload): (i64, String) = db
            .query_row(
                "SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let mut value: Value = serde_json::from_str(&payload).unwrap();
        match mode {
            "future_six" => {
                value["schema_version"] = json!(6);
                for key in [
                    "receipts",
                    "holding_versions",
                    "holding_heads",
                    "order_receipt_closures",
                ] {
                    value["state"][key] = json!([]);
                }
            }
            "old_receipt_kind" => value["state"]["business_records"][0]["kind"] = json!("receipt"),
            "old_receive" => {
                value["state"]["purchase_order_versions"][0]["action"] = json!("receive");
                value["state"]["purchase_order_versions"][0]["status"] = json!("received");
            }
            "missing_receipts" => {
                value["state"].as_object_mut().unwrap().remove("receipts");
            }
            "missing_closure" => value["state"]["order_receipt_closures"] = json!([]),
            "bad_closure" => {
                value["state"]["order_receipt_closures"][0]["receipt_id"] =
                    json!(Uuid::new_v4().to_string())
            }
            _ => value["state"]["holding_versions"][0]["value"] = json!("0"),
        }
        let changed = value.to_string();
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
fn database_semantics_reject_valid_fk_that_links_the_wrong_receipt_or_value() {
    for mode in ["closure", "holding"] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let item = item(&mut store);
        let order = ordered(&mut store, &item);
        let saved = save(&mut store, linked(&item, &order, "1", "1000"));
        confirm(&mut store, &saved);
        let saved = save(&mut store, direct(&item, "2", "2000"));
        let direct = confirm(&mut store, &saved);
        drop(store);
        let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        if mode == "closure" {
            db.execute(
                "UPDATE order_receipt_closures SET receipt_id=?1",
                [receipt_id(&direct)],
            )
            .unwrap();
        } else {
            db.execute(
                "UPDATE holding_versions SET value_micros=value_micros+1",
                [],
            )
            .unwrap();
        }
        drop(db);
        let before = fs::read(root.path().join("app.sqlite3")).unwrap();
        assert!(Store::open(root.path()).is_err(), "{mode}");
        assert_eq!(
            fs::read(root.path().join("app.sqlite3")).unwrap(),
            before,
            "{mode}"
        );
    }
}

#[test]
fn snapshot_receipts_require_their_real_confirmation_kind_revision_and_material_version() {
    let mut accepted = vec![];
    for mode in [
        "missing_confirmation",
        "other_confirmation",
        "wrong_record_kind",
        "wrong_revision",
        "wrong_sequence",
        "foreign_material_version",
        "other_result_kind",
        "missing_receipt_result",
        "wrong_confirmed_quantity",
        "malformed_holding_id",
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let material = item(&mut store);
        let foreign = item(&mut store);
        let price = save(
            &mut store,
            json!({"kind":"catalog_price","version":1,"data":{"material_id":material.id,"material_version_id":material.version_id,"quantity":"1","unit":"g","amount":"5","note":""}}),
        );
        let price = confirm(&mut store, &price);
        let saved = save(&mut store, direct(&material, "1", "1000"));
        let receipt = confirm(&mut store, &saved);
        drop(store);
        let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
        let (seq, payload): (i64, String) = db
            .query_row(
                "SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let mut value: Value = serde_json::from_str(&payload).unwrap();
        let record = id(&saved);
        let confirmation = receipt["result"]["confirmation"]["id"].as_str().unwrap();
        match mode {
            "missing_confirmation" => {
                value["state"]["receipts"][0]["confirmation_id"] = json!(Uuid::new_v4().to_string())
            }
            "other_confirmation" => {
                value["state"]["receipts"][0]["confirmation_id"] =
                    price["result"]["confirmation"]["id"].clone()
            }
            "wrong_record_kind" => {
                let row = value["state"]["business_records"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["id"] == record)
                    .unwrap();
                row["kind"] = json!("catalog_price");
            }
            "wrong_revision" => {
                let row = value["state"]["business_confirmations"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["id"] == confirmation)
                    .unwrap();
                row["revision_id"] = price["result"]["confirmation"]["revision_id"].clone();
            }
            "wrong_sequence" => {
                let sequence = price["result"]["confirmation"]["seq"].clone();
                value["state"]["receipts"][0]["seq"] = sequence.clone();
                value["state"]["holding_versions"][0]["seq"] = sequence.clone();
                value["state"]["holding_versions"][0]["baseline_seq"] = sequence;
            }
            "foreign_material_version" => {
                value["state"]["receipts"][0]["material_version_id"] = json!(foreign.version_id);
                value["state"]["holding_versions"][0]["material_version_id"] =
                    json!(foreign.version_id);
            }
            "other_result_kind" => {
                value["state"]["business_catalog_price_results"][0]["confirmation_id"] =
                    json!(confirmation);
            }
            "missing_receipt_result" => {
                for name in ["receipts", "holding_versions", "holding_heads"] {
                    value["state"][name] = json!([]);
                }
            }
            "wrong_confirmed_quantity" => {
                let row = value["state"]["business_revisions"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["id"] == receipt["result"]["confirmation"]["revision_id"])
                    .unwrap();
                row["payload"]["data"]["quantity"] = json!("2");
            }
            "malformed_holding_id" => {
                value["state"]["holding_versions"][0]["id"] = json!("not-a-uuid");
                value["state"]["holding_heads"][0]["current_version_id"] = json!("not-a-uuid");
                value["state"]["holding_heads"][0]["last_valid_version_id"] = json!("not-a-uuid");
            }
            _ => unreachable!(),
        }
        let changed = value.to_string();
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
        "accepted corrupt external references: {accepted:?}"
    );
}
