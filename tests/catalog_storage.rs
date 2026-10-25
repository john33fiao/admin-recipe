use rusqlite::{Connection, params};
use serde_json::Value;
use std::{fs, path::Path};
use sunchang_hansang::{
    catalog::{
        ArchiveCatalogMaterial, ArchiveFilter, CatalogFilter, CreateCatalogMaterial,
        CreateCatalogPrice, MaterialSummary, MutationMeta, ReviseCatalogMaterial,
    },
    store::{CreateMaterial, Store, StoreError},
};
use tempfile::TempDir;
use uuid::Uuid;

const LEGACY_ID: &str = "00000000-0000-4000-8000-000000000001";
const LEGACY_REQUEST: &str = "00000000-0000-4000-8000-000000000101";

fn meta(store: &Store) -> MutationMeta {
    let state = store.catalog_state().unwrap();
    MutationMeta {
        request_id: Uuid::new_v4().to_string(),
        epoch: state.epoch,
        expected_revision: state.revision,
    }
}

fn create_input(store: &Store, name: &str) -> CreateCatalogMaterial {
    CreateCatalogMaterial {
        meta: meta(store),
        name: name.into(),
        unit: "g".into(),
        kind: "raw".into(),
        description: "가상 설명".into(),
    }
}

fn add(store: &mut Store, name: &str) -> MaterialSummary {
    let input = create_input(store, name);
    store.catalog_create(input).unwrap().value
}

fn price(store: &Store, item: &MaterialSummary, amount: &str) -> CreateCatalogPrice {
    CreateCatalogPrice {
        meta: meta(store),
        expected_version_id: item.version_id.clone(),
        quantity: "1".into(),
        unit: "kg".into(),
        amount: amount.into(),
    }
}

fn revise(store: &Store, item: &MaterialSummary) -> ReviseCatalogMaterial {
    ReviseCatalogMaterial {
        meta: meta(store),
        expected_version_id: item.version_id.clone(),
        name: "가상 바뀐 이름".into(),
        unit: "kg".into(),
        kind: "prepared".into(),
        description: "첫 줄\n둘째 줄".into(),
    }
}

fn legacy(root: &Path) {
    let connection = Connection::open(root.join("app.sqlite3")).unwrap();
    connection
        .execute_batch(include_str!("fixtures/schema-v2.sql"))
        .unwrap();
    connection
        .execute(
            "INSERT INTO snapshot_outbox VALUES (1, ?1, 1)",
            [include_str!("fixtures/snapshot-v1.json")],
        )
        .unwrap();
    fs::create_dir(root.join("snapshots")).unwrap();
    fs::write(
        root.join("snapshots/00000000000000000001.json"),
        include_bytes!("fixtures/snapshot-v1.json"),
    )
    .unwrap();
}

#[test]
fn v2_migration_preserves_ids_requests_v1_bytes_and_unknown_origins() {
    let root = TempDir::new().unwrap();
    legacy(root.path());
    let mut store = Store::open(root.path()).unwrap();
    let detail = store.catalog_detail(LEGACY_ID).unwrap().unwrap();
    assert_eq!(detail.item.kind, "unclassified");
    assert_eq!(detail.item.price.source, "missing");
    assert_eq!(detail.versions[0].seq.as_deref(), Some("1"));
    let old = store
        .catalog_detail("00000000-0000-4000-8000-000000000002")
        .unwrap()
        .unwrap();
    assert_eq!(old.versions[0].seq, None);
    assert_eq!(old.versions[0].source, "legacy_migration");
    let revised = store
        .catalog_revise(LEGACY_ID, revise(&store, &detail.item))
        .unwrap();
    assert_ne!(revised.value.version_id, detail.item.version_id);
    let replay = store
        .create(CreateMaterial {
            request_id: LEGACY_REQUEST.into(),
            name: "  가상 이전 같은 이름  ".into(),
            unit: "g".into(),
        })
        .unwrap();
    assert_eq!(replay.id, LEGACY_ID);
    assert_eq!(replay.name, "가상 이전 같은 이름");
    assert_eq!(
        fs::read(root.path().join("snapshots/00000000000000000001.json")).unwrap(),
        include_bytes!("fixtures/snapshot-v1.json")
    );
    drop(store);
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    assert_eq!(
        db.pragma_query_value::<i64, _>(None, "user_version", |r| r.get(0))
            .unwrap(),
        15
    );
    assert_eq!(
        db.query_row("SELECT name FROM materials WHERE id=?1", [LEGACY_ID], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "가상 이전 같은 이름"
    );
    assert_eq!(
        fs::read_dir(root.path().join("backups")).unwrap().count(),
        1
    );
}

#[test]
fn revisions_prices_search_and_archive_survive_restart_without_holdings() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let a = add(&mut store, "가상 %_ 재료");
    let b = add(&mut store, "가상 %_ 재료");
    assert_ne!(a.id, b.id);
    let zero = store
        .catalog_add_price(&a.id, price(&store, &a, "0"))
        .unwrap();
    assert_eq!(zero.value.quantity, "1000");
    assert_eq!(zero.value.amount, "0");
    assert_eq!(
        store
            .catalog_detail(&a.id)
            .unwrap()
            .unwrap()
            .item
            .price
            .source,
        "preregistered"
    );
    store
        .catalog_add_price(&a.id, price(&store, &a, "12000"))
        .unwrap();
    let changed = store
        .catalog_revise(&a.id, revise(&store, &a))
        .unwrap()
        .value;
    store
        .catalog_archive(
            &a.id,
            ArchiveCatalogMaterial {
                meta: meta(&store),
                expected_version_id: changed.version_id,
            },
        )
        .unwrap();
    assert_eq!(
        store
            .catalog_list(CatalogFilter {
                query: "  %_  ".into(),
                archive: ArchiveFilter::Active
            })
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        store
            .catalog_list(CatalogFilter {
                query: "바뀐".into(),
                archive: ArchiveFilter::Archived
            })
            .unwrap()
            .len(),
        1
    );
    assert!(
        store
            .catalog_list(CatalogFilter {
                query: "없음".into(),
                archive: ArchiveFilter::All
            })
            .unwrap()
            .is_empty()
    );
    drop(store);
    let store = Store::open(root.path()).unwrap();
    let detail = store.catalog_detail(&a.id).unwrap().unwrap();
    assert!(detail.item.archived);
    assert_eq!(detail.versions.len(), 3);
    assert_eq!(detail.versions[0].unit, "g");
    assert_eq!(detail.item.unit, "kg");
    assert_eq!(detail.item.canonical_unit, "g");
    assert_eq!(detail.prices.len(), 2);
    assert_eq!(detail.prices[0].amount, "0");
    assert_eq!(detail.item.price.amount.as_deref(), Some("12000"));
    let serialized = serde_json::to_value(detail).unwrap();
    assert!(serialized.get("holding_quantity").is_none());
}

