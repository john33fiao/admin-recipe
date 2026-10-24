use rusqlite::Connection;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{fs, path::Path};
use sunchang_hansang::{
    catalog::MaterialSummary,
    store::{Store, StoreError},
};
use tempfile::TempDir;
use uuid::Uuid;

fn decode<T: DeserializeOwned>(input: Value) -> T {
    serde_json::from_value(input).unwrap()
}
fn value(input: impl Serialize) -> Value {
    serde_json::to_value(input).unwrap()
}
fn meta(store: &Store) -> Value {
    let s = store.catalog_state().unwrap();
    json!({"request_id":Uuid::new_v4().to_string(),"epoch":s.epoch,"expected_revision":s.revision})
}
fn item(store: &mut Store) -> MaterialSummary {
    let mut input = meta(store);
    input["name"] = json!("가상 폐기 원료");
    input["unit"] = json!("g");
    input["kind"] = json!("raw");
    input["description"] = json!("가상 보유 시험");
    store.catalog_create(decode(input)).unwrap().value
}
fn save(store: &mut Store, payload: Value) -> Value {
    let input = json!({"meta":meta(store),"payload":payload,"corrects_confirmation_id":null});
    value(store.business_create(decode(input)).unwrap())
}
fn id(saved: &Value) -> &str {
    saved["result"]["item"]["id"].as_str().unwrap()
}
fn confirm_input(store: &Store, saved: &Value) -> Value {
    json!({"meta":meta(store),"expected_head_id":saved["result"]["head"]["id"]})
}
fn confirm(store: &mut Store, saved: &Value) -> Value {
    let input = confirm_input(store, saved);
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
fn payload(item: &MaterialSummary, q: &str, reason: &str) -> Value {
    json!({"kind":"waste","version":1,"data":{"material_id":item.id,"material_version_id":item.version_id,"quantity":q,"unit":"g","reason":reason,"note":"가상 명시 기록"}})
}
fn receipt(store: &mut Store, item: &MaterialSummary, q: &str, amount: &str) {
    let d = save(
        store,
        json!({"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":item.id,"material_version_id":item.version_id},"quantity":q,"unit":"g","amount":amount,"receipt_date":"2026-01-01","note":"가상 입고","stock_effect":{"kind":"unreflected"}}}),
    );
    confirm(store, &d);
}
fn basis(store: &Store, item: &MaterialSummary) -> Value {
    value(store.holding_detail(&item.id).unwrap().unwrap())
}
fn counts(root: &Path) -> Vec<i64> {
    let db = Connection::open(root.join("app.sqlite3")).unwrap();
    [
        "wastes",
        "holding_versions",
        "holding_heads",
        "business_confirmations",
        "mutation_requests",
        "snapshot_outbox",
    ]
    .iter()
    .map(|t| {
        db.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))
            .unwrap()
    })
    .collect()
}

#[test]
fn partial_waste_keeps_value_full_staff_meal_ends_loss_and_next_receipt_starts_new_value() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let material = item(&mut store);
    receipt(&mut store, &material, "1000", "9000");
    let partial = save(&mut store, payload(&material, "100", "discard"));
    let request = confirm_input(&store, &partial);
    let result = value(
        store
            .business_confirm(id(&partial), decode(request.clone()))
            .unwrap(),
    );
    let waste = result["result"]["confirmation"]["result"]["waste_id"]
        .as_str()
        .unwrap();
    let detail = value(store.waste_detail(waste).unwrap().unwrap());
    assert_eq!(detail["item"]["effect"]["before"]["quantity"], "1000");
    assert_eq!(detail["item"]["effect"]["after"]["quantity"], "900");
    assert_eq!(detail["item"]["effect"]["after"]["value"], "9000");
    assert_eq!(detail["item"]["effect"]["loss_value"], "0");
    let full = save(&mut store, payload(&material, "900", "staff_meal"));
    let done = confirm(&mut store, &full);
    let full_id = done["result"]["confirmation"]["result"]["waste_id"]
        .as_str()
        .unwrap();
    let closed = value(store.waste_detail(full_id).unwrap().unwrap());
    assert_eq!(closed["item"]["effect"]["loss_value"], "9000");
    assert_eq!(closed["item"]["effect"]["reason"], "staff_meal");
    let zero = basis(&store, &material);
    assert_eq!(zero["settlement_basis"]["quantity"], "0");
    assert_eq!(zero["settlement_basis"]["value"], "0");
    assert_eq!(zero["price"]["source"], "last_valid");
    assert_eq!(zero["last_valid_price"]["amount"], "9000");
    assert_eq!(zero["last_valid_price"]["quantity"], "900");
    receipt(&mut store, &material, "200", "500");
    let current = basis(&store, &material);
    assert_eq!(current["settlement_basis"]["value"], "500");
    assert_eq!(current["settlement_basis"]["quantity"], "200");
    let before = counts(root.path());
    assert_eq!(
        value(
            store
                .business_confirm(id(&partial), decode(request))
                .unwrap()
        ),
        result
    );
    assert_eq!(counts(root.path()), before);
    drop(store);
    let store = Store::open(root.path()).unwrap();
    assert_eq!(
        value(store.waste_detail(waste).unwrap().unwrap())["item"],
        detail["item"]
    );
    assert_eq!(store.waste_list(Default::default()).unwrap().items.len(), 2);
}

