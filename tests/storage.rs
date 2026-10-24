use rusqlite::Connection;
use serde_json::Value;
use std::{fs, path::Path};
use sunchang_hansang::store::{CreateMaterial, Store, StoreError};
use tempfile::TempDir;
use uuid::Uuid;

fn input(name: &str, unit: &str) -> CreateMaterial {
    CreateMaterial {
        request_id: Uuid::new_v4().to_string(),
        name: name.into(),
        unit: unit.into(),
    }
}

fn snapshots(root: &Path) -> Vec<std::path::PathBuf> {
    let mut files: Vec<_> = fs::read_dir(root.join("snapshots"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    files.sort();
    files
}

#[test]
fn saves_trimmed_material_and_reopens_with_the_same_id() {
    let directory = TempDir::new().unwrap();
    let mut store = Store::open(directory.path()).unwrap();
    let material = store.create(input("  가상 재료  ", "  장  ")).unwrap();
    assert_eq!(material.name, "가상 재료");
    assert_eq!(material.unit, "장");
    assert!(Uuid::parse_str(&material.id).is_ok());
    assert!(chrono::DateTime::parse_from_rfc3339(&material.created_at).is_ok());
    assert_eq!(store.get(&material.id).unwrap().unwrap().name, "가상 재료");
    assert!(store.get(&Uuid::new_v4().to_string()).unwrap().is_none());
    drop(store);
    let store = Store::open(directory.path()).unwrap();
    let saved = store.list().unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].id, material.id);
    assert_eq!(saved[0].unit, "장");
}

#[test]
fn rejects_invalid_input_without_creating_rows_or_snapshots() {
    let directory = TempDir::new().unwrap();
    let mut store = Store::open(directory.path()).unwrap();
    for request in [
        input(" ", "g"),
        input(&"한".repeat(101), "g"),
        input("재료", ""),
        input("재료", &"가".repeat(21)),
        input("재\n료", "g"),
        input("재료", "g\0"),
        CreateMaterial {
            request_id: "invalid".into(),
            ..input("재료", "g")
        },
    ] {
        assert!(matches!(
            store.create(request),
            Err(StoreError::Validation(_))
        ));
    }
    assert!(store.list().unwrap().is_empty());
    assert!(snapshots(directory.path()).is_empty());
    assert!(
        store
            .create(input(&"한".repeat(100), &"단".repeat(20)))
            .is_ok()
    );
}

#[test]
fn repeated_request_is_idempotent_across_restart_and_conflicting_reuse_is_rejected() {
    let directory = TempDir::new().unwrap();
    let request = input("가상 무", "g");
    let mut store = Store::open(directory.path()).unwrap();
    let first = store.create(request.clone()).unwrap();
    let mut padded = request.clone();
    padded.name = "  가상 무  ".into();
    padded.request_id = padded.request_id.to_uppercase();
    assert_eq!(store.create(padded).unwrap().id, first.id);
    drop(store);
    let mut store = Store::open(directory.path()).unwrap();
    assert_eq!(store.create(request.clone()).unwrap().id, first.id);
    assert!(matches!(
        store.create(CreateMaterial {
            unit: "mL".into(),
            ..request
        }),
        Err(StoreError::Conflict(_))
    ));
    assert_eq!(store.list().unwrap().len(), 1);
    assert_eq!(snapshots(directory.path()).len(), 1);
}

