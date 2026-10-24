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
        name: "가상 주문 재료".into(),
        unit: "g".into(),
        kind: "raw".into(),
        description: String::new(),
    };
    store.catalog_create(input).unwrap().value
}

fn place(item: &MaterialSummary, quantity: &str) -> Value {
    json!({"kind":"purchase_order","version":1,"data":{
        "action":"place","material_id":item.id,"material_version_id":item.version_id,
        "quantity":quantity,"unit":"kg","supplier":"","expected_date":"",
        "estimated_amount":"","note":""}})
}

#[test]
fn order_draft_survives_restart_then_confirmation_creates_a_real_order() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = item(&mut store);
    let input = decode::<CreateBusinessDraft>(json!({"meta":meta(&store),
        "payload":place(&item,""),"corrects_confirmation_id":null}));
    let saved = serde_json::to_value(store.business_create(input).unwrap()).unwrap();
    let id = saved["result"]["item"]["id"].as_str().unwrap().to_owned();
    assert!(!saved["result"]["issues"].as_array().unwrap().is_empty());
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    let detail = serde_json::to_value(store.business_detail(&id).unwrap().unwrap()).unwrap();
    assert_eq!(detail["head"]["payload"]["data"]["quantity"], "");
    let revised = serde_json::to_value(store.business_revise(&id, decode(json!({
        "meta":meta(&store),"expected_head_id":detail["head"]["id"],"payload":place(&item,"10")
    }))).unwrap()).unwrap();
    let preview = serde_json::to_value(
        store
            .business_preview(
                &id,
                decode(json!({
                    "expected_head_id":revised["result"]["head"]["id"]
                })),
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(preview["can_confirm"], true);
    assert_eq!(preview["effect"]["after"]["quantity"], "10000");
    assert_eq!(preview["effect"]["after"]["estimated_amount"], Value::Null);
    let confirmed = serde_json::to_value(
        store
            .business_confirm(
                &id,
                decode(json!({
                    "meta":meta(&store),"expected_head_id":revised["result"]["head"]["id"]
                })),
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        confirmed["result"]["confirmation"]["result"]["kind"],
        "purchase_order"
    );
    assert_eq!(
        confirmed["result"]["confirmation"]["result"]["action"],
        "place"
    );
    assert!(confirmed["result"]["confirmation"]["result"]["order_id"].is_string());
    assert!(
        store
            .catalog_detail(&item.id)
            .unwrap()
            .unwrap()
            .prices
            .is_empty()
    );
}

fn save(store: &mut Store, payload: Value) -> Value {
    let input =
        decode(json!({"meta":meta(store),"payload":payload,"corrects_confirmation_id":null}));
    serde_json::to_value(store.business_create(input).unwrap()).unwrap()
}
fn record_id(saved: &Value) -> &str {
    saved["result"]["item"]["id"].as_str().unwrap()
}
fn confirm_input(store: &Store, saved: &Value) -> Value {
    json!({"meta":meta(store),"expected_head_id":saved["result"]["head"]["id"]})
}
fn confirm(store: &mut Store, saved: &Value) -> Value {
    let input = decode(confirm_input(store, saved));
    serde_json::to_value(store.business_confirm(record_id(saved), input).unwrap()).unwrap()
}
fn order_id(confirmed: &Value) -> &str {
    confirmed["result"]["confirmation"]["result"]["order_id"]
        .as_str()
        .unwrap()
}
fn order(store: &Store, id: &str) -> Value {
    serde_json::to_value(store.order_detail(id).unwrap().unwrap()).unwrap()
}
fn revise_order(detail: &Value, quantity: &str) -> Value {
    json!({"kind":"purchase_order","version":1,"data":{"action":"revise","order_id":detail["item"]["id"],"expected_order_version_id":detail["head"]["id"],"material_version_id":detail["head"]["material_version_id"],"quantity":quantity,"unit":"kg","supplier":"가상 거래처","expected_date":"2028-02-29","estimated_amount":"0","note":""}})
}
fn cancel_order(detail: &Value) -> Value {
    json!({"kind":"purchase_order","version":1,"data":{"action":"cancel","order_id":detail["item"]["id"],"expected_order_version_id":detail["head"]["id"],"note":""}})
}
fn count(root: &Path, table: &str) -> i64 {
    Connection::open(root.join("app.sqlite3"))
        .unwrap()
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

#[test]
fn revisions_cancellation_and_pure_receipt_closure_keep_real_stock_and_history_separate() {
    use sunchang_hansang::{
        domain::{Quantity, Unit},
        purchase_order::{
            OrderForReceipt, OrderReferences, OrderStatus, ReceiptReferences, ReceiptResult,
            close_order,
        },
    };
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = item(&mut store);
    let saved = save(&mut store, place(&item, "10"));
    let first = confirm(&mut store, &saved);
    let id = order_id(&first);
    let before = order(&store, id);
    let update = save(&mut store, revise_order(&before, "12"));
    confirm(&mut store, &update);
    let actual = store.order_detail(id).unwrap().unwrap();
    assert_eq!(actual.versions.len(), 2);
    assert_eq!(actual.head.quantity, "12000");
    assert_eq!(actual.head.estimated_amount.as_deref(), Some("0"));
    assert_eq!(actual.versions[0].quantity, "10000");
    assert_eq!(actual.versions[0].estimated_amount, None);
    let state = store.catalog_state().unwrap();
    let database = fs::read(root.path().join("app.sqlite3")).unwrap();
    let snapshots = count(root.path(), "snapshot_outbox");
    let refs = OrderReferences {
        order_id: Uuid::parse_str(id).unwrap(),
        material_id: Uuid::parse_str(&item.id).unwrap(),
        initial_order_version_id: Uuid::parse_str(&actual.versions[0].id).unwrap(),
        current_order_version_id: Uuid::parse_str(&actual.head.id).unwrap(),
    };
    let typed = OrderForReceipt::new(
        refs.clone(),
        OrderStatus::Open,
        Quantity::parse("10000").unwrap(),
        Quantity::parse("12000").unwrap(),
        Unit::parse("g").unwrap(),
    )
    .unwrap();
    let receipt = ReceiptResult::new(
        ReceiptReferences {
            receipt_id: Uuid::new_v4(),
            confirmation_id: Uuid::new_v4(),
            order_id: refs.order_id,
            expected_order_version_id: refs.current_order_version_id,
            material_id: refs.material_id,
            material_version_id: Uuid::parse_str(&item.version_id).unwrap(),
        },
        Quantity::parse("11.8").unwrap(),
        Unit::parse("kg").unwrap(),
    )
    .unwrap();
    let closure = close_order(&typed, &receipt).unwrap();
    assert_eq!(closure.initial_ordered_quantity, "10000");
    assert_eq!(closure.ordered_quantity, "12000");
    assert_eq!(closure.received_quantity, "11800");
    assert_eq!(closure.difference, "-200");
    assert_eq!(closure.status, OrderStatus::Received);
    let closed = OrderForReceipt::new(
        refs,
        closure.status,
        typed.initial_quantity(),
        typed.ordered_quantity(),
        Unit::parse("g").unwrap(),
    )
    .unwrap();
    assert!(close_order(&closed, &receipt).is_err());
    assert_eq!(store.catalog_state().unwrap(), state);
    assert_eq!(count(root.path(), "snapshot_outbox"), snapshots);
    assert_eq!(fs::read(root.path().join("app.sqlite3")).unwrap(), database);
    assert_eq!(
        store.order_detail(id).unwrap().unwrap().head.status,
        OrderStatus::Open
    );
    let latest = order(&store, id);
    let cancelled = save(&mut store, cancel_order(&latest));
    let result = confirm(&mut store, &cancelled);
    let detail = order(&store, id);
    assert_eq!(detail["head"]["status"], "cancelled");
    assert_eq!(detail["head"]["quantity"], "12000");
    assert_eq!(detail["head"]["supplier"], "가상 거래처");
    assert_eq!(detail["head"]["note"], "");
    assert_eq!(detail["versions"].as_array().unwrap().len(), 3);
    assert_eq!(order_id(&result), id);
    assert!(
        store
            .catalog_detail(&item.id)
            .unwrap()
            .unwrap()
            .prices
            .is_empty()
    );
    assert!(
        store
            .order_list(decode(json!({"status":"open"})))
            .unwrap()
            .items
            .is_empty()
    );
    assert!(
        store
            .order_list(decode(json!({"status":"received"})))
            .unwrap()
            .items
            .is_empty()
    );
    drop(store);
    let store = Store::open(root.path()).unwrap();
    assert_eq!(order(&store, id), detail);
}

#[test]
fn stale_order_heads_and_action_changes_do_not_apply_effects() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = item(&mut store);
    let saved = save(&mut store, place(&item, "10"));
    let confirmed = confirm(&mut store, &saved);
    let id = order_id(&confirmed);
    let original = order(&store, id);
    let cancellation = save(&mut store, cancel_order(&original));
    let revision = save(&mut store, revise_order(&original, "12"));
    confirm(&mut store, &revision);
    let before = store.catalog_state().unwrap();
    assert!(matches!(
        store.business_confirm(
            record_id(&cancellation),
            decode(confirm_input(&store, &cancellation))
        ),
        Err(StoreError::StaleState(_))
    ));
    assert_eq!(store.catalog_state().unwrap(), before);
    let mut changed = revise_order(&original, "12");
    changed["data"]["order_id"] = json!(Uuid::new_v4().to_string());
    let input = json!({"meta":meta(&store),"expected_head_id":cancellation["result"]["head"]["id"],"payload":changed});
    assert!(matches!(
        store.business_revise(record_id(&cancellation), decode(input)),
        Err(StoreError::Validation(_))
    ));
    let archived = store
        .business_archive(
            record_id(&cancellation),
            decode(confirm_input(&store, &cancellation)),
        )
        .unwrap();
    assert_eq!(
        serde_json::to_value(archived).unwrap()["result"]["head"]["status"],
        "archived"
    );
    assert_eq!(order(&store, id)["versions"].as_array().unwrap().len(), 2);
    let latest = order(&store, id);
    let new_cancel = save(&mut store, cancel_order(&latest));
    confirm(&mut store, &new_cancel);
    let latest = order(&store, id);
    let closed = save(&mut store, revise_order(&latest, "13"));
    assert!(matches!(
        store.business_confirm(record_id(&closed), decode(confirm_input(&store, &closed))),
        Err(StoreError::StaleState(_))
    ));
    assert_eq!(order(&store, id)["versions"].as_array().unwrap().len(), 3);
}

#[test]
fn original_order_responses_replay_after_revision_and_cancellation_without_merging_equal_orders() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = item(&mut store);
    let saved = save(&mut store, place(&item, "10"));
    let first_input = confirm_input(&store, &saved);
    let first = serde_json::to_value(
        store
            .business_confirm(record_id(&saved), decode(first_input.clone()))
            .unwrap(),
    )
    .unwrap();
    let id = order_id(&first);
    let detail = order(&store, id);
    let revised = save(&mut store, revise_order(&detail, "12"));
    let revised_input = confirm_input(&store, &revised);
    let revised_response = serde_json::to_value(
        store
            .business_confirm(record_id(&revised), decode(revised_input.clone()))
            .unwrap(),
    )
    .unwrap();
    let detail = order(&store, id);
    let cancelled = save(&mut store, cancel_order(&detail));
    let cancel_input = confirm_input(&store, &cancelled);
    let cancel_response = serde_json::to_value(
        store
            .business_confirm(record_id(&cancelled), decode(cancel_input.clone()))
            .unwrap(),
    )
    .unwrap();
    let second = save(&mut store, place(&item, "10"));
    let separate = confirm(&mut store, &second);
    assert_ne!(order_id(&separate), id);
    assert!(matches!(
        store.business_confirm(record_id(&second), decode(first_input.clone())),
        Err(StoreError::Conflict(_))
    ));
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    for (saved, input, expected) in [
        (&saved, first_input, first),
        (&revised, revised_input, revised_response),
        (&cancelled, cancel_input, cancel_response),
    ] {
        assert_eq!(
            serde_json::to_value(
                store
                    .business_confirm(record_id(saved), decode(input))
                    .unwrap()
            )
            .unwrap(),
            expected
        );
    }
    assert_eq!(count(root.path(), "purchase_orders"), 2);
    assert_eq!(count(root.path(), "purchase_order_versions"), 4);
}

#[test]
fn order_effect_confirmation_and_journal_failures_are_atomic() {
    for (name, trigger) in [
        (
            "effect",
            "CREATE TRIGGER injected BEFORE INSERT ON purchase_order_heads BEGIN SELECT RAISE(FAIL,'effect failure'); END;",
        ),
        (
            "confirmation",
            "CREATE TRIGGER injected BEFORE INSERT ON business_confirmations BEGIN SELECT RAISE(FAIL,'confirmation failure'); END;",
        ),
        (
            "journal",
            "CREATE TRIGGER injected BEFORE UPDATE OF payload ON snapshot_outbox BEGIN SELECT RAISE(FAIL,'journal failure'); END;",
        ),
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let item = item(&mut store);
        let saved = save(&mut store, place(&item, "10"));
        let state = store.catalog_state().unwrap();
        let database = Connection::open(root.path().join("app.sqlite3")).unwrap();
        database.execute_batch(trigger).unwrap();
        assert!(
            matches!(
                store.business_confirm(record_id(&saved), decode(confirm_input(&store, &saved))),
                Err(StoreError::Unavailable(_))
            ),
            "{name}"
        );
        assert_eq!(store.catalog_state().unwrap(), state);
        assert_eq!(count(root.path(), "purchase_orders"), 0);
        assert_eq!(count(root.path(), "purchase_order_versions"), 0);
        assert_eq!(count(root.path(), "business_confirmations"), 0);
        assert_eq!(count(root.path(), "business_revisions"), 1);
        assert_eq!(count(root.path(), "mutation_requests"), 2);
        assert_eq!(count(root.path(), "snapshot_outbox"), 2);
    }
}

#[test]
fn order_publication_failure_recovers_same_response_before_any_new_write() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = item(&mut store);
    let saved = save(&mut store, place(&item, "10"));
    let input = confirm_input(&store, &saved);
    let seq = store
        .catalog_state()
        .unwrap()
        .revision
        .parse::<i64>()
        .unwrap()
        + 1;
    let path = root.path().join(format!("snapshots/{seq:020}.json"));
    fs::create_dir(&path).unwrap();
    assert!(matches!(
        store.business_confirm(record_id(&saved), decode(input.clone())),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(count(root.path(), "purchase_orders"), 1);
    assert!(matches!(
        store.business_confirm(record_id(&saved), decode(input.clone())),
        Err(StoreError::Unavailable(_))
    ));
    let other = decode(
        json!({"meta":meta(&store),"payload":place(&item,"20"),"corrects_confirmation_id":null}),
    );
    assert!(matches!(
        store.business_create(other),
        Err(StoreError::Unavailable(_))
    ));
    fs::remove_dir(&path).unwrap();
    let first = serde_json::to_value(
        store
            .business_confirm(record_id(&saved), decode(input.clone()))
            .unwrap(),
    )
    .unwrap();
    let bytes = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(
        serde_json::to_value(
            store
                .business_confirm(record_id(&saved), decode(input))
                .unwrap()
        )
        .unwrap(),
        first
    );
    assert_eq!(count(root.path(), "purchase_order_versions"), 1);
}

#[test]
fn incomplete_order_fields_remain_drafts_and_invalid_confirmations_do_not_create_orders() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = item(&mut store);
    for (quantity, unit, date, amount) in [
        ("0", "kg", "", ""),
        ("1", "L", "", "0"),
        ("1", "kg", "2026-02-29", "0"),
        ("1", "kg", "2028-02-29", "-1"),
    ] {
        let mut payload = place(&item, quantity);
        payload["data"]["unit"] = json!(unit);
        payload["data"]["expected_date"] = json!(date);
        payload["data"]["estimated_amount"] = json!(amount);
        let saved = save(&mut store, payload);
        assert!(!saved["result"]["issues"].as_array().unwrap().is_empty());
        assert!(matches!(
            store.business_confirm(record_id(&saved), decode(confirm_input(&store, &saved))),
            Err(StoreError::Validation(_))
        ));
    }
    assert_eq!(count(root.path(), "purchase_orders"), 0);
    assert_eq!(count(root.path(), "material_prices"), 0);
    let bad = decode(
        json!({"meta":meta(&store),"payload":place(&item,"1"),"corrects_confirmation_id":Uuid::new_v4().to_string()}),
    );
    assert!(matches!(
        store.business_create(bad),
        Err(StoreError::Validation(_))
    ));
}

fn legacy_v4(root: &Path) {
    let mut connection = Connection::open(root.join("app.sqlite3")).unwrap();
    let tx = connection.transaction().unwrap();
    tx.execute_batch("PRAGMA defer_foreign_keys=ON;").unwrap();
    tx.execute_batch(include_str!("fixtures/schema-v4.sql"))
        .unwrap();
    tx.commit().unwrap();
    fs::create_dir(root.join("snapshots")).unwrap();
    let rows = connection
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")
        .unwrap()
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    for (seq, payload) in rows {
        fs::write(root.join(format!("snapshots/{seq:020}.json")), payload).unwrap();
    }
}

#[test]
fn frozen_schema_four_preserves_price_request_canonical_strings_and_immutable_files() {
    let root = TempDir::new().unwrap();
    legacy_v4(root.path());
    let frozen: Value =
        serde_json::from_str(include_str!("fixtures/snapshot-v2-schema4.json")).unwrap();
    let mut store = Store::open(root.path()).unwrap();
    assert_eq!(
        serde_json::to_value(store.catalog_state().unwrap()).unwrap(),
        frozen["state"]["catalog_state"]
    );
    for request in frozen["state"]["mutation_requests"].as_array().unwrap() {
        let operation = request["operation"].as_str().unwrap();
        let payload: Value = serde_json::from_str(request["payload"].as_str().unwrap()).unwrap();
        let expected: Value = serde_json::from_str(request["response"].as_str().unwrap()).unwrap();
        let replay = match operation {
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
            other => panic!("unexpected frozen fixture operation {other}"),
        };
        assert_eq!(replay, expected, "{operation}");
    }
    let oldfile = root.path().join("snapshots/00000000000000000005.json");
    assert_eq!(
        fs::read(&oldfile).unwrap(),
        include_bytes!("fixtures/snapshot-v2-schema4.json")
    );
    let item = store
        .catalog_list(Default::default())
        .unwrap()
        .pop()
        .unwrap();
    let saved = save(&mut store, place(&item, "10"));
    confirm(&mut store, &saved);
    let seq = store
        .catalog_state()
        .unwrap()
        .revision
        .parse::<i64>()
        .unwrap();
    let newest: Value = serde_json::from_slice(
        &fs::read(root.path().join(format!("snapshots/{seq:020}.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(newest["schema_version"], 14);
    assert_eq!(
        newest["state"]["purchase_orders"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        newest["state"]["business_corrections"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    fs::remove_file(&oldfile).unwrap();
    store.recover().unwrap();
    assert_eq!(
        fs::read(oldfile).unwrap(),
        include_bytes!("fixtures/snapshot-v2-schema4.json")
    );
}

#[test]
fn snapshot_readers_reject_cross_schema_kinds_and_invalid_known_order_states() {
    for mode in [
        "old_kind",
        "source_future",
        "missing",
        "unknown_action",
        "received",
        "cancel_open",
        "place_cancelled",
    ] {
        let root = TempDir::new().unwrap();
        if matches!(mode, "old_kind" | "source_future") {
            legacy_v4(root.path());
        } else {
            let mut store = Store::open(root.path()).unwrap();
            let item = item(&mut store);
            let saved = save(&mut store, place(&item, "10"));
            confirm(&mut store, &saved);
        }
        let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
        let (seq, json): (i64, String) = db
            .query_row(
                "SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let mut value: Value = serde_json::from_str(&json).unwrap();
        match mode {
            "old_kind" => value["state"]["business_records"][0]["kind"] = json!("purchase_order"),
            "source_future" => {
                value["schema_version"] = json!(5);
                for key in [
                    "purchase_orders",
                    "purchase_order_versions",
                    "purchase_order_heads",
                ] {
                    value["state"][key] = json!([]);
                }
            }
            "missing" => {
                value["state"]
                    .as_object_mut()
                    .unwrap()
                    .remove("purchase_orders");
            }
            "unknown_action" => {
                value["state"]["purchase_order_versions"][0]["action"] = json!("deliver")
            }
            "received" => {
                value["state"]["purchase_order_versions"][0]["status"] = json!("received")
            }
            "cancel_open" => {
                value["state"]["purchase_order_versions"][0]["action"] = json!("cancel")
            }
            _ => value["state"]["purchase_order_versions"][0]["status"] = json!("cancelled"),
        }
        let payload = value.to_string();
        db.execute(
            "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=?2",
            rusqlite::params![payload, seq],
        )
        .unwrap();
        drop(db);
        let path = root.path().join(format!("snapshots/{seq:020}.json"));
        fs::write(&path, payload).unwrap();
        let before = fs::read(root.path().join("app.sqlite3")).unwrap();
        let bytes = fs::read(&path).unwrap();
        assert!(Store::open(root.path()).is_err(), "{mode}");
        assert_eq!(
            fs::read(root.path().join("app.sqlite3")).unwrap(),
            before,
            "{mode}"
        );
        assert_eq!(fs::read(path).unwrap(), bytes, "{mode}");
    }
}

#[test]
fn database_validation_rejects_received_and_inconsistent_action_status_pairs() {
    for (action, status) in [
        ("place", "received"),
        ("cancel", "open"),
        ("place", "cancelled"),
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let item = item(&mut store);
        let saved = save(&mut store, place(&item, "10"));
        confirm(&mut store, &saved);
        drop(store);
        let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
        db.execute_batch("PRAGMA ignore_check_constraints=ON;")
            .unwrap();
        db.execute(
            "UPDATE purchase_order_versions SET action=?1,status=?2",
            rusqlite::params![action, status],
        )
        .unwrap();
        drop(db);
        let bytes = fs::read(root.path().join("app.sqlite3")).unwrap();
        assert!(Store::open(root.path()).is_err());
        assert_eq!(fs::read(root.path().join("app.sqlite3")).unwrap(), bytes);
    }
}
