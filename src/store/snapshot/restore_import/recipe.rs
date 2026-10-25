use super::super::{SnapshotStateV14, StoreError, unavailable};
use super::common::{money, nonnegative, q, seq};
use rusqlite::{Connection, params};

pub(super) fn insert(connection: &Connection, state: &SnapshotStateV14) -> Result<(), StoreError> {
    for r in &state.recipe_versions {
        let output = r.reference_output.as_ref();
        connection.execute("INSERT INTO recipe_versions(id,material_id,material_version_id,previous_version_id,copied_from_version_id,basis_count,reference_quantity_micros,reference_unit,reference_input_quantity,reference_input_unit,selling_price_micros,cooking_description,process,reference_time,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",params![r.id,r.material_id,r.material_version_id,r.previous_version_id,r.copied_from_version_id,seq(&r.basis_count)?,output.map(|v|q(&v.quantity)).transpose()?,output.map(|v|&v.unit),output.map(|v|&v.input_quantity),output.map(|v|&v.input_unit),r.selling_price.as_deref().map(money).transpose()?,r.cooking_description,r.process,r.reference_time,r.recorded_at,seq(&r.seq)?]).map_err(unavailable)?;
    }
    for r in &state.recipe_lines {
        let line = &r.line;
        connection.execute("INSERT INTO recipe_lines(id,recipe_version_id,position,material_id,material_version_id,quantity_micros,unit,input_quantity,input_unit,note) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![line.id,r.recipe_version_id,nonnegative(&line.position)?,line.material_id,line.material_version_id,q(&line.quantity)?,line.unit,line.input_quantity,line.input_unit,line.note]).map_err(unavailable)?;
    }
    for r in &state.recipe_heads {
        connection
            .execute(
                "INSERT INTO recipe_heads(material_id,current_version_id) VALUES(?1,?2)",
                params![r.material_id, r.current_version_id],
            )
            .map_err(unavailable)?;
    }
    Ok(())
}
