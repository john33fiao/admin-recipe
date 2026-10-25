use rusqlite::Connection;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{fs, io::Cursor, path::Path};
use sunchang_hansang::{
    catalog::MaterialSummary,
    photo_assets::{PhotoUpload, photo_upload},
    profile::MaterialProfileVersion,
    store::{CreateMaterial, Store, StoreError},
};
use tempfile::TempDir;
use uuid::Uuid;

#[test]
fn opening_store_provides_profile_relations_without_inventing_profile_history() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let material = store
        .create(CreateMaterial {
            request_id: Uuid::new_v4().to_string(),
            name: "가상 사진 없는 기존 품목".into(),
            unit: "g".into(),
        })
        .unwrap();
    drop(store);
    let store = Store::open(root.path()).unwrap();
    assert_eq!(store.list().unwrap()[0].id, material.id);
    let database = Connection::open(root.path().join("app.sqlite3")).unwrap();
    assert_eq!(
        database
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        15
    );
    for table in [
        "photo_assets",
        "material_profile_versions",
        "material_profile_heads",
        "material_profile_photos",
        "material_profile_suppliers",
    ] {
        assert_eq!(
            database
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row
                    .get::<_, i64>(0))
                .unwrap(),
            0,
            "{table}"
        );
    }
}

fn decode<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}
fn value(input: impl Serialize) -> Value {
    serde_json::to_value(input).unwrap()
}
fn meta(store: &Store) -> Value {
    let state = store.catalog_state().unwrap();
    json!({"request_id":Uuid::new_v4().to_string(),"epoch":state.epoch,"expected_revision":state.revision})
}
fn item(store: &mut Store, name: &str) -> MaterialSummary {
    let mut input = meta(store);
    input.as_object_mut().unwrap().extend(
        json!({"name":name,"unit":"g","kind":"raw","description":"가상 품목 설명"})
            .as_object()
            .unwrap()
            .clone(),
    );
    store.catalog_create(decode(input)).unwrap().value
}
fn request(
    store: &Store,
    item: &MaterialSummary,
    photos: Vec<Value>,
    primary: Option<usize>,
    suppliers: Vec<Value>,
) -> Value {
    let profile = store.profile_current(&item.id).unwrap().unwrap();
    json!({"meta":meta(store),"expected_material_version_id":profile.material.version_id,"expected_profile_version_id":profile.version.map(|v|v.id),"photos":photos,"primary_position":primary,"suppliers":suppliers})
}
fn bytes(seed: u8) -> Vec<u8> {
    let image = image::RgbImage::from_fn(3, 2, |x, y| image::Rgb([seed, x as u8, y as u8]));
    let mut output = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image)
        .write_to(&mut output, image::ImageFormat::Png)
        .unwrap();
    output.into_inner()
}
fn upload(seed: u8) -> PhotoUpload {
    photo_upload("가상 사진.png", Some("image/png"), bytes(seed)).unwrap()
}
fn save_photo(store: &mut Store, item: &MaterialSummary, seed: u8) -> MaterialProfileVersion {
    let input = request(
        store,
        item,
        vec![json!({"source":"upload","caption":"가상 사진"})],
        Some(0),
        vec![],
    );
    store
        .profile_revise(&item.id, decode(input), Some(upload(seed)))
        .unwrap()
        .version
        .unwrap()
}
fn existing(v: &MaterialProfileVersion, index: usize) -> Value {
    json!({"source":"existing","photo_id":v.photos[index].id,"caption":v.photos[index].caption})
}
fn count(root: &Path, table: &str) -> i64 {
    Connection::open(root.join("app.sqlite3"))
        .unwrap()
        .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}
