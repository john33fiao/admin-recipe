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

fn material(store: &mut Store, name: &str) -> MaterialSummary {
    let mut input = meta(store);
    input["name"] = json!(name);
    input["unit"] = json!("g");
    input["kind"] = json!("raw");
    input["description"] = json!("가상 정정 후속 저장 검증");
    store.catalog_create(decode(input)).unwrap().value
}

fn save(store: &mut Store, payload: Value, corrects: Option<&str>) -> Value {
    let input = json!({"meta":meta(store),"payload":payload,"corrects_confirmation_id":corrects});
    value(store.business_create(decode(input)).unwrap())
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
    let request = json!({"meta":meta(store),"expected_head_id":preview["head_id"]});
    value(store.business_confirm(id, decode(request)).unwrap())
}

fn record(store: &mut Store, payload: Value) -> Value {
    let draft = save(store, payload, None);
    confirm(store, &draft)
}

fn result_id<'a>(confirmed: &'a Value, field: &str) -> &'a str {
    confirmed["result"]["confirmation"]["result"][field]
        .as_str()
        .unwrap()
}

fn original_confirmation(confirmed: &Value) -> &str {
    confirmed["result"]["confirmation"]["id"].as_str().unwrap()
}

fn receipt(store: &mut Store, item: &MaterialSummary, quantity: &str, amount: &str) -> Value {
    record(
        store,
        json!({"kind":"receipt","version":1,"data":{
            "source":{"kind":"direct","material_id":item.id,"material_version_id":item.version_id},
            "quantity":quantity,"unit":"g","amount":amount,"receipt_date":"2020-02-29",
            "note":"가상 원입고","stock_effect":{"kind":"unreflected"}
        }}),
    )
}

fn correct(store: &mut Store, original: &Value, edit: impl FnOnce(&mut Value)) -> Value {
    let target = original_confirmation(original);
    let seed = value(store.correction_seed(target).unwrap().unwrap());
    let mut draft = seed["draft"].clone();
    draft["reason"] = json!("가상 원입력 확인 후 정정");
    edit(&mut draft);
    let saved = save(
        store,
        json!({"kind":"correction","version":1,"data":draft}),
        Some(target),
    );
    confirm(store, &saved)
}

fn production(
    store: &mut Store,
    input: &MaterialSummary,
    input_quantity: &str,
    output: &MaterialSummary,
    output_quantity: &str,
) -> Value {
    record(
        store,
        json!({"kind":"production","version":1,"data":{
            "inputs":[{"material_id":input.id,"material_version_id":input.version_id,"quantity":input_quantity,"unit":"g"}],
            "outputs":[{"material_id":output.id,"material_version_id":output.version_id,"quantity":output_quantity,"unit":"g","allocation_percent":"100"}],
            "note":"가상 실제 생산"
        }}),
    )
}

fn waste(store: &mut Store, item: &MaterialSummary, quantity: &str) -> Value {
    record(
        store,
        json!({"kind":"waste","version":1,"data":{
            "material_id":item.id,"material_version_id":item.version_id,"quantity":quantity,
            "unit":"g","reason":"discard","note":"가상 명시 폐기"
        }}),
    )
}

fn holding(store: &Store, item: &MaterialSummary) -> Value {
    value(store.holding_detail(&item.id).unwrap().unwrap())
}

fn assert_balance(store: &Store, item: &MaterialSummary, quantity: &str, amount: &str) {
    let detail = holding(store, item);
    assert_eq!(detail["settlement_basis"]["quantity"], quantity, "{detail}");
    assert_eq!(detail["settlement_basis"]["value"], amount, "{detail}");
}

fn assert_unknown(store: &Store, item: &MaterialSummary) {
    let detail = holding(store, item);
    assert!(detail["settlement_basis"].is_null(), "{detail}");
    assert!(detail["basis"].is_null(), "{detail}");
    assert!(detail["last_valid_price"].is_null(), "{detail}");
    assert_eq!(detail["price"]["source"], "missing", "{detail}");
}

