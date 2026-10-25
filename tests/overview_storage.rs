use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{fs, path::Path};
use sunchang_hansang::{
    catalog::MaterialSummary,
    overview::{OverviewError, OverviewListQuery},
    statistics::StatisticsQuery,
    store::Store,
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

fn overview(store: &Store) -> Value {
    value(store.operations_overview().unwrap())
}
fn material_row<'a>(report: &'a Value, id: &str) -> &'a Value {
    report["materials"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["material"]["id"] == id)
        .unwrap()
}
fn page_query(
    store: &Store,
    section: &str,
    offset: usize,
    material: Option<&str>,
) -> OverviewListQuery {
    let state = store.catalog_state().unwrap();
    decode(
        json!({"section":section,"offset":offset.to_string(),"epoch":state.epoch,"revision":state.revision,"material_id":material}),
    )
}
fn page(store: &Store, section: &str, offset: usize, material: Option<&str>) -> Value {
    value(
        store
            .overview_list(page_query(store, section, offset, material))
            .unwrap(),
    )
}
fn waste(store: &mut Store, raw: &MaterialSummary, quantity: &str) -> Value {
    run(
        store,
        json!({"kind":"waste","version":1,"data":{"material_id":raw.id,"material_version_id":raw.version_id,"quantity":quantity,"unit":"g","reason":"staff_meal","note":"가상 직원식"}}),
        None,
    )
}
fn returned(store: &mut Store, received: &Value, quantity: &str) -> Value {
    let id = result_id(received, "receipt_id");
    let source = value(store.purchase_return_source(id).unwrap().unwrap());
    run(
        store,
        json!({"kind":"purchase_return","version":1,"data":{"receipt_id":id,"expected_receipt_version_id":source["current"]["reference"]["version_id"],"quantity":quantity,"unit":"g","note":"가상 반품"}}),
        None,
    )
}

#[test]
fn complete_material_pages_include_archived_items_and_are_bound_to_one_state() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let empty = overview(&store);
    assert_eq!(empty["materials"]["total_count"], 0);
    assert_eq!(empty["materials"]["next_offset"], Value::Null);
    let mut first = None;
    for index in 0..26 {
        let item = material(&mut store, &format!("가상 품목 {index:02}"));
        if index == 0 {
            first = Some(item);
        }
    }
    let first = first.unwrap();
    let mut archive = meta(&store);
    archive["expected_version_id"] = json!(first.version_id);
    store.catalog_archive(&first.id, decode(archive)).unwrap();
    let before = all(&store);
    let db = fs::read(root.path().join("app.sqlite3")).unwrap();
    let files = journals(root.path());
    let summary = overview(&store);
    assert_eq!(summary["materials"]["total_count"], 26);
    assert_eq!(summary["materials"]["returned_count"], 25);
    assert_eq!(summary["materials"]["next_offset"], 25);
    assert!(
        summary["materials"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|m| m["material"]["archived"] == false)
    );
    let last = page(&store, "materials", 25, None);
    assert_eq!(last["state"], summary["state"]);
    assert_eq!(
        last["scope"],
        json!({"section":"materials","material_id":null})
    );
    assert_eq!(last["list"]["page"]["total_count"], 26);
    assert_eq!(last["list"]["page"]["rows"][0]["material"]["id"], first.id);
    assert_eq!(last["list"]["page"]["next_offset"], Value::Null);
    assert_eq!(
        last["list"]["page"]["rows"][0]["last_observation"],
        Value::Null
    );
    assert_eq!(
        last["list"]["page"]["rows"][0]["changes_boundary"]["kind"],
        "not_observed"
    );
    for section in [
        "materials",
        "open_orders",
        "received_orders",
        "receipts",
        "sales",
        "observations",
        "unallocated",
        "unconfirmed",
        "drafts",
    ] {
        let beyond = page(&store, section, usize::MAX, None);
        assert_eq!(beyond["list"]["section"], section);
        assert_eq!(beyond["list"]["page"]["rows"], json!([]));
        assert_eq!(beyond["list"]["page"]["next_offset"], Value::Null);
    }
    let scoped = page(&store, "material_changes", usize::MAX, Some(&first.id));
    assert_eq!(scoped["scope"]["material_id"], first.id);
    assert!(
        scoped["list"]["material"]
            .get("changes_after_observation")
            .is_none()
    );
    assert_eq!(all(&store), before);
    assert_eq!(fs::read(root.path().join("app.sqlite3")).unwrap(), db);
    assert_eq!(journals(root.path()), files);
    let old = page_query(&store, "materials", 25, None);
    material(&mut store, "가상 상태 변경");
    assert!(matches!(
        store.overview_list(old),
        Err(OverviewError::StaleState(_))
    ));
}

