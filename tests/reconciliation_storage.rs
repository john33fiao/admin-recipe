use rusqlite::{Connection, types::Value as SqlValue};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};
use sunchang_hansang::{
    catalog::MaterialSummary,
    store::{Store, StoreError},
};
use tempfile::TempDir;
use uuid::Uuid;

fn decode<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}
fn value(v: impl Serialize) -> Value {
    serde_json::to_value(v).unwrap()
}
fn meta(store: &Store) -> Value {
    let s = store.catalog_state().unwrap();
    json!({"request_id":Uuid::new_v4().to_string(),"epoch":s.epoch,"expected_revision":s.revision})
}
fn item(store: &mut Store, name: &str) -> MaterialSummary {
    let mut r = meta(store);
    r["name"] = json!(name);
    r["unit"] = json!("g");
    r["kind"] = json!("raw");
    r["description"] = json!("가상 보정 시험");
    store.catalog_create(decode(r)).unwrap().value
}
fn save(store: &mut Store, payload: Value) -> Value {
    value(
        store
            .business_create(decode(
                json!({"meta":meta(store),"payload":payload,"corrects_confirmation_id":null}),
            ))
            .unwrap(),
    )
}
fn bid(saved: &Value) -> &str {
    saved["result"]["item"]["id"].as_str().unwrap()
}
fn confirm_request(store: &Store, saved: &Value) -> Value {
    json!({"meta":meta(store),"expected_head_id":saved["result"]["head"]["id"]})
}
fn confirm(store: &mut Store, saved: &Value) -> Value {
    value(
        store
            .business_confirm(bid(saved), decode(confirm_request(store, saved)))
            .unwrap(),
    )
}
fn preview(store: &Store, saved: &Value) -> Value {
    value(
        store
            .business_preview(
                bid(saved),
                decode(json!({"expected_head_id":saved["result"]["head"]["id"]})),
            )
            .unwrap(),
    )
}
fn row(m: &MaterialSummary, q: &str) -> Value {
    json!({"material_id":m.id,"material_version_id":m.version_id,"quantity":q,"unit":"g","initial_value":null,"excess":null})
}
fn observe(store: &mut Store, m: &MaterialSummary, q: &str, initial: Option<&str>) -> Value {
    let mut r = row(m, q);
    if let Some(v) = initial {
        r["initial_value"] = json!({"kind":"manual_amount","amount":v});
    }
    let saved = save(
        store,
        json!({"kind":"stocktake","version":1,"data":{"rows":[r],"note":"가상 관측"}}),
    );
    let done = confirm(store, &saved);
    value(
        store
            .stocktake_detail(
                done["result"]["confirmation"]["result"]["stocktake_id"]
                    .as_str()
                    .unwrap(),
            )
            .unwrap()
            .unwrap(),
    )["observations"][0]
        .clone()
}
fn selected(o: &Value, q: &str) -> Value {
    json!({"observation_version_id":o["version_id"],"source":{"kind":"all_available"},"included_quantity":q})
}
fn payload(m: &MaterialSummary, q: &str, amount: &str, effect: Value) -> Value {
    json!({"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":m.id,"material_version_id":m.version_id},"quantity":q,"unit":"g","amount":amount,"receipt_date":"2026-01-01","note":"가상 입고","stock_effect":effect}})
}
fn late(store: &mut Store, m: &MaterialSummary, q: &str, a: &str, selections: Vec<Value>) -> Value {
    save(
        store,
        payload(
            m,
            q,
            a,
            json!({"kind":"reconciled","selections":selections}),
        ),
    )
}
fn receipt(store: &mut Store, m: &MaterialSummary, q: &str, a: &str) {
    let s = save(store, payload(m, q, a, json!({"kind":"unreflected"})));
    confirm(store, &s);
}
fn detail(store: &Store, done: &Value) -> Value {
    value(
        store
            .receipt_detail(
                done["result"]["confirmation"]["result"]["receipt_id"]
                    .as_str()
                    .unwrap(),
            )
            .unwrap()
            .unwrap(),
    )
}
fn holding(store: &Store, m: &MaterialSummary) -> Value {
    value(store.holding_detail(&m.id).unwrap().unwrap())
}
fn count(root: &Path, table: &str) -> i64 {
    Connection::open(root.join("app.sqlite3"))
        .unwrap()
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}
fn install(root: &Path) -> Vec<(i64, String)> {
    let mut db = Connection::open(root.join("app.sqlite3")).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    let tx = db.transaction().unwrap();
    tx.execute_batch("PRAGMA defer_foreign_keys=ON").unwrap();
    tx.execute_batch(include_str!("fixtures/schema-v10.sql"))
        .unwrap();
    tx.commit().unwrap();
    fs::create_dir(root.join("snapshots")).unwrap();
    let rows = db
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<(i64, String)>>>()
        .unwrap();
    for (s, p) in &rows {
        fs::write(root.join(format!("snapshots/{s:020}.json")), p).unwrap();
    }
    rows
}
fn columns(db: &Connection, table: &str) -> Vec<String> {
    db.prepare(&format!("PRAGMA table_info({table})"))
        .unwrap()
        .query_map([], |r| r.get(1))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}
