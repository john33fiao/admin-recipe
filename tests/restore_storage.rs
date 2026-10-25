use rusqlite::{Connection, types::Value as SqlValue};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path};
use sunchang_hansang::{
    restore::{
        ConfirmRestore, PreviewRestore, RestoreRequestStatus, RestoreTarget, RestoreTargetQuery,
        RestoreTargetSource,
    },
    restore_files::restore_upload,
    store::{CreateMaterial, Store, StoreError},
};
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
fn create_input(s: &Store, name: &str) -> Value {
    let mut v = meta(s);
    v.as_object_mut().unwrap().extend(
        json!({"name":name,"unit":"g","kind":"raw","description":""})
            .as_object()
            .unwrap()
            .clone(),
    );
    v
}
fn create(s: &mut Store, name: &str) -> Value {
    let input = create_input(s, name);
    value(s.catalog_create(decode(input)).unwrap())
}
fn latest(s: &Store) -> RestoreTarget {
    s.restore_targets(RestoreTargetQuery::default())
        .unwrap()
        .rows
        .remove(0)
}
fn request(s: &Store, t: &RestoreTarget) -> ConfirmRestore {
    decode(json!({"meta":meta(s),"target_id":t.target_id,"expected_sha256":t.sha256}))
}
fn preview(s: &mut Store, t: &RestoreTarget) -> sunchang_hansang::restore::RestorePreview {
    s.restore_preview(PreviewRestore {
        target_id: t.target_id.clone(),
        expected_sha256: t.sha256.clone(),
    })
    .unwrap()
}
fn db(root: &Path) -> Connection {
    Connection::open(root.join("app.sqlite3")).unwrap()
}
fn count(root: &Path, table: &str) -> i64 {
    db(root)
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}
fn journals(root: &Path) -> Vec<(i64, String)> {
    db(root)
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap()
}
fn original_bytes(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(root.join("snapshots"))
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                fs::read(e.path()).unwrap(),
            )
        })
        .collect()
}
fn snapshot(root: &Path) -> Value {
    serde_json::from_str(&journals(root).last().unwrap().1).unwrap()
}
fn normal_rows(root: &Path) -> BTreeMap<String, Vec<Vec<SqlValue>>> {
    let c = db(root);
    let names=c.prepare("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap().query_map([],|r|r.get::<_,String>(0)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
    names
        .into_iter()
        .map(|name| {
            let mut stmt = c
                .prepare(&format!("SELECT * FROM {name} ORDER BY rowid"))
                .unwrap();
            let width = stmt.column_count();
            let rows = stmt
                .query_map([], |r| (0..width).map(|i| r.get(i)).collect())
                .unwrap()
                .collect::<rusqlite::Result<_>>()
                .unwrap();
            (name, rows)
        })
        .collect()
}
fn copy_photos(root: &Path) {
    let snapshot: Value =
        serde_json::from_str(include_str!("fixtures/snapshot-v2-schema14.json")).unwrap();
    for row in snapshot["state"]["photo_assets"].as_array().unwrap() {
        let d = &row["descriptor"];
        let destination = root.join(d["asset_key"].as_str().unwrap());
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
                "tests/fixtures/schema-v14-photo-{}.png",
                d["sha256"].as_str().unwrap()
            )),
            destination,
        )
        .unwrap();
    }
}
fn install14(root: &Path) {
    let mut c = db(root);
    c.pragma_update(None, "foreign_keys", true).unwrap();
    let tx = c.transaction().unwrap();
    tx.pragma_update(None, "defer_foreign_keys", true).unwrap();
    tx.execute_batch(include_str!("fixtures/schema-v14.sql"))
        .unwrap();
    tx.commit().unwrap();
    fs::create_dir(root.join("snapshots")).unwrap();
    for (seq, body) in journals(root) {
        fs::write(root.join(format!("snapshots/{seq:020}.json")), body).unwrap();
    }
    copy_photos(root);
}

