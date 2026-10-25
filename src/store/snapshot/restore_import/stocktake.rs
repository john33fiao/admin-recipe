use super::super::{SnapshotStateV14, StoreError, unavailable};
use super::common::{json, seq};
use rusqlite::{Connection, params};

pub(super) fn insert(c: &Connection, s: &SnapshotStateV14) -> Result<(), StoreError> {
    for r in &s.stocktake_batches {
        c.execute("INSERT INTO stocktake_batches(id,confirmation_id,note,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5)",params![r.id,r.confirmation_id,r.note,r.recorded_at,seq(&r.seq)?]).map_err(unavailable)?;
    }
    for r in &s.stocktake_observations {
        c.execute("INSERT INTO stocktake_observations(id,stocktake_id,position,material_id) VALUES(?1,?2,?3,?4)",params![r.id,r.stocktake_id,i64::try_from(r.position).map_err(unavailable)?,r.material_id]).map_err(unavailable)?;
    }
    for r in &s.stocktake_versions {
        let mut e = r.effect.clone();
        e.after.source_id = None;
        c.execute("INSERT INTO stocktake_versions(id,observation_id,previous_version_id,previous_observation_version_id,before_holding_version_id,effect_json,recorded_at,sequence,correction_projection_id,logical_sequence,observed_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![r.id,r.observation_id,r.previous_version_id,r.previous_observation_version_id,e.before.as_ref().and_then(|b|b.source_id.as_deref()),json(&e)?,r.recorded_at,seq(&r.seq)?,r.correction_projection_id,r.logical_seq.as_deref().map(seq).transpose()?,r.observed_at]).map_err(unavailable)?;
    }
    for r in &s.stocktake_heads {
        c.execute(
            "INSERT INTO stocktake_heads(observation_id,current_version_id) VALUES(?1,?2)",
            params![r.observation_id, r.current_version_id],
        )
        .map_err(unavailable)?;
    }
    for r in &s.consumption_versions {
        c.execute("INSERT INTO consumption_versions(id,observation_version_id,previous_version_id,allocation_json,correction_projection_id,sequence) VALUES(?1,?2,?3,?4,?5,?6)",params![r.id,r.observation_version_id,r.previous_version_id,json(r)?,r.correction_projection_id,r.seq.as_deref().map(seq).transpose()?]).map_err(unavailable)?;
    }
    for r in &s.consumption_heads {
        c.execute(
            "INSERT INTO consumption_heads(observation_id,current_version_id) VALUES(?1,?2)",
            params![r.observation_id, r.current_version_id],
        )
        .map_err(unavailable)?;
    }
    for r in &s.consumption_rows {
        let row = &r.row;
        c.execute("INSERT INTO consumption_rows(consumption_version_id,position,sale_id,sale_version_id,recipe_version_id,row_json) VALUES(?1,?2,?3,?4,?5,?6)",params![r.consumption_version_id,i64::try_from(row.position).map_err(unavailable)?,row.sale_id,row.sale_version_id,row.recipe_version_id,json(row)?]).map_err(unavailable)?;
    }
    Ok(())
}
