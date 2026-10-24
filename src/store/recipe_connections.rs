use super::{Store, StoreError, catalog};
use crate::{domain::Quantity, recipe::RecipeLine, recipe_connections::*};
use std::collections::BTreeMap;

impl From<StoreError> for RecipeConnectionsError {
    fn from(error: StoreError) -> Self {
        match error {
            StoreError::Validation(message) => Self::Validation(message),
            other => unavailable(other),
        }
    }
}
fn unavailable(error: impl std::fmt::Display) -> RecipeConnectionsError {
    // Do not expose SQL, database paths, or raw rows in an HTTP error.
    let _ = error;
    RecipeConnectionsError::Unavailable("현재 레시피 연결 자료를 읽을 수 없습니다".into())
}

impl Store {
    pub fn recipe_connections(
        &self,
        material_id: &str,
    ) -> Result<Option<RecipeConnections>, RecipeConnectionsError> {
        let id = catalog::uuid(material_id, "품목 ID")?;
        let Some(owner) = catalog::current_version(&self.connection, &id)? else {
            return Ok(None);
        };
        // Read current heads and lines once. Historical recipe versions are not traversed.
        let heads = self.connection.prepare("SELECT h.material_id,h.current_version_id,r.material_id,r.basis_count FROM recipe_heads h LEFT JOIN recipe_versions r ON r.id=h.current_version_id ORDER BY h.material_id").map_err(unavailable)?
            .query_map([], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,Option<i64>>(3)?))).map_err(unavailable)?
            .collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
        let mut nodes = BTreeMap::<String, RecipeConnectionNode>::new();
        for (material, version, version_owner, basis_count) in heads {
            if version_owner.as_deref() != Some(material.as_str()) {
                return Err(unavailable("head owner mismatch"));
            }
            let basis_count = basis_count
                .filter(|value| (1..=1_000_000_000).contains(value))
                .ok_or_else(|| unavailable("invalid recipe basis"))?
                .to_string();
            nodes.insert(
                version.clone(),
                RecipeConnectionNode {
                    material_id: material,
                    recipe_version_id: version,
                    basis_count,
                    lines: vec![],
                },
            );
        }
        if !nodes.values().any(|node| node.material_id == id) {
            return Ok(None);
        }
        let mut statement=self.connection.prepare("SELECT l.recipe_version_id,l.id,l.position,l.material_id,l.material_version_id,m.name,l.quantity_micros,l.unit,l.input_quantity,l.input_unit,l.note,m.material_id FROM recipe_lines l JOIN recipe_heads h ON h.current_version_id=l.recipe_version_id LEFT JOIN material_versions m ON m.id=l.material_version_id ORDER BY l.recipe_version_id,l.position").map_err(unavailable)?;
        let rows = statement
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, i64>(6)?,
                    r.get::<_, String>(7)?,
                    r.get::<_, String>(8)?,
                    r.get::<_, String>(9)?,
                    r.get::<_, String>(10)?,
                    r.get::<_, Option<String>>(11)?,
                ))
            })
            .map_err(unavailable)?;
        for row in rows {
            let (
                recipe,
                id,
                position,
                material,
                version,
                name,
                quantity,
                unit,
                input_quantity,
                input_unit,
                note,
                version_owner,
            ) = row.map_err(unavailable)?;
            if !(0..100).contains(&position) || version_owner.as_deref() != Some(material.as_str())
            {
                return Err(unavailable("component owner mismatch"));
            }
            let line = RecipeLine {
                id,
                position: position.to_string(),
                material_id: material,
                material_version_id: version,
                material_name: name.ok_or_else(|| unavailable("missing component"))?,
                quantity: Quantity::from_micros(quantity)
                    .map_err(unavailable)?
                    .decimal(),
                unit,
                input_quantity,
                input_unit,
                note,
            };
            nodes
                .get_mut(&recipe)
                .ok_or_else(|| unavailable("missing recipe head"))?
                .lines
                .push(line);
        }
        build_recipe_connections(
            catalog::summary(&self.connection, owner)?,
            &nodes.into_values().collect::<Vec<_>>(),
            catalog::state(&self.connection)?,
        )
    }
}
