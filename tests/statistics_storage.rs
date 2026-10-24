use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{fs, path::Path};
use sunchang_hansang::{catalog::MaterialSummary, statistics::StatisticsQuery, store::Store};
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
    let mut request = meta(store);
    request["name"] = json!(name);
    request["unit"] = json!("g");
    request["kind"] = json!("raw");
    request["description"] = json!("가상 통계 검증");
    store.catalog_create(decode(request)).unwrap().value
}
fn run(store: &mut Store, payload: Value, target: Option<&str>) -> Value {
    let saved = value(
        store
            .business_create(decode(
                json!({"meta":meta(store),"payload":payload,"corrects_confirmation_id":target}),
            ))
            .unwrap(),
    );
    let id = saved["result"]["item"]["id"].as_str().unwrap();
    let head = &saved["result"]["head"]["id"];
    let preview = value(
        store
            .business_preview(id, decode(json!({"expected_head_id":head})))
            .unwrap(),
    );
    assert_eq!(preview["can_confirm"], true, "{preview}");
    value(
        store
            .business_confirm(
                id,
                decode(json!({"meta":meta(store),"expected_head_id":head})),
            )
            .unwrap(),
    )
}
fn cid(done: &Value) -> &str {
    done["result"]["confirmation"]["id"].as_str().unwrap()
}
fn result_id<'a>(done: &'a Value, key: &str) -> &'a str {
    done["result"]["confirmation"]["result"][key]
        .as_str()
        .unwrap()
}
fn at(done: &Value) -> &str {
    done["result"]["confirmation"]["recorded_at"]
        .as_str()
        .unwrap()
}
fn row(item: &MaterialSummary, quantity: &str) -> Value {
    json!({"material_id":item.id,"material_version_id":item.version_id,"quantity":quantity,"unit":"g","initial_value":null,"excess":null})
}
fn initial(item: &MaterialSummary, quantity: &str, amount: &str) -> Value {
    let mut result = row(item, quantity);
    result["initial_value"] = json!({"kind":"manual_amount","amount":amount});
    result
}
fn stocktake(store: &mut Store, rows: Vec<Value>) -> Value {
    run(
        store,
        json!({"kind":"stocktake","version":1,"data":{"rows":rows,"note":"가상 확인"}}),
        None,
    )
}
fn receipt_data(item: &MaterialSummary, quantity: &str, amount: &str) -> Value {
    json!({"source":{"kind":"direct","material_id":item.id,"material_version_id":item.version_id},"quantity":quantity,"unit":"g","amount":amount,"receipt_date":"2020-02-29","stock_effect":{"kind":"unreflected"},"note":"가상 입고"})
}
fn receipt(store: &mut Store, item: &MaterialSummary, quantity: &str, amount: &str) -> Value {
    run(
        store,
        json!({"kind":"receipt","version":1,"data":receipt_data(item,quantity,amount)}),
        None,
    )
}
fn recipe(
    store: &mut Store,
    raw: &MaterialSummary,
    name: &str,
    price: Option<&str>,
    quantity: &str,
) -> Value {
    value(store.recipe_create(decode(json!({"meta":meta(store),"owner":{"mode":"new","name":name,"unit":"개","kind":"menu","description":"가상 메뉴"},"data":{"basis_count":"1","reference_output":null,"selling_price":price,"cooking_description":"","process":"","reference_time":"","lines":[{"material_id":raw.id,"material_version_id":raw.version_id,"quantity":quantity,"unit":"g","note":""}]}}))).unwrap())
}
fn sale(store: &mut Store, recipe: &Value, count: &str, target: Option<&str>) -> Value {
    run(
        store,
        json!({"kind":"sale","version":1,"data":{"material_id":recipe["item"]["id"],"material_version_id":recipe["item"]["version_id"],"recipe_version_id":recipe["version"]["id"],"count":count,"note":"가상 판매"}}),
        target,
    )
}
fn correct(store: &mut Store, done: &Value, action: Value) -> Value {
    let seed = value(store.correction_seed(cid(done)).unwrap().unwrap());
    let mut data = seed["draft"].clone();
    data["reason"] = json!("가상 정정 확인");
    data["action"] = action;
    run(
        store,
        json!({"kind":"correction","version":1,"data":data}),
        Some(cid(done)),
    )
}
fn report(store: &Store, from: &str, until: &str) -> Value {
    value(
        store
            .statistics(StatisticsQuery {
                from: from.into(),
                until: until.into(),
            })
            .unwrap(),
    )
}
fn all(store: &Store) -> Value {
    report(store, "2000-01-01T00:00:00Z", "2100-01-01T00:00:00Z")
}
fn journals(root: &Path) -> Vec<Vec<u8>> {
    let mut paths = fs::read_dir(root.join("snapshots"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect::<Vec<_>>();
    paths.sort();
    paths.into_iter().map(|p| fs::read(p).unwrap()).collect()
}
fn find<'a>(rows: &'a Value, id: &str) -> &'a Value {
    rows.as_array()
        .unwrap()
        .iter()
        .find(|r| r["origin"]["id"] == id)
        .unwrap()
}