fn snapshots(root: &Path) -> Vec<(String, Vec<u8>)> {
    let mut rows = fs::read_dir(root.join("snapshots"))
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                fs::read(e.path()).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

#[test]
fn photos_suppliers_history_and_unlink_survive_restart_without_changing_material_description() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = item(&mut store, "가상 사진 품목");
    assert!(
        store
            .profile_current(&item.id)
            .unwrap()
            .unwrap()
            .version
            .is_none()
    );
    let first = save_photo(&mut store, &item, 12);
    let input = request(
        &store,
        &item,
        vec![
            existing(&first, 0),
            json!({"source":"upload","caption":" 두 번째\r\n사진 "}),
        ],
        Some(1),
        vec![
            json!({"url":"https://example.com/synthetic","description":""}),
            json!({"url":null,"description":" 가상 구입처\r\n방문 "}),
        ],
    );
    let second = store
        .profile_revise(&item.id, decode(input), Some(upload(34)))
        .unwrap()
        .version
        .unwrap();
    assert_eq!(second.photos[0].id, first.photos[0].id);
    assert_eq!(second.photos[1].caption, "두 번째\n사진");
    assert_eq!(second.suppliers[1].description, "가상 구입처\n방문");
    assert_eq!(second.photos.iter().filter(|p| p.is_primary).count(), 1);
    assert!(second.photos[1].is_primary);
    let input = request(&store, &item, vec![], None, vec![]);
    let empty = store
        .profile_revise(&item.id, decode(input), None)
        .unwrap()
        .version
        .unwrap();
    assert!(empty.photos.is_empty());
    assert_eq!(empty.previous_version_id, Some(second.id.clone()));
    assert_eq!(count(root.path(), "photo_assets"), 2);
    for (photo, seed) in second.photos.iter().zip([12, 34]) {
        assert_eq!(
            store
                .photo_asset_bytes(&photo.asset.sha256)
                .unwrap()
                .unwrap()
                .bytes,
            bytes(seed)
        );
    }
    drop(store);
    let store = Store::open(root.path()).unwrap();
    assert_eq!(
        value(
            store
                .profile_version(&item.id, &first.id)
                .unwrap()
                .unwrap()
                .version
                .unwrap()
        ),
        value(first)
    );
    assert_eq!(
        store
            .profile_versions(&item.id)
            .unwrap()
            .unwrap()
            .versions
            .len(),
        3
    );
    assert_eq!(
        store
            .profile_current(&item.id)
            .unwrap()
            .unwrap()
            .material
            .description,
        "가상 품목 설명"
    );
}

#[test]
fn original_request_replays_exact_response_after_changes_and_archive_but_different_input_conflicts()
{
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = item(&mut store, "가상 재시도");
    let input = request(
        &store,
        &item,
        vec![json!({"source":"upload","caption":"첫 사진"})],
        Some(0),
        vec![],
    );
    let response = store
        .profile_revise(&item.id, decode(input.clone()), Some(upload(51)))
        .unwrap();
    let v = response.version.clone().unwrap();
    let edit = request(
        &store,
        &item,
        vec![json!({"source":"existing","photo_id":v.photos[0].id,"caption":"설명 변경"})],
        Some(0),
        vec![],
    );
    store.profile_revise(&item.id, decode(edit), None).unwrap();
    let mut archive = meta(&store);
    archive["expected_version_id"] = json!(item.version_id);
    store.catalog_archive(&item.id, decode(archive)).unwrap();
    let before = snapshots(root.path());
    let state = store.catalog_state().unwrap();
    let renamed_upload = photo_upload("다른 이름.png", None, bytes(51)).unwrap();
    assert_eq!(
        value(
            store
                .profile_revise(&item.id, decode(input.clone()), Some(renamed_upload))
                .unwrap()
        ),
        value(response)
    );
    assert_eq!(store.catalog_state().unwrap(), state);
    assert_eq!(snapshots(root.path()), before);
    let mut changed = input.clone();
    changed["photos"][0]["caption"] = json!("다른 입력");
    assert!(matches!(
        store.profile_revise(&item.id, decode(changed), Some(upload(51))),
        Err(StoreError::Conflict(_))
    ));
    assert!(matches!(
        store.profile_revise(&item.id, decode(input), Some(upload(52))),
        Err(StoreError::Conflict(_))
    ));
    assert_eq!(store.catalog_state().unwrap(), state);
}

