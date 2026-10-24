use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sunchang_hansang::{catalog::MaterialSummary, store::Store};
use tempfile::TempDir;
use uuid::Uuid;

fn decode<T: DeserializeOwned>(input: Value) -> T {
    serde_json::from_value(input).unwrap()
}
fn value(input: impl Serialize) -> Value {
    serde_json::to_value(input).unwrap()
}
fn meta(store: &Store) -> Value {
    let state = store.catalog_state().unwrap();
    json!({"request_id":Uuid::new_v4().to_string(),"epoch":state.epoch,"expected_revision":state.revision})
}
fn material(store: &mut Store, name: &str, unit: &str) -> MaterialSummary {
    let mut request = meta(store);
    request["name"] = json!(name);
    request["unit"] = json!(unit);
    request["kind"] = json!("raw");
    request["description"] = json!("가상 반품 후속 검증");
    store.catalog_create(decode(request)).unwrap().value
}
fn record(store: &mut Store, payload: Value, target: Option<&str>) -> Value {
    let request = json!({"meta":meta(store),"payload":payload,"corrects_confirmation_id":target});
    let saved = value(store.business_create(decode(request)).unwrap());
    let id = saved["result"]["item"]["id"].as_str().unwrap();
    let head = &saved["result"]["head"]["id"];
    let preview = value(
        store
            .business_preview(id, decode(json!({"expected_head_id":head})))
            .unwrap(),
    );
    assert_eq!(preview["can_confirm"], true, "{preview}");
    let request = json!({"meta":meta(store),"expected_head_id":head});
    value(store.business_confirm(id, decode(request)).unwrap())
}
fn result_id<'a>(done: &'a Value, field: &str) -> &'a str {
    done["result"]["confirmation"]["result"][field]
        .as_str()
        .unwrap()
}
fn target(done: &Value) -> &str {
    done["result"]["confirmation"]["id"].as_str().unwrap()
}
fn receipt_data(item: &MaterialSummary, quantity: &str, amount: &str) -> Value {
    json!({"source":{"kind":"direct","material_id":item.id,"material_version_id":item.version_id},"quantity":quantity,"unit":item.unit,"amount":amount,"receipt_date":"2020-02-29","note":"가상 원입고","stock_effect":{"kind":"unreflected"}})
}
fn receipt(store: &mut Store, item: &MaterialSummary, quantity: &str, amount: &str) -> Value {
    record(
        store,
        json!({"kind":"receipt","version":1,"data":receipt_data(item,quantity,amount)}),
        None,
    )
}
fn returned(store: &mut Store, receipt_id: &str, quantity: &str) -> Value {
    let source = value(store.purchase_return_source(receipt_id).unwrap().unwrap());
    record(
        store,
        json!({"kind":"purchase_return","version":1,"data":{
            "receipt_id":receipt_id,"expected_receipt_version_id":source["current"]["reference"]["version_id"],
            "quantity":quantity,"unit":source["current"]["unit"],"note":"가상 실제 반품"
        }}),
        None,
    )
}
fn correct(store: &mut Store, done: &Value, edit: impl FnOnce(&mut Value)) -> Value {
    let seed = value(store.correction_seed(target(done)).unwrap().unwrap());
    let mut data = seed["draft"].clone();
    data["reason"] = json!("가상 원기록 확인");
    edit(&mut data);
    record(
        store,
        json!({"kind":"correction","version":1,"data":data}),
        Some(target(done)),
    )
}
fn balance(store: &Store, item: &MaterialSummary, q: &str, v: &str) {
    let h = value(store.holding_detail(&item.id).unwrap().unwrap());
    assert_eq!(h["settlement_basis"]["quantity"], q, "{h}");
    assert_eq!(h["settlement_basis"]["value"], v, "{h}");
}
fn detail(store: &Store, done: &Value) -> Value {
    value(
        store
            .purchase_return_detail(result_id(done, "purchase_return_id"))
            .unwrap()
            .unwrap(),
    )
}

