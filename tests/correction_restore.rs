use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sunchang_hansang::{catalog::MaterialSummary, store::Store};
use tempfile::TempDir;

fn decode<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}
fn value(value: impl Serialize) -> Value {
    serde_json::to_value(value).unwrap()
}
fn meta(store: &Store) -> Value {
    let state = store.catalog_state().unwrap();
    json!({"request_id":uuid::Uuid::new_v4().to_string(),"epoch":state.epoch,"expected_revision":state.revision})
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
fn confirm(store: &mut Store, saved: &Value) -> Value {
    let id = saved["result"]["item"]["id"].as_str().unwrap();
    let preview = value(
        store
            .business_preview(
                id,
                decode(json!({"expected_head_id":saved["result"]["head"]["id"]})),
            )
            .unwrap(),
    );
    assert_eq!(preview["can_confirm"], true, "{preview}");
    value(
        store
            .business_confirm(
                id,
                decode(
                    json!({"meta":meta(store),"expected_head_id":saved["result"]["head"]["id"]}),
                ),
            )
            .unwrap(),
    )
}
fn item(store: &mut Store, name: &str) -> MaterialSummary {
    let mut request = meta(store);
    request["name"] = json!(name);
    request["unit"] = json!("g");
    request["kind"] = json!("raw");
    request["description"] = json!("");
    store.catalog_create(decode(request)).unwrap().value
}

#[test]
fn whole_stocktake_void_can_be_replaced_from_public_seed_without_changing_original() {
    let dir = TempDir::new().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let positive = item(&mut store, "가상 복원 양수");
    let zero = item(&mut store, "가상 복원 실제 0");
    let saved = save(
        &mut store,
        json!({"kind":"stocktake","version":1,"data":{"rows":[
        {"material_id":positive.id,"material_version_id":positive.version_id,"quantity":"10","unit":"g","initial_value":{"kind":"manual_amount","amount":"100"},"excess":null},
        {"material_id":zero.id,"material_version_id":zero.version_id,"quantity":"0","unit":"g","initial_value":{"kind":"manual_amount","amount":"0"},"excess":null}
    ],"note":"가상 초기 실사"}}),
        None,
    );
    let done = confirm(&mut store, &saved);
    let target = done["result"]["confirmation"]["id"].as_str().unwrap();
    let first = value(store.correction_seed(target).unwrap().unwrap());
    let cancelled = save(
        &mut store,
        json!({"kind":"correction","version":1,"data":{
            "expected_target_version_id":first["current"]["reference"]["version_id"],"reason":"가상 전체 취소","action":{"kind":"void"},"observation_reviews":[],"linked_receipt_changes":[]
        }}),
        Some(target),
    );
    confirm(&mut store, &cancelled);
    let before = value(store.catalog_state().unwrap());
    let seed = value(store.correction_seed(target).unwrap().unwrap());
    assert_eq!(value(store.catalog_state().unwrap()), before);
    assert_eq!(seed["original"], first["original"]);
    assert!(seed["current"]["input"].is_null());
    assert!(seed["current"]["value"].is_null());
    assert_eq!(
        seed["replacement_template"]["kind"], "stocktake",
        "void seed must expose a separate editable template: {seed}"
    );
    let rows = seed["replacement_template"]["data"]["rows"]
        .as_array()
        .unwrap();
    assert_eq!(rows.len(), 2);
    let db = rusqlite::Connection::open(dir.path().join("app.sqlite3")).unwrap();
    for (position, row) in rows.iter().enumerate() {
        let expected = row["expected_version_id"].as_str().unwrap();
        assert_ne!(
            row["expected_version_id"],
            first["original"]["input"]["data"]["rows"][position]["expected_version_id"]
        );
        let count:i64=db.query_row("SELECT COUNT(*) FROM correction_projection_versions WHERE id=?1 AND kind='observation' AND subject_id=?2",[expected,row["observation_id"].as_str().unwrap()],|r|r.get(0)).unwrap();
        assert_eq!(
            count, 1,
            "template uses an existing tombstone, never a future UUID"
        );
    }
    let restored = save(
        &mut store,
        json!({"kind":"correction","version":1,"data":{
            "expected_target_version_id":seed["current"]["reference"]["version_id"],"reason":"가상 취소 후 입력 재확인","action":{"kind":"replace","replacement":seed["replacement_template"]},"observation_reviews":[],"linked_receipt_changes":[]
        }}),
        Some(target),
    );
    confirm(&mut store, &restored);
    let quantities = [(&positive, "10", "100"), (&zero, "0", "0")];
    for (material, quantity, amount) in quantities {
        let holding = value(store.holding_detail(&material.id).unwrap().unwrap());
        assert_eq!(holding["settlement_basis"]["quantity"], quantity);
        assert_eq!(holding["settlement_basis"]["value"], amount);
    }
    assert_eq!(
        value(store.correction_seed(target).unwrap().unwrap())["original"],
        first["original"]
    );
    drop(db);
    drop(store);
    let reopened = Store::open(dir.path()).unwrap();
    assert_eq!(
        value(reopened.holding_detail(&zero.id).unwrap().unwrap())["settlement_basis"]["quantity"],
        "0"
    );
}