#[test]
fn stale_and_foreign_photo_references_preserve_the_input_and_do_not_create_versions() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let a = item(&mut store, "가상 A");
    let b = item(&mut store, "가상 B");
    let first = save_photo(&mut store, &a, 7);
    let stale = request(&store, &a, vec![existing(&first, 0)], Some(0), vec![]);
    let edit = request(&store, &a, vec![], None, vec![]);
    store.profile_revise(&a.id, decode(edit), None).unwrap();
    let before = store.catalog_state().unwrap();
    assert!(matches!(
        store.profile_revise(&a.id, decode(stale.clone()), None),
        Err(StoreError::StaleState(_))
    ));
    assert_eq!(stale["photos"][0]["photo_id"], first.photos[0].id);
    let foreign = request(&store, &b, vec![existing(&first, 0)], Some(0), vec![]);
    assert!(matches!(
        store.profile_revise(&b.id, decode(foreign), None),
        Err(StoreError::Validation(_))
    ));
    assert_eq!(store.catalog_state().unwrap(), before);
    assert_eq!(count(root.path(), "material_profile_versions"), 2);
    assert!(store.profile_version(&b.id, &first.id).unwrap().is_none());
}

#[test]
fn metadata_limits_and_unsafe_urls_are_rejected_without_mutation() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = item(&mut store, "가상 입력 검증");
    let base = request(&store, &item, vec![], None, vec![]);
    let before = store.catalog_state().unwrap();
    for url in [
        "javascript:alert(1)",
        "file:///photo.png",
        "//example.com/x",
        "https://name:secret@example.com",
        "https://example.com/\npath",
        "data:text/plain,x",
    ] {
        let mut bad = base.clone();
        bad["suppliers"] = json!([{"url":url,"description":""}]);
        assert!(
            matches!(
                store.profile_revise(&item.id, decode(bad), None),
                Err(StoreError::Validation(_))
            ),
            "{url}"
        );
    }
    for field in [
        "empty_supplier",
        "many_suppliers",
        "long_description",
        "primary_without_photo",
        "missing_file",
        "duplicate_upload",
    ] {
        let mut bad = base.clone();
        match field {
            "empty_supplier" => bad["suppliers"] = json!([{"url":null,"description":" "}]),
            "many_suppliers" => {
                bad["suppliers"] = json!(vec![json!({"url":null,"description":"가상"}); 21])
            }
            "long_description" => {
                bad["suppliers"] = json!([{"url":null,"description":"가".repeat(2001)}])
            }
            "primary_without_photo" => bad["primary_position"] = json!(0),
            "missing_file" => {
                bad["photos"] = json!([{"source":"upload","caption":""}]);
                bad["primary_position"] = json!(0);
            }
            _ => {
                bad["photos"] =
                    json!([{"source":"upload","caption":""},{"source":"upload","caption":""}]);
                bad["primary_position"] = json!(0);
            }
        }
        assert!(
            matches!(
                store.profile_revise(&item.id, decode(bad), None),
                Err(StoreError::Validation(_))
            ),
            "{field}"
        );
    }
    assert_eq!(store.catalog_state().unwrap(), before);
    assert_eq!(count(root.path(), "photo_assets"), 0);
    let mut unknown = base;
    unknown["meta"]["unknown"] = json!(true);
    assert!(
        serde_json::from_value::<sunchang_hansang::profile::ReviseMaterialProfile>(unknown)
            .is_err()
    );
}