#[test]
fn empty_and_outside_period_have_different_coverage_without_writes() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let empty = all(&store);
    assert_eq!(
        empty["sales"]["coverage"],
        json!({"matching_records":0,"all_time_records":0})
    );
    let raw = material(&mut store, "가상 범위 원료");
    receipt(&mut store, &raw, "1", "0");
    let before = fs::read(root.path().join("app.sqlite3")).unwrap();
    let bytes = journals(root.path());
    let state = value(store.catalog_state().unwrap());
    let outside = report(&store, "2000-01-01T00:00:00Z", "2001-01-01T00:00:00Z");
    assert_eq!(
        outside["operations"]["coverage"],
        json!({"matching_records":0,"all_time_records":1})
    );
    assert_eq!(outside["state"], state);
    assert_eq!(fs::read(root.path().join("app.sqlite3")).unwrap(), before);
    assert_eq!(journals(root.path()), bytes);
}

#[test]
fn mixed_work_preserves_consumption_identity_and_keeps_operations_separate() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 혼합 원료");
    let output = material(&mut store, "가상 생산 결과");
    stocktake(&mut store, vec![initial(&raw, "10000", "10000")]);
    let a = recipe(&mut store, &raw, "가상 메뉴 A", Some("1000"), "1");
    let b = recipe(&mut store, &raw, "가상 메뉴 B", Some("1000"), "1");
    let sold_a = sale(&mut store, &a, "3", None);
    let sold_b = sale(&mut store, &b, "1", None);
    let received = receipt(&mut store, &raw, "2000", "2000");
    run(
        &mut store,
        json!({"kind":"production","version":1,"data":{"inputs":[{"material_id":raw.id,"material_version_id":raw.version_id,"quantity":"2000","unit":"g"}],"outputs":[{"material_id":output.id,"material_version_id":output.version_id,"quantity":"2000","unit":"g","allocation_percent":"100"}],"note":"가상 손질"}}),
        None,
    );
    let receipt_id = result_id(&received, "receipt_id");
    let source = value(store.purchase_return_source(receipt_id).unwrap().unwrap());
    run(
        &mut store,
        json!({"kind":"purchase_return","version":1,"data":{"receipt_id":receipt_id,"expected_receipt_version_id":source["current"]["reference"]["version_id"],"quantity":"1000","unit":"g","note":"가상 반품"}}),
        None,
    );
    run(
        &mut store,
        json!({"kind":"waste","version":1,"data":{"material_id":raw.id,"material_version_id":raw.version_id,"quantity":"1000","unit":"g","reason":"staff_meal","note":"가상 직원식"}}),
        None,
    );
    let observed = stocktake(&mut store, vec![row(&raw, "6000")]);
    let bytes = journals(root.path());
    let db = fs::read(root.path().join("app.sqlite3")).unwrap();
    let result = all(&store);
    assert_eq!(result["sales"]["summary"]["count"], "4");
    assert_eq!(result["sales"]["summary"]["known_revenue"], "4000");
    let total = &result["observations"]["summary_by_material"][0];
    for (key, expected) in [
        ("consumption_quantity", "2000"),
        ("consumption_value", "2250"),
        ("allocated_quantity", "2000"),
        ("allocated_value", "2250"),
        ("unallocated_quantity", "0"),
    ] {
        assert_eq!(total[key], expected);
    }
    for (done, q, v) in [(&sold_a, "1500", "1687.5"), (&sold_b, "500", "562.5")] {
        let row = find(&result["sales"]["rows"], result_id(done, "sale_id"));
        assert_eq!(row["original"]["recipe_cost"]["total_cost"], "1");
        assert_eq!(row["consumption"][0]["quantity"], q);
        assert_eq!(row["consumption"][0]["value"], v);
        assert_eq!(
            row["consumption"][0]["stocktake_id"],
            result_id(&observed, "stocktake_id")
        );
    }
    assert_eq!(result["operations"]["rows"].as_array().unwrap().len(), 6);
    let waste = result["operations"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["origin"]["kind"] == "waste")
        .unwrap();
    assert_eq!(waste["value"]["result"]["loss_value"], "0");
    assert_eq!(journals(root.path()), bytes);
    assert_eq!(fs::read(root.path().join("app.sqlite3")).unwrap(), db);
    drop(store);
    let store = Store::open(root.path()).unwrap();
    assert_eq!(all(&store), result);
}