#[test]
fn void_return_restores_with_current_source_version_and_preserves_original_audit_input() {
    let dir = TempDir::new().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let item = material(&mut store, "가상 반품 복원", "g");
    let received = receipt(&mut store, &item, "10", "100");
    let receipt_id = result_id(&received, "receipt_id");
    let done = returned(&mut store, receipt_id, "4");
    let original = detail(&store, &done)["item"].clone();
    let first_seed = value(store.correction_seed(target(&done)).unwrap().unwrap());
    correct(&mut store, &done, |d| d["action"] = json!({"kind":"void"}));
    correct(&mut store, &received, |d| {
        d["action"]["replacement"]["data"]["amount"] = json!("200")
    });
    balance(&store, &item, "10", "200");
    let state = value(store.catalog_state().unwrap());
    let seed = value(store.correction_seed(target(&done)).unwrap().unwrap());
    assert_eq!(value(store.catalog_state().unwrap()), state);
    assert_eq!(seed["original"], first_seed["original"]);
    assert!(seed["current"]["input"].is_null());
    let source = value(store.purchase_return_source(receipt_id).unwrap().unwrap());
    let template = &seed["replacement_template"];
    assert_eq!(template["data"]["receipt_id"], receipt_id);
    assert_eq!(template["data"]["quantity"], "4");
    assert_eq!(
        template["data"]["expected_receipt_version_id"],
        source["current"]["reference"]["version_id"]
    );
    assert_ne!(
        template["data"]["expected_receipt_version_id"],
        first_seed["original"]["input"]["data"]["expected_receipt_version_id"]
    );
    correct(&mut store, &done, |d| {
        d["action"] = json!({"kind":"replace","replacement":template})
    });
    balance(&store, &item, "6", "120");
    let after = detail(&store, &done);
    assert_eq!(after["item"], original);
    assert_eq!(after["item"]["effect"]["deducted_value"], "40");
    assert_eq!(after["effective"]["effect"]["deducted_value"], "80");
    assert_eq!(
        after["effective"]["effect"]["source"]["original_amount"],
        "100"
    );
    assert_eq!(after["effective"]["effect"]["source"]["amount"], "200");
    drop(store);
    let store = Store::open(dir.path()).unwrap();
    balance(&store, &item, "6", "120");
    assert_eq!(detail(&store, &done), after);
}

#[test]
fn void_source_leaves_restore_template_unselected_without_rewriting_original_or_draft() {
    let dir = TempDir::new().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let item = material(&mut store, "가상 취소 원입고", "g");
    let received = receipt(&mut store, &item, "10", "100");
    let done = returned(&mut store, result_id(&received, "receipt_id"), "2");
    let original =
        value(store.correction_seed(target(&done)).unwrap().unwrap())["original"].clone();
    correct(&mut store, &done, |d| d["action"] = json!({"kind":"void"}));
    correct(&mut store, &received, |d| {
        d["action"] = json!({"kind":"void"})
    });
    let before = value(store.catalog_state().unwrap());
    let seed = value(store.correction_seed(target(&done)).unwrap().unwrap());
    assert_eq!(value(store.catalog_state().unwrap()), before);
    assert_eq!(seed["original"], original);
    assert_eq!(
        seed["replacement_template"]["data"]["receipt_id"],
        result_id(&received, "receipt_id")
    );
    assert!(seed["replacement_template"]["data"]["expected_receipt_version_id"].is_null());
    assert_eq!(seed["replacement_template"]["data"]["quantity"], "2");
    assert_eq!(seed["draft"]["action"]["kind"], "void");
    let source = value(
        store
            .purchase_return_source(result_id(&received, "receipt_id"))
            .unwrap()
            .unwrap(),
    );
    assert!(source["current"].is_null());
    drop(store);
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(
        value(store.correction_seed(target(&done)).unwrap().unwrap()),
        seed
    );
}