#[test]
fn each_json_keeps_the_full_state_in_sequence_and_prior_bytes_never_change() {
    let directory = TempDir::new().unwrap();
    let mut store = Store::open(directory.path()).unwrap();
    let first = store.create(input("가상 무", "g")).unwrap();
    let first_path = snapshots(directory.path())[0].clone();
    let first_bytes = fs::read(&first_path).unwrap();
    let second = store.create(input("가상 우유", "mL")).unwrap();
    let files = snapshots(directory.path());
    assert_eq!(files.len(), 2);
    assert_eq!(fs::read(first_path).unwrap(), first_bytes);
    let old: Value = serde_json::from_slice(&first_bytes).unwrap();
    let new: Value = serde_json::from_slice(&fs::read(&files[1]).unwrap()).unwrap();
    assert_eq!(old["format_version"], 2);
    assert_eq!(old["schema_version"], 11);
    assert_eq!(old["sequence"], "1");
    assert_eq!(old["state"]["materials"].as_array().unwrap().len(), 1);
    assert_eq!(old["state"]["materials"][0]["id"], first.id);
    assert_eq!(new["sequence"], "2");
    let materials = new["state"]["materials"].as_array().unwrap();
    assert_eq!(materials.len(), 2);
    assert!(
        materials
            .iter()
            .any(|row| row["id"] == second.id && row["unit"] == "mL")
    );
}