#[test]
fn sales_and_observation_periods_retain_outside_candidates_and_exact_boundaries() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 기간 경계");
    receipt(&mut store, &raw, "10", "10");
    let menu = recipe(&mut store, &raw, "가상 기간 메뉴", Some("10"), "0");
    let a = sale(&mut store, &menu, "3", None);
    let b = sale(&mut store, &menu, "1", None);
    let observed = stocktake(&mut store, vec![row(&raw, "6")]);
    let selected = report(&store, at(&a), at(&b));
    assert_eq!(selected["sales"]["summary"]["count"], "3");
    assert_eq!(selected["observations"]["rows"], json!([]));
    assert_eq!(
        selected["sales"]["rows"][0]["consumption"][0]["observation_in_period"],
        false
    );
    assert_eq!(
        selected["sales"]["rows"][0]["consumption"][0]["quantity"],
        "3"
    );
    let later = report(&store, at(&observed), "2100-01-01T00:00:00Z");
    assert_eq!(later["sales"]["rows"], json!([]));
    assert_eq!(
        later["observations"]["rows"][0]["allocation"]["candidates"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(
        later["observations"]["rows"][0]["candidate_links"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["sale_in_period"] == false)
    );
}

#[test]
fn missing_price_zero_count_initial_and_zero_weight_are_not_zero_measurements() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 영과 미계산");
    let blank = material(&mut store, "가상 미확인 행");
    let menu = recipe(&mut store, &raw, "가상 가격 미등록", None, "1");
    let first = sale(&mut store, &menu, "0", None);
    let before = all(&store);
    assert_eq!(before["sales"]["rows"][0]["value"]["amount"], Value::Null);
    assert_eq!(
        before["sales"]["rows"][0]["original"]["recipe_cost"]["total_cost"],
        Value::Null
    );
    assert_eq!(
        before["sales"]["rows"][0]["consumption"][0]["reason"],
        "no_effective_allocation"
    );
    stocktake(&mut store, vec![initial(&raw, "10", "10"), row(&blank, "")]);
    let initial_report = all(&store);
    assert_eq!(
        initial_report["observations"]["rows"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        initial_report["observations"]["rows"][0]["status"],
        "initial"
    );
    assert_eq!(
        initial_report["observations"]["rows"][0]["allocation"],
        Value::Null
    );
    assert_eq!(
        find(
            &initial_report["sales"]["rows"],
            result_id(&first, "sale_id")
        )["consumption"][0]["reason"],
        "initial_basis_missing"
    );
    let zero = sale(&mut store, &menu, "0", None);
    stocktake(&mut store, vec![row(&raw, "8")]);
    let result = all(&store);
    let zero = find(&result["sales"]["rows"], result_id(&zero, "sale_id"));
    assert_eq!(zero["consumption"][0]["reason"], "zero_weight");
    assert_eq!(zero["consumption"][0]["quantity"], Value::Null);
    assert_eq!(
        result["observations"]["summary_by_material"][0]["unallocated_quantity"],
        "2"
    );
    assert_eq!(result["sales"]["summary"]["missing_revenue_records"], 2);
}

#[test]
fn active_zero_revenue_and_wide_revenue_remain_exact() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 넓은 합계");
    let free = recipe(&mut store, &raw, "가상 무료 메뉴", Some("0"), "0");
    let high = recipe(
        &mut store,
        &raw,
        "가상 상한 메뉴",
        Some("9000000000000"),
        "0",
    );
    sale(&mut store, &free, "1", None);
    sale(&mut store, &high, "1", None);
    sale(&mut store, &high, "1", None);
    let result = all(&store);
    assert_eq!(
        result["sales"]["summary"]["known_revenue"],
        "18000000000000"
    );
    assert_eq!(result["sales"]["summary"]["missing_revenue_records"], 0);
    assert_eq!(result["sales"]["rows"][0]["value"]["amount"], "0");
}

