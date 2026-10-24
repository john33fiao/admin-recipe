use rusqlite::Connection;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{fs, path::Path};
use sunchang_hansang::{
    business::{BusinessFilter, CreateBusinessDraft},
    catalog::{CreateCatalogMaterial, CreateCatalogPrice, MaterialSummary, MutationMeta},
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

fn add(store: &mut Store) -> MaterialSummary {
    let input = CreateCatalogMaterial {
        meta: decode::<MutationMeta>(meta(store)),
        name: "가상 초안 재료".into(),
        unit: "g".into(),
        kind: "raw".into(),
        description: String::new(),
    };
    store.catalog_create(input).unwrap().value
}

fn payload(item: &MaterialSummary, quantity: &str, amount: &str) -> Value {
    json!({"kind":"catalog_price","version":1,"data":{
        "material_id":item.id,"material_version_id":item.version_id,
        "quantity":quantity,"unit":"kg","amount":amount,"note":""}})
}

fn draft(store: &mut Store, item: &MaterialSummary, quantity: &str, amount: &str) -> Value {
    let input = decode::<CreateBusinessDraft>(json!({"meta":meta(store),
        "payload":payload(item,quantity,amount),"corrects_confirmation_id":null}));
    serde_json::to_value(store.business_create(input).unwrap()).unwrap()
}

fn confirm_input(store: &Store, draft: &Value) -> Value {
    json!({"meta":meta(store),"expected_head_id":draft["result"]["head"]["id"]})
}

#[test]
fn incomplete_draft_survives_restart_and_confirms_one_zero_price() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = add(&mut store);
    let saved = draft(&mut store, &item, "", "0");
    let id = saved["result"]["item"]["id"].as_str().unwrap().to_owned();
    assert!(!saved["result"]["issues"].as_array().unwrap().is_empty());
    assert!(
        store
            .catalog_detail(&item.id)
            .unwrap()
            .unwrap()
            .prices
            .is_empty()
    );
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    let detail = serde_json::to_value(store.business_detail(&id).unwrap().unwrap()).unwrap();
    assert_eq!(detail["head"]["payload"]["data"]["quantity"], "");
    assert_eq!(detail["head"]["payload"]["data"]["amount"], "0");
    let revised = store
        .business_revise(
            &id,
            decode(json!({"meta":meta(&store),
        "expected_head_id":detail["head"]["id"],"payload":payload(&item,"1","0")})),
        )
        .unwrap();
    let revised = serde_json::to_value(revised).unwrap();
    let before = store.catalog_state().unwrap();
    let preview = store
        .business_preview(
            &id,
            decode(json!({"expected_head_id":revised["result"]["head"]["id"]})),
        )
        .unwrap();
    let preview = serde_json::to_value(preview).unwrap();
    assert_eq!(preview["can_confirm"], true);
    assert_eq!(preview["effect"]["quantity"], "1000");
    assert_eq!(store.catalog_state().unwrap(), before);
    let confirmed = store
        .business_confirm(&id, decode(confirm_input(&store, &revised)))
        .unwrap();
    let confirmed = serde_json::to_value(confirmed).unwrap();
    let prices = store.catalog_detail(&item.id).unwrap().unwrap().prices;
    assert_eq!(prices.len(), 1);
    assert_eq!(prices[0].amount, "0");
    assert_eq!(prices[0].quantity, "1000");
    assert_eq!(confirmed["result"]["confirmation"]["seq"], prices[0].seq);
    assert_eq!(confirmed["result"]["head"]["status"], "confirmed");
    let detail = serde_json::to_value(store.business_detail(&id).unwrap().unwrap()).unwrap();
    assert_eq!(detail["revisions"].as_array().unwrap().len(), 3);
    assert_eq!(detail["confirmations"].as_array().unwrap().len(), 1);
    assert!(
        serde_json::to_value(store.catalog_detail(&item.id).unwrap().unwrap())
            .unwrap()
            .get("holding_quantity")
            .is_none()
    );
}

