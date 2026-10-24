use rusqlite::Connection;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, path::Path};
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
    let state = store.catalog_state().unwrap();
    json!({"request_id":Uuid::new_v4().to_string(),"epoch":state.epoch,"expected_revision":state.revision})
}

fn material(store: &mut Store, name: &str) -> MaterialSummary {
    let mut request = meta(store);
    request["name"] = json!(name);
    request["unit"] = json!("g");
    request["kind"] = json!("raw");
    request["description"] = json!("가상 모아 입력 시험");
    store.catalog_create(decode(request)).unwrap().value
}

fn create_request(store: &Store, payload: Value) -> Value {
    json!({"meta":meta(store),"payload":payload,"corrects_confirmation_id":null})
}

fn save(store: &mut Store, payload: Value) -> Value {
    let request = create_request(store, payload);
    value(store.business_create(decode(request)).unwrap())
}

fn id(saved: &Value) -> &str {
    saved["result"]["item"]["id"].as_str().unwrap()
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

fn confirm_request(store: &Store, saved: &Value) -> Value {
    json!({"meta":meta(store),"expected_head_id":saved["result"]["head"]["id"]})
}

fn confirm(store: &mut Store, saved: &Value) -> Value {
    let checked = preview(store, saved);
    assert_eq!(checked["can_confirm"], true, "{checked}");
    let request = confirm_request(store, saved);
    value(store.business_confirm(id(saved), decode(request)).unwrap())
}

fn holding(store: &Store, item: &MaterialSummary) -> Value {
    value(store.holding_detail(&item.id).unwrap().unwrap())
}

fn assert_holding(store: &Store, item: &MaterialSummary, quantity: &str, amount: &str) {
    let current = holding(store, item);
    assert_eq!(current["settlement_basis"]["quantity"], quantity);
    assert_eq!(current["settlement_basis"]["value"], amount);
}

fn row(item: &MaterialSummary, quantity: &str) -> Value {
    json!({"material_id":item.id,"material_version_id":item.version_id,"quantity":quantity,"unit":"g","initial_value":null,"excess":null})
}

fn initial_row(item: &MaterialSummary, quantity: &str, amount: &str) -> Value {
    let mut result = row(item, quantity);
    result["initial_value"] = json!({"kind":"manual_amount","amount":amount});
    result
}

fn stocktake(rows: Vec<Value>) -> Value {
    json!({"kind":"stocktake","version":1,"data":{"rows":rows,"note":"가상 선택 실사"}})
}

fn receipt_payload(item: &MaterialSummary, quantity: &str, amount: &str) -> Value {
    json!({"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":item.id,"material_version_id":item.version_id},"quantity":quantity,"unit":"g","amount":amount,"receipt_date":"2020-02-29","note":"가상 입고","stock_effect":{"kind":"unreflected"}}})
}

fn waste_payload(item: &MaterialSummary, quantity: &str) -> Value {
    json!({"kind":"waste","version":1,"data":{"material_id":item.id,"material_version_id":item.version_id,"quantity":quantity,"unit":"g","reason":"staff_meal","note":"가상 직원식"}})
}

fn production_payload(raw: &MaterialSummary, output: &MaterialSummary, quantity: &str) -> Value {
    json!({"kind":"production","version":1,"data":{"inputs":[{"material_id":raw.id,"material_version_id":raw.version_id,"quantity":quantity,"unit":"g"}],"outputs":[{"material_id":output.id,"material_version_id":output.version_id,"quantity":"2000","unit":"g","allocation_percent":"100"}],"note":"가상 손질"}})
}

fn recipe(store: &mut Store, raw: &MaterialSummary, name: &str) -> Value {
    let request = json!({"meta":meta(store),"owner":{"mode":"new","name":name,"unit":"개","kind":"menu","description":"가상 메뉴"},"data":{"basis_count":"1","reference_output":null,"selling_price":"1000","cooking_description":"","process":"","reference_time":"","lines":[{"material_id":raw.id,"material_version_id":raw.version_id,"quantity":"0","unit":"g","note":""}]}});
    value(store.recipe_create(decode(request)).unwrap())
}

fn sale_payload(recipe: &Value, count: &str) -> Value {
    json!({"kind":"sale","version":1,"data":{"material_id":recipe["item"]["id"],"material_version_id":recipe["item"]["version_id"],"recipe_version_id":recipe["version"]["id"],"count":count,"note":"가상 판매"}})
}

