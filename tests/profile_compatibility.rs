use rusqlite::{Connection, types::Value as SqlValue};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};
use sunchang_hansang::store::Store;
use tempfile::TempDir;
use uuid::Uuid;

fn decode<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}
fn encoded(value: impl Serialize) -> String {
    serde_json::to_string(&value).unwrap()
}
fn install(root: &Path) -> Vec<(i64, String)> {
    let mut db = Connection::open(root.join("app.sqlite3")).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    let tx = db.transaction().unwrap();
    tx.execute_batch("PRAGMA defer_foreign_keys=ON").unwrap();
    tx.execute_batch(include_str!("fixtures/schema-v13.sql"))
        .unwrap();
    tx.commit().unwrap();
    fs::create_dir(root.join("snapshots")).unwrap();
    let journals = db
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<(i64, String)>>>()
        .unwrap();
    for (seq, payload) in &journals {
        fs::write(root.join(format!("snapshots/{seq:020}.json")), payload).unwrap();
    }
    journals
}
fn columns(db: &Connection, table: &str) -> Vec<String> {
    db.prepare(&format!("PRAGMA table_info({table})"))
        .unwrap()
        .query_map([], |r| r.get(1))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}
fn rows(db: &Connection, table: &str, cols: &[String]) -> Vec<Vec<SqlValue>> {
    db.prepare(&format!(
        "SELECT {} FROM {table} ORDER BY rowid",
        cols.join(",")
    ))
    .unwrap()
    .query_map([], |r| (0..cols.len()).map(|i| r.get(i)).collect())
    .unwrap()
    .collect::<rusqlite::Result<_>>()
    .unwrap()
}
fn check_journals(root: &Path, journals: &[(i64, String)]) {
    for (seq, original) in journals {
        assert_eq!(
            fs::read_to_string(root.join(format!("snapshots/{seq:020}.json"))).unwrap(),
            *original
        );
    }
}
fn replay(store: &mut Store, op: &str, payload: Value) -> String {
    match op {
        "material.create" => encoded(store.catalog_create(decode(payload)).unwrap()),
        "material.price" => encoded(
            store
                .catalog_add_price(payload[0].as_str().unwrap(), decode(payload[1].clone()))
                .unwrap(),
        ),
        "material.archive" => encoded(
            store
                .catalog_archive(payload[0].as_str().unwrap(), decode(payload[1].clone()))
                .unwrap(),
        ),
        "business.create" => encoded(store.business_create(decode(payload)).unwrap()),
        "business.confirm" => encoded(
            store
                .business_confirm(payload[0].as_str().unwrap(), decode(payload[1].clone()))
                .unwrap(),
        ),
        "recipe.create" => encoded(store.recipe_create(decode(payload)).unwrap()),
        other => panic!("uncovered source operation {other}"),
    }
}

#[test]
fn schema_thirteen_migration_preserves_original_columns_and_all_journal_bytes() {
    let root = TempDir::new().unwrap();
    let journals = install(root.path());
    assert_eq!(journals.len(), 46);
    assert_eq!(
        journals.last().unwrap().1,
        include_str!("fixtures/snapshot-v2-schema13.json")
    );
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let tables=db.prepare("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap()
        .query_map([],|r|r.get::<_,String>(0)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
    let before = tables
        .into_iter()
        .map(|table| {
            let cols = columns(&db, &table);
            let original = rows(&db, &table, &cols);
            (table, (cols, original))
        })
        .collect::<BTreeMap<_, _>>();
    drop(db);
    let store = Store::open(root.path()).unwrap();
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        15
    );
    for (table, (cols, original)) in before {
        assert_eq!(rows(&db, &table, &cols), original, "{table}");
    }
    for table in [
        "photo_assets",
        "material_profile_versions",
        "material_profile_heads",
        "material_profile_photos",
        "material_profile_suppliers",
    ] {
        assert_eq!(
            db.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r
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
    assert_eq!(store.catalog_state().unwrap().revision, "46");
    check_journals(root.path(), &journals);
    drop(db);
    drop(store);
    Store::open(root.path()).unwrap();
    check_journals(root.path(), &journals);
}

#[test]
fn schema_thirteen_requests_replay_exactly_after_new_schema_write_and_restart() {
    let root = TempDir::new().unwrap();
    let journals = install(root.path());
    let source: Value = serde_json::from_str(&journals.last().unwrap().1).unwrap();
    let requests = source["state"]["mutation_requests"].as_array().unwrap();
    assert_eq!(requests.len(), 46);
    let mut store = Store::open(root.path()).unwrap();
    let state = store.catalog_state().unwrap();
    store.catalog_create(decode(json!({"request_id":Uuid::new_v4().to_string(),"epoch":state.epoch,"expected_revision":state.revision,"name":"가상 사진 확장 후 재료","unit":"g","kind":"raw","description":"이전 기록 보존 확인"}))).unwrap();
    let after = encoded(store.catalog_state().unwrap());
    for request in requests {
        assert_eq!(
            replay(
                &mut store,
                request["operation"].as_str().unwrap(),
                serde_json::from_str(request["payload"].as_str().unwrap()).unwrap()
            ),
            request["response"].as_str().unwrap()
        );
    }
    assert_eq!(encoded(store.catalog_state().unwrap()), after);
    drop(store);
    // Recovery republishes original bytes without upgrading their schema header.
    fs::remove_file(root.path().join("snapshots/00000000000000000001.json")).unwrap();
    let mut store = Store::open(root.path()).unwrap();
    for request in requests {
        assert_eq!(
            replay(
                &mut store,
                request["operation"].as_str().unwrap(),
                serde_json::from_str(request["payload"].as_str().unwrap()).unwrap()
            ),
            request["response"].as_str().unwrap()
        );
    }
    assert_eq!(encoded(store.catalog_state().unwrap()), after);
    check_journals(root.path(), &journals);
    let latest: Value = serde_json::from_slice(
        &fs::read(root.path().join("snapshots/00000000000000000047.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(latest["schema_version"], 15);
    assert_eq!(latest["state"]["material_profile_versions"], json!([]));
}

#[test]
fn schema_thirteen_database_with_future_outbox_is_rejected_before_any_migration() {
    let root = TempDir::new().unwrap();
    let journals = install(root.path());
    let mut future: Value = serde_json::from_str(&journals.last().unwrap().1).unwrap();
    future["schema_version"] = json!(14);
    for field in [
        "photo_assets",
        "material_profile_versions",
        "material_profile_heads",
        "material_profile_photos",
        "material_profile_suppliers",
    ] {
        future["state"][field] = json!([]);
    }
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    db.execute(
        "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=46",
        [future.to_string()],
    )
    .unwrap();
    drop(db);
    let before = fs::read(root.path().join("app.sqlite3")).unwrap();
    assert!(Store::open(root.path()).is_err());
    assert_eq!(fs::read(root.path().join("app.sqlite3")).unwrap(), before);
    check_journals(root.path(), &journals);
}