#[test]
fn recipe_copy_shares_bytes_but_copies_profile_ids_independently_and_replays_original_result() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let create = json!({"meta":meta(&store),"owner":{"mode":"new","name":"가상 사진 메뉴","unit":"개","kind":"menu","description":"가상"},"data":{"basis_count":"1","reference_output":null,"selling_price":null,"cooking_description":"","process":"","reference_time":"","lines":[]}});
    let recipe = store.recipe_create(decode(create)).unwrap();
    let source = save_photo(&mut store, &recipe.item, 83);
    let input = json!({"meta":meta(&store),"expected_material_version_id":recipe.item.version_id,"source_recipe_version_id":recipe.version.id});
    let copied = store
        .recipe_copy(&recipe.item.id, decode(input.clone()))
        .unwrap();
    let target = store
        .profile_current(&copied.item.id)
        .unwrap()
        .unwrap()
        .version
        .unwrap();
    assert_ne!(target.id, source.id);
    assert_ne!(target.photos[0].id, source.photos[0].id);
    assert_eq!(target.photos[0].asset, source.photos[0].asset);
    assert_eq!(target.copied_from_version_id, Some(source.id.clone()));
    assert_eq!(target.seq, copied.version.seq);
    assert_eq!(count(root.path(), "photo_assets"), 1);
    let clear = request(&store, &recipe.item, vec![], None, vec![]);
    store
        .profile_revise(&recipe.item.id, decode(clear), None)
        .unwrap();
    assert_eq!(
        value(
            store
                .profile_current(&copied.item.id)
                .unwrap()
                .unwrap()
                .version
                .unwrap()
        ),
        value(target)
    );
    let before = snapshots(root.path());
    assert_eq!(
        value(store.recipe_copy(&recipe.item.id, decode(input)).unwrap()),
        value(copied)
    );
    assert_eq!(snapshots(root.path()), before);
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn profile_failure_rolls_back_recipe_owner_recipe_and_profile_copy_together() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let create = json!({"meta":meta(&store),"owner":{"mode":"new","name":"가상 복제 원자성","unit":"개","kind":"menu","description":"가상"},"data":{"basis_count":"1","reference_output":null,"selling_price":null,"cooking_description":"","process":"","reference_time":"","lines":[]}});
    let recipe = store.recipe_create(decode(create)).unwrap();
    save_photo(&mut store, &recipe.item, 98);
    let input = json!({"meta":meta(&store),"expected_material_version_id":recipe.item.version_id,"source_recipe_version_id":recipe.version.id});
    let database = Connection::open(root.path().join("app.sqlite3")).unwrap();
    database.execute_batch("CREATE TRIGGER reject_copy BEFORE INSERT ON material_profile_heads BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    let tables = [
        "materials",
        "material_versions",
        "recipe_versions",
        "recipe_heads",
        "material_profile_versions",
        "material_profile_heads",
        "material_profile_photos",
        "photo_assets",
        "mutation_requests",
        "snapshot_outbox",
    ];
    let before = tables
        .iter()
        .map(|t| count(root.path(), t))
        .collect::<Vec<_>>();
    let files = snapshots(root.path());
    let state = store.catalog_state().unwrap();
    assert!(
        store
            .recipe_copy(&recipe.item.id, decode(input.clone()))
            .is_err()
    );
    assert_eq!(
        tables
            .iter()
            .map(|t| count(root.path(), t))
            .collect::<Vec<_>>(),
        before
    );
    assert_eq!(snapshots(root.path()), files);
    assert_eq!(store.catalog_state().unwrap(), state);
    database.execute_batch("DROP TRIGGER reject_copy;").unwrap();
    let copied = store.recipe_copy(&recipe.item.id, decode(input)).unwrap();
    assert!(
        store
            .profile_current(&copied.item.id)
            .unwrap()
            .unwrap()
            .version
            .is_some()
    );
}

#[test]
fn file_first_publication_db_failure_and_semantic_failure_keep_orphans_unregistered_and_retry_once()
{
    for semantic in [false, true] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let item = item(&mut store, "가상 원자 저장");
        let input = request(
            &store,
            &item,
            vec![json!({"source":"upload","caption":"원문"})],
            Some(0),
            vec![],
        );
        let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
        db.execute_batch(if semantic {"CREATE TRIGGER profile_failure AFTER INSERT ON material_profile_photos BEGIN UPDATE material_profile_photos SET caption='변조' WHERE profile_version_id=NEW.profile_version_id; END;"} else {"CREATE TRIGGER profile_failure BEFORE INSERT ON material_profile_heads BEGIN SELECT RAISE(ABORT,'injected'); END;"}).unwrap();
        let before = snapshots(root.path());
        let state = store.catalog_state().unwrap();
        let requests = count(root.path(), "mutation_requests");
        assert!(
            store
                .profile_revise(&item.id, decode(input.clone()), Some(upload(90)))
                .is_err()
        );
        assert_eq!(store.catalog_state().unwrap(), state);
        assert_eq!(snapshots(root.path()), before);
        for table in [
            "photo_assets",
            "material_profile_versions",
            "material_profile_heads",
            "material_profile_photos",
            "material_profile_suppliers",
        ] {
            assert_eq!(count(root.path(), table), 0, "{table}");
        }
        assert_eq!(count(root.path(), "mutation_requests"), requests);
        let validated = sunchang_hansang::photo_assets::validate_photo(upload(90)).unwrap();
        assert!(root.path().join(&validated.descriptor().asset_key).exists());
        assert!(
            store
                .photo_asset_bytes(&validated.descriptor().sha256)
                .unwrap()
                .is_none()
        );
        db.execute_batch("DROP TRIGGER profile_failure;").unwrap();
        let response = store
            .profile_revise(&item.id, decode(input.clone()), Some(upload(90)))
            .unwrap();
        assert_eq!(
            value(
                store
                    .profile_revise(&item.id, decode(input), Some(upload(90)))
                    .unwrap()
            ),
            value(response)
        );
        assert_eq!(count(root.path(), "photo_assets"), 1);
        assert_eq!(count(root.path(), "material_profile_versions"), 1);
    }
}