#[test]
fn database_drafts_are_paged_without_preview_and_survive_restart() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let mut ids = vec![];
    for index in 0..21 {
        let saved = value(store.business_create(decode(json!({"meta":meta(&store),"payload":{"kind":"production","version":1,"data":{"inputs":[],"outputs":[{"material_id":null,"material_version_id":null,"quantity":if index == 0 { "0" } else { "" },"unit":"","allocation_percent":""}],"note":"가상 미완성 입력"}},"corrects_confirmation_id":null}))).unwrap());
        ids.push(saved["result"]["item"]["id"].as_str().unwrap().to_owned());
    }
    let before = overview(&store);
    assert_eq!(before["drafts"]["total_count"], 21);
    assert_eq!(before["drafts"]["returned_count"], 20);
    assert_eq!(before["drafts"]["rows"][0]["id"], ids[20]);
    assert!(
        before["drafts"]["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|d| d["validation"] == "not_evaluated" && d["material_name"].is_null())
    );
    let next = page(&store, "drafts", 20, None);
    assert_eq!(next["list"]["page"]["rows"][0]["id"], ids[0]);
    let original = value(store.business_detail(&ids[0]).unwrap().unwrap());
    assert_eq!(
        original["head"]["payload"]["data"]["outputs"][0]["quantity"],
        "0"
    );
    drop(store);
    let store = Store::open(root.path()).unwrap();
    let db = fs::read(root.path().join("app.sqlite3")).unwrap();
    let files = journals(root.path());
    assert_eq!(overview(&store), before);
    assert_eq!(page(&store, "drafts", 20, None), next);
    assert_eq!(fs::read(root.path().join("app.sqlite3")).unwrap(), db);
    assert_eq!(journals(root.path()), files);
}

#[test]
fn logical_changes_show_same_material_inputs_and_outputs_without_estimated_inventory() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 같은 품목 생산");
    let observed = stocktake(&mut store, vec![initial(&raw, "10", "100")]);
    let a = recipe(&mut store, &raw, "가상 직접 메뉴 A", Some("5"), "0");
    let b = recipe(&mut store, &raw, "가상 직접 메뉴 B", None, "0");
    let sold_a = sale(&mut store, &a, "3", None);
    let sold_b = sale(&mut store, &b, "1", None);
    let received = receipt(&mut store, &raw, "2", "20");
    let produced = run(
        &mut store,
        json!({"kind":"production","version":1,"data":{"inputs":[{"material_id":raw.id,"material_version_id":raw.version_id,"quantity":"2","unit":"g"}],"outputs":[{"material_id":raw.id,"material_version_id":raw.version_id,"quantity":"1","unit":"g","allocation_percent":"100"}],"note":"가상 동일품목 작업"}}),
        None,
    );
    returned(&mut store, &received, "1");
    waste(&mut store, &raw, "1");
    let result = overview(&store);
    let m = material_row(&result, &raw.id);
    assert_eq!(m["last_observation"]["quantity"], "10");
    assert_eq!(m["last_observation"]["observed_at"], at(&observed));
    assert_eq!(m["changes_after_observation"]["total_count"], 4);
    let effect = &find(
        &m["changes_after_observation"]["rows"],
        result_id(&produced, "production_id"),
    )["effect"];
    assert_eq!(effect["input_quantity"], "2");
    assert_eq!(effect["output_quantity"], "1");
    assert_eq!(effect["input_value"], "20");
    assert_eq!(effect["output_value"], "20");
    for field in [
        "holding",
        "settlement_basis",
        "expected_quantity",
        "quantity",
        "value",
    ] {
        assert!(m.get(field).is_none(), "unexpected inventory field {field}");
    }
    let observed_again = stocktake(&mut store, vec![row(&raw, "6")]);
    let before = all(&store);
    let result = overview(&store);
    let m = material_row(&result, &raw.id);
    assert_eq!(
        m["last_observation"]["origin"]["id"],
        result_id(&observed_again, "stocktake_id")
    );
    assert_eq!(m["last_observation"]["consumption"]["quantity"], "3");
    assert_eq!(m["last_observation"]["consumption"]["value"], "36.363636");
    assert_eq!(m["changes_after_observation"]["total_count"], 0);
    assert_eq!(result["unconfirmed"]["total_count"], 0);
    for (sold, quantity, amount) in [
        (&sold_a, "2.25", "27.272727"),
        (&sold_b, "0.75", "9.090909"),
    ] {
        let sale = find(&result["sales"]["rows"], result_id(sold, "sale_id"));
        assert_eq!(sale["consumption_counts"]["allocated"], 1);
        let preserved = find(&before["sales"]["rows"], result_id(sold, "sale_id"));
        assert_eq!(preserved["consumption"][0]["quantity"], quantity);
        assert_eq!(preserved["consumption"][0]["value"], amount);
    }
    assert_eq!(all(&store), before);
    drop(store);
    assert_eq!(overview(&Store::open(root.path()).unwrap()), result);
}