#[test]
fn unfinished_and_excessive_waste_remain_drafts_without_inventory_effects() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let material = item(&mut store);
    let draft = save(&mut store, payload(&material, "1.", "discard"));
    assert_eq!(preview(&store, &draft)["can_confirm"], false);
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    let reopened = value(store.business_detail(id(&draft)).unwrap().unwrap());
    assert_eq!(reopened["head"]["payload"]["data"]["quantity"], "1.");
    let unknown = save(&mut store, payload(&material, "1", "discard"));
    assert_eq!(
        preview(&store, &unknown)["issues"][0]["code"],
        "needs_reconciliation"
    );
    receipt(&mut store, &material, "5", "0");
    for q in ["0", "6", "-1", "0.0000001"] {
        let d = save(&mut store, payload(&material, q, "discard"));
        let before = counts(root.path());
        let request = confirm_input(&store, &d);
        assert!(!preview(&store, &d)["can_confirm"].as_bool().unwrap());
        assert!(store.business_confirm(id(&d), decode(request)).is_err());
        assert_eq!(counts(root.path()), before);
    }
    let d = save(&mut store, payload(&material, "5", "discard"));
    let done = confirm(&mut store, &d);
    let waste = done["result"]["confirmation"]["result"]["waste_id"]
        .as_str()
        .unwrap();
    assert_eq!(
        store
            .waste_detail(waste)
            .unwrap()
            .unwrap()
            .item
            .effect
            .loss_value,
        "0"
    );
}

#[test]
fn waste_after_real_observation_retains_observed_boundary_and_changes_only_following_balance() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let material = item(&mut store);
    let start = save(
        &mut store,
        json!({"kind":"stocktake","version":1,"data":{"rows":[{"material_id":material.id,"material_version_id":material.version_id,"quantity":"1000","unit":"g","initial_value":{"kind":"manual_amount","amount":"9000"},"excess":null}],"note":"가상 시작 확인"}}),
    );
    confirm(&mut store, &start);
    let observed = basis(&store, &material);
    let draft = save(&mut store, payload(&material, "100", "discard"));
    let done = confirm(&mut store, &draft);
    let waste = done["result"]["confirmation"]["result"]["waste_id"]
        .as_str()
        .unwrap();
    let record = store.waste_detail(waste).unwrap().unwrap().item;
    assert_eq!(value(&record.effect.observation), observed["observation"]);
    assert_eq!(value(&record.effect.basis_before), observed["basis"]);
    let current = basis(&store, &material);
    assert_eq!(current["observation"], observed["observation"]);
    assert_eq!(current["settlement_basis"]["quantity"], "900");
    assert_eq!(current["confirmed_changes"][0]["kind"], "waste");
    assert_eq!(current["confirmed_changes"][0]["event_id"], waste);
    let again = save(
        &mut store,
        json!({"kind":"stocktake","version":1,"data":{"rows":[{"material_id":material.id,"material_version_id":material.version_id,"quantity":"600","unit":"g","initial_value":null,"excess":null}],"note":"가상 종료 확인"}}),
    );
    let calculated = preview(&store, &again);
    assert_eq!(
        calculated["effect"]["rows"][0]["calculation"]["consumption_quantity"],
        "300"
    );
    assert_eq!(
        calculated["effect"]["rows"][0]["calculation"]["consumption_value"],
        "3000"
    );
    confirm(&mut store, &again);
    drop(store);
    let store = Store::open(root.path()).unwrap();
    let current = basis(&store, &material);
    assert_eq!(current["settlement_basis"]["quantity"], "600");
    assert_eq!(current["settlement_basis"]["value"], "6000");
    assert_eq!(current["confirmed_changes"], json!([]));
}

