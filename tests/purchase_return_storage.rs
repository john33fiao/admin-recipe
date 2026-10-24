use rusqlite::{Connection, types::Value as SqlValue};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
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
    let c = s.catalog_state().unwrap();
    json!({"request_id":Uuid::new_v4().to_string(),"epoch":c.epoch,"expected_revision":c.revision})
}
fn material(s: &mut Store) -> MaterialSummary {
    let mut m = meta(s);
    m["name"] = json!("가상 반품 재료");
    m["unit"] = json!("g");
    m["kind"] = json!("raw");
    m["description"] = json!("");
    s.catalog_create(decode(m)).unwrap().value
}
fn save(s: &mut Store, payload: Value) -> Value {
    let request = json!({"meta":meta(s),"payload":payload,"corrects_confirmation_id":null});
    value(s.business_create(decode(request)).unwrap())
}
fn id(saved: &Value) -> &str {
    saved["result"]["item"]["id"].as_str().unwrap()
}
fn preview(s: &Store, saved: &Value) -> Value {
    value(
        s.business_preview(
            id(saved),
            decode(json!({"expected_head_id":saved["result"]["head"]["id"]})),
        )
        .unwrap(),
    )
}
fn request(s: &Store, saved: &Value) -> Value {
    json!({"meta":meta(s),"expected_head_id":saved["result"]["head"]["id"]})
}
fn confirm(s: &mut Store, saved: &Value) -> Value {
    let p = preview(s, saved);
    assert_eq!(p["can_confirm"], true, "{p}");
    value(
        s.business_confirm(id(saved), decode(request(s, saved)))
            .unwrap(),
    )
}
fn receipt(s: &mut Store, m: &MaterialSummary, q: &str, a: &str) -> String {
    let d = save(
        s,
        json!({"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":m.id,"material_version_id":m.version_id},"quantity":q,"unit":"g","amount":a,"receipt_date":"2020-02-29","note":"가상 구입","stock_effect":{"kind":"unreflected"}}}),
    );
    confirm(s, &d)["result"]["confirmation"]["result"]["receipt_id"]
        .as_str()
        .unwrap()
        .into()
}
fn return_payload(s: &Store, source: &str, q: &str) -> Value {
    let source = value(s.purchase_return_source(source).unwrap().unwrap());
    json!({"kind":"purchase_return","version":1,"data":{"receipt_id":source["original"]["id"],"expected_receipt_version_id":source["current"]["reference"]["version_id"],"quantity":q,"unit":"g","note":"가상 실제 반품"}})
}
fn return_draft(s: &mut Store, source: &str, q: &str) -> Value {
    let p = return_payload(s, source, q);
    save(s, p)
}
fn return_id(done: &Value) -> &str {
    done["result"]["confirmation"]["result"]["purchase_return_id"]
        .as_str()
        .unwrap()
}
fn h(s: &Store, m: &MaterialSummary) -> Value {
    value(s.holding_detail(&m.id).unwrap().unwrap())
}
fn journals(root: &Path) -> Vec<Vec<u8>> {
    let mut p = fs::read_dir(root.join("snapshots"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect::<Vec<_>>();
    p.sort();
    p.into_iter().map(|p| fs::read(p).unwrap()).collect()
}
fn counts(root: &Path) -> BTreeMap<String, i64> {
    let c = Connection::open(root.join("app.sqlite3")).unwrap();
    [
        "business_revisions",
        "business_confirmations",
        "purchase_returns",
        "holding_versions",
        "mutation_requests",
        "snapshot_outbox",
    ]
    .into_iter()
    .map(|t| {
        (
            t.into(),
            c.query_row(&format!("SELECT COUNT(*) FROM {t}"), [], |r| r.get(0))
                .unwrap(),
        )
    })
    .collect()
}

fn install(root: &Path) -> Vec<(i64, String)> {
    let mut db = Connection::open(root.join("app.sqlite3")).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    let tx = db.transaction().unwrap();
    tx.execute_batch("PRAGMA defer_foreign_keys=ON").unwrap();
    tx.execute_batch(include_str!("fixtures/schema-v12.sql"))
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
fn schema_twelve_migration_preserves_original_columns_and_every_journal_byte() {
    let root = TempDir::new().unwrap();
    let journals = install(root.path());
    assert_eq!(journals.len(), 34);
    assert_eq!(
        journals.last().unwrap().1,
        include_str!("fixtures/snapshot-v2-schema12.json")
    );
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let tables = db
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    let before = tables
        .into_iter()
        .map(|table| {
            let columns = columns(&db, &table);
            let original = rows(&db, &table, &columns);
            (table, (columns, original))
        })
        .collect::<BTreeMap<_, _>>();
    drop(db);
    let store = Store::open(root.path()).unwrap();
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        14
    );
    for (table, (columns, original)) in before {
        assert_eq!(rows(&db, &table, &columns), original, "{table}");
    }
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM purchase_returns", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
    assert_eq!(store.catalog_state().unwrap().revision, "34");
    for (seq, original) in journals {
        assert_eq!(
            fs::read_to_string(root.path().join(format!("snapshots/{seq:020}.json"))).unwrap(),
            original
        );
    }
}

#[test]
fn partial_return_uses_current_average_and_full_return_keeps_last_valid() {
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    let m = material(&mut s);
    let source = receipt(&mut s, &m, "10", "100");
    let draft = return_draft(&mut s, &source, "3");
    let before = journals(dir.path());
    let state = value(s.catalog_state().unwrap());
    let p = preview(&s, &draft);
    assert_eq!(p["effect"]["purchase_return"]["deducted_value"], "30");
    assert_eq!(value(s.catalog_state().unwrap()), state);
    assert_eq!(journals(dir.path()), before);
    let done = confirm(&mut s, &draft);
    let detail = value(s.purchase_return_detail(return_id(&done)).unwrap().unwrap());
    assert_eq!(detail["item"]["effect"]["source"]["original_amount"], "100");
    assert_eq!(detail["item"]["effect"]["after"]["quantity"], "7");
    assert_eq!(detail["item"]["effect"]["after"]["value"], "70");
    assert!(detail["item"]["effect"]["after"]["source_id"].is_string());
    let last = return_draft(&mut s, &source, "7");
    confirm(&mut s, &last);
    let holding = h(&s, &m);
    assert_eq!(holding["settlement_basis"]["quantity"], "0");
    assert_eq!(holding["settlement_basis"]["value"], "0");
    assert_eq!(holding["price"]["source"], "last_valid");
    assert_eq!(holding["price"]["amount"], "70");
    assert_eq!(holding["price"]["quantity"], "7");
    assert_eq!(holding["confirmed_changes"].as_array().unwrap().len(), 2);
    drop(s);
    let s = Store::open(dir.path()).unwrap();
    assert_eq!(h(&s, &m), holding);
}

#[test]
fn return_uses_weighted_holding_value_and_original_receipt_quota_separately() {
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    let m = material(&mut s);
    let source = receipt(&mut s, &m, "10", "100");
    receipt(&mut s, &m, "10", "300");
    let draft = return_draft(&mut s, &source, "5");
    let done = confirm(&mut s, &draft);
    let detail = value(s.purchase_return_detail(return_id(&done)).unwrap().unwrap());
    assert_eq!(detail["item"]["effect"]["deducted_value"], "100");
    assert_eq!(detail["item"]["effect"]["source"]["amount"], "100");
    assert_eq!(h(&s, &m)["settlement_basis"]["value"], "300");
    let over = return_draft(&mut s, &source, "6");
    assert_eq!(preview(&s, &over)["can_confirm"], false);
    let exact = return_draft(&mut s, &source, "5");
    confirm(&mut s, &exact);
    assert_eq!(
        value(s.purchase_return_source(&source).unwrap().unwrap())["current"]["remaining_quantity"],
        "0"
    );
    assert_eq!(h(&s, &m)["settlement_basis"]["quantity"], "10");
    drop(s);
    Store::open(dir.path()).unwrap();
}

#[test]
fn zero_value_is_returnable_but_zero_or_shortage_quantity_stays_in_draft() {
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    let m = material(&mut s);
    let source = receipt(&mut s, &m, "2", "0");
    for q in ["", "0", "-1", "3"] {
        let draft = return_draft(&mut s, &source, q);
        assert_eq!(preview(&s, &draft)["can_confirm"], false);
    }
    let draft = return_draft(&mut s, &source, "2");
    let done = confirm(&mut s, &draft);
    assert_eq!(
        value(s.purchase_return_detail(return_id(&done)).unwrap().unwrap())["item"]["effect"]["deducted_value"],
        "0"
    );
    assert_eq!(h(&s, &m)["price"]["source"], "last_valid");
    drop(s);
    Store::open(dir.path()).unwrap();
}

#[test]
fn original_request_replays_after_later_returns_without_consuming_quota_twice() {
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    let m = material(&mut s);
    let source = receipt(&mut s, &m, "10", "100");
    let draft = return_draft(&mut s, &source, "2");
    let req = request(&s, &draft);
    let original = value(s.business_confirm(id(&draft), decode(req.clone())).unwrap());
    let later = return_draft(&mut s, &source, "3");
    confirm(&mut s, &later);
    let before = counts(dir.path());
    let bytes = journals(dir.path());
    let state = value(s.catalog_state().unwrap());
    assert_eq!(
        value(s.business_confirm(id(&draft), decode(req.clone())).unwrap()),
        original
    );
    assert_eq!(counts(dir.path()), before);
    assert_eq!(journals(dir.path()), bytes);
    assert_eq!(value(s.catalog_state().unwrap()), state);
    let mut collision = req.clone();
    collision["expected_head_id"] = later["result"]["head"]["id"].clone();
    assert!(matches!(
        s.business_confirm(id(&later), decode(collision)),
        Err(StoreError::Conflict(_))
    ));
    drop(s);
    let mut s = Store::open(dir.path()).unwrap();
    assert_eq!(
        value(s.business_confirm(id(&draft), decode(req)).unwrap()),
        original
    );
    assert_eq!(
        value(s.purchase_return_source(&source).unwrap().unwrap())["current"]["returned_quantity"],
        "5"
    );
}

#[test]
fn semantic_failure_rolls_back_every_relation_and_same_uuid_can_retry() {
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    let m = material(&mut s);
    let source = receipt(&mut s, &m, "10", "100");
    let draft = return_draft(&mut s, &source, "2");
    let req = request(&s, &draft);
    let before = counts(dir.path());
    let bytes = journals(dir.path());
    let state = value(s.catalog_state().unwrap());
    let db = Connection::open(dir.path().join("app.sqlite3")).unwrap();
    db.execute_batch("CREATE TRIGGER corrupt_return AFTER INSERT ON purchase_returns BEGIN UPDATE purchase_returns SET quantity_micros=quantity_micros+1 WHERE id=NEW.id; END").unwrap();
    assert!(s.business_confirm(id(&draft), decode(req.clone())).is_err());
    assert_eq!(counts(dir.path()), before);
    assert_eq!(journals(dir.path()), bytes);
    assert_eq!(value(s.catalog_state().unwrap()), state);
    db.execute_batch("DROP TRIGGER corrupt_return").unwrap();
    let done = value(s.business_confirm(id(&draft), decode(req)).unwrap());
    assert!(!return_id(&done).is_empty());
}

#[test]
fn publication_failure_recovers_exact_response_before_successful_retry() {
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    let m = material(&mut s);
    let source = receipt(&mut s, &m, "10", "100");
    let draft = return_draft(&mut s, &source, "2");
    let req = request(&s, &draft);
    let next = s.catalog_state().unwrap().revision.parse::<i64>().unwrap() + 1;
    let path = dir.path().join(format!("snapshots/{next:020}.json"));
    fs::create_dir(&path).unwrap();
    assert!(s.business_confirm(id(&draft), decode(req.clone())).is_err());
    let db = Connection::open(dir.path().join("app.sqlite3")).unwrap();
    let (response,payload):(String,String)=db.query_row("SELECT r.response,o.payload FROM mutation_requests r JOIN snapshot_outbox o ON o.sequence=r.sequence WHERE r.request_id=?1",[req["meta"]["request_id"].as_str().unwrap()],|r|Ok((r.get(0)?,r.get(1)?))).unwrap();
    assert!(s.business_confirm(id(&draft), decode(req.clone())).is_err());
    fs::remove_dir(&path).unwrap();
    let done = value(s.business_confirm(id(&draft), decode(req)).unwrap());
    assert_eq!(done, serde_json::from_str::<Value>(&response).unwrap());
    assert_eq!(fs::read_to_string(path).unwrap(), payload);
    assert_eq!(counts(dir.path())["purchase_returns"], 1);
}

#[test]
fn schema_twelve_rejects_known_future_header_without_rewriting_original_data() {
    let dir = TempDir::new().unwrap();
    let original = install(dir.path());
    let db = Connection::open(dir.path().join("app.sqlite3")).unwrap();
    let seq = original.last().unwrap().0;
    let mut payload: Value = serde_json::from_str(&original.last().unwrap().1).unwrap();
    payload["schema_version"] = json!(13);
    payload["state"]["purchase_returns"] = json!([]);
    let changed = serde_json::to_string(&payload).unwrap();
    db.execute(
        "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=?2",
        rusqlite::params![changed, seq],
    )
    .unwrap();
    fs::write(
        dir.path().join(format!("snapshots/{seq:020}.json")),
        &changed,
    )
    .unwrap();
    drop(db);
    let before = fs::read(dir.path().join("app.sqlite3")).unwrap();
    let bytes = journals(dir.path());
    assert!(Store::open(dir.path()).is_err());
    assert_eq!(fs::read(dir.path().join("app.sqlite3")).unwrap(), before);
    assert_eq!(journals(dir.path()), bytes);
}

#[test]
fn return_physical_columns_and_matching_json_tampering_are_both_rejected() {
    for mode in [
        "scalar_quantity",
        "scalar_note",
        "receipt_ref",
        "amount",
        "unknown_field",
    ] {
        let dir = TempDir::new().unwrap();
        let mut s = Store::open(dir.path()).unwrap();
        let m = material(&mut s);
        let source = receipt(&mut s, &m, "10", "100");
        let draft = return_draft(&mut s, &source, "2");
        let done = confirm(&mut s, &draft);
        let id = return_id(&done).to_owned();
        drop(s);
        let db = Connection::open(dir.path().join("app.sqlite3")).unwrap();
        if mode == "scalar_quantity" {
            db.execute(
                "UPDATE purchase_returns SET quantity_micros=quantity_micros+1 WHERE id=?1",
                [&id],
            )
            .unwrap();
        } else if mode == "scalar_note" {
            db.execute(
                "UPDATE purchase_returns SET note='변조된 가상 메모' WHERE id=?1",
                [&id],
            )
            .unwrap();
        } else {
            let (seq, payload): (i64, String) = db
                .query_row(
                    "SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence DESC LIMIT 1",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            let mut payload: Value = serde_json::from_str(&payload).unwrap();
            let row = &mut payload["state"]["purchase_returns"][0];
            match mode {
                "receipt_ref" => {
                    row["effect"]["source"]["reference"]["version_id"] =
                        json!(Uuid::new_v4().to_string())
                }
                "amount" => row["effect"]["deducted_value"] = json!("19"),
                _ => row["effect"]["source"]["extra"] = json!(true),
            }
            let changed = serde_json::to_string(&payload).unwrap();
            db.execute(
                "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=?2",
                rusqlite::params![changed, seq],
            )
            .unwrap();
            fs::write(
                dir.path().join(format!("snapshots/{seq:020}.json")),
                changed,
            )
            .unwrap();
        }
        drop(db);
        let before = fs::read(dir.path().join("app.sqlite3")).unwrap();
        let bytes = journals(dir.path());
        assert!(Store::open(dir.path()).is_err(), "{mode}");
        assert_eq!(
            fs::read(dir.path().join("app.sqlite3")).unwrap(),
            before,
            "{mode}"
        );
        assert_eq!(journals(dir.path()), bytes, "{mode}");
    }
}

#[test]
fn source_amount_correction_revalues_active_return_without_rewriting_its_original_cas() {
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    let m = material(&mut s);
    let source = receipt(&mut s, &m, "10", "100");
    let draft = return_draft(&mut s, &source, "2");
    let done = confirm(&mut s, &draft);
    let rid = return_id(&done).to_owned();
    let original = value(s.purchase_return_detail(&rid).unwrap().unwrap())["item"].clone();
    let target = s
        .receipt_detail(&source)
        .unwrap()
        .unwrap()
        .item
        .confirmation_id;
    let seed = value(s.correction_seed(&target).unwrap().unwrap());
    let mut replacement = seed["replacement_template"].clone();
    replacement["data"]["amount"] = json!("200");
    let saved=value(s.business_create(decode(json!({"meta":meta(&s),"corrects_confirmation_id":target,"payload":{"kind":"correction","version":1,"data":{"expected_target_version_id":seed["current"]["reference"]["version_id"],"reason":"가상 구입금액 오입력","action":{"kind":"replace","replacement":replacement},"observation_reviews":[],"linked_receipt_changes":[]}}}))).unwrap());
    confirm(&mut s, &saved);
    let detail = value(s.purchase_return_detail(&rid).unwrap().unwrap());
    assert_eq!(detail["item"], original);
    assert_eq!(detail["effective"]["kind"], "active");
    assert_eq!(detail["effective"]["effect"]["deducted_value"], "40");
    assert_eq!(h(&s, &m)["settlement_basis"]["quantity"], "8");
    assert_eq!(h(&s, &m)["settlement_basis"]["value"], "160");
    let raw = s.business_detail(id(&draft)).unwrap().unwrap();
    assert_eq!(
        value(raw)["head"]["payload"]["data"]["expected_receipt_version_id"],
        json!(source)
    );
    drop(s);
    let s = Store::open(dir.path()).unwrap();
    assert_eq!(
        value(s.purchase_return_detail(&rid).unwrap().unwrap()),
        detail
    );
}