#[test]
fn legacy_and_allocated_sale_corrections_use_original_time_and_real_change_ids() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 정정 원료");
    receipt(&mut store, &raw, "10", "100");
    let menu = recipe(&mut store, &raw, "가상 정정 메뉴", Some("20"), "1");
    let original = sale(&mut store, &menu, "3", None);
    let legacy = sale(&mut store, &menu, "1", Some(cid(&original)));
    let legacy_report = all(&store);
    let row = &legacy_report["sales"]["rows"][0];
    assert_eq!(row["value"]["count"], "1");
    assert_eq!(row["changes"][0]["kind"], "legacy_sale_revision");
    assert_eq!(
        row["changes"][0]["id"],
        result_id(&legacy, "sale_version_id")
    );
    assert_eq!(
        row["origin"]["business_record_id"],
        original["result"]["item"]["id"]
    );
    stocktake(&mut store, vec![row_for(&raw, "6")]);
    let corrected = correct(
        &mut store,
        &original,
        json!({"kind":"replace","replacement":{"kind":"sale_count","data":{"count":"0"}}}),
    );
    let result = report(&store, at(&original), at(&corrected));
    let row = &result["sales"]["rows"][0];
    assert_eq!(row["value"]["count"], "0");
    assert_eq!(row["origin"]["recorded_at"], at(&original));
    assert_eq!(
        row["changes"][0]["id"],
        result_id(&corrected, "correction_id")
    );
    assert_eq!(row["consumption"][0]["quantity"], Value::Null);
    assert_eq!(row["consumption"][0]["reason"], "zero_weight");
    let allocation = &result["observations"]["rows"][0]["allocation"];
    assert_eq!(allocation["total_quantity"], "4");
    assert_eq!(allocation["unallocated_quantity"], "4");
    assert_eq!(
        result["observations"]["rows"][0]["effect"]["calculation"]["allocation"],
        *allocation
    );
    correct(&mut store, &original, json!({"kind":"void"}));
    let voided = all(&store);
    assert_eq!(voided["sales"]["summary"]["active_records"], 0);
    assert_eq!(voided["sales"]["summary"]["void_records"], 1);
    assert_eq!(voided["sales"]["rows"][0]["value"], Value::Null);
    assert_eq!(
        voided["sales"]["rows"][0]["original"]["recipe_cost"]["total_cost"],
        "10"
    );
}
fn row_for(item: &MaterialSummary, quantity: &str) -> Value {
    row(item, quantity)
}

