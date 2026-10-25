use super::super::{SnapshotStateV14, StoreError, unavailable};
use super::common::{money, nonnegative, q, seq};
use rusqlite::{Connection, params};

pub(super) fn insert(c: &Connection, s: &SnapshotStateV14) -> Result<(), StoreError> {
    for r in &s.productions {
        c.execute("INSERT INTO productions(id,confirmation_id,note,total_input_value_micros,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6)",params![r.id,r.confirmation_id,r.note,money(&r.total_input_value)?,r.recorded_at,seq(&r.seq)?]).map_err(unavailable)?;
    }
    for r in &s.production_effects {
        c.execute("INSERT INTO production_effects(id,production_id,material_id,material_version_id,input_quantity_micros,input_value_micros,output_quantity_micros,output_value_micros) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![r.id,r.production_id,r.material_id,r.material_version_id,q(&r.input_quantity)?,money(&r.input_value)?,q(&r.output_quantity)?,money(&r.output_value)?]).map_err(unavailable)?;
    }
    for r in &s.production_inputs {
        let row = &r.row;
        c.execute("INSERT INTO production_inputs(production_id,position,effect_id,material_id,material_version_id,quantity_micros,unit,input_quantity,input_unit,value_micros) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![r.production_id,nonnegative(&row.position)?,r.effect_id,row.material_id,row.material_version_id,q(&row.quantity)?,row.unit,row.input_quantity,row.input_unit,money(&row.value)?]).map_err(unavailable)?;
    }
    for r in &s.production_outputs {
        let row = &r.row;
        c.execute("INSERT INTO production_outputs(production_id,position,effect_id,material_id,material_version_id,quantity_micros,unit,input_quantity,input_unit,value_micros,allocation_micros) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![r.production_id,nonnegative(&row.position)?,r.effect_id,row.material_id,row.material_version_id,q(&row.quantity)?,row.unit,row.input_quantity,row.input_unit,money(&row.value)?,crate::production::AllocationPercent::parse(&row.allocation_percent).map_err(unavailable)?.micros()]).map_err(unavailable)?;
    }
    Ok(())
}