#[test]
fn corrected_receipt_basis_survives_normal_production_waste_and_restart() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 정정 원료");
    let prepared = material(&mut store, "가상 후속 결과");
    let received = receipt(&mut store, &raw, "10", "100");
    let receipt_id = result_id(&received, "receipt_id");
    let original = value(store.receipt_detail(receipt_id).unwrap().unwrap())["item"].clone();
    correct(&mut store, &received, |draft| {
        draft["action"]["replacement"]["data"]["quantity"] = json!("12");
        draft["action"]["replacement"]["data"]["amount"] = json!("240");
    });
    assert_balance(&store, &raw, "12", "240");
    let produced = production(&mut store, &raw, "2", &prepared, "1");
    let production_id = result_id(&produced, "production_id");
    let production_before = value(store.production_detail(production_id).unwrap().unwrap());
    assert_eq!(production_before["item"]["total_input_value"], "40");
    let discarded = waste(&mut store, &raw, "1");
    let waste_id = result_id(&discarded, "waste_id");
    let waste_before = value(store.waste_detail(waste_id).unwrap().unwrap());
    assert_eq!(waste_before["item"]["effect"]["before"]["quantity"], "10");
    assert_eq!(waste_before["item"]["effect"]["before"]["value"], "200");
    assert_eq!(waste_before["item"]["effect"]["loss_value"], "0");
    assert_balance(&store, &raw, "9", "200");
    assert_balance(&store, &prepared, "1", "40");
    let state = value(store.catalog_state().unwrap());
    drop(store);

    let store = Store::open(root.path()).unwrap();
    assert_eq!(value(store.catalog_state().unwrap()), state);
    assert_balance(&store, &raw, "9", "200");
    assert_balance(&store, &prepared, "1", "40");
    assert_eq!(
        value(store.receipt_detail(receipt_id).unwrap().unwrap())["item"],
        original
    );
    assert_eq!(
        value(store.production_detail(production_id).unwrap().unwrap())["item"],
        production_before["item"]
    );
    assert_eq!(
        value(store.waste_detail(waste_id).unwrap().unwrap())["item"],
        waste_before["item"]
    );
}

#[test]
fn corrected_observation_basis_keeps_logical_time_through_production_waste_and_restart() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 실사 원료");
    let prepared = material(&mut store, "가상 실사 후 결과");
    receipt(&mut store, &raw, "10", "100");
    let observed = record(
        &mut store,
        json!({"kind":"stocktake","version":1,"data":{"rows":[{
            "material_id":raw.id,"material_version_id":raw.version_id,"quantity":"8","unit":"g","initial_value":null,"excess":null
        }],"note":"가상 실제 확인"}}),
    );
    let original_observation = holding(&store, &raw)["observation"].clone();
    correct(&mut store, &observed, |draft| {
        draft["action"]["replacement"]["data"]["rows"][0]["action"]["quantity"] = json!("6");
    });
    assert_balance(&store, &raw, "6", "60");
    let corrected_observation = holding(&store, &raw)["observation"].clone();
    assert_eq!(corrected_observation["quantity"], "6");
    assert_ne!(corrected_observation["id"], original_observation["id"]);
    assert_eq!(corrected_observation["seq"], original_observation["seq"]);
    assert_eq!(
        corrected_observation["recorded_at"],
        original_observation["recorded_at"]
    );
    let produced = production(&mut store, &raw, "2", &prepared, "1");
    let production_id = result_id(&produced, "production_id");
    let production_before = value(store.production_detail(production_id).unwrap().unwrap());
    assert_eq!(production_before["item"]["total_input_value"], "20");
    let raw_effect = production_before["materials"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["material_id"] == raw.id)
        .unwrap();
    assert_eq!(raw_effect["observation"], corrected_observation);
    let discarded = waste(&mut store, &raw, "1");
    let waste_id = result_id(&discarded, "waste_id");
    let waste_before = value(store.waste_detail(waste_id).unwrap().unwrap());
    assert_eq!(
        waste_before["item"]["effect"]["observation"],
        corrected_observation
    );
    assert_balance(&store, &raw, "3", "40");
    drop(store);

    let store = Store::open(root.path()).unwrap();
    assert_balance(&store, &raw, "3", "40");
    assert_balance(&store, &prepared, "1", "20");
    assert_eq!(holding(&store, &raw)["observation"], corrected_observation);
    assert_eq!(
        value(store.production_detail(production_id).unwrap().unwrap())["item"],
        production_before["item"]
    );
    assert_eq!(
        value(store.waste_detail(waste_id).unwrap().unwrap())["item"],
        waste_before["item"]
    );
}