#[test]
fn corrected_observation_keeps_logical_boundary_and_void_falls_back_then_restores() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 논리 관측");
    let first = stocktake(&mut store, vec![initial(&raw, "10", "100")]);
    let old = overview(&store);
    let received = receipt(&mut store, &raw, "2", "20");
    let seed = value(store.correction_seed(cid(&first)).unwrap().unwrap());
    let mut replacement = seed["replacement_template"].clone();
    replacement["data"]["rows"][0]["action"]["quantity"] = json!("8");
    // Use the typed replacement row from the seed, preserving its stable observation CAS.
    correct(
        &mut store,
        &first,
        json!({"kind":"replace","replacement":replacement}),
    );
    let changed = overview(&store);
    let m = material_row(&changed, &raw.id);
    assert_eq!(m["last_observation"]["quantity"], "8");
    assert_eq!(m["last_observation"]["observed_at"], at(&first));
    assert_ne!(
        m["last_observation"]["observation_version_id"],
        material_row(&old, &raw.id)["last_observation"]["observation_version_id"]
    );
    assert_eq!(m["changes_after_observation"]["total_count"], 1);
    assert_eq!(
        m["changes_after_observation"]["rows"][0]["origin"]["id"],
        result_id(&received, "receipt_id")
    );
    let second = stocktake(&mut store, vec![row(&raw, "9")]);
    correct(&mut store, &second, json!({"kind":"void"}));
    let fallback = overview(&store);
    assert_eq!(
        material_row(&fallback, &raw.id)["last_observation"]["quantity"],
        "8"
    );
    assert_eq!(fallback["observations"]["total_count"], 1);
    let seed = value(store.correction_seed(cid(&second)).unwrap().unwrap());
    correct(
        &mut store,
        &second,
        json!({"kind":"replace","replacement":seed["replacement_template"]}),
    );
    let restored = overview(&store);
    assert_eq!(
        material_row(&restored, &raw.id)["last_observation"]["quantity"],
        "9"
    );
    assert_eq!(
        material_row(&restored, &raw.id)["last_observation"]["observed_at"],
        at(&second)
    );
    assert_eq!(restored["observations"]["total_count"], 2);
    assert_eq!(restored["unallocated"]["total_count"], 1);
    assert_eq!(
        restored["unallocated"]["rows"][0]["observation"]["consumption"]["reason"],
        "no_sales"
    );
    assert_eq!(
        material_row(&restored, &raw.id)["changes_after_observation"]["total_count"],
        0
    );
    drop(store);
    assert_eq!(overview(&Store::open(root.path()).unwrap()), restored);
}