fn table_rows(db: &Connection, table: &str, columns: &[String]) -> Vec<Vec<SqlValue>> {
    db.prepare(&format!(
        "SELECT {} FROM {table} ORDER BY rowid",
        columns.join(",")
    ))
    .unwrap()
    .query_map([], |r| (0..columns.len()).map(|n| r.get(n)).collect())
    .unwrap()
    .collect::<rusqlite::Result<_>>()
    .unwrap()
}

#[test]
fn schema_ten_migrates_all_holding_edges_and_replays_twenty_three_original_responses() {
    let root = TempDir::new().unwrap();
    let journals = install(root.path());
    assert_eq!(journals.len(), 23);
    assert_eq!(
        journals.last().unwrap().1,
        include_str!("fixtures/snapshot-v2-schema10.json")
    );
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let names=db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap().query_map([],|r|r.get::<_,String>(0)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
    let before = names
        .into_iter()
        .map(|t| {
            let c = columns(&db, &t);
            let rows = table_rows(&db, &t, &c);
            (t, (c, rows))
        })
        .collect::<BTreeMap<_, _>>();
    drop(db);
    let mut store = Store::open(root.path()).unwrap();
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        13
    );
    for (t, (c, r)) in before {
        assert_eq!(table_rows(&db, &t, &c), r, "{t}");
    }
    for t in [
        "receipt_reconciliations",
        "inclusion_links",
        "inclusion_link_versions",
        "inclusion_link_heads",
        "inclusion_reservations",
        "reconciliation_material_effects",
    ] {
        assert_eq!(count(root.path(), t), 0);
    }
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
    drop(db);
    let frozen: Value = serde_json::from_str(&journals.last().unwrap().1).unwrap();
    let state = store.catalog_state().unwrap();
    for r in frozen["state"]["mutation_requests"].as_array().unwrap() {
        let p: Value = serde_json::from_str(r["payload"].as_str().unwrap()).unwrap();
        let actual = match r["operation"].as_str().unwrap() {
            "material.create" => value(store.catalog_create(decode(p)).unwrap()),
            "material.price" => value(
                store
                    .catalog_add_price(p[0].as_str().unwrap(), decode(p[1].clone()))
                    .unwrap(),
            ),
            "business.create" => value(store.business_create(decode(p)).unwrap()),
            "business.confirm" => value(
                store
                    .business_confirm(p[0].as_str().unwrap(), decode(p[1].clone()))
                    .unwrap(),
            ),
            "recipe.create" => value(store.recipe_create(decode(p)).unwrap()),
            "recipe.copy" => value(
                store
                    .recipe_copy(p[0].as_str().unwrap(), decode(p[1].clone()))
                    .unwrap(),
            ),
            op => panic!("unexpected fixture {op}"),
        };
        assert_eq!(
            actual,
            serde_json::from_str::<Value>(r["response"].as_str().unwrap()).unwrap()
        );
    }
    assert_eq!(store.catalog_state().unwrap(), state);
    for (s, p) in journals {
        let file = root.path().join(format!("snapshots/{s:020}.json"));
        assert_eq!(fs::read_to_string(&file).unwrap(), p);
        fs::remove_file(&file).unwrap();
        store.recover().unwrap();
        assert_eq!(fs::read_to_string(file).unwrap(), p);
    }
}

