use serde_json::json;
use sunchang_hansang::{
    catalog::{CatalogState, MaterialSummary},
    recipe::RecipeLine,
    recipe_connections::{RecipeConnectionNode, RecipeConnectionsError, build_recipe_connections},
};

fn item(id: &str) -> MaterialSummary {
    serde_json::from_value(
        json!({"id":id,"version_id":"item-version","name":"가상 연결 품목",
        "unit":"g","canonical_unit":"g","dimension":"mass","kind":"raw","description":"",
        "archived":false,"created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z",
        "price":{"source":"missing","amount":null,"quantity":null,"source_id":null}}),
    )
    .unwrap()
}
fn state() -> CatalogState {
    CatalogState {
        epoch: "epoch".into(),
        revision: "8".into(),
    }
}
fn line(position: usize, child: &str) -> RecipeLine {
    RecipeLine {
        id: format!("line-{position}-{child}"),
        position: position.to_string(),
        material_id: child.into(),
        material_version_id: format!("saved-{child}"),
        material_name: format!("가상 {child}"),
        quantity: "0".into(),
        unit: "g".into(),
        input_quantity: "0".into(),
        input_unit: "g".into(),
        note: "적당량".into(),
    }
}
fn node(id: &str, children: &[&str]) -> RecipeConnectionNode {
    RecipeConnectionNode {
        material_id: id.into(),
        recipe_version_id: format!("current-{id}"),
        basis_count: "1".into(),
        lines: children
            .iter()
            .enumerate()
            .map(|(p, c)| line(p, c))
            .collect(),
    }
}

#[test]
fn current_child_versions_and_zero_rows_preserve_each_distinct_path() {
    let mut nodes = vec![
        node("root", &["a", "b"]),
        node("a", &["leaf"]),
        node("b", &["leaf"]),
    ];
    nodes[0].basis_count = "30".into();
    nodes[1].basis_count = "2".into();
    let report = build_recipe_connections(item("root"), &nodes, state())
        .unwrap()
        .unwrap();
    assert_eq!(report.root_recipe_version_id, "current-root");
    assert_eq!(report.direct.len(), 2);
    assert_eq!(report.paths.len(), 2);
    assert_eq!(report.direct[0].line.material_version_id, "saved-a");
    assert_eq!(
        report.direct[0].child_recipe_version_id.as_deref(),
        Some("current-a")
    );
    assert_eq!(report.direct[0].line.quantity, "0");
    assert_eq!(report.direct[0].basis_count, "30");
    assert_eq!(report.paths[0].steps[1].basis_count, "2");
    assert_eq!(report.paths[0].steps[1].line.material_id, "leaf");
    assert_eq!(report.paths[1].steps[1].line.material_id, "leaf");
    assert_ne!(
        report.paths[0].steps[0].line.material_id,
        report.paths[1].steps[0].line.material_id
    );
    let value = serde_json::to_value(report).unwrap();
    assert_eq!(value["basis"], "current_root_current_children");
    assert!(value.get("cost").is_none());
}

#[test]
fn no_recipe_is_none_but_an_empty_current_recipe_is_a_real_graph() {
    assert!(
        build_recipe_connections(item("root"), &[], state())
            .unwrap()
            .is_none()
    );
    let graph = build_recipe_connections(item("root"), &[node("root", &[])], state())
        .unwrap()
        .unwrap();
    assert!(graph.direct.is_empty() && graph.paths.is_empty());
}

#[test]
fn depth_is_edge_count_and_accepts_64_but_rejects_65_without_a_partial_graph() {
    let mut nodes = (0..64)
        .map(|i| node(&format!("n{i}"), &[&format!("n{}", i + 1)]))
        .collect::<Vec<_>>();
    let graph = build_recipe_connections(item("n0"), &nodes, state())
        .unwrap()
        .unwrap();
    assert_eq!(graph.paths.last().unwrap().steps.len(), 64);
    nodes.push(node("n64", &["n65"]));
    assert!(matches!(
        build_recipe_connections(item("n0"), &nodes, state()),
        Err(RecipeConnectionsError::TooLarge)
    ));
}

#[test]
fn all_returned_direct_and_indirect_paths_count_towards_1000() {
    let children = (0..10).map(|i| format!("child{i}")).collect::<Vec<_>>();
    let mut nodes = vec![node(
        "root",
        &children.iter().map(String::as_str).collect::<Vec<_>>(),
    )];
    for child in &children {
        let mut n = node(child, &[]);
        n.lines = (0..99).map(|i| line(i, "leaf")).collect();
        nodes.push(n);
    }
    let graph = build_recipe_connections(item("root"), &nodes, state())
        .unwrap()
        .unwrap();
    assert_eq!(graph.direct.len() + graph.paths.len(), 1000);
    nodes[1].lines.push(line(99, "leaf"));
    assert!(matches!(
        build_recipe_connections(item("root"), &nodes, state()),
        Err(RecipeConnectionsError::TooLarge)
    ));
}

#[test]
fn cycles_and_duplicate_current_heads_are_corruption_not_truncated_success() {
    for nodes in [
        vec![node("root", &["a"]), node("a", &["root"])],
        vec![node("root", &[]), node("root", &[])],
    ] {
        assert!(matches!(
            build_recipe_connections(item("root"), &nodes, state()),
            Err(RecipeConnectionsError::Unavailable(_))
        ));
    }
}
