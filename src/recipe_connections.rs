//! Current recipe connections are navigation paths, never recursive cost totals.
use crate::{
    catalog::{CatalogState, MaterialSummary},
    recipe::RecipeLine,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub const MAX_CONNECTION_DEPTH: usize = 64;
pub const MAX_CONNECTION_PATHS: usize = 1_000;

#[derive(Debug)]
pub enum RecipeConnectionsError {
    Validation(String),
    TooLarge,
    Unavailable(String),
}
impl std::fmt::Display for RecipeConnectionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(message) | Self::Unavailable(message) => f.write_str(message),
            Self::TooLarge => f.write_str(
                "연결 경로가 깊이 64 또는 1000개 한도를 넘었습니다. 더 가까운 품목에서 조회하세요",
            ),
        }
    }
}
impl std::error::Error for RecipeConnectionsError {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeConnectionsBasis {
    CurrentRootCurrentChildren,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeConnectionStep {
    pub recipe_material_id: String,
    pub recipe_version_id: String,
    pub basis_count: String,
    /// The component version saved in this current recipe, not its child recipe version.
    pub line: RecipeLine,
    pub child_recipe_version_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeConnectionPath {
    pub steps: Vec<RecipeConnectionStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeConnections {
    pub item: MaterialSummary,
    pub root_recipe_version_id: String,
    pub basis: RecipeConnectionsBasis,
    pub direct: Vec<RecipeConnectionStep>,
    /// Only paths with at least two edges; direct and indirect together share the cap.
    pub paths: Vec<RecipeConnectionPath>,
    pub state: CatalogState,
}

/// One current head per owner. Storage supplies only current recipe lines.
#[derive(Debug, Clone)]
pub struct RecipeConnectionNode {
    pub material_id: String,
    pub recipe_version_id: String,
    pub basis_count: String,
    pub lines: Vec<RecipeLine>,
}

struct Walker<'a> {
    nodes: HashMap<&'a str, &'a RecipeConnectionNode>,
    active: HashSet<&'a str>,
    direct: Vec<RecipeConnectionStep>,
    paths: Vec<RecipeConnectionPath>,
}
impl Walker<'_> {
    fn visit(
        &mut self,
        owner: &str,
        path: &mut Vec<RecipeConnectionStep>,
    ) -> Result<(), RecipeConnectionsError> {
        let node = self.nodes[owner];
        for line in &node.lines {
            if self.active.contains(line.material_id.as_str()) {
                return Err(RecipeConnectionsError::Unavailable(
                    "저장된 현재 레시피 연결에 순환이 있습니다".into(),
                ));
            }
            if path.len() >= MAX_CONNECTION_DEPTH
                || self.direct.len() + self.paths.len() >= MAX_CONNECTION_PATHS
            {
                return Err(RecipeConnectionsError::TooLarge);
            }
            let child = self.nodes.get(line.material_id.as_str()).copied();
            path.push(RecipeConnectionStep {
                recipe_material_id: node.material_id.clone(),
                recipe_version_id: node.recipe_version_id.clone(),
                basis_count: node.basis_count.clone(),
                line: line.clone(),
                child_recipe_version_id: child.map(|value| value.recipe_version_id.clone()),
            });
            if path.len() == 1 {
                self.direct.push(path[0].clone());
            } else {
                self.paths.push(RecipeConnectionPath {
                    steps: path.clone(),
                });
            }
            if let Some(child) = child {
                self.active.insert(child.material_id.as_str());
                self.visit(&child.material_id, path)?;
                self.active.remove(child.material_id.as_str());
            }
            path.pop();
        }
        Ok(())
    }
}

pub fn build_recipe_connections(
    item: MaterialSummary,
    nodes: &[RecipeConnectionNode],
    state: CatalogState,
) -> Result<Option<RecipeConnections>, RecipeConnectionsError> {
    let mut by_owner = HashMap::new();
    for node in nodes {
        if by_owner.insert(node.material_id.as_str(), node).is_some() {
            return Err(RecipeConnectionsError::Unavailable(
                "현재 레시피의 소유 품목이 중복됐습니다".into(),
            ));
        }
    }
    let Some(root) = by_owner.get(item.id.as_str()).copied() else {
        return Ok(None);
    };
    let root_recipe_version_id = root.recipe_version_id.clone();
    let mut walker = Walker {
        nodes: by_owner,
        active: HashSet::from([root.material_id.as_str()]),
        direct: vec![],
        paths: vec![],
    };
    walker.visit(&item.id, &mut vec![])?;
    Ok(Some(RecipeConnections {
        item,
        root_recipe_version_id,
        basis: RecipeConnectionsBasis::CurrentRootCurrentChildren,
        direct: walker.direct,
        paths: walker.paths,
        state,
    }))
}