#[test]
fn stale_preview_and_duplicate_confirmation_do_not_add_effects() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = add(&mut store);
    let saved = draft(&mut store, &item, "1", "10");
    let id = saved["result"]["item"]["id"].as_str().unwrap();
    let old = confirm_input(&store, &saved);
    store
        .catalog_add_price(
            &item.id,
            CreateCatalogPrice {
                meta: decode(meta(&store)),
                expected_version_id: item.version_id.clone(),
                quantity: "1".into(),
                unit: "kg".into(),
                amount: "20".into(),
            },
        )
        .unwrap();
    assert!(matches!(
        store.business_confirm(id, decode(old)),
        Err(StoreError::StaleState(_))
    ));
    let current = confirm_input(&store, &saved);
    let first = store.business_confirm(id, decode(current.clone())).unwrap();
    let replay = store.business_confirm(id, decode(current)).unwrap();
    assert_eq!(
        serde_json::to_value(first).unwrap(),
        serde_json::to_value(replay).unwrap()
    );
    assert!(matches!(
        store.business_confirm(id, decode(confirm_input(&store, &saved))),
        Err(StoreError::StaleState(_)) | Err(StoreError::Validation(_))
    ));
    assert_eq!(
        store
            .catalog_detail(&item.id)
            .unwrap()
            .unwrap()
            .prices
            .len(),
        2
    );
}

