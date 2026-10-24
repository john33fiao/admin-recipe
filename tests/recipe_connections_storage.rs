use serde_json::{Value, json};
use std::{collections::BTreeMap, fs};
use sunchang_hansang::{
    catalog::MaterialSummary, recipe::RecipeMutation, recipe_connections::RecipeConnectionsError,
    store::Store,
};

fn decode<T: serde::de::DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}
fn meta(store: &Store) -> Value {
    let s = store.catalog_state().unwrap();
    json!({"request_id":uuid::Uuid::new_v4().to_string(),"epoch":s.epoch,"expected_revision":s.revision})
}
fn item(store: &mut Store, name: &str) -> MaterialSummary {
    let mut input = meta(store);
    input["name"] = json!(name);
    input["unit"] = json!("g");
    input["kind"] = json!("raw");
    input["description"] = json!("");
    store.catalog_create(decode(input)).unwrap().value
}
fn data(basis: &str, parts: &[(&MaterialSummary, &str)]) -> Value {
    json!({"basis_count":basis,"reference_output":null,"selling_price":null,"cooking_description":"","process":"","reference_time":"","lines":parts.iter().map(|(m,q)|json!({"material_id":m.id,"material_version_id":m.version_id,"quantity":q,"unit":"g","note":"적당량"})).collect::<Vec<_>>()})
}
fn attach(
    store: &mut Store,
    owner: &MaterialSummary,
    basis: &str,
    parts: &[(&MaterialSummary, &str)],
) -> RecipeMutation {
    let request = json!({"meta":meta(store),"owner":{"mode":"existing","material_id":owner.id,"expected_material_version_id":owner.version_id},"data":data(basis,parts)});
    store.recipe_create(decode(request)).unwrap()
}
fn snapshots(root: &std::path::Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(root.join("snapshots"))
        .unwrap()
        .map(|p| {
            let p = p.unwrap();
            (
                p.file_name().to_string_lossy().into_owned(),
                fs::read(p.path()).unwrap(),
            )
        })
        .collect()
}

#[test]
fn current_root_and_current_children_keep_saved_component_versions_and_archive_references() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(temp.path()).unwrap();
    let raw = item(&mut store, "가상 잎 재료");
    let child = item(&mut store, "가상 비축");
    let root = item(&mut store, "가상 메뉴");
    let first_child = attach(&mut store, &child, "2", &[(&raw, "50")]);
    let first_root = attach(&mut store, &root, "30", &[(&child, "300"), (&raw, "0")]);
    let request = json!({"meta":meta(&store),"expected_material_version_id":child.version_id,"expected_recipe_version_id":first_child.version.id,"data":data("3",&[(&raw,"75")])});
    let latest_child = store.recipe_revise(&child.id, decode(request)).unwrap();
    let mut archive = meta(&store);
    archive["expected_version_id"] = json!(child.version_id);
    archive["archived"] = json!(true);
    store.catalog_archive(&child.id, decode(archive)).unwrap();
    let request = json!({"meta":meta(&store),"expected_material_version_id":root.version_id,"expected_recipe_version_id":first_root.version.id,"data":data("60",&[(&child,"600"),(&raw,"0")])});
    let latest_root = store.recipe_revise(&root.id, decode(request)).unwrap();
    let before = store.catalog_state().unwrap();
    let journal = snapshots(temp.path());
    let db = fs::read(temp.path().join("app.sqlite3")).unwrap();
    let graph = store.recipe_connections(&root.id).unwrap().unwrap();
    assert_eq!(graph.state, before);
    assert_eq!(graph.root_recipe_version_id, latest_root.version.id);
    assert_eq!(graph.direct[0].basis_count, "60");
    assert_eq!(graph.direct[0].line.quantity, "600");
    assert_eq!(graph.direct[0].line.material_version_id, child.version_id);
    assert_eq!(
        graph.direct[0].child_recipe_version_id.as_deref(),
        Some(latest_child.version.id.as_str())
    );
    assert_eq!(graph.paths[0].steps[1].basis_count, "3");
    assert_eq!(graph.paths[0].steps[1].line.quantity, "75");
    assert_eq!(graph.direct[1].line.quantity, "0");
    let old = store
        .recipe_version(&root.id, &first_root.version.id)
        .unwrap()
        .unwrap();
    assert_eq!(old.basis_count, "30");
    let uses = store.recipe_uses(&child.id).unwrap().unwrap();
    assert!(uses.items.iter().any(|r| r.item.id == root.id));
    assert_eq!(store.catalog_state().unwrap(), before);
    assert_eq!(snapshots(temp.path()), journal);
    assert_eq!(fs::read(temp.path().join("app.sqlite3")).unwrap(), db);
}

#[test]
fn existing_raw_material_without_recipe_is_none_while_empty_recipe_is_present() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(temp.path()).unwrap();
    let raw = item(&mut store, "가상 빈 연결");
    assert!(store.recipe_connections(&raw.id).unwrap().is_none());
    let empty = attach(&mut store, &raw, "1", &[]);
    let graph = store.recipe_connections(&raw.id).unwrap().unwrap();
    assert_eq!(graph.root_recipe_version_id, empty.version.id);
    assert!(graph.direct.is_empty() && graph.paths.is_empty());
    assert!(matches!(
        store.recipe_connections("not-a-uuid"),
        Err(RecipeConnectionsError::Validation(_))
    ));
}

#[test]
fn a_damaged_current_cycle_is_rejected_without_repair_or_partial_result() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(temp.path()).unwrap();
    let raw = item(&mut store, "가상 원물");
    let child = item(&mut store, "가상 자식");
    let root = item(&mut store, "가상 뿌리");
    let child_recipe = attach(&mut store, &child, "1", &[(&raw, "1")]);
    attach(&mut store, &root, "1", &[(&child, "1")]);
    // Deliberate corruption only: normal fixtures above use the public Store APIs.
    let connection = rusqlite::Connection::open(temp.path().join("app.sqlite3")).unwrap();
    connection.execute("UPDATE recipe_lines SET material_id=?1,material_version_id=?2 WHERE recipe_version_id=?3",rusqlite::params![root.id,root.version_id,child_recipe.version.id]).unwrap();
    let db = fs::read(temp.path().join("app.sqlite3")).unwrap();
    let journal = snapshots(temp.path());
    assert!(matches!(
        store.recipe_connections(&root.id),
        Err(RecipeConnectionsError::Unavailable(_))
    ));
    assert_eq!(fs::read(temp.path().join("app.sqlite3")).unwrap(), db);
    assert_eq!(snapshots(temp.path()), journal);
}