#[test]
fn schema_fourteen_migration_retargets_all_sequence_foreign_keys_and_preserves_every_original_row()
{
    let dir = TempDir::new().unwrap();
    install14(dir.path());
    let before = normal_rows(dir.path());
    let files = original_bytes(dir.path());
    let store = Store::open(dir.path()).unwrap();
    assert_eq!(
        db(dir.path())
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        15
    );
    let after = normal_rows(dir.path());
    for (name, rows) in before {
        assert_eq!(after[&name], rows, "{name}");
    }
    assert_eq!(files, original_bytes(dir.path()));
    assert_eq!(count(dir.path(), "state_sequences"), 50);
    assert_eq!(count(dir.path(), "preserved_mutation_requests"), 50);
    assert_eq!(count(dir.path(), "preserved_photo_assets"), 2);
    let c = db(dir.path());
    assert_eq!(
        c.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
    let fk:String=c.query_row("SELECT \"table\" FROM pragma_foreign_key_list('material_versions') WHERE \"from\"='sequence'",[],|r|r.get(0)).unwrap();
    assert_eq!(fk, "state_sequences");
    assert_eq!(store.catalog_state().unwrap().revision, "50");
    drop(store);
    Store::open(dir.path()).unwrap();
}

#[test]
fn restores_entire_selected_state_and_repeatedly_restores_the_exact_before_capture() {
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    let a = create(&mut s, "가상 A");
    let target = latest(&s);
    let old_files = original_bytes(dir.path());
    let b = create(&mut s, "가상 B");
    let b_id = b["id"].as_str().unwrap();
    let before = s.catalog_state().unwrap();
    let p = preview(&mut s, &target);
    assert!(p.can_restore);
    assert_eq!(p.before.areas[0].records, 2);
    assert_eq!(p.after.areas[0].records, 1);
    assert_eq!(s.catalog_state().unwrap(), before);
    let input = request(&s, &target);
    let result = s.restore(input.clone()).unwrap();
    assert_ne!(result.state.epoch, before.epoch);
    assert_eq!(result.state.revision, "3");
    assert!(s.get(b_id).unwrap().is_none());
    assert!(s.get(a["id"].as_str().unwrap()).unwrap().is_some());
    assert_eq!(value(s.restore(input).unwrap()), value(&result));
    assert_eq!(count(dir.path(), "restore_records"), 1);
    for (name, bytes) in old_files {
        assert_eq!(
            fs::read(dir.path().join("snapshots").join(name)).unwrap(),
            bytes
        );
    }
    let previous = result.previous_target.clone();
    let restored = s.restore(request(&s, &previous)).unwrap();
    assert!(s.get(b_id).unwrap().is_some());
    assert_eq!(restored.state.revision, "4");
    s.restore(request(&s, &target)).unwrap();
    assert!(s.get(b_id).unwrap().is_none());
    drop(s);
    let s = Store::open(dir.path()).unwrap();
    assert_eq!(s.list().unwrap().len(), 1);
}

#[test]
fn processed_and_unprocessed_old_generations_are_distinct_without_reapplying_legacy_requests() {
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    let legacy = CreateMaterial {
        request_id: Uuid::new_v4().to_string(),
        name: "가상 옛 등록".into(),
        unit: "g".into(),
    };
    s.create(legacy.clone()).unwrap();
    let target = latest(&s);
    let written = create_input(&s, "가상 처리된 이후 입력");
    let saved = s.catalog_create(decode(written.clone())).unwrap();
    let queued = create_input(&s, "가상 미처리 입력");
    let restore = request(&s, &target);
    let result = s.restore(restore.clone()).unwrap();
    let unchanged = s.catalog_state().unwrap();
    for (input, status) in [
        (written.clone(), RestoreRequestStatus::AlreadyProcessed),
        (queued, RestoreRequestStatus::NotProcessed),
    ] {
        assert!(
            matches!(s.catalog_create(decode(input)),Err(StoreError::StaleGeneration{request_status,..}) if request_status==status)
        );
    }
    let mut collision = written;
    collision["name"] = json!("가상 다른 본문");
    assert!(matches!(
        s.catalog_create(decode(collision)),
        Err(StoreError::Conflict(_))
    ));
    assert!(matches!(
        s.create(legacy),
        Err(StoreError::StaleGeneration {
            request_status: RestoreRequestStatus::AlreadyProcessed,
            ..
        })
    ));
    assert!(matches!(
        s.create(CreateMaterial {
            request_id: Uuid::new_v4().to_string(),
            name: "가상 새 옛입력".into(),
            unit: "g".into()
        }),
        Err(StoreError::StaleGeneration {
            request_status: RestoreRequestStatus::NotProcessed,
            ..
        })
    ));
    assert_eq!(s.catalog_state().unwrap(), unchanged);
    assert!(s.get(&saved.value.id).unwrap().is_none());
    create(&mut s, "가상 현재 정상 입력");
    assert_eq!(value(s.restore(restore.clone()).unwrap()), value(&result));
    s.restore(request(&s, &target)).unwrap();
    assert!(matches!(
        s.restore(restore),
        Err(StoreError::StaleGeneration {
            request_status: RestoreRequestStatus::AlreadyProcessed,
            ..
        })
    ));
}

#[test]
fn an_empty_revision_zero_before_capture_is_a_restorable_whole_state() {
    let source = TempDir::new().unwrap();
    let mut other = Store::open(source.path()).unwrap();
    create(&mut other, "가상 외부 자료");
    let bytes = journals(source.path())[0].1.clone();
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    let target = s
        .restore_import(restore_upload(bytes.into_bytes()).unwrap())
        .unwrap();
    assert_eq!(s.catalog_state().unwrap().revision, "0");
    let result = s.restore(request(&s, &target)).unwrap();
    assert_eq!(result.previous_target.sequence, "0");
    assert_eq!(
        result.previous_target.source_kind,
        sunchang_hansang::restore::RestoreSourceKind::BeforeRestore
    );
    s.restore(request(&s, &result.previous_target)).unwrap();
    assert!(s.list().unwrap().is_empty());
    assert_eq!(count(dir.path(), "snapshot_outbox"), 2);
    drop(s);
    Store::open(dir.path()).unwrap();
}

#[test]
fn imported_full_schema_fourteen_has_no_fabricated_journals_and_restores_every_typed_relation() {
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    copy_photos(dir.path());
    let target = s
        .restore_import(
            restore_upload(include_bytes!("fixtures/snapshot-v2-schema14.json").to_vec()).unwrap(),
        )
        .unwrap();
    assert!(preview(&mut s, &target).can_restore);
    let result = s.restore(request(&s, &target)).unwrap();
    assert_eq!(result.state.revision, "51");
    assert_eq!(count(dir.path(), "snapshot_outbox"), 1);
    assert_eq!(journals(dir.path())[0].0, 51);
    let original: Value =
        serde_json::from_str(include_str!("fixtures/snapshot-v2-schema14.json")).unwrap();
    let mut actual = snapshot(dir.path())["state"].clone();
    let mut expected = original["state"].clone();
    actual["catalog_state"] = Value::Null;
    expected["catalog_state"] = Value::Null;
    actual["mutation_requests"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["operation"] != "state.restore");
    for obj in [&mut actual, &mut expected] {
        for (_, v) in obj.as_object_mut().unwrap() {
            if let Some(rows) = v.as_array_mut() {
                rows.sort_by_key(Value::to_string);
            }
        }
    }
    assert_eq!(actual, expected);
    assert_eq!(count(dir.path(), "preserved_photo_assets"), 2);
    drop(s);
    Store::open(dir.path()).unwrap();
}

#[test]
fn missing_photo_blocks_confirmation_without_mutating_state_and_preserved_photos_remain_registered()
{
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    let blank = s.catalog_state().unwrap();
    let target = s
        .restore_import(
            restore_upload(include_bytes!("fixtures/snapshot-v2-schema14.json").to_vec()).unwrap(),
        )
        .unwrap();
    let p = preview(&mut s, &target);
    assert!(!p.can_restore);
    assert_eq!(p.photos.missing, 2);
    assert_eq!(s.catalog_state().unwrap(), blank);
    assert!(s.restore(request(&s, &target)).is_err());
    copy_photos(dir.path());
    let result = s.restore(request(&s, &target)).unwrap();
    let before = result.previous_target;
    let asset = snapshot(dir.path())["state"]["photo_assets"][0]["descriptor"]["sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    let bytes = s.photo_asset_bytes(&asset).unwrap().unwrap().bytes;
    let asset_path = snapshot(dir.path())["state"]["photo_assets"][0]["descriptor"]["asset_key"]
        .as_str()
        .unwrap()
        .to_owned();
    fs::remove_file(dir.path().join(&asset_path)).unwrap();
    let blocked = preview(&mut s, &before);
    assert!(!blocked.can_restore);
    assert_eq!(
        blocked.photos.required, 2,
        "current preserved assets are checked even when selected empty capture has none"
    );
    assert_eq!(blocked.photos.missing, 1);
    fs::write(dir.path().join(asset_path), &bytes).unwrap();
    s.restore(request(&s, &before)).unwrap();
    assert_eq!(count(dir.path(), "photo_assets"), 0);
    assert_eq!(s.photo_asset_bytes(&asset).unwrap().unwrap().bytes, bytes);
    assert_eq!(count(dir.path(), "preserved_photo_assets"), 2);
    drop(s);
    Store::open(dir.path()).unwrap();
}

#[test]
fn v1_import_preserves_ids_and_free_units_without_fabricating_requests_or_stock() {
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    let target = s
        .restore_import(
            restore_upload(include_bytes!("fixtures/snapshot-v1.json").to_vec()).unwrap(),
        )
        .unwrap();
    let restored = s.restore(request(&s, &target)).unwrap();
    let first = snapshot(dir.path())["state"].clone();
    assert_eq!(count(dir.path(), "creation_requests"), 0);
    assert_eq!(count(dir.path(), "holding_versions"), 0);
    assert_eq!(count(dir.path(), "material_prices"), 0);
    assert_eq!(
        s.get("00000000-0000-4000-8000-000000000002")
            .unwrap()
            .unwrap()
            .unit,
        "장"
    );
    s.restore(request(&s, &restored.previous_target)).unwrap();
    s.restore(request(&s, &target)).unwrap();
    assert_eq!(
        snapshot(dir.path())["state"]["material_versions"],
        first["material_versions"]
    );
    assert_eq!(
        snapshot(dir.path())["state"]["materials"],
        first["materials"]
    );
    drop(s);
    Store::open(dir.path()).unwrap();
}

#[test]
fn current_preserved_response_tampering_blocks_same_uuid_replay_and_reopen() {
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    let input = create_input(&s, "가상 원응답");
    s.catalog_create(decode(input.clone())).unwrap();
    let id = input["request_id"].as_str().unwrap();
    let c = db(dir.path());
    let response: String = c
        .query_row(
            "SELECT response FROM preserved_mutation_requests WHERE request_id=?1",
            [id],
            |r| r.get(0),
        )
        .unwrap();
    let mut altered: Value = serde_json::from_str(&response).unwrap();
    altered["name"] = json!("가상 잘못된 원응답");
    c.execute(
        "UPDATE preserved_mutation_requests SET response=?1 WHERE request_id=?2",
        rusqlite::params![altered.to_string(), id],
    )
    .unwrap();
    assert!(s.catalog_create(decode(input.clone())).is_err());
    drop(s);
    assert!(Store::open(dir.path()).is_err());
    c.execute(
        "UPDATE preserved_mutation_requests SET response=?1 WHERE request_id=?2",
        rusqlite::params![response, id],
    )
    .unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    s.catalog_create(decode(input)).unwrap();
}

#[test]
fn source_fourteen_future_journal_rejects_migration_without_changing_original_database() {
    let dir = TempDir::new().unwrap();
    install14(dir.path());
    let c = db(dir.path());
    let mut source: Value = serde_json::from_str(&journals(dir.path()).last().unwrap().1).unwrap();
    source["schema_version"] = json!(15);
    let raw = source.to_string();
    c.execute(
        "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=50",
        [&raw],
    )
    .unwrap();
    fs::write(dir.path().join("snapshots/00000000000000000050.json"), raw).unwrap();
    drop(c);
    let before = fs::read(dir.path().join("app.sqlite3")).unwrap();
    let files = original_bytes(dir.path());
    assert!(Store::open(dir.path()).is_err());
    assert_eq!(fs::read(dir.path().join("app.sqlite3")).unwrap(), before);
    assert_eq!(original_bytes(dir.path()), files);
    assert_eq!(
        db(dir.path())
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        14
    );
}

#[test]
fn invalid_imports_and_sql_constraints_roll_back_preflight_without_registration() {
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    create(&mut s, "가상 검증 기준");
    let before = normal_rows(dir.path());
    let file = fs::read(dir.path().join("app.sqlite3")).unwrap();
    let original = journals(dir.path())[0].1.clone();
    for mode in 0..2 {
        let mut source: Value = serde_json::from_str(&original).unwrap();
        if mode == 0 {
            source["state"]["material_heads"][0]["current_version_id"] =
                json!(Uuid::new_v4().to_string());
        } else {
            source["state"]["material_versions"][0]["name"] = json!("x".repeat(101));
        }
        assert!(
            s.restore_import(restore_upload(serde_json::to_vec(&source).unwrap()).unwrap())
                .is_err()
        );
        assert_eq!(normal_rows(dir.path()), before);
        let actual = fs::read(dir.path().join("app.sqlite3")).unwrap();
        assert!(
            actual == file,
            "database bytes changed: length {} -> {}, first differing offset {}",
            file.len(),
            actual.len(),
            actual
                .iter()
                .zip(&file)
                .position(|(a, b)| a != b)
                .unwrap_or(actual.len().min(file.len()))
        );
    }
    assert!(
        s.restore_import(restore_upload(b"{\"a\":1,\"a\":2}".to_vec()).unwrap())
            .is_err()
    );
    assert_eq!(count(dir.path(), "restore_imports"), 0);
}

#[test]
fn late_transaction_failure_keeps_active_rows_and_same_uuid_can_be_retried() {
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    create(&mut s, "가상 A");
    let target = latest(&s);
    create(&mut s, "가상 B");
    let input = request(&s, &target);
    let before = normal_rows(dir.path());
    let files = original_bytes(dir.path());
    let c = db(dir.path());
    c.execute_batch("CREATE TRIGGER fail_restore BEFORE INSERT ON restore_records BEGIN SELECT RAISE(FAIL,'synthetic final insert failure'); END;").unwrap();
    assert!(s.restore(input.clone()).is_err());
    assert_eq!(normal_rows(dir.path()), before);
    assert_eq!(original_bytes(dir.path()), files);
    c.execute_batch("DROP TRIGGER fail_restore").unwrap();
    s.restore(input).unwrap();
    assert_eq!(s.list().unwrap().len(), 1);
    assert_eq!(count(dir.path(), "restore_records"), 1);
}

#[test]
fn committed_publication_failure_requires_same_uuid_recovery_before_other_writes() {
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    create(&mut s, "가상 A");
    let target = latest(&s);
    create(&mut s, "가상 B");
    let input = request(&s, &target);
    let blocked = dir.path().join("snapshots/00000000000000000003.json");
    fs::create_dir(&blocked).unwrap();
    assert!(matches!(
        s.restore(input.clone()),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(count(dir.path(), "restore_records"), 1);
    assert_eq!(s.list().unwrap().len(), 1);
    let current = create_input(&s, "가상 대기 입력");
    assert!(s.catalog_create(decode(current.clone())).is_err());
    assert_eq!(count(dir.path(), "snapshot_outbox"), 3);
    fs::remove_dir(blocked).unwrap();
    let result = s.restore(input.clone()).unwrap();
    assert_eq!(value(s.restore(input).unwrap()), value(result));
    s.catalog_create(decode(current)).unwrap();
    assert_eq!(count(dir.path(), "restore_records"), 1);
    drop(s);
    Store::open(dir.path()).unwrap();
}

#[test]
fn import_registry_is_hash_deduplicated_and_cursor_paging_has_no_hidden_limit() {
    let dir = TempDir::new().unwrap();
    let mut s = Store::open(dir.path()).unwrap();
    for n in 0..26 {
        create(&mut s, &format!("가상 지점 {n}"));
    }
    let first = s.restore_targets(RestoreTargetQuery::default()).unwrap();
    assert_eq!(first.rows.len(), 25);
    let next = s
        .restore_targets(RestoreTargetQuery {
            source: RestoreTargetSource::Journal,
            cursor: first.next_cursor,
        })
        .unwrap();
    assert_eq!(next.rows.len(), 1);
    assert!(next.next_cursor.is_none());
    let before = s.catalog_state().unwrap();
    let bytes = journals(dir.path())[0].1.clone();
    let first = s
        .restore_import(restore_upload(bytes.clone().into_bytes()).unwrap())
        .unwrap();
    let again = s
        .restore_import(restore_upload(bytes.into_bytes()).unwrap())
        .unwrap();
    assert_eq!(first, again);
    assert_eq!(s.catalog_state().unwrap(), before);
    assert_eq!(count(dir.path(), "restore_imports"), 1);
}

#[test]
fn imported_business_uuid_conflicting_with_legacy_registration_is_a_request_blocker() {
    let dir = TempDir::new().unwrap();
    let c = db(dir.path());
    c.execute_batch(include_str!("fixtures/schema-v2.sql"))
        .unwrap();
    c.execute(
        "INSERT INTO snapshot_outbox VALUES(1,?1,1)",
        [include_str!("fixtures/snapshot-v1.json")],
    )
    .unwrap();
    drop(c);
    let mut local = Store::open(dir.path()).unwrap();
    assert_eq!(count(dir.path(), "preserved_creation_requests"), 1);
    assert_eq!(count(dir.path(), "preserved_mutation_requests"), 0);

    let source_dir = TempDir::new().unwrap();
    let mut source = Store::open(source_dir.path()).unwrap();
    let mut request_meta = meta(&source);
    request_meta["request_id"] = json!("00000000-0000-4000-8000-000000000101");
    source
        .business_create(decode(json!({
            "meta":request_meta,
            "payload":{"kind":"catalog_price","version":1,"data":{
                "material_id":null,"material_version_id":null,
                "quantity":"","unit":"g","amount":"","note":"가상 미완성 초안"
            }},
            "corrects_confirmation_id":null
        })))
        .unwrap();
    let source_bytes = journals(source_dir.path())[0].1.clone().into_bytes();
    let target = local
        .restore_import(restore_upload(source_bytes).unwrap())
        .unwrap();
    let before = normal_rows(dir.path());
    let bytes = fs::read(dir.path().join("app.sqlite3")).unwrap();
    let files = original_bytes(dir.path());
    let state = local.catalog_state().unwrap();
    let blocked = preview(&mut local, &target);
    assert!(!blocked.can_restore);
    assert!(
        blocked
            .blockers
            .iter()
            .any(|b| { b.code == sunchang_hansang::restore::RestoreBlockerCode::RequestConflict })
    );
    assert!(matches!(
        local.restore(request(&local, &target)),
        Err(StoreError::Conflict(_))
    ));
    assert_eq!(local.catalog_state().unwrap(), state);
    assert_eq!(normal_rows(dir.path()), before);
    let actual = fs::read(dir.path().join("app.sqlite3")).unwrap();
    assert!(
        actual == bytes,
        "database bytes changed: length {} -> {}, first differing offset {}",
        bytes.len(),
        actual.len(),
        actual
            .iter()
            .zip(&bytes)
            .position(|(a, b)| a != b)
            .unwrap_or(actual.len().min(bytes.len()))
    );
    assert_eq!(original_bytes(dir.path()), files);
}

#[test]
fn imported_active_and_preserved_request_ids_cannot_be_reused_for_a_new_restore() {
    let source_dir = TempDir::new().unwrap();
    let mut source = Store::open(source_dir.path()).unwrap();
    create(&mut source, "가상 외부 기준");
    let earlier = latest(&source);
    let source_request = create_input(&source, "가상 외부 보존 요청");
    let source_id = source_request["request_id"].as_str().unwrap().to_owned();
    source.catalog_create(decode(source_request)).unwrap();
    let active_bytes = journals(source_dir.path()).last().unwrap().1.clone();
    source.restore(request(&source, &earlier)).unwrap();
    let preserved_bytes = journals(source_dir.path()).last().unwrap().1.clone();
    let mut failures = Vec::new();

    for (mode, raw, active) in [
        ("active", active_bytes, true),
        ("preserved_only", preserved_bytes, false),
    ] {
        let decoded: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            decoded["state"]["mutation_requests"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["request_id"] == source_id),
            active
        );
        assert!(
            decoded["preserved"]["mutation_requests"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["request_id"] == source_id)
        );
        let dir = TempDir::new().unwrap();
        let mut local = Store::open(dir.path()).unwrap();
        create(&mut local, "가상 로컬 기준");
        let target = local
            .restore_import(restore_upload(raw.into_bytes()).unwrap())
            .unwrap();
        assert!(preview(&mut local, &target).can_restore);
        let rows = normal_rows(dir.path());
        let bytes = fs::read(dir.path().join("app.sqlite3")).unwrap();
        let files = original_bytes(dir.path());
        let imported_path = dir.path().join(format!("imports/{}.json", target.sha256));
        let imported_bytes = fs::read(&imported_path).unwrap();
        let import_files = || {
            fs::read_dir(dir.path().join("imports"))
                .unwrap()
                .map(|entry| {
                    let entry = entry.unwrap();
                    (entry.file_name(), fs::read(entry.path()).unwrap())
                })
                .collect::<BTreeMap<_, _>>()
        };
        let imports_before = import_files();
        let state = local.catalog_state().unwrap();
        let mut input = request(&local, &target);
        input.meta.request_id = source_id.clone();

        let result = local.restore(input);
        if !matches!(result, Err(StoreError::Conflict(_))) {
            failures.push(format!("{mode}: expected request conflict, got {result:?}"));
        }
        assert_eq!(local.catalog_state().unwrap(), state, "{mode}");
        assert!(
            normal_rows(dir.path()) == rows,
            "{mode}: database rows changed"
        );
        let actual = fs::read(dir.path().join("app.sqlite3")).unwrap();
        assert!(actual == bytes, "{mode}: database bytes changed");
        assert!(
            original_bytes(dir.path()) == files,
            "{mode}: journal bytes changed"
        );
        assert!(
            fs::read(imported_path).unwrap() == imported_bytes,
            "{mode}: imported original bytes changed"
        );
        assert!(
            import_files() == imports_before,
            "{mode}: conflicting request published a new capture or changed an import"
        );

        // The target itself remains valid. A genuinely new request succeeds and
        // repeats its original response without a second restore operation.
        let valid = request(&local, &target);
        let result = local.restore(valid.clone()).unwrap();
        assert_eq!(value(local.restore(valid).unwrap()), value(result));
        assert_eq!(
            count(dir.path(), "restore_records"),
            if active { 1 } else { 2 }
        );
        drop(local);
        Store::open(dir.path()).unwrap();
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