#[test]
fn allocation_only_sale_correction_selects_current_observation_and_zero_weight() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 배분 관측");
    stocktake(&mut store, vec![initial(&raw, "10", "100")]);
    let menu = recipe(&mut store, &raw, "가상 개수 배분", None, "0");
    let sold = sale(&mut store, &menu, "3", None);
    let measured = stocktake(&mut store, vec![row(&raw, "6")]);
    let before = overview(&store);
    correct(
        &mut store,
        &sold,
        json!({"kind":"replace","replacement":{"kind":"sale_count","data":{"count":"0"}}}),
    );
    let result = overview(&store);
    let current = &material_row(&result, &raw.id)["last_observation"];
    assert_eq!(current["quantity"], "6");
    assert_eq!(current["observed_at"], at(&measured));
    assert_ne!(
        current["observation_version_id"],
        material_row(&before, &raw.id)["last_observation"]["observation_version_id"]
    );
    assert_eq!(current["consumption"]["reason"], "zero_weight");
    assert_eq!(current["consumption"]["quantity"], "4");
    assert_eq!(current["consumption"]["unallocated_quantity"], "4");
    assert_eq!(result["unallocated"]["total_count"], 1);
    assert_eq!(result["unconfirmed"]["total_count"], 0);
    assert_eq!(result["sales"]["rows"][0]["count"], "0");
    assert_eq!(result["sales"]["rows"][0]["amount"], Value::Null);
    assert_eq!(
        result["sales"]["rows"][0]["consumption_counts"]["unallocated"],
        1
    );
    drop(store);
    assert_eq!(overview(&Store::open(root.path()).unwrap()), result);
}

#[test]
fn included_receipt_preserves_real_purchase_and_separate_new_inflow() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 포함 입고");
    let observed = stocktake(&mut store, vec![initial(&raw, "10", "10")]);
    let detail = value(
        store
            .stocktake_detail(result_id(&observed, "stocktake_id"))
            .unwrap()
            .unwrap(),
    );
    let mut data = receipt_data(&raw, "3", "6");
    data["stock_effect"] = json!({"kind":"reconciled","selections":[{"observation_version_id":detail["observations"][0]["version_id"],"source":{"kind":"all_available"},"included_quantity":"2"}]});
    let received = run(
        &mut store,
        json!({"kind":"receipt","version":1,"data":data}),
        None,
    );
    let result = overview(&store);
    let r = find(
        &result["receipts"]["rows"],
        result_id(&received, "receipt_id"),
    );
    assert_eq!(r["effective"]["quantity"], "3");
    assert_eq!(r["effective"]["amount"], "6");
    assert_eq!(r["reconciliation"]["included_quantity"], "2");
    assert_eq!(r["reconciliation"]["included_amount"], "4");
    assert_eq!(r["reconciliation"]["unreflected_quantity"], "1");
    assert_eq!(r["reconciliation"]["unreflected_amount"], "2");
    let effect = &material_row(&result, &raw.id)["changes_after_observation"]["rows"][0]["effect"];
    assert_eq!(effect["kind"], "reconciled_receipt");
    assert_eq!(effect["inflow_quantity"], "1");
    assert_eq!(effect["inflow_value"], "2");
    assert_eq!(effect["value_delta"], "2");
    assert_eq!(effect["separate_adjustment"], "0");
    let before = all(&store);
    assert_eq!(
        page(&store, "material_changes", 0, Some(&raw.id))["list"]["page"]["rows"][0]["effect"],
        *effect
    );
    assert_eq!(all(&store), before);
}

#[test]
fn zero_observation_initial_missing_and_actual_zero_allocation_remain_distinct() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 미관측 원료");
    let menu = recipe(&mut store, &raw, "가상 0개 판매", Some("0"), "0");
    let early = sale(&mut store, &menu, "0", None);
    let unknown = overview(&store);
    assert_eq!(
        material_row(&unknown, &raw.id)["last_observation"],
        Value::Null
    );
    assert_eq!(unknown["unconfirmed"]["total_count"], 1);
    assert_eq!(
        unknown["unconfirmed"]["rows"][0]["reason"],
        "no_effective_allocation"
    );
    let zero = stocktake(&mut store, vec![initial(&raw, "0", "0")]);
    let initial = overview(&store);
    assert_eq!(
        material_row(&initial, &raw.id)["last_observation"]["quantity"],
        "0"
    );
    assert_eq!(
        material_row(&initial, &raw.id)["last_observation"]["mode"],
        "initial"
    );
    assert_eq!(
        material_row(&initial, &raw.id)["last_observation"]["consumption"],
        Value::Null
    );
    assert_eq!(
        initial["unconfirmed"]["rows"][0]["reason"],
        "initial_basis_missing"
    );
    assert_eq!(
        initial["unconfirmed"]["rows"][0]["initial_observation"]["stocktake_id"],
        result_id(&zero, "stocktake_id")
    );
    assert_eq!(
        find(&initial["sales"]["rows"], result_id(&early, "sale_id"))["amount"],
        "0"
    );
    let later = sale(&mut store, &menu, "1", None);
    stocktake(&mut store, vec![row(&raw, "0")]);
    let allocated = overview(&store);
    assert_eq!(
        find(&allocated["sales"]["rows"], result_id(&later, "sale_id"))["consumption_counts"]["allocated"],
        1
    );
    assert_eq!(allocated["unallocated"]["total_count"], 0);
    assert_eq!(allocated["unconfirmed"]["total_count"], 1);
    assert_eq!(
        material_row(&allocated, &raw.id)["last_observation"]["consumption"]["quantity"],
        "0"
    );
    assert_eq!(
        material_row(&allocated, &raw.id)["last_observation"]["consumption"]["value"],
        "0"
    );
}