#[test]
fn waste_writes_are_atomic_at_each_stage_and_retry_uses_the_same_request() {
    for (table, event) in [
        ("wastes", "INSERT"),
        ("holding_versions", "INSERT"),
        ("holding_heads", "UPDATE"),
        ("mutation_requests", "INSERT"),
        ("snapshot_outbox", "UPDATE"),
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let material = item(&mut store);
        receipt(&mut store, &material, "1000", "9000");
        let draft = save(&mut store, payload(&material, "100", "discard"));
        let request = confirm_input(&store, &draft);
        let before = counts(root.path());
        let state = store.catalog_state().unwrap();
        let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
        db.execute_batch(&format!("CREATE TRIGGER fail_waste BEFORE {event} ON {table} BEGIN SELECT RAISE(ABORT,'fixture write failure'); END;")).unwrap();
        assert!(
            store
                .business_confirm(id(&draft), decode(request.clone()))
                .is_err(),
            "{table}"
        );
        assert_eq!(counts(root.path()), before, "{table}");
        assert_eq!(store.catalog_state().unwrap(), state);
        db.execute_batch("DROP TRIGGER fail_waste").unwrap();
        let done = value(
            store
                .business_confirm(id(&draft), decode(request.clone()))
                .unwrap(),
        );
        assert_eq!(
            value(store.business_confirm(id(&draft), decode(request)).unwrap()),
            done
        );
        assert_eq!(store.waste_list(Default::default()).unwrap().items.len(), 1);
    }
}

#[test]
fn waste_publication_failure_recovers_once_and_conflicting_body_never_reuses_the_request() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let material = item(&mut store);
    receipt(&mut store, &material, "1000", "9000");
    let draft = save(&mut store, payload(&material, "100", "discard"));
    let request = confirm_input(&store, &draft);
    let next = store
        .catalog_state()
        .unwrap()
        .revision
        .parse::<i64>()
        .unwrap()
        + 1;
    let blocked = root.path().join(format!("snapshots/{next:020}.json"));
    fs::create_dir(&blocked).unwrap();
    assert!(matches!(
        store.business_confirm(id(&draft), decode(request.clone())),
        Err(StoreError::Unavailable(_))
    ));
    let after = counts(root.path());
    assert_eq!(after[0], 1);
    fs::remove_dir(blocked).unwrap();
    let done = value(
        store
            .business_confirm(id(&draft), decode(request.clone()))
            .unwrap(),
    );
    assert_eq!(counts(root.path()), after);
    assert_eq!(
        value(
            store
                .business_confirm(id(&draft), decode(request.clone()))
                .unwrap()
        ),
        done
    );
    let mut wrong = request;
    wrong["expected_head_id"] = json!(Uuid::new_v4().to_string());
    assert!(matches!(
        store.business_confirm(id(&draft), decode(wrong)),
        Err(StoreError::Conflict(_))
    ));
    assert_eq!(counts(root.path()), after);
    drop(store);
    assert_eq!(
        Store::open(root.path())
            .unwrap()
            .waste_list(Default::default())
            .unwrap()
            .items
            .len(),
        1
    );
}

#[test]
fn waste_snapshot_rejects_matching_file_and_outbox_tampering_without_repairing_it() {
    for mode in [
        "partial_value",
        "full_loss",
        "observation",
        "request",
        "missing_record",
        "wrong_kind",
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let material = item(&mut store);
        receipt(&mut store, &material, "1000", "9000");
        let draft = save(&mut store, payload(&material, "100", "discard"));
        confirm(&mut store, &draft);
        drop(store);
        let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
        let (seq, raw): (i64, String) = db
            .query_row(
                "SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let mut snapshot: Value = serde_json::from_str(&raw).unwrap();
        let state = &mut snapshot["state"];
        match mode {
            "partial_value" => state["wastes"][0]["effect"]["after"]["value"] = json!("8100"),
            "full_loss" => state["wastes"][0]["effect"]["loss_value"] = json!("900"),
            "observation" => {
                state["wastes"][0]["effect"]["observation"] = json!({"id":Uuid::new_v4().to_string(),"quantity":"1000","recorded_at":"2026-01-01T00:00:00Z","seq":"1"})
            }
            "request" => {
                let c = state["wastes"][0]["confirmation_id"]
                    .as_str()
                    .unwrap()
                    .to_string();
                let row = state["business_confirmations"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|v| v["id"] == c)
                    .unwrap();
                row["request_id"] = json!(Uuid::new_v4().to_string());
            }
            "missing_record" => state["wastes"] = json!([]),
            "wrong_kind" => state["wastes"][0]["effect"]["reason"] = json!("consumption"),
            _ => unreachable!(),
        }
        let changed = serde_json::to_string_pretty(&snapshot).unwrap() + "\n";
        db.execute(
            "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=?2",
            rusqlite::params![changed, seq],
        )
        .unwrap();
        let path = root.path().join(format!("snapshots/{seq:020}.json"));
        fs::write(&path, &changed).unwrap();
        drop(db);
        assert!(Store::open(root.path()).is_err(), "{mode}");
        assert_eq!(fs::read_to_string(path).unwrap(), changed);
    }
}