#[test]
fn json_publication_failure_recovers_same_request_and_blocks_other_changes() {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let item = item(&mut store, "가상 JSON 재시도");
    let input = request(
        &store,
        &item,
        vec![json!({"source":"upload","caption":"원문"})],
        Some(0),
        vec![],
    );
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
        store.profile_revise(&item.id, decode(input.clone()), Some(upload(17))),
        Err(StoreError::Unavailable(_))
    ));
    assert_eq!(count(root.path(), "material_profile_versions"), 1);
    let saved = Connection::open(root.path().join("app.sqlite3"))
        .unwrap()
        .query_row(
            "SELECT response FROM mutation_requests WHERE request_id=?1",
            [input["meta"]["request_id"].as_str().unwrap()],
            |r| r.get::<_, String>(0),
        )
        .unwrap();
    let other = request(&store, &item, vec![], None, vec![]);
    assert!(matches!(
        store.profile_revise(&item.id, decode(other), None),
        Err(StoreError::Unavailable(_))
    ));
    fs::remove_dir(&path).unwrap();
    assert_eq!(
        value(
            store
                .profile_revise(&item.id, decode(input), Some(upload(17)))
                .unwrap()
        ),
        serde_json::from_str::<Value>(&saved).unwrap()
    );
    assert_eq!(count(root.path(), "material_profile_versions"), 1);
    assert!(path.is_file());
    drop(store);
    Store::open(root.path()).unwrap();
}

#[test]
fn registered_missing_or_changed_bytes_fail_read_retry_and_startup_but_unrelated_writes_do_not_rehash_assets()
 {
    for missing in [true, false] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let item = item(&mut store, "가상 손상");
        let input = request(
            &store,
            &item,
            vec![json!({"source":"upload","caption":"원문"})],
            Some(0),
            vec![],
        );
        let v = store
            .profile_revise(&item.id, decode(input), Some(upload(24)))
            .unwrap()
            .version
            .unwrap();
        let edit = request(&store, &item, vec![existing(&v, 0)], Some(0), vec![]);
        let response = store
            .profile_revise(&item.id, decode(edit.clone()), None)
            .unwrap();
        let path = root.path().join(&v.photos[0].asset.asset_key);
        if missing {
            fs::remove_file(&path).unwrap();
        } else {
            fs::write(&path, vec![0; bytes(24).len()]).unwrap();
        }
        assert!(matches!(
            store.photo_asset_bytes(&v.photos[0].asset.sha256),
            Err(StoreError::Unavailable(_))
        ));
        assert!(matches!(
            store.profile_revise(&item.id, decode(edit.clone()), None),
            Err(StoreError::Unavailable(_))
        ));
        // Ordinary writes validate metadata, not every historical image file.
        item_without_profile(&mut store);
        drop(store);
        assert!(Store::open(root.path()).is_err());
        fs::write(&path, bytes(24)).unwrap();
        let mut store = Store::open(root.path()).unwrap();
        assert_eq!(
            value(store.profile_revise(&item.id, decode(edit), None).unwrap()),
            value(response)
        );
    }
}
fn item_without_profile(store: &mut Store) {
    item(store, "가상 사진 없는 별도 품목");
}