#[test]
fn voided_initial_receipt_then_new_receipt_does_not_restore_old_last_valid_price() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 취소 뒤 새 입고");
    let original = receipt(&mut store, &raw, "10", "100");
    correct(&mut store, &original, |draft| {
        draft["action"] = json!({"kind":"void"})
    });
    assert_unknown(&store, &raw);
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    assert_unknown(&store, &raw);
    receipt(&mut store, &raw, "2", "6");
    assert_balance(&store, &raw, "2", "6");
    let discarded = waste(&mut store, &raw, "2");
    let waste_id = result_id(&discarded, "waste_id");
    assert_eq!(
        store
            .waste_detail(waste_id)
            .unwrap()
            .unwrap()
            .item
            .effect
            .loss_value,
        "6"
    );
    assert_balance(&store, &raw, "0", "0");
    let current = holding(&store, &raw);
    assert_eq!(current["last_valid_price"]["amount"], "6");
    assert_eq!(current["last_valid_price"]["quantity"], "2");
    drop(store);
    let store = Store::open(root.path()).unwrap();
    assert_balance(&store, &raw, "0", "0");
    assert_eq!(
        holding(&store, &raw)["last_valid_price"],
        current["last_valid_price"]
    );
    let seed = value(
        store
            .correction_seed(original_confirmation(&original))
            .unwrap()
            .unwrap(),
    );
    assert!(seed["current"]["value"].is_null());
    assert_eq!(seed["original"]["input"]["data"]["amount"], "100");
}

#[test]
fn voided_output_material_starts_from_unknown_in_followup_production_and_restart() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let source = material(&mut store, "가상 정상 투입");
    let output = material(&mut store, "가상 취소된 결과 품목");
    receipt(&mut store, &source, "2", "20");
    let original = receipt(&mut store, &output, "10", "100");
    correct(&mut store, &original, |draft| {
        draft["action"] = json!({"kind":"void"})
    });
    assert_unknown(&store, &output);
    let produced = production(&mut store, &source, "1", &output, "2");
    let production_id = result_id(&produced, "production_id");
    let detail = value(store.production_detail(production_id).unwrap().unwrap());
    let effect = detail["materials"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["material_id"] == output.id)
        .unwrap();
    assert!(effect["before"].is_null(), "{effect}");
    assert!(effect["basis_before"].is_null(), "{effect}");
    assert_eq!(effect["output_quantity"], "2");
    assert_eq!(effect["output_value"], "10");
    assert_balance(&store, &output, "2", "10");
    let discarded = waste(&mut store, &output, "2");
    let waste_id = result_id(&discarded, "waste_id");
    assert_eq!(
        store
            .waste_detail(waste_id)
            .unwrap()
            .unwrap()
            .item
            .effect
            .loss_value,
        "10"
    );
    let current = holding(&store, &output);
    assert_eq!(current["last_valid_price"]["amount"], "10");
    assert_eq!(current["last_valid_price"]["quantity"], "2");
    drop(store);

    let store = Store::open(root.path()).unwrap();
    assert_balance(&store, &source, "1", "10");
    assert_balance(&store, &output, "0", "0");
    assert_eq!(
        holding(&store, &output)["last_valid_price"],
        current["last_valid_price"]
    );
    assert_eq!(
        value(store.production_detail(production_id).unwrap().unwrap())["item"],
        detail["item"]
    );
}

#[test]
fn earlier_production_and_waste_keep_frozen_costs_after_an_upstream_receipt_correction() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 과거 원료");
    let prepared = material(&mut store, "가상 과거 생산 결과");
    let received = receipt(&mut store, &raw, "10", "100");
    let produced = production(&mut store, &raw, "2", &prepared, "1");
    let production_id = result_id(&produced, "production_id");
    let original_production =
        value(store.production_detail(production_id).unwrap().unwrap())["item"].clone();
    let discarded = waste(&mut store, &raw, "1");
    let waste_id = result_id(&discarded, "waste_id");
    let original_waste = value(store.waste_detail(waste_id).unwrap().unwrap())["item"].clone();
    assert_eq!(original_production["total_input_value"], "20");
    assert_eq!(original_waste["effect"]["after"]["value"], "80");
    correct(&mut store, &received, |draft| {
        draft["action"]["replacement"]["data"]["amount"] = json!("200");
    });
    assert_balance(&store, &raw, "7", "160");
    assert_balance(&store, &prepared, "1", "40");
    assert_eq!(
        value(store.production_detail(production_id).unwrap().unwrap())["item"],
        original_production
    );
    assert_eq!(
        value(store.waste_detail(waste_id).unwrap().unwrap())["item"],
        original_waste
    );
    drop(store);

    let store = Store::open(root.path()).unwrap();
    assert_balance(&store, &raw, "7", "160");
    assert_balance(&store, &prepared, "1", "40");
    assert_eq!(
        value(store.production_detail(production_id).unwrap().unwrap())["item"],
        original_production
    );
    assert_eq!(
        value(store.waste_detail(waste_id).unwrap().unwrap())["item"],
        original_waste
    );
}