#[test]
fn stale_state_request_conflicts_and_invalid_prices_do_not_change_data() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let input = create_input(&store, "가상 동시 수정");
    let saved = store.catalog_create(input.clone()).unwrap();
    assert_eq!(
        store.catalog_create(input.clone()).unwrap().value.id,
        saved.value.id
    );
    let mut duplicate = input.clone();
    duplicate.description = "다른 내용".into();
    assert!(matches!(
        store.catalog_create(duplicate),
        Err(StoreError::Conflict(_))
    ));
    let mut stale = revise(&store, &saved.value);
    stale.meta.expected_revision = "0".into();
    assert!(matches!(
        store.catalog_revise(&saved.value.id, stale),
        Err(StoreError::StaleState(_))
    ));
    let mut unit = revise(&store, &saved.value);
    unit.unit = "mL".into();
    assert!(matches!(
        store.catalog_revise(&saved.value.id, unit),
        Err(StoreError::Validation(_))
    ));
    for (quantity, amount, unit) in [
        ("0", "100", "g"),
        ("1", "-1", "g"),
        ("1", "0.0000001", "g"),
        ("1", "1", "mL"),
    ] {
        let mut p = price(&store, &saved.value, amount);
        p.quantity = quantity.into();
        p.unit = unit.into();
        assert!(matches!(
            store.catalog_add_price(&saved.value.id, p),
            Err(StoreError::Validation(_))
        ));
    }
    assert_eq!(
        store
            .catalog_detail(&saved.value.id)
            .unwrap()
            .unwrap()
            .versions
            .len(),
        1
    );
    assert!(
        store
            .catalog_detail(&saved.value.id)
            .unwrap()
            .unwrap()
            .prices
            .is_empty()
    );
    assert_eq!(store.catalog_state().unwrap(), saved.state);
}

#[test]
fn legacy_keys_cannot_be_reused_for_new_commands_or_extra_fields() {
    let root = TempDir::new().unwrap();
    legacy(root.path());
    let mut store = Store::open(root.path()).unwrap();
    let item = store.catalog_detail(LEGACY_ID).unwrap().unwrap().item;
    let mut new = create_input(&store, "가상 이전 같은 이름");
    new.meta.request_id = LEGACY_REQUEST.into();
    assert!(matches!(
        store.catalog_create(new),
        Err(StoreError::Conflict(_))
    ));
    let mut p = price(&store, &item, "1000");
    p.meta.request_id = LEGACY_REQUEST.into();
    assert!(matches!(
        store.catalog_add_price(LEGACY_ID, p),
        Err(StoreError::Conflict(_))
    ));
    let fresh = create_input(&store, "가상 새 명령");
    let key = fresh.meta.request_id.clone();
    store.catalog_create(fresh).unwrap();
    assert!(matches!(
        store.create(CreateMaterial {
            request_id: key,
            name: "가상 새 명령".into(),
            unit: "g".into()
        }),
        Err(StoreError::Conflict(_))
    ));
}

