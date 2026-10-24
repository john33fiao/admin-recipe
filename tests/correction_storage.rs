use rusqlite::{Connection, types::Value as SqlValue};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use serde_json::json;
use std::{collections::BTreeMap, fs, path::Path};
use sunchang_hansang::store::Store;
use sunchang_hansang::{catalog::MaterialSummary, store::StoreError};
use tempfile::TempDir;
use uuid::Uuid;

fn decode<T: DeserializeOwned>(v: Value) -> T {
    serde_json::from_value(v).unwrap()
}
fn value(v: impl Serialize) -> Value {
    serde_json::to_value(v).unwrap()
}
fn meta(s: &Store) -> Value {
    let state = s.catalog_state().unwrap();
    json!({"request_id":Uuid::new_v4().to_string(),"epoch":state.epoch,"expected_revision":state.revision})
}
fn item(s: &mut Store, name: &str) -> MaterialSummary {
    let mut request = meta(s);
    request["name"] = json!(name);
    request["unit"] = json!("g");
    request["kind"] = json!("raw");
    request["description"] = json!("가상 정정 검증");
    s.catalog_create(decode(request)).unwrap().value
}
fn save(s: &mut Store, payload: Value, target: Option<&str>) -> Value {
    value(
        s.business_create(decode(
            json!({"meta":meta(s),"payload":payload,"corrects_confirmation_id":target}),
        ))
        .unwrap(),
    )
}
fn bid(s: &Value) -> &str {
    s["result"]["item"]["id"].as_str().unwrap()
}
fn preview(s: &Store, saved: &Value) -> Value {
    value(
        s.business_preview(
            bid(saved),
            decode(json!({"expected_head_id":saved["result"]["head"]["id"]})),
        )
        .unwrap(),
    )
}
fn confirmation_request(s: &Store, saved: &Value) -> Value {
    json!({"meta":meta(s),"expected_head_id":saved["result"]["head"]["id"]})
}
fn confirm(s: &mut Store, saved: &Value) -> Value {
    let p = preview(s, saved);
    assert_eq!(p["can_confirm"], true, "{p}");
    value(
        s.business_confirm(bid(saved), decode(confirmation_request(s, saved)))
            .unwrap(),
    )
}
fn receipt_payload(m: &MaterialSummary, q: &str, a: &str) -> Value {
    json!({"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":m.id,"material_version_id":m.version_id},"quantity":q,"unit":"g","amount":a,"receipt_date":"2020-02-29","note":"가상 입고","stock_effect":{"kind":"unreflected"}}})
}
fn receipt(s: &mut Store, m: &MaterialSummary, q: &str, a: &str) -> Value {
    let saved = save(s, receipt_payload(m, q, a), None);
    confirm(s, &saved)
}
fn cid(done: &Value) -> &str {
    done["result"]["confirmation"]["id"].as_str().unwrap()
}
fn correction(s: &mut Store, target: &str, action: Value) -> Value {
    let seed = value(s.correction_seed(target).unwrap().unwrap());
    save(
        s,
        json!({"kind":"correction","version":1,"data":{"expected_target_version_id":seed["current"]["reference"]["version_id"],"reason":"가상 오입력 정정","action":action,"observation_reviews":[],"linked_receipt_changes":[]}}),
        Some(target),
    )
}
fn replace_receipt(s: &mut Store, target: &str, m: &MaterialSummary, q: &str, a: &str) -> Value {
    let payload = receipt_payload(m, q, a);
    correction(
        s,
        target,
        json!({"kind":"replace","replacement":{"kind":"receipt","data":payload["data"]}}),
    )
}
fn holding(s: &Store, m: &MaterialSummary) -> Value {
    value(s.holding_detail(&m.id).unwrap().unwrap())
}
fn observe(s: &mut Store, m: &MaterialSummary, q: &str) -> Value {
    let saved = save(
        s,
        json!({"kind":"stocktake","version":1,"data":{"rows":[{"material_id":m.id,"material_version_id":m.version_id,"quantity":q,"unit":"g","initial_value":null,"excess":null}],"note":"가상 실측"}}),
        None,
    );
    confirm(s, &saved)
}
fn waste(s: &mut Store, m: &MaterialSummary, q: &str) -> Value {
    let saved = save(
        s,
        json!({"kind":"waste","version":1,"data":{"material_id":m.id,"material_version_id":m.version_id,"quantity":q,"unit":"g","reason":"discard","note":"가상 폐기"}}),
        None,
    );
    confirm(s, &saved)
}
fn journals(root: &Path) -> Vec<Vec<u8>> {
    let mut paths = fs::read_dir(root.join("snapshots"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect::<Vec<_>>();
    paths.sort();
    paths.into_iter().map(|p| fs::read(p).unwrap()).collect()
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
    tx.execute_batch(include_str!("fixtures/schema-v11.sql"))
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
    for (seq, payload) in &rows {
        fs::write(root.join(format!("snapshots/{seq:020}.json")), payload).unwrap();
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
fn rows(db: &Connection, table: &str, columns: &[String]) -> Vec<Vec<SqlValue>> {
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
fn schema_eleven_migration_preserves_all_original_columns_and_journal_bytes() {
    let root = TempDir::new().unwrap();
    let journals = install(root.path());
    assert_eq!(journals.len(), 27);
    assert_eq!(
        journals.last().unwrap().1,
        include_str!("fixtures/snapshot-v2-schema11.json")
    );
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let names = db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap()
        .query_map([], |r| r.get::<_,String>(0)).unwrap()
        .collect::<rusqlite::Result<Vec<_>>>().unwrap();
    let before = names
        .into_iter()
        .map(|name| {
            let cols = columns(&db, &name);
            let data = rows(&db, &name, &cols);
            (name, (cols, data))
        })
        .collect::<BTreeMap<_, _>>();
    drop(db);
    let store = Store::open(root.path()).unwrap();
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        12
    );
    for (name, (columns, original)) in before {
        assert_eq!(rows(&db, &name, &columns), original, "{name}");
    }
    for name in [
        "correction_records",
        "correction_event_versions",
        "correction_event_heads",
        "correction_projection_versions",
        "correction_projection_heads",
        "correction_price_bases",
    ] {
        assert_eq!(
            db.query_row(&format!("SELECT COUNT(*) FROM {name}"), [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
    assert_eq!(store.catalog_state().unwrap().revision, "27");
    for (seq, payload) in &journals {
        assert_eq!(
            fs::read_to_string(root.path().join(format!("snapshots/{seq:020}.json"))).unwrap(),
            *payload
        );
    }
    let newest: Value = serde_json::from_str(&journals.last().unwrap().1).unwrap();
    assert_eq!(newest["schema_version"], 11);
    assert_eq!(
        newest["state"]["mutation_requests"]
            .as_array()
            .unwrap()
            .len(),
        27
    );
}

#[test]
fn void_first_receipt_restores_unknown_and_new_receipt_establishes_a_fresh_basis() {
    let root = TempDir::new().unwrap();
    let mut s = Store::open(root.path()).unwrap();
    let m = item(&mut s, "가상 취소 후 시작");
    let original = receipt(&mut s, &m, "10", "100");
    let old = journals(root.path());
    let saved = correction(&mut s, cid(&original), json!({"kind":"void"}));
    let request = confirmation_request(&s, &saved);
    let done = value(
        s.business_confirm(bid(&saved), decode(request.clone()))
            .unwrap(),
    );
    let h = holding(&s, &m);
    assert!(h["settlement_basis"].is_null(), "{h}");
    assert!(h["last_valid_price"].is_null());
    assert_eq!(h["price"]["source"], "missing");
    assert_eq!(count(root.path(), "receipts"), 1);
    assert_eq!(count(root.path(), "holding_versions"), 1);
    receipt(&mut s, &m, "2", "60");
    let now = holding(&s, &m);
    assert_eq!(now["settlement_basis"]["quantity"], "2");
    assert_eq!(now["settlement_basis"]["value"], "60");
    assert_eq!(
        value(s.business_confirm(bid(&saved), decode(request)).unwrap()),
        done
    );
    assert_eq!(&journals(root.path())[..old.len()], old);
    drop(s);
    let reopened = Store::open(root.path()).unwrap();
    assert_eq!(holding(&reopened, &m)["settlement_basis"]["value"], "60");
}

#[test]
fn receipt_replacement_is_atomic_preserves_raw_rows_and_rejects_stale_subject_heads() {
    let root = TempDir::new().unwrap();
    let mut s = Store::open(root.path()).unwrap();
    let m = item(&mut s, "가상 수량 금액 정정");
    let original = receipt(&mut s, &m, "10", "100");
    let stale = replace_receipt(&mut s, cid(&original), &m, "12", "240");
    let saved = replace_receipt(&mut s, cid(&original), &m, "20", "300");
    let p = preview(&s, &saved);
    let before = s.catalog_state().unwrap();
    let files = journals(root.path());
    assert_eq!(p["can_confirm"], true);
    assert_eq!(value(s.catalog_state().unwrap()), value(before));
    assert_eq!(journals(root.path()), files);
    confirm(&mut s, &saved);
    let h = holding(&s, &m);
    assert_eq!(h["settlement_basis"]["quantity"], "20");
    assert_eq!(h["settlement_basis"]["value"], "300");
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    assert_eq!(
        db.query_row("SELECT quantity_micros FROM receipts", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        10_000_000
    );
    let stale_request = confirmation_request(&s, &stale);
    assert!(matches!(
        s.business_confirm(bid(&stale), decode(stale_request)),
        Err(StoreError::StaleState(_))
    ));
    assert_eq!(count(root.path(), "correction_records"), 1);
    drop(db);
    drop(s);
    Store::open(root.path()).unwrap();
}

#[test]
fn correction_preserves_later_measurement_and_uses_logical_observation_boundary() {
    let root = TempDir::new().unwrap();
    let mut s = Store::open(root.path()).unwrap();
    let m = item(&mut s, "가상 관측 경계");
    let original = receipt(&mut s, &m, "10", "100");
    let observed = observe(&mut s, &m, "5");
    let before = holding(&s, &m)["observation"].clone();
    let saved = replace_receipt(&mut s, cid(&original), &m, "8", "160");
    let p = preview(&s, &saved);
    assert_eq!(p["can_confirm"], true, "{p}");
    confirm(&mut s, &saved);
    let h = holding(&s, &m);
    assert_eq!(h["settlement_basis"]["quantity"], "5");
    assert_eq!(h["settlement_basis"]["value"], "100");
    assert_eq!(h["observation"]["seq"], before["seq"]);
    assert_eq!(h["observation"]["recorded_at"], before["recorded_at"]);
    let next = observe(&mut s, &m, "4");
    let detail = value(
        s.stocktake_detail(
            next["result"]["confirmation"]["result"]["stocktake_id"]
                .as_str()
                .unwrap(),
        )
        .unwrap()
        .unwrap(),
    );
    assert_eq!(
        detail["observations"][0]["effect"]["calculation"]["consumption_quantity"],
        "1"
    );
    assert_eq!(
        detail["observations"][0]["effect"]["calculation"]["consumption_value"],
        "20"
    );
    let original_detail = value(
        s.business_detail(observed["result"]["item"]["id"].as_str().unwrap())
            .unwrap()
            .unwrap(),
    );
    assert!(
        !original_detail["confirmations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    drop(s);
    Store::open(root.path()).unwrap();
}

#[test]
fn known_zero_retains_a_real_corrected_price_basis_without_fake_positive_holding() {
    let root = TempDir::new().unwrap();
    let mut s = Store::open(root.path()).unwrap();
    let m = item(&mut s, "가상 전량 폐기 단가");
    let original = receipt(&mut s, &m, "10", "100");
    waste(&mut s, &m, "10");
    let saved = replace_receipt(&mut s, cid(&original), &m, "10", "200");
    confirm(&mut s, &saved);
    let h = holding(&s, &m);
    assert_eq!(h["settlement_basis"]["quantity"], "0");
    assert_eq!(h["settlement_basis"]["value"], "0");
    assert_eq!(h["price"]["source"], "last_valid");
    assert_eq!(h["price"]["amount"], "200");
    assert_eq!(h["price"]["quantity"], "10");
    assert_eq!(count(root.path(), "holding_versions"), 3);
    assert_eq!(count(root.path(), "correction_price_bases"), 1);
    let recipe=value(s.recipe_create(decode(json!({"meta":meta(&s),"owner":{"mode":"new","name":"가상 정정 단가 메뉴","unit":"개","kind":"menu","description":""},"data":{"basis_count":"1","reference_output":null,"selling_price":"100","cooking_description":"","process":"","reference_time":"","lines":[{"material_id":m.id,"material_version_id":m.version_id,"quantity":"1","unit":"g","note":""}]}}))).unwrap());
    let sale = save(
        &mut s,
        json!({"kind":"sale","version":1,"data":{"material_id":recipe["item"]["id"],"material_version_id":recipe["item"]["version_id"],"recipe_version_id":recipe["version"]["id"],"count":"1","note":"가상 실제 단가 FK"}}),
        None,
    );
    let sold = confirm(&mut s, &sale);
    let sale = value(
        s.sale_detail(
            sold["result"]["confirmation"]["result"]["sale_id"]
                .as_str()
                .unwrap(),
        )
        .unwrap()
        .unwrap(),
    );
    assert_eq!(sale["item"]["recipe_cost"]["total_cost"], "20");
    assert_eq!(
        sale["item"]["recipe_cost"]["rows"][0]["price"]["source"],
        "last_valid"
    );
    assert_eq!(
        sale["item"]["recipe_cost"]["rows"][0]["price"]["source_id"],
        h["price"]["source_id"]
    );
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    assert_eq!(db.query_row("SELECT COUNT(*) FROM sale_cost_rows WHERE holding_version_id IS NULL AND correction_price_basis_id IS NOT NULL",[],|r|r.get::<_,i64>(0)).unwrap(),1);
    assert_eq!(count(root.path(), "holding_versions"), 3);
    drop(db);
    drop(s);
    Store::open(root.path()).unwrap();
}

#[test]
fn normal_stocktake_after_correction_retains_logical_basis_when_original_receipt_is_corrected_again()
 {
    let root = TempDir::new().unwrap();
    let mut s = Store::open(root.path()).unwrap();
    let m = item(&mut s, "가상 정정 뒤 실사 경계");
    let original = receipt(&mut s, &m, "10", "100");
    let first = replace_receipt(&mut s, cid(&original), &m, "10", "200");
    confirm(&mut s, &first);
    let observed = observe(&mut s, &m, "8");
    let stocktake_id = observed["result"]["confirmation"]["result"]["stocktake_id"]
        .as_str()
        .unwrap();
    let original_detail = value(s.stocktake_detail(stocktake_id).unwrap().unwrap());
    let calculation = &original_detail["observations"][0]["effect"]["calculation"];
    assert_eq!(calculation["allocation"]["basis"]["kind"], "correction");
    assert_eq!(calculation["consumption_quantity"], "2");
    assert_eq!(calculation["consumption_value"], "40");
    let boundary = holding(&s, &m)["observation"].clone();
    assert_eq!(holding(&s, &m)["settlement_basis"]["value"], "160");

    let second = replace_receipt(&mut s, cid(&original), &m, "10", "300");
    confirm(&mut s, &second);
    let current = holding(&s, &m);
    assert_eq!(current["settlement_basis"]["quantity"], "8");
    assert_eq!(current["settlement_basis"]["value"], "240");
    assert_eq!(current["observation"]["seq"], boundary["seq"]);
    assert_eq!(
        current["observation"]["recorded_at"],
        boundary["recorded_at"]
    );
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let latest: Value = serde_json::from_str(
        &db.query_row(
            "SELECT allocation_json FROM consumption_versions ORDER BY rowid DESC LIMIT 1",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(latest["total_quantity"], "2");
    assert_eq!(latest["total_value"], "60");
    assert_eq!(count(root.path(), "receipts"), 1);
    assert_eq!(count(root.path(), "correction_records"), 2);
    drop(db);
    drop(s);
    let reopened = Store::open(root.path()).unwrap();
    assert_eq!(holding(&reopened, &m), current);
}

#[test]
fn void_waste_restores_value_then_normal_waste_uses_corrected_state() {
    let root = TempDir::new().unwrap();
    let mut s = Store::open(root.path()).unwrap();
    let m = item(&mut s, "가상 폐기 취소");
    receipt(&mut s, &m, "10", "100");
    let original = waste(&mut s, &m, "2");
    let saved = correction(&mut s, cid(&original), json!({"kind":"void"}));
    confirm(&mut s, &saved);
    assert_eq!(holding(&s, &m)["settlement_basis"]["quantity"], "10");
    waste(&mut s, &m, "1");
    let after = holding(&s, &m);
    assert_eq!(after["settlement_basis"]["quantity"], "9");
    // Partial waste preserves total holding value under ADR-007/T-010.
    assert_eq!(after["settlement_basis"]["value"], "100");
    drop(s);
    Store::open(root.path()).unwrap();
}

#[test]
fn correction_draft_survives_restart_and_failed_confirmation_does_not_append_any_domain_row() {
    let root = TempDir::new().unwrap();
    let mut s = Store::open(root.path()).unwrap();
    let m = item(&mut s, "가상 초안 원자성");
    let original = receipt(&mut s, &m, "10", "100");
    let mut saved = replace_receipt(&mut s, cid(&original), &m, "", "100");
    let business = bid(&saved).to_string();
    drop(s);
    let mut s = Store::open(root.path()).unwrap();
    saved["result"] = value(s.business_detail(&business).unwrap().unwrap());
    let p = preview(&s, &saved);
    assert_eq!(p["can_confirm"], false);
    let state = value(s.catalog_state().unwrap());
    let files = journals(root.path());
    let request = confirmation_request(&s, &saved);
    assert!(matches!(
        s.business_confirm(&business, decode(request)),
        Err(StoreError::Validation(_))
    ));
    assert_eq!(value(s.catalog_state().unwrap()), state);
    assert_eq!(journals(root.path()), files);
    assert_eq!(count(root.path(), "correction_records"), 0);
    assert_eq!(holding(&s, &m)["settlement_basis"]["quantity"], "10");
}

#[test]
fn failure_at_snapshot_write_rolls_back_every_correction_relation_then_same_request_succeeds() {
    let root = TempDir::new().unwrap();
    let mut s = Store::open(root.path()).unwrap();
    let m = item(&mut s, "가상 말단 실패");
    let original = receipt(&mut s, &m, "10", "100");
    let saved = replace_receipt(&mut s, cid(&original), &m, "20", "400");
    let request = confirmation_request(&s, &saved);
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let state = value(s.catalog_state().unwrap());
    let files = journals(root.path());
    let holdings = count(root.path(), "holding_versions");
    db.execute_batch("CREATE TRIGGER correction_tail_failure BEFORE UPDATE OF payload ON snapshot_outbox BEGIN SELECT RAISE(FAIL,'injected correction tail failure'); END").unwrap();
    assert!(matches!(
        s.business_confirm(bid(&saved), decode(request.clone())),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(value(s.catalog_state().unwrap()), state);
    assert_eq!(journals(root.path()), files);
    assert_eq!(count(root.path(), "holding_versions"), holdings);
    for table in [
        "correction_records",
        "correction_event_versions",
        "correction_event_heads",
        "correction_projection_versions",
        "correction_projection_heads",
        "correction_price_bases",
    ] {
        assert_eq!(count(root.path(), table), 0, "{table}");
    }
    db.execute_batch("DROP TRIGGER correction_tail_failure")
        .unwrap();
    s.business_confirm(bid(&saved), decode(request)).unwrap();
    assert_eq!(holding(&s, &m)["settlement_basis"]["value"], "400");
}

#[test]
fn committed_publication_failure_retries_exact_response_before_stale_target_checks() {
    let root = TempDir::new().unwrap();
    let mut s = Store::open(root.path()).unwrap();
    let m = item(&mut s, "가상 발행 재시도");
    let original = receipt(&mut s, &m, "10", "100");
    let saved = replace_receipt(&mut s, cid(&original), &m, "20", "400");
    let request = confirmation_request(&s, &saved);
    let sequence = s.catalog_state().unwrap().revision.parse::<i64>().unwrap() + 1;
    let path = root.path().join(format!("snapshots/{sequence:020}.json"));
    fs::create_dir(&path).unwrap();
    assert!(matches!(
        s.business_confirm(bid(&saved), decode(request.clone())),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(count(root.path(), "correction_records"), 1);
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let expected: String = db
        .query_row(
            "SELECT response FROM mutation_requests WHERE request_id=?1",
            [request["meta"]["request_id"].as_str().unwrap()],
            |r| r.get(0),
        )
        .unwrap();
    let journal: String = db
        .query_row(
            "SELECT payload FROM snapshot_outbox WHERE sequence=?1",
            [sequence],
            |r| r.get(0),
        )
        .unwrap();
    assert!(matches!(
        s.business_confirm(bid(&saved), decode(request.clone())),
        Err(StoreError::Unavailable(_))
    ));
    fs::remove_dir(&path).unwrap();
    let actual = s
        .business_confirm(bid(&saved), decode(request.clone()))
        .unwrap();
    assert_eq!(serde_json::to_string(&actual).unwrap(), expected);
    assert_eq!(fs::read_to_string(&path).unwrap(), journal);
    receipt(&mut s, &m, "1", "30");
    assert_eq!(
        serde_json::to_string(&s.business_confirm(bid(&saved), decode(request)).unwrap()).unwrap(),
        expected
    );
    assert_eq!(count(root.path(), "correction_records"), 1);
    drop(db);
    drop(s);
    Store::open(root.path()).unwrap();
}

#[test]
fn allocated_sale_zero_and_void_keep_consumption_totals_and_never_change_holding() {
    let root = TempDir::new().unwrap();
    install(root.path());
    let mut s = Store::open(root.path()).unwrap();
    let original: Value =
        serde_json::from_str(include_str!("fixtures/snapshot-v2-schema11.json")).unwrap();
    let sale = &original["state"]["sales"][0];
    let target = sale["original_confirmation_id"].as_str().unwrap();
    let sale_id = sale["id"].as_str().unwrap();
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let old_holdings = rows(&db, "holding_versions", &columns(&db, "holding_versions"));
    let old_sale = s.sale_detail(sale_id).unwrap().unwrap().item;
    let saved = correction(
        &mut s,
        target,
        json!({"kind":"replace","replacement":{"kind":"sale_count","data":{"count":"0"}}}),
    );
    let p = preview(&s, &saved);
    assert_eq!(p["can_confirm"], true, "{p}");
    confirm(&mut s, &saved);
    assert_eq!(
        rows(&db, "holding_versions", &columns(&db, "holding_versions")),
        old_holdings
    );
    let d = value(s.sale_detail(sale_id).unwrap().unwrap());
    assert_eq!(d["effective"]["kind"], "active");
    assert_eq!(d["effective"]["count"], "0");
    assert_eq!(d["item"], value(old_sale));
    let latest: Value = serde_json::from_str(
        &db.query_row(
            "SELECT allocation_json FROM consumption_versions ORDER BY rowid DESC LIMIT 1",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(latest["total_quantity"], "500");
    assert_eq!(latest["total_value"], "7500");
    assert_eq!(latest["unallocated_quantity"], "500");
    assert_eq!(latest["unallocated_value"], "7500");
    let cancel = correction(&mut s, target, json!({"kind":"void"}));
    confirm(&mut s, &cancel);
    let d = value(s.sale_detail(sale_id).unwrap().unwrap());
    assert_eq!(d["effective"]["kind"], "void");
    assert_eq!(count(root.path(), "sale_versions"), 3);
    assert_eq!(
        rows(&db, "holding_versions", &columns(&db, "holding_versions")),
        old_holdings
    );
    drop(db);
    drop(s);
    Store::open(root.path()).unwrap();
}

#[test]
fn order_receipt_void_reopens_real_order_and_a_new_receipt_preserves_both_original_closures() {
    let root = TempDir::new().unwrap();
    let mut s = Store::open(root.path()).unwrap();
    let m = item(&mut s, "가상 주문 재입고");
    let order = save(
        &mut s,
        json!({"kind":"purchase_order","version":1,"data":{"action":"place","material_id":m.id,"material_version_id":m.version_id,"quantity":"10","unit":"g","supplier":"","expected_date":"","estimated_amount":"100","note":"가상 주문"}}),
        None,
    );
    let ordered = confirm(&mut s, &order);
    let order_id = ordered["result"]["confirmation"]["result"]["order_id"]
        .as_str()
        .unwrap();
    let head = value(s.order_detail(order_id).unwrap().unwrap())["head"].clone();
    let mut payload = receipt_payload(&m, "10", "100");
    payload["data"]["source"] = json!({"kind":"order","order_id":order_id,"expected_order_version_id":head["id"],"material_version_id":m.version_id});
    let saved = save(&mut s, payload.clone(), None);
    let original = confirm(&mut s, &saved);
    let cancel = correction(&mut s, cid(&original), json!({"kind":"void"}));
    confirm(&mut s, &cancel);
    let reopened = value(s.order_detail(order_id).unwrap().unwrap());
    assert_eq!(reopened["head"]["status"], "open");
    assert_eq!(reopened["head"]["action"], "correct");
    payload["data"]["source"]["expected_order_version_id"] = reopened["head"]["id"].clone();
    payload["data"]["quantity"] = json!("9");
    let next = save(&mut s, payload, None);
    confirm(&mut s, &next);
    assert_eq!(count(root.path(), "order_receipt_closures"), 2);
    assert_eq!(holding(&s, &m)["settlement_basis"]["quantity"], "9");
    drop(s);
    Store::open(root.path()).unwrap();
}

#[test]
fn ordinary_receipt_can_be_corrected_to_inclusion_and_next_normal_inclusion_reuses_effective_reservations()
 {
    let root = TempDir::new().unwrap();
    let mut s = Store::open(root.path()).unwrap();
    let m = item(&mut s, "가상 포함 전환");
    receipt(&mut s, &m, "10", "100");
    let observed = observe(&mut s, &m, "10");
    let observation = value(
        s.stocktake_detail(
            observed["result"]["confirmation"]["result"]["stocktake_id"]
                .as_str()
                .unwrap(),
        )
        .unwrap()
        .unwrap(),
    )["observations"][0]
        .clone();
    let original = receipt(&mut s, &m, "2", "40");
    let mut replacement = receipt_payload(&m, "2", "40")["data"].clone();
    replacement["stock_effect"] = json!({"kind":"reconciled","selections":[{"observation_version_id":observation["version_id"],"source":{"kind":"all_available"},"included_quantity":"2"}]});
    let saved = correction(
        &mut s,
        cid(&original),
        json!({"kind":"replace","replacement":{"kind":"receipt","data":replacement}}),
    );
    eprintln!("confirm ordinary-to-reconciled correction");
    confirm(&mut s, &saved);
    let h = holding(&s, &m);
    assert_eq!(h["settlement_basis"]["quantity"], "10");
    assert_eq!(h["settlement_basis"]["value"], "120");
    let candidates = value(s.receipt_inclusion_candidates(&m.id).unwrap());
    let anchor = candidates["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["observation_id"] == observation["id"])
        .unwrap();
    assert_eq!(anchor["available_quantity"], "8");
    let mut payload = receipt_payload(&m, "1", "30");
    payload["data"]["stock_effect"] = json!({"kind":"reconciled","selections":[{"observation_version_id":anchor["observation_version_id"],"source":{"kind":"all_available"},"included_quantity":"1"}]});
    let next = save(&mut s, payload, None);
    eprintln!("confirm next normal reconciled receipt");
    confirm(&mut s, &next);
    assert_eq!(holding(&s, &m)["settlement_basis"]["value"], "140");
    assert_eq!(count(root.path(), "receipts"), 3);
    drop(s);
    Store::open(root.path()).unwrap();
}

#[test]
fn semantic_mismatch_before_commit_rolls_back_head_rows_request_and_outbox() {
    let root = TempDir::new().unwrap();
    let mut s = Store::open(root.path()).unwrap();
    let m = item(&mut s, "가상 의미 불일치 주입");
    let saved = save(&mut s, receipt_payload(&m, "10", "100"), None);
    let request = confirmation_request(&s, &saved);
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let state = value(s.catalog_state().unwrap());
    let files = journals(root.path());
    let draft = value(s.business_detail(bid(&saved)).unwrap().unwrap());
    // SQL/FK-valid corruption: the input and holding still say 100, but the
    // immutable receipt is changed to 101 inside this transaction.
    db.execute_batch("CREATE TRIGGER semantic_receipt_mismatch AFTER INSERT ON receipts BEGIN UPDATE receipts SET amount_micros=amount_micros+1000000 WHERE id=NEW.id; END").unwrap();
    assert!(matches!(
        s.business_confirm(bid(&saved), decode(request.clone())),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(value(s.catalog_state().unwrap()), state);
    assert_eq!(journals(root.path()), files);
    assert_eq!(
        value(s.business_detail(bid(&saved)).unwrap().unwrap()),
        draft
    );
    assert_eq!(count(root.path(), "receipts"), 0);
    assert_eq!(count(root.path(), "holding_versions"), 0);
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM mutation_requests WHERE request_id=?1",
            [request["meta"]["request_id"].as_str().unwrap()],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    db.execute_batch("DROP TRIGGER semantic_receipt_mismatch")
        .unwrap();
    s.business_confirm(bid(&saved), decode(request)).unwrap();
    assert_eq!(holding(&s, &m)["settlement_basis"]["value"], "100");
}

#[test]
fn shrinking_initial_observation_requires_linked_receipt_revision_and_restarts_with_preserved_originals()
 {
    let root = TempDir::new().unwrap();
    let mut s = Store::open(root.path()).unwrap();
    let m = item(&mut s, "가상 초기 포함 근거 축소");
    let initial = save(
        &mut s,
        json!({"kind":"stocktake","version":1,"data":{"rows":[{"material_id":m.id,"material_version_id":m.version_id,"quantity":"10","unit":"g","initial_value":{"kind":"manual_amount","amount":"10"},"excess":null}],"note":"가상 시작 기준"}}),
        None,
    );
    let measured = confirm(&mut s, &initial);
    let stocktake_id = measured["result"]["confirmation"]["result"]["stocktake_id"]
        .as_str()
        .unwrap();
    let observation =
        value(s.stocktake_detail(stocktake_id).unwrap().unwrap())["observations"][0].clone();
    let mut payload = receipt_payload(&m, "3", "6");
    payload["data"]["stock_effect"] = json!({"kind":"reconciled","selections":[{"observation_version_id":observation["version_id"],"source":{"kind":"all_available"},"included_quantity":"3"}]});
    let saved = save(&mut s, payload.clone(), None);
    let received = confirm(&mut s, &saved);
    let receipt_id = received["result"]["confirmation"]["result"]["receipt_id"]
        .as_str()
        .unwrap();
    let original = value(s.receipt_detail(receipt_id).unwrap().unwrap());
    assert_eq!(holding(&s, &m)["settlement_basis"]["quantity"], "10");
    assert_eq!(holding(&s, &m)["settlement_basis"]["value"], "13");

    let seed = value(s.correction_seed(cid(&measured)).unwrap().unwrap());
    let mut data = seed["draft"].clone();
    data["reason"] = json!("가상 초기 실사 오입력과 연결 정정");
    data["action"]["replacement"]["data"]["rows"][0]["action"]["quantity"] = json!("2");
    let mut correction_payload = json!({"kind":"correction","version":1,"data":data});
    let draft = save(&mut s, correction_payload.clone(), Some(cid(&measured)));
    let blocked = preview(&s, &draft);
    assert_eq!(blocked["can_confirm"], false, "{blocked}");
    let state = value(s.catalog_state().unwrap());
    let files = journals(root.path());
    let request = confirmation_request(&s, &draft);
    assert!(matches!(
        s.business_confirm(bid(&draft), decode(request)),
        Err(StoreError::Validation(_))
    ));
    assert_eq!(value(s.catalog_state().unwrap()), state);
    assert_eq!(journals(root.path()), files);
    assert_eq!(count(root.path(), "correction_records"), 0);

    let receipt_seed = value(s.correction_seed(cid(&received)).unwrap().unwrap());
    payload["data"]["stock_effect"]["selections"][0]["included_quantity"] = json!("2");
    correction_payload["data"]["linked_receipt_changes"] = json!([{"receipt_id":receipt_id,"expected_version_id":receipt_seed["current"]["reference"]["version_id"],"action":{"kind":"replace","data":payload["data"]}}]);
    let revised = value(
        s.business_revise(
            bid(&draft),
            decode(json!({"meta":meta(&s),"expected_head_id":draft["result"]["head"]["id"],"payload":correction_payload})),
        )
        .unwrap(),
    );
    confirm(&mut s, &revised);
    // The corrected observation retains the explicitly confirmed value 10.
    // Its entire measured 2 is replaced by Ai=4; U=1 and Au=2 then enter once.
    let current = holding(&s, &m);
    assert_eq!(current["settlement_basis"]["quantity"], "3");
    assert_eq!(current["settlement_basis"]["value"], "6");
    assert_eq!(
        value(s.receipt_detail(receipt_id).unwrap().unwrap())["item"],
        original["item"]
    );
    assert_eq!(count(root.path(), "receipts"), 1);
    assert_eq!(count(root.path(), "correction_event_versions"), 2);
    drop(s);
    let reopened = Store::open(root.path()).unwrap();
    assert_eq!(holding(&reopened, &m), current);
}

#[test]
fn unified_sale_void_rejects_new_legacy_corrections_but_replays_saved_request_exactly() {
    let root = TempDir::new().unwrap();
    let mut s = Store::open(root.path()).unwrap();
    let m = item(&mut s, "가상 판매 정정 경계");
    let recipe = value(s.recipe_create(decode(json!({"meta":meta(&s),"owner":{"mode":"new","name":"가상 정정 경계 메뉴","unit":"개","kind":"menu","description":""},"data":{"basis_count":"1","reference_output":null,"selling_price":"100","cooking_description":"","process":"","reference_time":"","lines":[{"material_id":m.id,"material_version_id":m.version_id,"quantity":"0","unit":"g","note":""}]}}))).unwrap());
    let mut payload = json!({"kind":"sale","version":1,"data":{"material_id":recipe["item"]["id"],"material_version_id":recipe["item"]["version_id"],"recipe_version_id":recipe["version"]["id"],"count":"3","note":"가상 최초 판매"}});
    let original_draft = save(&mut s, payload.clone(), None);
    let original = confirm(&mut s, &original_draft);
    let sale_id = original["result"]["confirmation"]["result"]["sale_id"]
        .as_str()
        .unwrap();
    payload["data"]["count"] = json!("2");
    payload["data"]["note"] = json!("가상 이전 방식 정정 초안");
    let request =
        json!({"meta":meta(&s),"payload":payload,"corrects_confirmation_id":cid(&original)});
    let pending = value(s.business_create(decode(request.clone())).unwrap());
    let cancel = correction(&mut s, cid(&original), json!({"kind":"void"}));
    confirm(&mut s, &cancel);
    assert_eq!(
        value(s.sale_detail(sale_id).unwrap().unwrap())["effective"]["kind"],
        "void"
    );
    let state = value(s.catalog_state().unwrap());
    let files = journals(root.path());
    assert_eq!(
        value(s.business_create(decode(request.clone())).unwrap()),
        pending
    );
    assert_eq!(value(s.catalog_state().unwrap()), state);
    assert_eq!(journals(root.path()), files);

    let pending_preview = preview(&s, &pending);
    let mut fresh = request;
    fresh["meta"] = meta(&s);
    let fresh_result = s.business_create(decode(fresh)).map(value);
    let pending_request = confirmation_request(&s, &pending);
    let pending_result = s
        .business_confirm(bid(&pending), decode(pending_request))
        .map(value);
    assert!(
        pending_preview["can_confirm"] == false
            && matches!(
                &fresh_result,
                Err(StoreError::Validation(_) | StoreError::StaleState(_))
            )
            && matches!(
                &pending_result,
                Err(StoreError::Validation(_) | StoreError::StaleState(_))
            ),
        "preview: {pending_preview}; new legacy draft: {fresh_result:?}; pending legacy confirm: {pending_result:?}"
    );
    assert_eq!(value(s.catalog_state().unwrap()), state);
    assert_eq!(journals(root.path()), files);
    assert_eq!(count(root.path(), "sale_versions"), 1);
    drop(s);
    let reopened = Store::open(root.path()).unwrap();
    assert_eq!(
        value(reopened.sale_detail(sale_id).unwrap().unwrap())["effective"]["kind"],
        "void"
    );
}