fn count(root: &Path, table: &str) -> i64 {
    Connection::open(root.join("app.sqlite3"))
        .unwrap()
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })
        .unwrap()
}

fn journals(root: &Path) -> Vec<Vec<u8>> {
    let mut paths = fs::read_dir(root.join("snapshots"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .into_iter()
        .map(|path| fs::read(path).unwrap())
        .collect()
}

#[test]
fn mixed_confirmed_work_is_counted_once_and_requires_a_fresh_stocktake_preview() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 모아 입력 원료");
    let output = material(&mut store, "가상 손질 결과");
    let initial = save(
        &mut store,
        stocktake(vec![initial_row(&raw, "10000", "10000")]),
    );
    confirm(&mut store, &initial);
    let menu_a = recipe(&mut store, &raw, "가상 메뉴 A");
    let menu_b = recipe(&mut store, &raw, "가상 메뉴 B");
    let sale_a = save(&mut store, sale_payload(&menu_a, "3"));
    let sale_a = confirm(&mut store, &sale_a);
    let sale_b = save(&mut store, sale_payload(&menu_b, "1"));
    let sale_b = confirm(&mut store, &sale_b);
    assert_holding(&store, &raw, "10000", "10000");

    let measured = save(&mut store, stocktake(vec![row(&raw, "6000")]));
    let old_preview = preview(&store, &measured);
    assert_eq!(
        old_preview["effect"]["rows"][0]["calculation"]["consumption_quantity"],
        "4000"
    );
    let stale_request = confirm_request(&store, &measured);
    let incomplete = save(&mut store, production_payload(&raw, &output, ""));
    assert!(
        !incomplete["result"]["issues"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let after_draft = journals(root.path());
    assert!(matches!(
        store.business_confirm(id(&measured), decode(stale_request)),
        Err(StoreError::StaleState(_))
    ));
    assert_eq!(journals(root.path()), after_draft);
    assert_holding(&store, &raw, "10000", "10000");

    let received = save(&mut store, receipt_payload(&raw, "2000", "2000"));
    let received = confirm(&mut store, &received);
    let source_id = received["result"]["confirmation"]["result"]["receipt_id"]
        .as_str()
        .unwrap();
    let produced = save(&mut store, production_payload(&raw, &output, "2000"));
    confirm(&mut store, &produced);
    let source = value(store.purchase_return_source(source_id).unwrap().unwrap());
    let returned = save(
        &mut store,
        json!({"kind":"purchase_return","version":1,"data":{"receipt_id":source_id,"expected_receipt_version_id":source["current"]["reference"]["version_id"],"quantity":"1000","unit":"g","note":"가상 구입 취소"}}),
    );
    confirm(&mut store, &returned);
    let discarded = save(&mut store, waste_payload(&raw, "1000"));
    confirm(&mut store, &discarded);
    assert_holding(&store, &raw, "8000", "9000");
    assert_holding(&store, &output, "2000", "2000");

    let state = value(store.catalog_state().unwrap());
    let bytes = journals(root.path());
    let reviewed = preview(&store, &measured);
    assert_eq!(value(store.catalog_state().unwrap()), state);
    assert_eq!(journals(root.path()), bytes);
    assert_eq!(
        reviewed["effect"]["rows"][0]["calculation"]["consumption_quantity"],
        "2000"
    );
    assert_eq!(
        reviewed["effect"]["rows"][0]["calculation"]["consumption_value"],
        "2250"
    );
    let done = confirm(&mut store, &measured);
    let stocktake_id = done["result"]["confirmation"]["result"]["stocktake_id"]
        .as_str()
        .unwrap();
    let detail = value(store.stocktake_detail(stocktake_id).unwrap().unwrap());
    let allocation = &detail["observations"][0]["effect"]["calculation"]["allocation"];
    let candidates = allocation["candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 2);
    for (sale, quantity, amount) in [(&sale_a, "1500", "1687.5"), (&sale_b, "500", "562.5")] {
        let candidate = candidates
            .iter()
            .find(|candidate| {
                candidate["sale_id"] == sale["result"]["confirmation"]["result"]["sale_id"]
            })
            .unwrap();
        assert_eq!(candidate["quantity"], quantity);
        assert_eq!(candidate["value"], amount);
    }
    assert_eq!(allocation["unallocated_quantity"], "0");
    assert_eq!(allocation["unallocated_value"], "0");
    assert_holding(&store, &raw, "6000", "6750");
    assert_eq!(count(root.path(), "productions"), 1);
    assert_eq!(count(root.path(), "purchase_returns"), 1);
    assert_eq!(count(root.path(), "wastes"), 1);
    assert_eq!(
        store
            .business_detail(id(&incomplete))
            .unwrap()
            .unwrap()
            .item
            .status,
        sunchang_hansang::business::BusinessStatus::Draft
    );
    let final_journals = journals(root.path());
    drop(store);
    let store = Store::open(root.path()).unwrap();
    assert_holding(&store, &raw, "6000", "6750");
    assert_eq!(
        value(store.stocktake_detail(stocktake_id).unwrap().unwrap()),
        detail
    );
    assert_eq!(journals(root.path()), final_journals);
}

#[test]
fn database_drafts_survive_restart_and_blank_stocktake_differs_from_explicit_zero() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 미확인 유지");
    let other = material(&mut store, "가상 명시 영 확인");
    let output = material(&mut store, "가상 미완성 결과");
    let initial = save(
        &mut store,
        stocktake(vec![
            initial_row(&raw, "10", "10"),
            initial_row(&other, "5", "5"),
        ]),
    );
    confirm(&mut store, &initial);
    let original_raw = holding(&store, &raw);
    let original_other = holding(&store, &other);
    let production = save(&mut store, production_payload(&raw, &output, "1."));
    let waste = save(&mut store, waste_payload(&raw, ""));
    let measured = save(&mut store, stocktake(vec![row(&raw, ""), row(&other, "0")]));
    let expected_ids = [&production, &waste, &measured]
        .into_iter()
        .map(|saved| id(saved).to_owned())
        .collect::<BTreeSet<_>>();
    let before = journals(root.path());
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    let list = store.business_list(Default::default()).unwrap();
    assert_eq!(
        list.items
            .iter()
            .map(|item| item.id.clone())
            .collect::<BTreeSet<_>>(),
        expected_ids
    );
    for saved in [&production, &waste, &measured] {
        let restored = value(store.business_detail(id(saved)).unwrap().unwrap());
        assert_eq!(
            restored["head"]["payload"],
            saved["result"]["head"]["payload"]
        );
        assert_eq!(restored["head"]["id"], saved["result"]["head"]["id"]);
        assert_eq!(restored["confirmations"], json!([]));
    }
    assert_eq!(preview(&store, &production)["can_confirm"], false);
    assert_eq!(preview(&store, &waste)["can_confirm"], false);
    assert_eq!(
        preview(&store, &measured)["effect"]["unchecked_positions"],
        json!([0])
    );
    assert_eq!(
        holding(&store, &raw)["settlement_basis"],
        original_raw["settlement_basis"]
    );
    assert_eq!(
        holding(&store, &other)["settlement_basis"],
        original_other["settlement_basis"]
    );
    assert_eq!(journals(root.path()), before);
    let done = confirm(&mut store, &measured);
    assert_holding(&store, &raw, "10", "10");
    assert_holding(&store, &other, "0", "0");
    assert_eq!(
        holding(&store, &raw)["observation"],
        original_raw["observation"]
    );
    assert_ne!(
        holding(&store, &other)["observation"]["id"],
        original_other["observation"]["id"]
    );
    let detail = value(
        store
            .stocktake_detail(
                done["result"]["confirmation"]["result"]["stocktake_id"]
                    .as_str()
                    .unwrap(),
            )
            .unwrap()
            .unwrap(),
    );
    assert_eq!(detail["observations"].as_array().unwrap().len(), 1);
    assert_eq!(detail["observations"][0]["effect"]["material_id"], other.id);
    assert_eq!(detail["observations"][0]["effect"]["input_quantity"], "0");
    assert_eq!(count(root.path(), "productions"), 0);
    assert_eq!(count(root.path(), "wastes"), 0);
}

#[test]
fn lost_create_response_replays_original_identity_without_merging_identical_drafts() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 독립 초안");
    let payload = waste_payload(&raw, "");
    let saved_a = save(&mut store, payload.clone());
    let request_b = create_request(&store, payload.clone());
    // The server accepted B; the caller loses its response before attaching its ID.
    let lost_b = value(store.business_create(decode(request_b.clone())).unwrap());
    let unsent_c = create_request(&store, payload);
    let revised_a = value(store.business_revise(id(&saved_a), decode(json!({"meta":meta(&store),"expected_head_id":saved_a["result"]["head"]["id"],"payload":waste_payload(&raw,"1.")}))).unwrap());
    let latest = value(store.catalog_state().unwrap());
    let before = journals(root.path());
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    let replay_b = value(store.business_create(decode(request_b.clone())).unwrap());
    assert_eq!(replay_b, lost_b);
    assert_ne!(id(&replay_b), id(&saved_a));
    assert_ne!(replay_b["state"], latest);
    assert_eq!(value(store.catalog_state().unwrap()), latest);
    assert_eq!(count(root.path(), "business_records"), 2);
    assert_eq!(
        value(store.business_detail(id(&saved_a)).unwrap().unwrap())["head"]["id"],
        revised_a["result"]["head"]["id"]
    );
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let sent_c: bool = db
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM mutation_requests WHERE request_id=?1)",
            [unsent_c["meta"]["request_id"].as_str().unwrap()],
            |row| row.get(0),
        )
        .unwrap();
    assert!(!sent_c);
    let mut conflicting_b = request_b;
    conflicting_b["payload"]["data"]["note"] = json!("다른 가상 입력");
    assert!(matches!(
        store.business_create(decode(conflicting_b)),
        Err(StoreError::Conflict(_))
    ));
    assert_eq!(journals(root.path()), before);
    assert_eq!(count(root.path(), "holding_versions"), 0);
}