#[test]
fn late_receipt_separates_consumed_value_and_reobservation_uses_corrected_average() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let m = item(&mut store, "가상 연속 보정");
    let first = observe(&mut store, &m, "10", Some("100"));
    observe(&mut store, &m, "6", None);
    let saved = late(&mut store, &m, "10", "150", vec![selected(&first, "10")]);
    let before = store.catalog_state().unwrap();
    let p = preview(&store, &saved);
    assert!(p["issues"].as_array().unwrap().is_empty(), "{p}");
    let e = &p["effect"]["receipt"];
    assert_eq!(e["unreflected_quantity"], "0");
    assert_eq!(e["value_difference"], "50");
    assert_eq!(e["separate_adjustment"], "20");
    assert_eq!(store.catalog_state().unwrap(), before);
    let done = confirm(&mut store, &saved);
    let d = detail(&store, &done);
    assert_eq!(d["item"]["quantity"], "10");
    assert!(d["holding_version_id"].is_string());
    assert_eq!(
        d["reconciliation"]["material_results"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let h = holding(&store, &m);
    assert_eq!(h["settlement_basis"]["quantity"], "6");
    assert_eq!(h["settlement_basis"]["value"], "90");
    assert_eq!(h["observation"]["quantity"], "6");
    let next = observe(&mut store, &m, "5", None);
    assert_eq!(next["effect"]["calculation"]["consumption_value"], "15");
    assert_eq!(next["effect"]["after"]["value"], "75");
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn normal_observation_supports_included_receipt_without_excess_and_partial_new_quantity() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let m = item(&mut store, "가상 정상 실사");
    receipt(&mut store, &m, "10", "100");
    let o = observe(&mut store, &m, "10", None);
    let s = late(&mut store, &m, "4", "80.00", vec![selected(&o, "3")]);
    let p = preview(&store, &s);
    let e = &p["effect"]["receipt"];
    assert_eq!(e["included_amount"], "60");
    assert_eq!(e["unreflected_amount"], "20");
    assert_eq!(e["booked_value"], "30");
    assert_eq!(e["materials"][0]["after"]["quantity"], "11");
    assert_eq!(e["materials"][0]["after"]["value"], "150");
    confirm(&mut store, &s);
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn repeated_old_and_later_observation_selections_cannot_double_reserve_live_shares() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let m = item(&mut store, "가상 교차 예약");
    let first = observe(&mut store, &m, "10", Some("100"));
    let second = observe(&mut store, &m, "5", None);
    let a = late(&mut store, &m, "5", "100", vec![selected(&first, "5")]);
    confirm(&mut store, &a);
    let candidates = value(store.receipt_inclusion_candidates(&m.id).unwrap());
    let current = candidates["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["observation_version_id"] == second["version_id"])
        .unwrap();
    assert_eq!(current["available_quantity"], "2.5");
    let bad = late(&mut store, &m, "5", "100", vec![selected(&second, "5")]);
    assert!(
        !preview(&store, &bad)["issues"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        store.business_confirm(bid(&bad), decode(confirm_request(&store, &bad))),
        Err(StoreError::Validation(_))
    ));
    let b = late(&mut store, &m, "2.5", "50", vec![selected(&second, "2.5")]);
    let p = preview(&store, &b);
    assert_eq!(p["effect"]["receipt"]["booked_value"], "25");
    confirm(&mut store, &b);
    assert_eq!(holding(&store, &m)["settlement_basis"]["value"], "100");
    let backwards = late(&mut store, &m, "2.5", "50", vec![selected(&first, "2.5")]);
    assert!(
        !preview(&store, &backwards)["issues"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn fully_wasted_inclusion_has_no_fake_holding_and_keeps_last_valid_price() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let m = item(&mut store, "가상 전량 폐기");
    let o = observe(&mut store, &m, "10", Some("100"));
    let w = save(
        &mut store,
        json!({"kind":"waste","version":1,"data":{"material_id":m.id,"material_version_id":m.version_id,"quantity":"10","unit":"g","reason":"discard","note":"가상 폐기"}}),
    );
    confirm(&mut store, &w);
    let previous = holding(&store, &m);
    let before = count(root.path(), "holding_versions");
    let s = late(&mut store, &m, "10", "150", vec![selected(&o, "10")]);
    let done = confirm(&mut store, &s);
    let d = detail(&store, &done);
    assert_eq!(d["reconciliation"]["effect"]["separate_adjustment"], "50");
    assert!(
        d["reconciliation"]["material_results"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(d["holding_version_id"].is_null());
    assert_eq!(count(root.path(), "holding_versions"), before);
    assert_eq!(
        holding(&store, &m)["last_valid_price"],
        previous["last_valid_price"]
    );
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn negative_difference_and_explicit_zero_inclusion_preserve_signed_and_zero_values() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let m = item(&mut store, "가상 음수 차액");
    let o = observe(&mut store, &m, "10", Some("100"));
    let s = late(&mut store, &m, "2", "0", vec![selected(&o, "2")]);
    let done = confirm(&mut store, &s);
    assert_eq!(
        detail(&store, &done)["reconciliation"]["effect"]["value_difference"],
        "-20"
    );
    assert_eq!(holding(&store, &m)["settlement_basis"]["value"], "80");
    let fresh = item(&mut store, "가상 명시 포함 영");
    let s = late(&mut store, &fresh, "1", "0", vec![]);
    confirm(&mut store, &s);
    let h = holding(&store, &fresh);
    assert_eq!(h["basis"]["kind"], "reconciliation");
    assert_eq!(h["settlement_basis"]["quantity"], "1");
    assert_eq!(h["settlement_basis"]["value"], "0");
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn production_transfers_value_difference_without_rewriting_original_production() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let raw = item(&mut store, "가상 생산 원료");
    let out = item(&mut store, "가상 생산 결과");
    let o = observe(&mut store, &raw, "10", Some("100"));
    let p = save(
        &mut store,
        json!({"kind":"production","version":1,"data":{"inputs":[{"material_id":raw.id,"material_version_id":raw.version_id,"quantity":"4","unit":"g"}],"outputs":[{"material_id":out.id,"material_version_id":out.version_id,"quantity":"2","unit":"g","allocation_percent":"100"}],"note":"가상 생산"}}),
    );
    let done = confirm(&mut store, &p);
    let pid = done["result"]["confirmation"]["result"]["production_id"]
        .as_str()
        .unwrap();
    let original = value(store.production_detail(pid).unwrap().unwrap());
    let s = late(&mut store, &raw, "10", "200", vec![selected(&o, "10")]);
    let p = preview(&store, &s);
    assert_eq!(
        p["effect"]["receipt"]["materials"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    confirm(&mut store, &s);
    assert_eq!(holding(&store, &raw)["settlement_basis"]["value"], "120");
    assert_eq!(holding(&store, &out)["settlement_basis"]["value"], "80");
    let after = value(store.production_detail(pid).unwrap().unwrap());
    for key in ["item", "inputs", "outputs", "materials"] {
        assert_eq!(after[key], original[key], "{key}");
    }
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn linked_actual_receipt_closes_order_with_r_not_only_unreflected_u() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let m = item(&mut store, "가상 주문 보정");
    let o = observe(&mut store, &m, "10", Some("100"));
    let placed = save(
        &mut store,
        json!({"kind":"purchase_order","version":1,"data":{"action":"place","material_id":m.id,"material_version_id":m.version_id,"quantity":"12","unit":"g","supplier":"가상 공급","expected_date":"2026-01-01","estimated_amount":"150","note":"가상 주문"}}),
    );
    let order = confirm(&mut store, &placed);
    let order_id = order["result"]["confirmation"]["result"]["order_id"]
        .as_str()
        .unwrap();
    let before = value(store.order_detail(order_id).unwrap().unwrap());
    let mut p = payload(
        &m,
        "11",
        "165",
        json!({"kind":"reconciled","selections":[selected(&o,"10")]}),
    );
    p["data"]["source"] = json!({"kind":"order","order_id":order_id,"expected_order_version_id":before["head"]["id"],"material_version_id":m.version_id});
    let s = save(&mut store, p);
    let done = confirm(&mut store, &s);
    let d = detail(&store, &done);
    assert_eq!(d["item"]["quantity"], "11");
    assert_eq!(d["reconciliation"]["effect"]["unreflected_quantity"], "1");
    assert_eq!(d["order_closure"]["received_quantity"], "11");
    assert_eq!(d["order_closure"]["difference"], "-1");
    assert_eq!(
        value(store.order_detail(order_id).unwrap().unwrap())["head"]["status"],
        "received"
    );
    assert_eq!(count(root.path(), "order_receipt_closures"), 1);
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn incomplete_selections_survive_restart_and_stale_preview_preserves_input() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let m = item(&mut store, "가상 미완성 포함");
    let incomplete = json!({"observation_version_id":null,"source":null,"included_quantity":"1."});
    let s = late(&mut store, &m, "3", "30", vec![incomplete.clone()]);
    assert!(!preview(&store, &s)["issues"].as_array().unwrap().is_empty());
    let id = bid(&s).to_owned();
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    let loaded = value(store.business_detail(&id).unwrap().unwrap());
    assert_eq!(
        loaded["head"]["payload"]["data"]["stock_effect"]["selections"][0],
        incomplete
    );
    let o = observe(&mut store, &m, "10", Some("100"));
    let ready = late(&mut store, &m, "2", "30", vec![selected(&o, "2")]);
    let request = confirm_request(&store, &ready);
    receipt(&mut store, &m, "1", "10");
    let before = store.catalog_state().unwrap();
    assert!(matches!(
        store.business_confirm(bid(&ready), decode(request)),
        Err(StoreError::StaleState(_))
    ));
    assert_eq!(store.catalog_state().unwrap(), before);
    assert_eq!(count(root.path(), "receipt_reconciliations"), 0);
}

#[test]
fn reconciliation_relations_holding_and_journal_roll_back_as_one_transaction() {
    for target in [
        "receipt_reconciliations",
        "inclusion_links",
        "inclusion_link_versions",
        "inclusion_reservations",
        "reconciliation_material_effects",
        "holding_versions",
        "snapshot_outbox",
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let m = item(&mut store, "가상 원자 저장");
        let o = observe(&mut store, &m, "10", Some("100"));
        let s = late(&mut store, &m, "3", "60", vec![selected(&o, "3")]);
        let before = store.catalog_state().unwrap();
        let held = holding(&store, &m);
        let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
        let trigger = if target == "snapshot_outbox" {
            "CREATE TRIGGER injected BEFORE UPDATE OF payload ON snapshot_outbox BEGIN SELECT RAISE(FAIL,'injected'); END".into()
        } else {
            format!(
                "CREATE TRIGGER injected BEFORE INSERT ON {target} BEGIN SELECT RAISE(FAIL,'injected'); END"
            )
        };
        db.execute_batch(&trigger).unwrap();
        assert!(
            store
                .business_confirm(bid(&s), decode(confirm_request(&store, &s)))
                .is_err(),
            "{target}"
        );
        assert_eq!(store.catalog_state().unwrap(), before, "{target}");
        assert_eq!(count(root.path(), "receipt_reconciliations"), 0);
        assert_eq!(count(root.path(), "receipts"), 0);
        assert_eq!(holding(&store, &m), held);
        db.execute_batch("DROP TRIGGER injected").unwrap();
        confirm(&mut store, &s);
        drop(store);
        Store::open(root.path()).unwrap();
    }
}

#[test]
fn publication_failure_and_retry_preserve_response_before_reserved_and_cas_checks() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let m = item(&mut store, "가상 보정 재시도");
    let o = observe(&mut store, &m, "10", Some("100"));
    let s = late(&mut store, &m, "10", "150", vec![selected(&o, "10")]);
    let request = confirm_request(&store, &s);
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
        store.business_confirm(bid(&s), decode(request.clone())),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(count(root.path(), "receipt_reconciliations"), 1);
    assert!(matches!(
        store.business_confirm(bid(&s), decode(request.clone())),
        Err(StoreError::Unavailable(_))
    ));
    let other = json!({"meta":meta(&store),"payload":payload(&m,"1","10",json!({"kind":"unreflected"})),"corrects_confirmation_id":null});
    assert!(matches!(
        store.business_create(decode(other)),
        Err(StoreError::Unavailable(_))
    ));
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let original: String = db
        .query_row(
            "SELECT payload FROM snapshot_outbox WHERE sequence=?1",
            [seq],
            |r| r.get(0),
        )
        .unwrap();
    drop(db);
    fs::remove_dir(&path).unwrap();
    let response = value(
        store
            .business_confirm(bid(&s), decode(request.clone()))
            .unwrap(),
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    receipt(&mut store, &m, "1", "10");
    let state = store.catalog_state().unwrap();
    assert_eq!(
        value(
            store
                .business_confirm(bid(&s), decode(request.clone()))
                .unwrap()
        ),
        response
    );
    assert_eq!(store.catalog_state().unwrap(), state);
    let mut different = request;
    different["expected_head_id"] = json!(Uuid::new_v4().to_string());
    assert!(matches!(
        store.business_confirm(bid(&s), decode(different)),
        Err(StoreError::Conflict(_))
    ));
    assert_eq!(count(root.path(), "receipt_reconciliations"), 1);
}

fn change_snapshot(root: &Path, edit: impl FnOnce(&mut Value)) -> (std::path::PathBuf, String) {
    let db = Connection::open(root.join("app.sqlite3")).unwrap();
    let (seq, text): (i64, String) = db
        .query_row(
            "SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    let mut json: Value = serde_json::from_str(&text).unwrap();
    edit(&mut json);
    let text = serde_json::to_string_pretty(&json).unwrap() + "\n";
    db.execute(
        "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=?2",
        rusqlite::params![text, seq],
    )
    .unwrap();
    let path = root.join(format!("snapshots/{seq:020}.json"));
    fs::write(&path, &text).unwrap();
    (path, text)
}

#[test]
fn schema_eleven_rejects_forged_exact_reservations_value_trace_and_confirmation_relations() {
    for mode in [
        "reservation",
        "portion",
        "source_version",
        "observation",
        "effect_value",
        "trace",
        "request",
        "time",
        "sequence",
        "missing_collection",
        "unknown_field",
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let m = item(&mut store, "가상 변조 확인");
        let o = observe(&mut store, &m, "10", Some("100"));
        let s = late(&mut store, &m, "3", "60", vec![selected(&o, "3")]);
        confirm(&mut store, &s);
        drop(store);
        let (path, text) = change_snapshot(root.path(), |snapshot| {
            let state = &mut snapshot["state"];
            match mode {
                "reservation" => {
                    state["inclusion_reservations"][0]["reserved_source_share"]["numerator"] =
                        json!("4")
                }
                "portion" => {
                    state["receipt_reconciliations"][0]["effect"]["selections"][0]["portions"][0]
                        ["included_share"]["denominator"] = json!("2")
                }
                "source_version" => {
                    state["inclusion_link_versions"][0]["portion"]["source_version_id"] =
                        json!(Uuid::new_v4().to_string())
                }
                "observation" => {
                    state["inclusion_link_versions"][0]["observation_version_id"] =
                        json!(Uuid::new_v4().to_string())
                }
                "effect_value" => {
                    state["reconciliation_material_effects"][0]["effect"]["value_delta"] =
                        json!("31")
                }
                "trace" => {
                    state["receipt_reconciliations"][0]["effect"]["trace"][0]["proposed_value_after"] =
                        json!("999")
                }
                "request" => {
                    let id = state["receipt_reconciliations"][0]["confirmation_id"].clone();
                    state["business_confirmations"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|c| c["id"] == id)
                        .unwrap()["request_id"] = json!(Uuid::new_v4().to_string());
                }
                "time" => state["receipt_reconciliations"][0]["recorded_at"] = json!("invalid"),
                "sequence" => state["receipt_reconciliations"][0]["seq"] = json!("99999"),
                "missing_collection" => {
                    state
                        .as_object_mut()
                        .unwrap()
                        .remove("inclusion_reservations");
                }
                "unknown_field" => {
                    state["receipt_reconciliations"][0]["effect"]["unreviewed"] = json!(true)
                }
                _ => unreachable!(),
            }
        });
        let dbpath = root.path().join("app.sqlite3");
        let before = fs::read(&dbpath).unwrap();
        assert!(Store::open(root.path()).is_err(), "{mode}");
        assert_eq!(fs::read(dbpath).unwrap(), before, "{mode}");
        assert_eq!(fs::read_to_string(path).unwrap(), text, "{mode}");
    }
}

#[test]
fn source_ten_rejects_current_future_schema_or_new_tag_before_migration() {
    for mode in ["future_schema", "future_tag", "future_collection"] {
        let root = TempDir::new().unwrap();
        install(root.path());
        let (path, text) = change_snapshot(root.path(), |snapshot| match mode {
            "future_schema" => snapshot["schema_version"] = json!(11),
            "future_tag" => {
                let r = snapshot["state"]["business_revisions"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["payload"]["kind"] == "receipt")
                    .unwrap();
                r["payload"]["data"]["stock_effect"] = json!({"kind":"reconciled","selections":[]});
            }
            "future_collection" => snapshot["state"]["receipt_reconciliations"] = json!([]),
            _ => unreachable!(),
        });
        let dbpath = root.path().join("app.sqlite3");
        let before = fs::read(&dbpath).unwrap();
        assert!(Store::open(root.path()).is_err(), "{mode}");
        assert_eq!(fs::read(dbpath).unwrap(), before);
        assert_eq!(fs::read_to_string(path).unwrap(), text);
    }
}

#[test]
fn booked_value_residual_is_not_duplicated_by_rows_or_later_confirmations() {
    let mut totals = vec![];
    for split in ["single", "two_rows", "two_confirmations"] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let m = item(&mut store, "가상 최소 금액 잔여");
        let o = observe(&mut store, &m, "0.000002", Some("0.000001"));
        let mut booked = 0i64;
        if split == "two_confirmations" {
            for _ in 0..2 {
                let s = late(
                    &mut store,
                    &m,
                    "0.000001",
                    "0.000001",
                    vec![selected(&o, "0.000001")],
                );
                let done = confirm(&mut store, &s);
                booked += sunchang_hansang::domain::Money::parse(
                    detail(&store, &done)["reconciliation"]["effect"]["booked_value"]
                        .as_str()
                        .unwrap(),
                )
                .unwrap()
                .micros();
            }
        } else {
            let rows = if split == "two_rows" {
                vec![selected(&o, "0.000001"), selected(&o, "0.000001")]
            } else {
                vec![selected(&o, "0.000002")]
            };
            let s = late(&mut store, &m, "0.000002", "0.000002", rows);
            let done = confirm(&mut store, &s);
            booked = sunchang_hansang::domain::Money::parse(
                detail(&store, &done)["reconciliation"]["effect"]["booked_value"]
                    .as_str()
                    .unwrap(),
            )
            .unwrap()
            .micros();
        }
        totals.push((
            split.to_string(),
            holding(&store, &m)["settlement_basis"]["value"].clone(),
            booked,
        ));
        drop(store);
        Store::open(root.path()).unwrap();
    }
    assert!(
        totals
            .iter()
            .all(|(_, v, b)| *v == json!("0.000002") && *b == 1),
        "{totals:?}"
    );
}

#[test]
fn mixed_excess_and_pool_share_one_observation_booked_value_residual() {
    let mut observed = vec![];
    for separate in [false, true] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let m = item(&mut store, "가상 혼합 최소잔여");
        observe(&mut store, &m, "0.000002", Some("0.000001"));
        let mut excess = row(&m, "0.000004");
        excess["excess"] =
            json!({"reviewed":true,"reason":"가상 확인 초과","value":{"kind":"existing_price"}});
        let s = save(
            &mut store,
            json!({"kind":"stocktake","version":1,"data":{"rows":[excess],"note":"가상 초과"}}),
        );
        let done = confirm(&mut store, &s);
        let o = value(
            store
                .stocktake_detail(
                    done["result"]["confirmation"]["result"]["stocktake_id"]
                        .as_str()
                        .unwrap(),
                )
                .unwrap()
                .unwrap(),
        )["observations"][0]
            .clone();
        assert_eq!(o["effect"]["calculation"]["excess_value"], "0.000001");
        let mut specific = selected(&o, "0.000001");
        specific["source"] = json!({"kind":"source","source_key":{"kind":"observation_excess","id":o["id"],"position":0}});
        if separate {
            let s = late(&mut store, &m, "0.000001", "0.000001", vec![specific]);
            confirm(&mut store, &s);
            let s = late(
                &mut store,
                &m,
                "0.000003",
                "0.000003",
                vec![selected(&o, "0.000003")],
            );
            confirm(&mut store, &s);
        } else {
            let s = late(
                &mut store,
                &m,
                "0.000004",
                "0.000004",
                vec![specific, selected(&o, "0.000003")],
            );
            confirm(&mut store, &s);
        }
        observed.push((
            separate,
            holding(&store, &m)["settlement_basis"]["value"].clone(),
        ));
        drop(store);
        Store::open(root.path()).unwrap();
    }
    assert!(
        observed.iter().all(|(_, v)| *v == json!("0.000004")),
        "{observed:?}"
    );
}

#[test]
fn live_recover_rejects_physical_reconciliation_columns_that_disagree_with_json() {
    let mut accepted = vec![];
    for mode in ["receipt", "link", "version", "reservation", "effect"] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let m = item(&mut store, "가상 물리키 확인");
        let o = observe(&mut store, &m, "10", Some("100"));
        let s = late(&mut store, &m, "3", "60", vec![selected(&o, "3")]);
        confirm(&mut store, &s);
        let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
        db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        match mode {
            "receipt" => {
                db.execute(
                    "UPDATE receipt_reconciliations SET recorded_at='2026-01-01T00:00:00Z'",
                    [],
                )
                .unwrap();
            }
            "link" => {
                db.execute("UPDATE inclusion_links SET selection_position=1", [])
                    .unwrap();
            }
            "version" => {
                db.execute("UPDATE inclusion_link_versions SET sequence=sequence-1", [])
                    .unwrap();
            }
            "reservation" => {
                db.execute(
                    "UPDATE inclusion_reservations SET id=?1",
                    [Uuid::new_v4().to_string()],
                )
                .unwrap();
            }
            "effect" => {
                db.execute(
                    "UPDATE reconciliation_material_effects SET sequence=sequence-1",
                    [],
                )
                .unwrap();
            }
            _ => unreachable!(),
        }
        drop(db);
        let dbpath = root.path().join("app.sqlite3");
        let before = fs::read(&dbpath).unwrap();
        if store.recover().is_ok() {
            accepted.push(mode);
        }
        assert_eq!(fs::read(&dbpath).unwrap(), before);
    }
    assert!(
        accepted.is_empty(),
        "physical-only corruption accepted: {accepted:?}"
    );
}

#[test]
fn explicit_zero_selection_preserves_anchor_without_creating_reservations() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let m = item(&mut store, "가상 명시 영 선택");
    let o = observe(&mut store, &m, "0", Some("0"));
    let s = late(&mut store, &m, "2", "0", vec![selected(&o, "0")]);
    let p = preview(&store, &s);
    assert!(p["issues"].as_array().unwrap().is_empty(), "{p}");
    let done = confirm(&mut store, &s);
    let d = detail(&store, &done);
    assert_eq!(
        d["reconciliation"]["effect"]["selections"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        d["reconciliation"]["effect"]["selections"][0]["included_quantity"],
        "0"
    );
    assert_eq!(
        d["reconciliation"]["effect"]["selections"][0]["observation_version_id"],
        o["version_id"]
    );
    assert!(
        d["reconciliation"]["effect"]["selections"][0]["portions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(count(root.path(), "inclusion_links"), 0);
    assert_eq!(count(root.path(), "inclusion_reservations"), 0);
    assert_eq!(holding(&store, &m)["settlement_basis"]["quantity"], "2");
    drop(store);
    Store::open(root.path()).unwrap();
}