#[test]
fn publication_failure_blocks_success_and_new_writes_until_same_request_recovers() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = add(&mut store, "가상 기록 장애");
    let snapshots = root.path().join("snapshots");
    let old = fs::read(snapshots.join("00000000000000000001.json")).unwrap();
    fs::remove_file(snapshots.join("00000000000000000001.json")).unwrap();
    fs::remove_dir(&snapshots).unwrap();
    fs::write(&snapshots, b"blocked").unwrap();
    let request = price(&store, &item, "1000");
    // First restore the missing older record, then fail only the new publication.
    fs::remove_file(&snapshots).unwrap();
    fs::create_dir(&snapshots).unwrap();
    fs::write(snapshots.join("00000000000000000001.json"), &old).unwrap();
    fs::create_dir(snapshots.join("00000000000000000002.json")).unwrap();
    assert!(matches!(
        store.catalog_add_price(&item.id, request.clone()),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(
        store
            .catalog_detail(&item.id)
            .unwrap()
            .unwrap()
            .prices
            .len(),
        1
    );
    assert!(matches!(
        store.catalog_add_price(&item.id, request.clone()),
        Err(StoreError::Unavailable(_))
    ));
    let other = price(&store, &item, "2000");
    assert!(matches!(
        store.catalog_add_price(&item.id, other),
        Err(StoreError::Unavailable(_))
    ));
    fs::remove_dir(snapshots.join("00000000000000000002.json")).unwrap();
    let saved = store.catalog_add_price(&item.id, request.clone()).unwrap();
    let raw = fs::read(snapshots.join("00000000000000000002.json")).unwrap();
    fs::remove_file(snapshots.join("00000000000000000002.json")).unwrap();
    assert_eq!(
        store.catalog_add_price(&item.id, request).unwrap().value.id,
        saved.value.id
    );
    assert_eq!(
        fs::read(snapshots.join("00000000000000000002.json")).unwrap(),
        raw
    );
    let snapshot: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(snapshot["format_version"], 2);
    assert_eq!(snapshot["sequence"], "2");
    assert_eq!(
        snapshot["state"]["material_prices"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn outbox_failure_rolls_back_versions_prices_requests_and_revision() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = add(&mut store, "가상 원자성");
    let before = store.catalog_state().unwrap();
    drop(store);
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    db.execute_batch("CREATE TRIGGER reject_new_snapshot BEFORE INSERT ON snapshot_outbox BEGIN SELECT RAISE(FAIL,'test outbox failure'); END;").unwrap();
    drop(db);
    let mut store = Store::open(root.path()).unwrap();
    let p = price(&store, &item, "1000");
    let key = p.meta.request_id.clone();
    assert!(matches!(
        store.catalog_add_price(&item.id, p),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(store.catalog_state().unwrap(), before);
    assert!(
        store
            .catalog_detail(&item.id)
            .unwrap()
            .unwrap()
            .prices
            .is_empty()
    );
    drop(store);
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT COUNT(*) FROM mutation_requests WHERE request_id=?1",
            params![key],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
}

#[test]
fn original_response_replays_after_later_changes_and_keys_include_target_id() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let original = create_input(&store, "가상 응답 원본");
    let result = store.catalog_create(original.clone()).unwrap();
    let original_json = serde_json::to_value(&result).unwrap();
    let other = add(&mut store, "가상 다른 대상");
    let price_request = price(&store, &result.value, "1000");
    store
        .catalog_add_price(&result.value.id, price_request.clone())
        .unwrap();
    assert!(matches!(
        store.catalog_add_price(&other.id, price_request),
        Err(StoreError::Conflict(_))
    ));
    store
        .catalog_revise(&result.value.id, revise(&store, &result.value))
        .unwrap();
    assert_eq!(
        serde_json::to_value(store.catalog_create(original.clone()).unwrap()).unwrap(),
        original_json
    );
    drop(store);
    let mut store = Store::open(root.path()).unwrap();
    assert_eq!(
        serde_json::to_value(store.catalog_create(original).unwrap()).unwrap(),
        original_json
    );
}

#[test]
fn failed_v2_migration_and_future_journal_preserve_original_bytes() {
    for future_journal in [false, true] {
        let root = TempDir::new().unwrap();
        legacy(root.path());
        let path = root.path().join("app.sqlite3");
        let db = Connection::open(&path).unwrap();
        if future_journal {
            let mut value: Value =
                serde_json::from_str(include_str!("fixtures/snapshot-v1.json")).unwrap();
            value["format_version"] = 999.into();
            db.execute("UPDATE snapshot_outbox SET payload=?1", [value.to_string()])
                .unwrap();
        } else {
            db.execute_batch("ALTER TABLE materials RENAME COLUMN unit TO unexpected_unit;")
                .unwrap();
        }
        drop(db);
        let before = fs::read(&path).unwrap();
        assert!(Store::open(root.path()).is_err());
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(
            fs::read(root.path().join("snapshots/00000000000000000001.json")).unwrap(),
            include_bytes!("fixtures/snapshot-v1.json")
        );
    }
}