#[test]
fn stocktake_void_and_restore_keep_original_labels_and_do_not_reuse_tombstone_effects() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 취소 관측");
    let observed = stocktake(&mut store, vec![initial(&raw, "0", "0")]);
    let initial_report = all(&store);
    assert_eq!(
        initial_report["observations"]["rows"][0]["effect"]["measured_quantity"],
        "0"
    );
    correct(&mut store, &observed, json!({"kind":"void"}));
    let result = all(&store);
    let row = &result["observations"]["rows"][0];
    assert_eq!(row["status"], "void");
    assert_eq!(row["effect"], Value::Null);
    assert_eq!(row["allocation"], Value::Null);
    assert_eq!(row["material_name"], raw.name);
    assert_eq!(row["unit"], "g");
    assert_eq!(row["position"], 0);
    assert!(row["tombstone_projection_id"].is_string());
    assert_eq!(row["effective_observation_version_id"], Value::Null);
    let seed = value(store.correction_seed(cid(&observed)).unwrap().unwrap());
    correct(
        &mut store,
        &observed,
        json!({"kind":"replace","replacement":seed["replacement_template"]}),
    );
    let restored = all(&store);
    assert_eq!(restored["observations"]["rows"][0]["status"], "initial");
    assert_eq!(
        restored["observations"]["rows"][0]["origin"]["recorded_at"],
        at(&observed)
    );
    assert_eq!(
        restored["observations"]["rows"][0]["tombstone_projection_id"],
        Value::Null
    );
}

#[test]
fn receipt_correction_canonical_basis_and_reconciliation_stay_distinct_from_purchase_amount() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 입고 표시");
    let done = receipt(&mut store, &raw, "1000", "10");
    let mut data = receipt_data(&raw, "2", "40");
    data["unit"] = json!("kg");
    correct(
        &mut store,
        &done,
        json!({"kind":"replace","replacement":{"kind":"receipt","data":data}}),
    );
    let result = all(&store);
    let row = &result["operations"]["rows"][0];
    assert_eq!(row["receipt_basis"]["original"]["quantity"], "1000");
    assert_eq!(row["receipt_basis"]["effective"]["quantity"], "2000");
    assert_eq!(row["receipt_basis"]["effective"]["unit"], "g");
    assert_eq!(row["receipt_basis"]["effective"]["amount"], "40");
    assert_eq!(
        row["receipt_basis"]["effective"]["receipt_date"],
        "2020-02-29"
    );
    let observed = stocktake(&mut store, vec![row_for(&raw, "2000")]);
    let detail = value(
        store
            .stocktake_detail(result_id(&observed, "stocktake_id"))
            .unwrap()
            .unwrap(),
    );
    let obs = &detail["observations"][0];
    let mut included = receipt_data(&raw, "100", "3");
    included["stock_effect"] = json!({"kind":"reconciled","selections":[{"observation_version_id":obs["version_id"],"source":{"kind":"all_available"},"included_quantity":"100"}]});
    let late = run(
        &mut store,
        json!({"kind":"receipt","version":1,"data":included}),
        None,
    );
    let result = all(&store);
    let row = find(
        &result["operations"]["rows"],
        result_id(&late, "receipt_id"),
    );
    assert_eq!(row["receipt_basis"]["effective"]["quantity"], "100");
    assert_eq!(row["receipt_basis"]["effective"]["amount"], "3");
    assert!(row["value"]["reconciliation"].is_object());
}