#[test]
fn effective_order_closure_follows_void_reopen_and_new_receipt() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 주문 품목");
    let placed = run(
        &mut store,
        json!({"kind":"purchase_order","version":1,"data":{"action":"place","material_id":raw.id,"material_version_id":raw.version_id,"quantity":"10","unit":"g","supplier":"가상 공급처","expected_date":"","estimated_amount":"","note":""}}),
        None,
    );
    let order_id = result_id(&placed, "order_id");
    let order = value(store.order_detail(order_id).unwrap().unwrap());
    let mut data = receipt_data(&raw, "9.8", "98");
    data["source"] = json!({"kind":"order","order_id":order_id,"expected_order_version_id":order["head"]["id"],"material_version_id":raw.version_id});
    let received = run(
        &mut store,
        json!({"kind":"receipt","version":1,"data":data}),
        None,
    );
    let done = overview(&store);
    assert_eq!(
        done["order_counts"],
        json!({"open":0,"received":1,"cancelled":0})
    );
    assert_eq!(
        done["received_orders"]["rows"][0]["closure"]["receipt_id"],
        result_id(&received, "receipt_id")
    );
    correct(&mut store, &received, json!({"kind":"void"}));
    let reopened = overview(&store);
    assert_eq!(reopened["order_counts"]["open"], 1);
    assert_eq!(reopened["received_orders"]["total_count"], 0);
    assert_eq!(reopened["receipts"]["total_count"], 0);
    let order = value(store.order_detail(order_id).unwrap().unwrap());
    let mut data = receipt_data(&raw, "9", "90");
    data["source"] = json!({"kind":"order","order_id":order_id,"expected_order_version_id":order["head"]["id"],"material_version_id":raw.version_id});
    let next = run(
        &mut store,
        json!({"kind":"receipt","version":1,"data":data}),
        None,
    );
    let final_result = overview(&store);
    assert_eq!(final_result["received_orders"]["total_count"], 1);
    assert_eq!(
        final_result["received_orders"]["rows"][0]["closure"]["receipt_id"],
        result_id(&next, "receipt_id")
    );
    assert_eq!(final_result["receipts"]["total_count"], 1);
    drop(store);
    assert_eq!(overview(&Store::open(root.path()).unwrap()), final_result);
}

#[test]
fn all_changes_can_be_walked_beyond_twenty_with_scope_and_no_duplicates() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 많은 변동");
    let mut ids = vec![];
    for _ in 0..21 {
        ids.push(result_id(&receipt(&mut store, &raw, "1", "0"), "receipt_id").to_owned());
    }
    ids.reverse();
    let summary = overview(&store);
    assert_eq!(
        material_row(&summary, &raw.id)["last_observation"],
        Value::Null
    );
    assert_eq!(
        material_row(&summary, &raw.id)["changes_after_observation"]["total_count"],
        21
    );
    let first = page(&store, "material_changes", 0, Some(&raw.id));
    let second = page(&store, "material_changes", 20, Some(&raw.id));
    assert_eq!(first["list"]["boundary"]["kind"], "not_observed");
    assert_eq!(first["list"]["page"]["next_offset"], 20);
    assert_eq!(second["list"]["page"]["total_count"], 21);
    assert_eq!(second["list"]["page"]["next_offset"], Value::Null);
    let actual = first["list"]["page"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .chain(second["list"]["page"]["rows"].as_array().unwrap())
        .map(|row| row["origin"]["id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(actual, ids);
    assert_eq!(first["state"], second["state"]);
    assert_eq!(first["scope"], second["scope"]);
    let unknown = Uuid::new_v4().to_string();
    assert!(matches!(
        store.overview_list(page_query(&store, "material_changes", 0, Some(&unknown))),
        Err(OverviewError::Validation(_))
    ));
}