fn count(root: &Path, table: &str) -> i64 {
    Connection::open(root.join("app.sqlite3"))
        .unwrap()
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

fn saved_id(saved: &Value) -> &str {
    saved["result"]["item"]["id"].as_str().unwrap()
}

#[test]
fn publication_failure_and_missing_file_recover_original_response_once() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = add(&mut store);
    let saved = draft(&mut store, &item, "1", "10");
    let input = confirm_input(&store, &saved);
    let sequence = store
        .catalog_state()
        .unwrap()
        .revision
        .parse::<i64>()
        .unwrap()
        + 1;
    let destination = root.path().join(format!("snapshots/{sequence:020}.json"));
    fs::create_dir(&destination).unwrap();
    assert!(matches!(
        store.business_confirm(saved_id(&saved), decode(input.clone())),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(count(root.path(), "material_prices"), 1);
    assert_eq!(count(root.path(), "business_confirmations"), 1);
    assert!(matches!(
        store.business_confirm(saved_id(&saved), decode(input.clone())),
        Err(StoreError::Unavailable(_))
    ));
    let other = decode(
        json!({"meta":meta(&store),"payload":payload(&item,"1","20"),"corrects_confirmation_id":null}),
    );
    assert!(matches!(
        store.business_create(other),
        Err(StoreError::Unavailable(_))
    ));
    fs::remove_dir(&destination).unwrap();
    let recovered = serde_json::to_value(
        store
            .business_confirm(saved_id(&saved), decode(input.clone()))
            .unwrap(),
    )
    .unwrap();
    let bytes = fs::read(&destination).unwrap();
    let snapshot: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(snapshot["schema_version"], 8);
    assert_eq!(
        snapshot["state"]["business_confirmations"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    fs::remove_file(&destination).unwrap();
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    assert_eq!(fs::read(&destination).unwrap(), bytes);
    let _later = draft(&mut store, &item, "2", "20");
    let replay = serde_json::to_value(
        store
            .business_confirm(saved_id(&saved), decode(input))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(replay, recovered);
    assert_eq!(count(root.path(), "material_prices"), 1);
}

#[test]
fn confirmation_and_correction_failures_roll_back_every_related_row() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = add(&mut store);
    let saved = draft(&mut store, &item, "1", "10");
    let before = store.catalog_state().unwrap();
    let database = Connection::open(root.path().join("app.sqlite3")).unwrap();
    database.execute_batch("CREATE TRIGGER fail_confirmation BEFORE INSERT ON business_confirmations BEGIN SELECT RAISE(FAIL,'injected confirmation failure'); END;").unwrap();
    assert!(matches!(
        store.business_confirm(saved_id(&saved), decode(confirm_input(&store, &saved))),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(store.catalog_state().unwrap(), before);
    for table in [
        "material_prices",
        "business_confirmations",
        "business_catalog_price_results",
        "business_corrections",
    ] {
        assert_eq!(count(root.path(), table), 0, "{table}");
    }
    assert_eq!(count(root.path(), "business_revisions"), 1);
    assert_eq!(count(root.path(), "mutation_requests"), 2);
    assert_eq!(count(root.path(), "snapshot_outbox"), 2);
    database
        .execute_batch("DROP TRIGGER fail_confirmation;")
        .unwrap();
    let confirmed = serde_json::to_value(
        store
            .business_confirm(saved_id(&saved), decode(confirm_input(&store, &saved)))
            .unwrap(),
    )
    .unwrap();
    let target = confirmed["result"]["confirmation"]["id"].as_str().unwrap();
    let mut p = payload(&item, "1", "30");
    p["data"]["note"] = json!("가상 금액 오입력 정정");
    let correction = serde_json::to_value(
        store
            .business_create(decode(
                json!({"meta":meta(&store),"payload":p,"corrects_confirmation_id":target}),
            ))
            .unwrap(),
    )
    .unwrap();
    let before = store.catalog_state().unwrap();
    let tables = [
        "material_prices",
        "business_confirmations",
        "business_catalog_price_results",
        "business_corrections",
        "business_revisions",
        "mutation_requests",
        "snapshot_outbox",
    ];
    let counts = tables.map(|t| count(root.path(), t));
    database.execute_batch("CREATE TRIGGER fail_correction BEFORE INSERT ON business_corrections BEGIN SELECT RAISE(FAIL,'injected correction failure'); END;").unwrap();
    assert!(matches!(
        store.business_confirm(
            saved_id(&correction),
            decode(confirm_input(&store, &correction))
        ),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(store.catalog_state().unwrap(), before);
    for (table, expected) in tables.into_iter().zip(counts) {
        assert_eq!(count(root.path(), table), expected, "{table}");
    }
    assert_eq!(
        serde_json::to_value(
            store
                .business_detail(saved_id(&correction))
                .unwrap()
                .unwrap()
        )
        .unwrap()["head"]["status"],
        "draft"
    );
}

#[test]
fn independent_identical_drafts_remain_distinct_and_request_keys_include_target() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = add(&mut store);
    let input = json!({"meta":meta(&store),"payload":payload(&item,"1","10"),"corrects_confirmation_id":null});
    let first =
        serde_json::to_value(store.business_create(decode(input.clone())).unwrap()).unwrap();
    let second = draft(&mut store, &item, "1", "10");
    let confirmed_input = confirm_input(&store, &first);
    store
        .business_confirm(saved_id(&first), decode(confirmed_input.clone()))
        .unwrap();
    assert!(matches!(
        store.business_confirm(saved_id(&second), decode(confirmed_input)),
        Err(StoreError::Conflict(_))
    ));
    store
        .business_confirm(saved_id(&second), decode(confirm_input(&store, &second)))
        .unwrap();
    assert_eq!(count(root.path(), "material_prices"), 2);
    assert_eq!(count(root.path(), "business_confirmations"), 2);
    assert_eq!(
        serde_json::to_value(store.business_create(decode(input.clone())).unwrap()).unwrap(),
        first
    );
    let mut changed = input.clone();
    changed["payload"]["data"]["amount"] = json!("11");
    assert!(matches!(
        store.business_create(decode(changed)),
        Err(StoreError::Conflict(_))
    ));
    let key = input["meta"]["request_id"].as_str().unwrap();
    assert!(matches!(
        store.create(sunchang_hansang::store::CreateMaterial {
            request_id: key.into(),
            name: "가상 다른 명령".into(),
            unit: "g".into()
        }),
        Err(StoreError::Conflict(_))
    ));
    let legacy_key = Uuid::new_v4().to_string();
    store
        .create(sunchang_hansang::store::CreateMaterial {
            request_id: legacy_key.clone(),
            name: "가상 기존 등록".into(),
            unit: "g".into(),
        })
        .unwrap();
    let mut collision = json!({"meta":meta(&store),"payload":payload(&item,"1","10"),"corrects_confirmation_id":null});
    collision["meta"]["request_id"] = json!(legacy_key);
    assert!(matches!(
        store.business_create(decode(collision)),
        Err(StoreError::Conflict(_))
    ));
}

#[test]
fn malformed_payloads_are_rejected_but_incomplete_values_stay_editable() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = add(&mut store);
    let valid = json!({"meta":meta(&store),"payload":payload(&item,"1","10"),"corrects_confirmation_id":null});
    for (path, value) in [
        ("kind", json!("sale")),
        ("version", json!(99)),
        ("inventory_delta", json!("1")),
    ] {
        let mut bad = valid.clone();
        bad["payload"][path] = value;
        assert!(serde_json::from_value::<CreateBusinessDraft>(bad).is_err());
    }
    let mut bad = valid.clone();
    bad["payload"]["data"]["amount"] = json!(1);
    assert!(serde_json::from_value::<CreateBusinessDraft>(bad).is_err());
    let mut bad = valid;
    bad["payload"]["data"]["quantity"] = json!("1".repeat(81));
    assert!(matches!(
        store.business_create(decode(bad)),
        Err(StoreError::Validation(_))
    ));
    assert_eq!(count(root.path(), "business_records"), 0);
    for (quantity, amount, unit) in [("0", "0", "kg"), ("1.", "", "kg"), ("1", "10", "L")] {
        let mut p = payload(&item, quantity, amount);
        p["data"]["unit"] = json!(unit);
        let saved = serde_json::to_value(
            store
                .business_create(decode(
                    json!({"meta":meta(&store),"payload":p,"corrects_confirmation_id":null}),
                ))
                .unwrap(),
        )
        .unwrap();
        assert!(!saved["result"]["issues"].as_array().unwrap().is_empty());
        assert!(matches!(
            store.business_confirm(saved_id(&saved), decode(confirm_input(&store, &saved))),
            Err(StoreError::Validation(_))
        ));
    }
    assert_eq!(count(root.path(), "material_prices"), 0);
}

#[test]
fn archiving_preserves_revisions_and_does_not_cancel_confirmed_work() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = add(&mut store);
    let saved = draft(&mut store, &item, "", "0");
    let archived = serde_json::to_value(
        store
            .business_archive(saved_id(&saved), decode(confirm_input(&store, &saved)))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(archived["result"]["item"]["issue_count"], 0);
    assert!(
        store
            .business_list(BusinessFilter::default())
            .unwrap()
            .items
            .is_empty()
    );
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    let detail =
        serde_json::to_value(store.business_detail(saved_id(&saved)).unwrap().unwrap()).unwrap();
    assert_eq!(detail["revisions"].as_array().unwrap().len(), 2);
    assert_eq!(detail["head"]["status"], "archived");
    assert!(matches!(
        store.business_confirm(saved_id(&saved), decode(confirm_input(&store, &archived))),
        Err(StoreError::Validation(_))
    ));
    let saved = draft(&mut store, &item, "1", "0");
    let confirmed = serde_json::to_value(
        store
            .business_confirm(saved_id(&saved), decode(confirm_input(&store, &saved)))
            .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        store.business_archive(saved_id(&saved), decode(confirm_input(&store, &confirmed))),
        Err(StoreError::Validation(_))
    ));
    assert_eq!(count(root.path(), "material_prices"), 1);
}

#[test]
fn correction_chain_keeps_original_current_and_new_prices_distinct() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = add(&mut store);
    let saved = draft(&mut store, &item, "1", "10");
    let original = serde_json::to_value(
        store
            .business_confirm(saved_id(&saved), decode(confirm_input(&store, &saved)))
            .unwrap(),
    )
    .unwrap();
    let target = original["result"]["confirmation"]["id"].as_str().unwrap();
    store
        .catalog_add_price(
            &item.id,
            CreateCatalogPrice {
                meta: decode(meta(&store)),
                expected_version_id: item.version_id.clone(),
                quantity: "1".into(),
                amount: "20".into(),
                unit: "kg".into(),
            },
        )
        .unwrap();
    let correction=serde_json::to_value(store.business_create(decode(json!({"meta":meta(&store),"payload":payload(&item,"1","30"),"corrects_confirmation_id":target}))).unwrap()).unwrap();
    assert!(
        correction["result"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["field"] == "note")
    );
    assert!(matches!(
        store.business_confirm(
            saved_id(&correction),
            decode(confirm_input(&store, &correction))
        ),
        Err(StoreError::Validation(_))
    ));
    let mut p = payload(&item, "1", "30");
    p["data"]["note"] = json!("가상 가격 오입력 정정");
    let revised=serde_json::to_value(store.business_revise(saved_id(&correction),decode(json!({"meta":meta(&store),"expected_head_id":correction["result"]["head"]["id"],"payload":p}))).unwrap()).unwrap();
    let preview = serde_json::to_value(
        store
            .business_preview(
                saved_id(&correction),
                decode(json!({"expected_head_id":revised["result"]["head"]["id"]})),
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(preview["effect"]["corrected_price"]["amount"], "10");
    assert_eq!(preview["effect"]["previous_price"]["amount"], "20");
    assert_eq!(preview["effect"]["amount"], "30");
    let pending = serde_json::to_value(
        store
            .business_detail(saved_id(&correction))
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(pending["item"]["effective_confirmation_id"], target);
    let correction_input = confirm_input(&store, &revised);
    let applied = serde_json::to_value(
        store
            .business_confirm(saved_id(&correction), decode(correction_input.clone()))
            .unwrap(),
    )
    .unwrap();
    let replay = serde_json::to_value(
        store
            .business_confirm(saved_id(&correction), decode(correction_input))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(applied, replay);
    let detail =
        serde_json::to_value(store.business_detail(saved_id(&saved)).unwrap().unwrap()).unwrap();
    assert_eq!(detail["confirmations"].as_array().unwrap().len(), 2);
    assert_eq!(detail["corrections"].as_array().unwrap().len(), 1);
    assert_eq!(
        detail["item"]["effective_confirmation_id"],
        applied["result"]["confirmation"]["id"]
    );
    assert_eq!(detail["head"]["payload"]["data"]["amount"], "10");
    assert_eq!(
        store
            .catalog_detail(&item.id)
            .unwrap()
            .unwrap()
            .prices
            .iter()
            .map(|p| p.amount.as_str())
            .collect::<Vec<_>>(),
        vec!["10", "20", "30"]
    );
    let mut p = payload(&item, "1", "40");
    p["data"]["note"] = json!("가상 두 번째 정정");
    let next=serde_json::to_value(store.business_create(decode(json!({"meta":meta(&store),"payload":p,"corrects_confirmation_id":applied["result"]["confirmation"]["id"]}))).unwrap()).unwrap();
    store
        .business_confirm(saved_id(&next), decode(confirm_input(&store, &next)))
        .unwrap();
    assert_eq!(
        store
            .business_detail(saved_id(&correction))
            .unwrap()
            .unwrap()
            .confirmations
            .len(),
        3
    );
    let mut old = payload(&item, "1", "50");
    old["data"]["note"] = json!("가상 이전 대상 재시도");
    assert!(matches!(
        store.business_create(decode(
            json!({"meta":meta(&store),"payload":old,"corrects_confirmation_id":target})
        )),
        Err(StoreError::StaleState(_))
    ));
}

fn legacy_v3(root: &Path) {
    let mut connection = Connection::open(root.join("app.sqlite3")).unwrap();
    let database = connection.transaction().unwrap();
    // The fixed catalog rows refer to the parameterized journal payloads below.
    // Validate their foreign keys together when the complete fixture commits.
    database
        .execute_batch("PRAGMA defer_foreign_keys=ON;")
        .unwrap();
    database
        .execute_batch(include_str!("fixtures/schema-v3.sql"))
        .unwrap();
    let old: Value =
        serde_json::from_str(include_str!("fixtures/snapshot-v2-schema3.json")).unwrap();
    for (seq, bytes) in [
        (1, include_str!("fixtures/snapshot-v1.json")),
        (2, include_str!("fixtures/snapshot-v2-schema3.json")),
    ] {
        database
            .execute(
                "INSERT INTO snapshot_outbox VALUES(?1,?2,1)",
                rusqlite::params![seq, bytes],
            )
            .unwrap();
    }
    let r = &old["state"]["mutation_requests"][0];
    database
        .execute(
            "INSERT INTO mutation_requests VALUES(?1,?2,?3,?4,?5,2)",
            rusqlite::params![
                r["request_id"].as_str().unwrap(),
                r["operation"].as_str().unwrap(),
                r["payload"].as_str().unwrap(),
                r["response"].as_str().unwrap(),
                r["epoch"].as_str().unwrap()
            ],
        )
        .unwrap();
    database.commit().unwrap();
    fs::create_dir(root.join("snapshots")).unwrap();
    fs::write(
        root.join("snapshots/00000000000000000001.json"),
        include_bytes!("fixtures/snapshot-v1.json"),
    )
    .unwrap();
    fs::write(
        root.join("snapshots/00000000000000000002.json"),
        include_bytes!("fixtures/snapshot-v2-schema3.json"),
    )
    .unwrap();
}

#[test]
fn schema_three_migration_preserves_old_ids_responses_and_snapshot_bytes() {
    let root = TempDir::new().unwrap();
    legacy_v3(root.path());
    let mut store = Store::open(root.path()).unwrap();
    assert_eq!(
        store.catalog_state().unwrap().epoch,
        "00000000-0000-4000-8000-000000000501"
    );
    assert_eq!(store.catalog_state().unwrap().revision, "2");
    let old: Value =
        serde_json::from_str(include_str!("fixtures/snapshot-v2-schema3.json")).unwrap();
    let request: Value = serde_json::from_str(
        old["state"]["mutation_requests"][0]["payload"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let response = serde_json::to_value(
        store
            .catalog_add_price(request[0].as_str().unwrap(), decode(request[1].clone()))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        response,
        serde_json::from_str::<Value>(
            old["state"]["mutation_requests"][0]["response"]
                .as_str()
                .unwrap()
        )
        .unwrap()
    );
    let item = store
        .catalog_detail(request[0].as_str().unwrap())
        .unwrap()
        .unwrap()
        .item;
    let saved = draft(&mut store, &item, "1", "30");
    store
        .catalog_add_price(
            &item.id,
            CreateCatalogPrice {
                meta: decode(meta(&store)),
                expected_version_id: item.version_id.clone(),
                quantity: "1".into(),
                amount: "20".into(),
                unit: "kg".into(),
            },
        )
        .unwrap();
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
    assert_eq!(newest["schema_version"], 8);
    assert_eq!(
        newest["state"]["business_records"][0]["id"],
        saved["result"]["item"]["id"]
    );
    assert_eq!(
        fs::read(root.path().join("snapshots/00000000000000000002.json")).unwrap(),
        include_bytes!("fixtures/snapshot-v2-schema3.json")
    );
    for seq in [1, 2] {
        fs::remove_file(root.path().join(format!("snapshots/{seq:020}.json"))).unwrap();
    }
    store.recover().unwrap();
    assert_eq!(
        fs::read(root.path().join("snapshots/00000000000000000001.json")).unwrap(),
        include_bytes!("fixtures/snapshot-v1.json")
    );
    assert_eq!(
        fs::read(root.path().join("snapshots/00000000000000000002.json")).unwrap(),
        include_bytes!("fixtures/snapshot-v2-schema3.json")
    );
}

#[test]
fn corrupt_or_future_snapshots_abort_migration_and_never_default_business_state() {
    for mode in [
        "future_schema",
        "source_future_schema",
        "missing_business",
        "unknown_kind",
        "unknown_version",
        "unknown_field",
    ] {
        let root = TempDir::new().unwrap();
        if matches!(mode, "future_schema" | "source_future_schema") {
            legacy_v3(root.path());
        } else {
            let mut store = Store::open(root.path()).unwrap();
            let item = add(&mut store);
            draft(&mut store, &item, "1", "10");
            drop(store);
        }
        let database = Connection::open(root.path().join("app.sqlite3")).unwrap();
        let (seq, payload): (i64, String) = database
            .query_row(
                "SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let mut payload: Value = serde_json::from_str(&payload).unwrap();
        match mode {
            "future_schema" => payload["schema_version"] = json!(999),
            "source_future_schema" => {
                payload["schema_version"] = json!(4);
                for field in [
                    "business_records",
                    "business_revisions",
                    "business_heads",
                    "business_confirmations",
                    "business_catalog_price_results",
                    "business_corrections",
                ] {
                    payload["state"][field] = json!([]);
                }
            }
            "missing_business" => {
                payload["state"]
                    .as_object_mut()
                    .unwrap()
                    .remove("business_records");
            }
            "unknown_kind" => {
                payload["state"]["business_revisions"][0]["payload"]["kind"] = json!("sale")
            }
            "unknown_version" => {
                payload["state"]["business_revisions"][0]["payload"]["version"] = json!(99)
            }
            _ => {
                payload["state"]["business_revisions"][0]["payload"]["data"]["inventory_delta"] =
                    json!("1")
            }
        }
        let invalid_payload = payload.to_string();
        database
            .execute(
                "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=?2",
                rusqlite::params![invalid_payload, seq],
            )
            .unwrap();
        drop(database);
        let file = root.path().join(format!("snapshots/{seq:020}.json"));
        // Match both copies so immutable-file mismatch cannot mask a parser
        // that incorrectly accepts missing, future or unknown schema fields.
        fs::write(&file, invalid_payload.as_bytes()).unwrap();
        let original = fs::read(root.path().join("app.sqlite3")).unwrap();
        let original_file = fs::read(&file).unwrap();
        assert!(Store::open(root.path()).is_err(), "{mode}");
        assert_eq!(
            fs::read(root.path().join("app.sqlite3")).unwrap(),
            original,
            "{mode}"
        );
        assert_eq!(fs::read(file).unwrap(), original_file, "{mode}");
    }
}