#[test]
fn relocated_database_preserves_original_asset_bytes_and_history() {
    let source = TempDir::new().unwrap();
    let mut store = Store::open(source.path()).unwrap();
    let item = item(&mut store, "가상 이동");
    let profile = save_photo(&mut store, &item, 44);
    drop(store);
    let target = TempDir::new().unwrap();
    fn copy(source: &Path, target: &Path) {
        fs::create_dir_all(target).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let to = target.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &to)
            } else {
                fs::copy(entry.path(), to).unwrap();
            }
        }
    }
    copy(source.path(), target.path());
    let store = Store::open(target.path()).unwrap();
    assert_eq!(
        value(
            store
                .profile_current(&item.id)
                .unwrap()
                .unwrap()
                .version
                .unwrap()
        ),
        value(profile.clone())
    );
    assert_eq!(
        store
            .photo_asset_bytes(&profile.photos[0].asset.sha256)
            .unwrap()
            .unwrap()
            .bytes,
        bytes(44)
    );
    assert_eq!(snapshots(source.path()), snapshots(target.path()));
}

#[test]
fn snapshot_profile_tampering_is_rejected_even_when_file_and_outbox_bytes_match() {
    for mode in [
        "missing_profile",
        "wrong_asset",
        "wrong_request",
        "future_request_revision",
        "future_profile",
        "wrong_primary",
        "wrong_copy_source",
    ] {
        let root = TempDir::new().unwrap();
        let mut store = Store::open(root.path()).unwrap();
        let item = item(&mut store, "가상 무결성");
        let profile = save_photo(&mut store, &item, 31);
        drop(store);
        let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
        let seq: i64 = profile.seq.parse().unwrap();
        let raw: String = db
            .query_row(
                "SELECT payload FROM snapshot_outbox WHERE sequence=?1",
                [seq],
                |r| r.get(0),
            )
            .unwrap();
        let mut snapshot: Value = serde_json::from_str(&raw).unwrap();
        match mode {
            "missing_profile" => snapshot["state"]["material_profile_versions"] = json!([]),
            "wrong_asset" => snapshot["state"]["photo_assets"][0]["descriptor"]["width"] = json!(9),
            "wrong_request" => {
                let rows = snapshot["state"]["mutation_requests"]
                    .as_array_mut()
                    .unwrap();
                let r = rows
                    .iter_mut()
                    .find(|r| r["operation"] == "profile.revise")
                    .unwrap();
                r["request_id"] = json!(Uuid::new_v4().to_string());
            }
            "future_request_revision" => {
                let rows = snapshot["state"]["mutation_requests"]
                    .as_array_mut()
                    .unwrap();
                let r = rows
                    .iter_mut()
                    .find(|r| r["operation"] == "profile.revise")
                    .unwrap();
                let mut payload: Value =
                    serde_json::from_str(r["payload"].as_str().unwrap()).unwrap();
                payload["request"]["meta"]["expected_revision"] = json!("999999");
                r["payload"] = json!(serde_json::to_string(&payload).unwrap());
            }
            "future_profile" => {
                snapshot["state"]["material_profile_versions"][0]["seq"] = json!("999999")
            }
            "wrong_primary" => {
                snapshot["state"]["material_profile_photos"][0]["photo"]["is_primary"] =
                    json!(false)
            }
            _ => {
                snapshot["state"]["material_profile_versions"][0]["copied_from_version_id"] =
                    json!(Uuid::new_v4().to_string())
            }
        }
        let raw = serde_json::to_string(&snapshot).unwrap();
        db.execute(
            "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=?2",
            rusqlite::params![raw, seq],
        )
        .unwrap();
        drop(db);
        fs::write(root.path().join(format!("snapshots/{seq:020}.json")), raw).unwrap();
        let before = fs::read(root.path().join("app.sqlite3")).unwrap();
        let files = snapshots(root.path());
        assert!(Store::open(root.path()).is_err(), "{mode}");
        assert_eq!(
            fs::read(root.path().join("app.sqlite3")).unwrap(),
            before,
            "{mode}"
        );
        assert_eq!(snapshots(root.path()), files, "{mode}");
    }
}