#[test]
fn zero_recipe_revised_to_five_keeps_settled_sale_and_allows_archived_history_correction() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 분량 변경 재료");
    receipt(&mut store, &raw, "10", "100");
    let data = json!({"basis_count":"1","reference_output":null,"selling_price":"100",
        "cooking_description":"가상 원배합","process":"","reference_time":"",
        "lines":[{"material_id":raw.id,"material_version_id":raw.version_id,
            "quantity":"0","unit":"g","note":"가상 직접 참조"}]});
    let menu = value(store.recipe_create(decode(json!({"meta":meta(&store),
        "owner":{"mode":"new","name":"가상 분량 변경 메뉴","unit":"개","kind":"menu","description":""},
        "data":data}))).unwrap());
    let sale_payload = |recipe: &Value| {
        json!({"kind":"sale","version":1,"data":{
        "material_id":recipe["item"]["id"],"material_version_id":recipe["item"]["version_id"],
        "recipe_version_id":recipe["version"]["id"],"count":"1","note":"가상 판매"}})
    };
    let first = record(&mut store, sale_payload(&menu));
    let first_id = result_id(&first, "sale_id");
    let observed = record(
        &mut store,
        json!({"kind":"stocktake","version":1,"data":{
        "rows":[{"material_id":raw.id,"material_version_id":raw.version_id,"quantity":"8",
            "unit":"g","initial_value":null,"excess":null}],"note":"가상 배분 확인"}}),
    );
    let observation_id = result_id(&observed, "stocktake_id");
    let first_before = value(store.sale_detail(first_id).unwrap().unwrap());
    let observation_before = value(store.stocktake_detail(observation_id).unwrap().unwrap());
    assert_eq!(first_before["item"]["recipe_cost"]["total_cost"], "0");
    assert_eq!(first_before["consumption_status"], "settled");
    assert_eq!(
        observation_before["observations"][0]["effect"]["calculation"]["consumption_value"],
        "20"
    );

    let mut changed_data = data.clone();
    changed_data["lines"][0]["quantity"] = json!("5");
    changed_data["selling_price"] = json!("200");
    let revised = value(
        store
            .recipe_revise(
                menu["item"]["id"].as_str().unwrap(),
                decode(json!({
        "meta":meta(&store),"expected_material_version_id":menu["item"]["version_id"],
        "expected_recipe_version_id":menu["version"]["id"],"data":changed_data})),
            )
            .unwrap(),
    );
    let second = record(&mut store, sale_payload(&revised));
    let second_id = result_id(&second, "sale_id");
    let second_before = value(store.sale_detail(second_id).unwrap().unwrap());
    assert_eq!(second_before["item"]["recipe_cost"]["total_cost"], "50");
    assert_eq!(
        second_before["item"]["recipe_cost"]["rows"][0]["numerator_quantity"],
        "5"
    );
    let first_after = value(store.sale_detail(first_id).unwrap().unwrap());
    for field in [
        "item",
        "head",
        "versions",
        "consumption",
        "consumption_status",
        "effective",
    ] {
        assert_eq!(first_before[field], first_after[field], "{field}");
    }
    let observation_after = value(store.stocktake_detail(observation_id).unwrap().unwrap());
    assert_eq!(observation_before["item"], observation_after["item"]);
    assert_eq!(
        observation_before["observations"],
        observation_after["observations"]
    );
    assert_balance(&store, &raw, "8", "80");

    for (id, version) in [
        (raw.id.as_str(), raw.version_id.as_str()),
        (
            revised["item"]["id"].as_str().unwrap(),
            revised["item"]["version_id"].as_str().unwrap(),
        ),
    ] {
        let mut request = meta(&store);
        request["expected_version_id"] = json!(version);
        assert!(
            store
                .catalog_archive(id, decode(request))
                .unwrap()
                .value
                .archived
        );
    }
    correct(&mut store, &first, |draft| {
        draft["action"]["replacement"]["data"]["count"] = json!("2");
    });
    let corrected = value(store.sale_detail(first_id).unwrap().unwrap());
    assert_eq!(corrected["item"], first_before["item"]);
    assert_eq!(corrected["effective"]["count"], "2");
    assert_eq!(
        value(store.sale_detail(second_id).unwrap().unwrap())["item"],
        second_before["item"]
    );
    assert_balance(&store, &raw, "8", "80");
    drop(store);
    let store = Store::open(root.path()).unwrap();
    assert_eq!(
        value(store.sale_detail(first_id).unwrap().unwrap()),
        corrected
    );
    assert_balance(&store, &raw, "8", "80");
}