#[test]
fn saved_sale_retries_and_current_price_name_archive_do_not_change_historical_cost() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 원래 품목명");
    receipt(&mut store, &raw, "10", "100");
    let menu = recipe(&mut store, &raw, "가상 원가 보존 메뉴", Some("20"), "1");
    let payload = json!({"kind":"sale","version":1,"data":{"material_id":menu["item"]["id"],"material_version_id":menu["item"]["version_id"],"recipe_version_id":menu["version"]["id"],"count":"2","note":"가상 재시도"}});
    let saved = value(
        store
            .business_create(decode(
                json!({"meta":meta(&store),"payload":payload,"corrects_confirmation_id":null}),
            ))
            .unwrap(),
    );
    let id = saved["result"]["item"]["id"].as_str().unwrap().to_owned();
    let request = json!({"meta":meta(&store),"expected_head_id":saved["result"]["head"]["id"]});
    let done = value(
        store
            .business_confirm(&id, decode(request.clone()))
            .unwrap(),
    );
    receipt(&mut store, &raw, "10", "300");
    let mut price = meta(&store);
    price["expected_version_id"] = json!(raw.version_id);
    price["quantity"] = json!("1");
    price["amount"] = json!("999");
    price["unit"] = json!("g");
    store.catalog_add_price(&raw.id, decode(price)).unwrap();
    let mut revise = meta(&store);
    revise["expected_version_id"] = json!(raw.version_id);
    revise["name"] = json!("가상 현재 품목명");
    revise["unit"] = json!("g");
    revise["kind"] = json!("raw");
    revise["description"] = json!("가상 변경");
    let current = store.catalog_revise(&raw.id, decode(revise)).unwrap().value;
    let mut archive = meta(&store);
    archive["expected_version_id"] = menu["item"]["version_id"].clone();
    store
        .catalog_archive(menu["item"]["id"].as_str().unwrap(), decode(archive))
        .unwrap();
    stocktake(&mut store, vec![row(&current, "18")]);
    let before = all(&store);
    assert_eq!(
        before["sales"]["rows"][0]["original"]["recipe_cost"]["total_cost"],
        "10"
    );
    assert_eq!(before["sales"]["rows"][0]["value"]["amount"], "40");
    assert_eq!(before["sales"]["rows"][0]["consumption"][0]["value"], "40");
    assert_eq!(
        before["sales"]["rows"][0]["consumption"][0]["material_name"],
        raw.name
    );
    assert_eq!(
        before["observations"]["summary_by_material"][0]["material_name"],
        current.name
    );
    let state = value(store.catalog_state().unwrap());
    let bytes = journals(root.path());
    assert_eq!(
        value(store.business_confirm(&id, decode(request)).unwrap()),
        done
    );
    assert_eq!(all(&store), before);
    assert_eq!(value(store.catalog_state().unwrap()), state);
    assert_eq!(journals(root.path()), bytes);
}

#[test]
fn explicit_zero_allocation_and_sales_before_first_receipt_are_distinguished() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 기준 이전 판매");
    let menu = recipe(&mut store, &raw, "가상 영 소모 메뉴", Some("0"), "0");
    let before_basis = sale(&mut store, &menu, "1", None);
    receipt(&mut store, &raw, "10", "10");
    let inside = sale(&mut store, &menu, "1", None);
    stocktake(&mut store, vec![row(&raw, "10")]);
    let result = all(&store);
    let older = find(
        &result["sales"]["rows"],
        result_id(&before_basis, "sale_id"),
    );
    assert_eq!(older["consumption"][0]["reason"], "no_effective_allocation");
    assert_eq!(older["consumption"][0]["quantity"], Value::Null);
    let newer = find(&result["sales"]["rows"], result_id(&inside, "sale_id"));
    assert_eq!(newer["consumption"][0]["status"], "allocated");
    assert_eq!(newer["consumption"][0]["quantity"], "0");
    assert_eq!(newer["consumption"][0]["value"], "0");
    assert_eq!(newer["consumption"][0]["reason"], Value::Null);
}