#[test]
fn schema_thirteen_migration_preserves_every_existing_row_and_journal_and_old_requests() {
    let root = TempDir::new().unwrap();
    let mut database = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let transaction = database.transaction().unwrap();
    transaction
        .execute_batch("PRAGMA defer_foreign_keys=ON;")
        .unwrap();
    transaction
        .execute_batch(include_str!("fixtures/schema-v13.sql"))
        .unwrap();
    transaction.commit().unwrap();
    fn rows(db: &Connection, table: &str) -> Vec<Vec<rusqlite::types::Value>> {
        let mut statement = db
            .prepare(&format!("SELECT * FROM \"{table}\" ORDER BY rowid"))
            .unwrap();
        let columns = statement.column_count();
        statement
            .query_map([], |r| (0..columns).map(|i| r.get(i)).collect())
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap()
    }
    let tables=database.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap().query_map([],|r|r.get::<_,String>(0)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
    let before = tables
        .iter()
        .map(|t| (t.clone(), rows(&database, t)))
        .collect::<Vec<_>>();
    let journals = database
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")
        .unwrap()
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    let requests=database.prepare("SELECT operation,payload,response FROM mutation_requests WHERE operation IN ('recipe.copy','business.create') ORDER BY sequence").unwrap().query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
    fs::create_dir(root.path().join("snapshots")).unwrap();
    for (seq, payload) in &journals {
        fs::write(
            root.path().join(format!("snapshots/{seq:020}.json")),
            payload,
        )
        .unwrap();
    }
    let files = snapshots(root.path());
    drop(database);
    let mut store = Store::open(root.path()).unwrap();
    let state = store.catalog_state().unwrap();
    assert!(!requests.is_empty());
    for (op, payload, response) in requests {
        let actual = if op == "recipe.copy" {
            let (id, input) =
                serde_json::from_str::<(String, sunchang_hansang::recipe::CopyRecipe)>(&payload)
                    .unwrap();
            value(store.recipe_copy(&id, input).unwrap())
        } else {
            value(
                store
                    .business_create(serde_json::from_str(&payload).unwrap())
                    .unwrap(),
            )
        };
        assert_eq!(actual, serde_json::from_str::<Value>(&response).unwrap());
    }
    assert_eq!(store.catalog_state().unwrap(), state);
    drop(store);
    let database = Connection::open(root.path().join("app.sqlite3")).unwrap();
    assert_eq!(
        database
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        15
    );
    for (table, expected) in before {
        assert_eq!(rows(&database, &table), expected, "{table}");
    }
    assert_eq!(snapshots(root.path()), files);
    for table in [
        "photo_assets",
        "material_profile_versions",
        "material_profile_heads",
        "material_profile_photos",
        "material_profile_suppliers",
    ] {
        assert_eq!(count(root.path(), table), 0, "{table}");
    }
}

#[test]
fn source_thirteen_rejects_a_well_formed_schema_fourteen_journal_before_migration() {
    let root = TempDir::new().unwrap();
    let mut database = Connection::open(root.path().join("app.sqlite3")).unwrap();
    let transaction = database.transaction().unwrap();
    transaction
        .execute_batch("PRAGMA defer_foreign_keys=ON;")
        .unwrap();
    transaction
        .execute_batch(include_str!("fixtures/schema-v13.sql"))
        .unwrap();
    transaction.commit().unwrap();
    let (seq, payload): (i64, String) = database
        .query_row(
            "SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    let mut future: Value = serde_json::from_str(&payload).unwrap();
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
    let future = serde_json::to_string(&future).unwrap();
    database
        .execute(
            "UPDATE snapshot_outbox SET payload=?1 WHERE sequence=?2",
            rusqlite::params![future, seq],
        )
        .unwrap();
    fs::create_dir(root.path().join("snapshots")).unwrap();
    let rows = database
        .prepare("SELECT sequence,payload FROM snapshot_outbox ORDER BY sequence")
        .unwrap()
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    for (seq, payload) in rows {
        fs::write(
            root.path().join(format!("snapshots/{seq:020}.json")),
            payload,
        )
        .unwrap();
    }
    drop(database);
    let db_before = fs::read(root.path().join("app.sqlite3")).unwrap();
    let files = snapshots(root.path());
    assert!(Store::open(root.path()).is_err());
    assert_eq!(
        fs::read(root.path().join("app.sqlite3")).unwrap(),
        db_before
    );
    assert_eq!(snapshots(root.path()), files);
    let database = Connection::open(root.path().join("app.sqlite3")).unwrap();
    assert_eq!(
        database
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        13
    );
    assert_eq!(
        database
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='photo_assets'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
}
