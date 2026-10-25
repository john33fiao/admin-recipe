use super::super::{SnapshotStateV14, StoreError, unavailable};
use super::common::{json, seq};
use rusqlite::{Connection, params};

pub(super) fn insert(c: &Connection, s: &SnapshotStateV14) -> Result<(), StoreError> {
    for r in &s.receipt_reconciliations {
        c.execute("INSERT INTO receipt_reconciliations(id,receipt_id,confirmation_id,result_json,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6)",params![r.id,r.receipt_id,r.confirmation_id,json(r)?,r.recorded_at,seq(&r.seq)?]).map_err(unavailable)?;
    }
    for r in &s.inclusion_links {
        c.execute("INSERT INTO inclusion_links(id,reconciliation_id,selection_position,portion_position,row_json) VALUES(?1,?2,?3,?4,?5)",params![r.id,r.reconciliation_id,i64::try_from(r.selection_position).map_err(unavailable)?,i64::try_from(r.portion_position).map_err(unavailable)?,json(r)?]).map_err(unavailable)?;
    }
    for r in &s.inclusion_link_versions {
        c.execute("INSERT INTO inclusion_link_versions(id,link_id,previous_version_id,observation_version_id,row_json,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![r.id,r.link_id,r.previous_version_id,r.observation_version_id,json(r)?,r.recorded_at,seq(&r.seq)?]).map_err(unavailable)?;
    }
    for r in &s.inclusion_link_heads {
        c.execute(
            "INSERT INTO inclusion_link_heads(link_id,current_version_id) VALUES(?1,?2)",
            params![r.link_id, r.current_version_id],
        )
        .map_err(unavailable)?;
    }
    for r in &s.inclusion_reservations {
        c.execute(
            "INSERT INTO inclusion_reservations(id,link_version_id,row_json) VALUES(?1,?2,?3)",
            params![r.id, r.link_version_id, json(r)?],
        )
        .map_err(unavailable)?;
    }
    for r in &s.reconciliation_material_effects {
        c.execute("INSERT INTO reconciliation_material_effects(id,reconciliation_id,material_id,material_version_id,before_holding_version_id,row_json,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![r.id,r.reconciliation_id,r.effect.material_id,r.effect.material_version_id,r.effect.before.as_ref().and_then(|v|v.source_id.as_deref()),json(r)?,r.recorded_at,seq(&r.seq)?]).map_err(unavailable)?;
    }
    Ok(())
}
