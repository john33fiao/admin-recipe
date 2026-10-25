use super::super::{SnapshotStateV14, StoreError, unavailable};
use super::common::{money, q, seq};
use rusqlite::{Connection, params};

pub(super) fn insert(connection: &Connection, state: &SnapshotStateV14) -> Result<(), StoreError> {
    for r in &state.materials {
        connection
            .execute(
                "INSERT INTO materials(id,name,unit,created_at) VALUES(?1,?2,?3,?4)",
                params![r.id, r.name, r.unit, r.created_at],
            )
            .map_err(unavailable)?;
    }
    for r in &state.material_versions {
        connection.execute("INSERT INTO material_versions(id,material_id,name,unit,canonical_unit,dimension,kind,description,archived,recorded_at,sequence,source) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![r.id,r.material_id,r.name,r.unit,r.canonical_unit,r.dimension,r.kind,r.description,r.archived,r.recorded_at,r.seq.as_deref().map(seq).transpose()?,r.source]).map_err(unavailable)?;
    }
    for r in &state.material_heads {
        connection
            .execute(
                "INSERT INTO material_heads(material_id,current_version_id) VALUES(?1,?2)",
                params![r.material_id, r.current_version_id],
            )
            .map_err(unavailable)?;
    }
    for r in &state.material_prices {
        connection.execute("INSERT INTO material_prices(id,material_id,material_version_id,quantity_micros,amount_micros,unit,input_quantity,input_unit,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![r.id,r.material_id,r.material_version_id,q(&r.quantity)?,money(&r.amount)?,r.unit,r.input_quantity,r.input_unit,r.recorded_at,seq(&r.seq)?]).map_err(unavailable)?;
    }
    for r in &state.creation_requests {
        connection.execute("INSERT INTO creation_requests(request_id,name,unit,material_id) VALUES(?1,?2,?3,?4)",params![r.request_id,r.name,r.unit,r.material_id]).map_err(unavailable)?;
    }
    for r in &state.mutation_requests {
        connection.execute("INSERT INTO mutation_requests(request_id,operation,payload,response,epoch,sequence) VALUES(?1,?2,?3,?4,?5,?6)",params![r.request_id,r.operation,r.payload,r.response,r.epoch,seq(&r.sequence)?]).map_err(unavailable)?;
    }
    Ok(())
}