#[test]
fn json_failure_is_durable_and_blocks_later_changes_until_recovered() {
    let directory = TempDir::new().unwrap();
    let mut store = Store::open(directory.path()).unwrap();
    let path = directory.path().join("snapshots");
    fs::remove_dir(&path).unwrap();
    fs::write(&path, b"blocked by a file").unwrap();
    let request = input("가상 무", "g");
    assert!(matches!(
        store.create(request.clone()),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(store.list().unwrap().len(), 1);
    let id = store.list().unwrap()[0].id.clone();
    assert!(matches!(
        store.create(input("두 번째", "개")),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(store.list().unwrap().len(), 1);
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap();
    assert_eq!(store.create(request).unwrap().id, id);
    assert_eq!(snapshots(directory.path()).len(), 1);
}

#[test]
fn startup_recovers_pending_and_deleted_snapshots_without_duplicate_rows() {
    let directory = TempDir::new().unwrap();
    let mut store = Store::open(directory.path()).unwrap();
    let request = input("가상 무", "g");
    let snapshots_path = directory.path().join("snapshots");
    fs::remove_dir(&snapshots_path).unwrap();
    fs::write(&snapshots_path, b"block").unwrap();
    assert!(store.create(request.clone()).is_err());
    drop(store);
    fs::remove_file(&snapshots_path).unwrap();
    let mut store = Store::open(directory.path()).unwrap();
    let id = store.create(request.clone()).unwrap().id;
    let original = fs::read(&snapshots(directory.path())[0]).unwrap();
    drop(store);
    fs::remove_file(&snapshots(directory.path())[0]).unwrap();
    let mut store = Store::open(directory.path()).unwrap();
    assert_eq!(store.create(request).unwrap().id, id);
    assert_eq!(store.list().unwrap().len(), 1);
    assert_eq!(fs::read(&snapshots(directory.path())[0]).unwrap(), original);
}

#[test]
fn changed_immutable_snapshot_is_preserved_and_prevents_opening() {
    let directory = TempDir::new().unwrap();
    let mut store = Store::open(directory.path()).unwrap();
    store.create(input("가상 무", "g")).unwrap();
    let file = snapshots(directory.path())[0].clone();
    fs::write(&file, b"{\"changed\":true}").unwrap();
    assert!(matches!(
        store.create(input("가상 후속 재료", "개")),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(store.list().unwrap().len(), 1);
    drop(store);
    assert!(Store::open(directory.path()).is_err());
    assert_eq!(fs::read(file).unwrap(), b"{\"changed\":true}");
}

#[test]
fn outbox_insert_failure_rolls_back_material_and_idempotency_record() {
    let directory = TempDir::new().unwrap();
    drop(Store::open(directory.path()).unwrap());
    let connection = Connection::open(directory.path().join("app.sqlite3")).unwrap();
    connection.execute_batch(
        "CREATE TRIGGER reject_snapshot BEFORE INSERT ON snapshot_outbox BEGIN SELECT RAISE(FAIL, 'simulated outbox failure'); END;",
    ).unwrap();
    drop(connection);
    let mut store = Store::open(directory.path()).unwrap();
    assert!(matches!(
        store.create(input("가상 재료", "g")),
        Err(StoreError::Unavailable(_))
    ));
    assert!(store.list().unwrap().is_empty());
    assert!(snapshots(directory.path()).is_empty());
    drop(store);
    let connection = Connection::open(directory.path().join("app.sqlite3")).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM creation_requests", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}

#[test]
fn one_writer_lock_lasts_until_store_is_dropped() {
    let directory = TempDir::new().unwrap();
    let first = Store::open(directory.path()).unwrap();
    assert!(Store::open(directory.path()).is_err());
    drop(first);
    assert!(Store::open(directory.path()).is_ok());
}

fn synthetic_v1(root: &Path) {
    let connection = Connection::open(root.join("app.sqlite3")).unwrap();
    connection
        .execute_batch(include_str!("fixtures/schema-v1.sql"))
        .unwrap();
}

#[test]
fn synthetic_v1_is_backed_up_and_migrated_preserving_materials() {
    let directory = TempDir::new().unwrap();
    synthetic_v1(directory.path());
    let mut store = Store::open(directory.path()).unwrap();
    assert_eq!(store.list().unwrap()[0].name, "가상 이전 재료");
    assert_eq!(store.list().unwrap()[0].unit, "장");
    store.create(input("가상 신규 재료", "개")).unwrap();
    assert_eq!(store.list().unwrap().len(), 2);
    drop(store);
    let connection = Connection::open(directory.path().join("app.sqlite3")).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        11
    );
    let backups: Vec<_> = fs::read_dir(directory.path().join("backups"))
        .unwrap()
        .collect();
    assert_eq!(backups.len(), 1);
    let backup = Connection::open(backups[0].as_ref().unwrap().path()).unwrap();
    assert_eq!(
        backup
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        backup
            .query_row("SELECT name FROM materials", [], |row| row
                .get::<_, String>(0))
            .unwrap(),
        "가상 이전 재료"
    );
}

#[test]
fn failed_migration_rolls_back_original_database_bytes() {
    let directory = TempDir::new().unwrap();
    synthetic_v1(directory.path());
    let db_path = directory.path().join("app.sqlite3");
    let connection = Connection::open(&db_path).unwrap();
    // The migration can create its new table and update user_version before
    // schema validation discovers the malformed legacy table; all must roll back.
    connection
        .execute_batch("ALTER TABLE materials RENAME COLUMN unit TO legacy_unit;")
        .unwrap();
    drop(connection);
    let before = fs::read(&db_path).unwrap();
    assert!(Store::open(directory.path()).is_err());
    assert_eq!(fs::read(&db_path).unwrap(), before);
}

#[test]
fn future_schema_is_refused_without_mutating_database() {
    let directory = TempDir::new().unwrap();
    let db_path = directory.path().join("app.sqlite3");
    let connection = Connection::open(&db_path).unwrap();
    connection.execute_batch("CREATE TABLE future (value TEXT); INSERT INTO future VALUES ('keep'); PRAGMA user_version=999;").unwrap();
    drop(connection);
    let before = fs::read(&db_path).unwrap();
    assert!(Store::open(directory.path()).is_err());
    assert_eq!(fs::read(&db_path).unwrap(), before);
    assert!(!directory.path().join("backups").exists());
    assert!(!directory.path().join("snapshots").exists());
}

#[test]
fn normal_close_leaves_portable_database_without_wal_dependency() {
    let source = TempDir::new().unwrap();
    let target = TempDir::new().unwrap();
    let mut store = Store::open(source.path()).unwrap();
    let material = store.create(input("가상 재료", "개")).unwrap();
    drop(store);
    assert!(!source.path().join("app.sqlite3-wal").exists());
    fs::copy(
        source.path().join("app.sqlite3"),
        target.path().join("app.sqlite3"),
    )
    .unwrap();
    let store = Store::open(target.path()).unwrap();
    assert_eq!(store.list().unwrap()[0].id, material.id);
    assert_eq!(snapshots(target.path()).len(), 1);
}