#[test]
fn return_after_receipt_material_correction_keeps_original_quantity_unit_and_amount() {
    let dir = TempDir::new().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let weight = material(&mut store, "가상 질량 원입고", "g");
    let count = material(&mut store, "가상 수량 정정품", "개");
    let received = receipt(&mut store, &weight, "10", "100");
    correct(&mut store, &received, |d| {
        d["action"]["replacement"]["data"] = receipt_data(&count, "5", "50")
    });
    let done = returned(&mut store, result_id(&received, "receipt_id"), "1");
    let result = detail(&store, &done);
    let effect = &result["item"]["effect"];
    assert_eq!(effect["unit"], "개");
    assert_eq!(effect["source"]["original_unit"], "g");
    assert_eq!(effect["source"]["original_quantity"], "10");
    assert_eq!(effect["source"]["original_amount"], "100");
    assert_eq!(effect["source"]["quantity"], "5");
    assert_eq!(effect["source"]["amount"], "50");
    assert_eq!(effect["deducted_value"], "10");
    balance(&store, &count, "4", "40");
    drop(store);
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(detail(&store, &done), result);
}

#[test]
fn delayed_amount_splits_returned_share_and_next_stocktake_excludes_returns_once() {
    let dir = TempDir::new().unwrap();
    let mut store = Store::open(dir.path()).unwrap();
    let item = material(&mut store, "가상 반품 포함 보정", "g");
    let received = receipt(&mut store, &item, "4", "4");
    let first = record(
        &mut store,
        json!({"kind":"stocktake","version":1,"data":{"rows":[{
        "material_id":item.id,"material_version_id":item.version_id,"quantity":"10","unit":"g","initial_value":null,
        "excess":{"reviewed":true,"reason":"가상 실물 확인","value":{"kind":"existing_price"}}
    }],"note":"가상 시작 확인"}}),
        None,
    );
    let observed = value(
        store
            .stocktake_detail(result_id(&first, "stocktake_id"))
            .unwrap()
            .unwrap(),
    );
    let done = returned(&mut store, result_id(&received, "receipt_id"), "4");
    let original = detail(&store, &done)["item"].clone();
    balance(&store, &item, "6", "6");
    let mut late = receipt_data(&item, "3", "6");
    late["stock_effect"] = json!({"kind":"reconciled","selections":[{
        "observation_version_id":observed["observations"][0]["version_id"],"source":{"kind":"all_available"},"included_quantity":"3"
    }]});
    let late = record(
        &mut store,
        json!({"kind":"receipt","version":1,"data":late}),
        None,
    );
    balance(&store, &item, "6", "7.8");
    let late_detail = value(
        store
            .receipt_detail(result_id(&late, "receipt_id"))
            .unwrap()
            .unwrap(),
    );
    assert_eq!(
        late_detail["reconciliation"]["effect"]["separate_adjustment"],
        "1.2"
    );
    assert_eq!(detail(&store, &done)["item"], original);
    // This fully included receipt has U=0, but its actual R=3 remains the cap.
    let second = returned(&mut store, result_id(&late, "receipt_id"), "1");
    assert_eq!(
        detail(&store, &second)["item"]["effect"]["deducted_value"],
        "1.3"
    );
    balance(&store, &item, "5", "6.5");
    let measured = record(
        &mut store,
        json!({"kind":"stocktake","version":1,"data":{"rows":[{
        "material_id":item.id,"material_version_id":item.version_id,"quantity":"4","unit":"g","initial_value":null,"excess":null
    }],"note":"가상 마지막 실사"}}),
        None,
    );
    let measured = value(
        store
            .stocktake_detail(result_id(&measured, "stocktake_id"))
            .unwrap()
            .unwrap(),
    );
    assert_eq!(
        measured["observations"][0]["effect"]["calculation"]["consumption_quantity"],
        "1"
    );
    assert_eq!(
        measured["observations"][0]["effect"]["calculation"]["consumption_value"],
        "1.3"
    );
    balance(&store, &item, "4", "5.2");
    drop(store);
    let store = Store::open(dir.path()).unwrap();
    balance(&store, &item, "4", "5.2");
    assert_eq!(detail(&store, &done)["item"], original);
}