#[test]
fn failed_json_publication_keeps_per_record_results_and_blocks_dependent_writes_until_retry() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = material(&mut store, "가상 부분 처리 결과");
    let received = save(&mut store, receipt_payload(&raw, "10", "100"));
    confirm(&mut store, &received);
    let menu = recipe(&mut store, &raw, "가상 영 판매 메뉴");
    let a = save(&mut store, sale_payload(&menu, "0"));
    let done_a = confirm(&mut store, &a);
    assert_holding(&store, &raw, "10", "100");
    let b = save(&mut store, waste_payload(&raw, "1"));
    let request_b = confirm_request(&store, &b);
    let request_c = create_request(&store, stocktake(vec![row(&raw, "8")]));
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
        store.business_confirm(id(&b), decode(request_b.clone())),
        Err(StoreError::Unavailable(_))
    ));
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let (response, payload): (String, String) = db.query_row("SELECT r.response,o.payload FROM mutation_requests r JOIN snapshot_outbox o ON o.sequence=r.sequence WHERE r.request_id=?1", [request_b["meta"]["request_id"].as_str().unwrap()], |row| Ok((row.get(0)?,row.get(1)?))).unwrap();
    let committed_count = count(root.path(), "mutation_requests");
    assert!(matches!(
        store.business_confirm(id(&b), decode(request_b.clone())),
        Err(StoreError::Unavailable(_))
    ));
    assert!(matches!(
        store.business_create(decode(request_c.clone())),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(count(root.path(), "mutation_requests"), committed_count);
    assert_eq!(count(root.path(), "business_records"), 3);
    assert_eq!(count(root.path(), "stocktake_batches"), 0);
    assert_eq!(count(root.path(), "wastes"), 1);
    assert_eq!(
        value(store.business_detail(id(&a)).unwrap().unwrap())["confirmations"][0],
        done_a["result"]["confirmation"]
    );
    fs::remove_dir(&blocked).unwrap();
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    let retried = value(store.business_confirm(id(&b), decode(request_b)).unwrap());
    assert_eq!(retried, serde_json::from_str::<Value>(&response).unwrap());
    assert_eq!(fs::read_to_string(&blocked).unwrap(), payload);
    assert_eq!(count(root.path(), "mutation_requests"), committed_count);
    assert_holding(&store, &raw, "9", "100");
    assert!(matches!(
        store.business_create(decode(request_c)),
        Err(StoreError::StaleState(_))
    ));
    let c = save(&mut store, stocktake(vec![row(&raw, "8")]));
    let checked = preview(&store, &c);
    assert_eq!(
        checked["effect"]["rows"][0]["calculation"]["consumption_quantity"],
        "1"
    );
    assert_eq!(count(root.path(), "stocktake_batches"), 0);
}
