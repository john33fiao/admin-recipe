use rusqlite::{Connection, params};
use std::{fs, path::Path, process::Command};
use sunchang_hansang::store::{CreateMaterial, Store};
use tempfile::TempDir;
use uuid::Uuid;

const CHILD_ROOT: &str = "HANSANG_HOT_JOURNAL_TEST_ROOT";
const CHILD_VERSION: &str = "HANSANG_HOT_JOURNAL_TEST_VERSION";

#[test]
fn crash_writer_child() {
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        let connection = Connection::open(std::path::Path::new(&root).join("app.sqlite3")).unwrap();
        connection
            .execute_batch(
                "PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA cache_size=2; \
             BEGIN IMMEDIATE; UPDATE materials SET name='uncommitted change';",
            )
            .unwrap();
        if let Ok(version) = std::env::var(CHILD_VERSION) {
            connection
                .pragma_update(None, "user_version", version.parse::<i64>().unwrap())
                .unwrap();
        }
        // Force dirty database pages to spill, producing a hot rollback journal
        // that must be replayed before even a read-only query can proceed.
        for index in 0..4_000 {
            connection.execute(
                "INSERT INTO materials (id, name, unit, created_at) VALUES (?1, ?2, 'g', '2026-10-24T00:00:00Z')",
                params![Uuid::new_v4().to_string(), format!("가상 미확정 {index:04} {}", "재료".repeat(30))],
            ).unwrap();
        }
        // Deliberately bypass SQLite/Rust destructors, simulating process loss.
        std::process::exit(0);
    }
}

fn saved_database() -> (TempDir, String) {
    let directory = TempDir::new().unwrap();
    let mut store = Store::open(directory.path()).unwrap();
    let original = store
        .create(CreateMaterial {
            request_id: Uuid::new_v4().to_string(),
            name: "가상 확정 재료".into(),
            unit: "장".into(),
        })
        .unwrap();
    drop(store);
    (directory, original.id)
}

fn leave_hot_journal(root: &Path, uncommitted_version: Option<i64>) {
    let mut child = Command::new(std::env::current_exe().unwrap());
    child
        .args(["--exact", "crash_writer_child", "--nocapture"])
        .env(CHILD_ROOT, root);
    if let Some(version) = uncommitted_version {
        child.env(CHILD_VERSION, version.to_string());
    }
    let status = child.status().unwrap();
    assert!(status.success());
    let journal = fs::read(root.join("app.sqlite3-journal")).unwrap();
    assert!(
        journal.len() > 512,
        "the subprocess must leave a real journal"
    );
    assert_eq!(
        &journal[..8],
        &[0xd9, 0xd5, 0x05, 0xf9, 0x20, 0xa1, 0x63, 0xd7],
        "the rollback journal must be hot"
    );
}

fn assert_original_recovers(root: &Path, original_id: &str) {
    let reopened =
        Store::open(root).expect("an interrupted transaction must recover on app startup");
    let materials = reopened.list().unwrap();
    assert_eq!(materials.len(), 1);
    assert_eq!(materials[0].id, original_id);
    assert_eq!(materials[0].name, "가상 확정 재료");
    assert_eq!(materials[0].unit, "장");
}

#[test]
fn hot_rollback_journal_recovers_after_process_exit() {
    let (directory, original_id) = saved_database();
    leave_hot_journal(directory.path(), None);
    assert_original_recovers(directory.path(), &original_id);
}

#[test]
fn uncommitted_future_version_is_rolled_back_to_supported_version() {
    let (directory, original_id) = saved_database();
    leave_hot_journal(directory.path(), Some(999));
    assert_original_recovers(directory.path(), &original_id);
}

#[test]
fn committed_future_version_with_hot_journal_is_refused_without_touching_originals() {
    let (directory, _) = saved_database();
    let database = directory.path().join("app.sqlite3");
    let journal = directory.path().join("app.sqlite3-journal");
    let connection = Connection::open(&database).unwrap();
    connection.pragma_update(None, "user_version", 999).unwrap();
    drop(connection);
    // The current file header may claim a supported version; the committed
    // version recovered from its journal must determine compatibility.
    leave_hot_journal(directory.path(), Some(2));
    let original_db = fs::read(&database).unwrap();
    let original_journal = fs::read(&journal).unwrap();
    let error = Store::open(directory.path())
        .err()
        .expect("future schema must be refused");
    assert!(
        error.to_string().contains("999"),
        "expected recovered schema version refusal, got {error:#}"
    );
    assert_eq!(fs::read(database).unwrap(), original_db);
    assert_eq!(fs::read(journal).unwrap(), original_journal);
}
