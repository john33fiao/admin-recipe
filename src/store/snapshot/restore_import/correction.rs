use super::super::{SnapshotStateV14, StoreError, unavailable};
use super::common::{json, money, q, seq, tag};
use rusqlite::{Connection, params};

pub(super) fn insert(c: &Connection, s: &SnapshotStateV14) -> Result<(), StoreError> {
    for r in &s.correction_records {
        c.execute("INSERT INTO correction_records(id,original_id,target_confirmation_id,previous_version_id,confirmation_id,business_record_id,recorded_at,sequence,record_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![r.id,r.original_id,r.target_confirmation_id,r.previous_version_id,r.confirmation_id,r.business_record_id,r.recorded_at,seq(&r.seq)?,json(r)?]).map_err(unavailable)?;
    }
    for r in &s.correction_event_versions {
        c.execute("INSERT INTO correction_event_versions(id,correction_id,kind,original_id,original_confirmation_id,previous_version_id,previous_original_version_id,logical_sequence,logical_position,recorded_at,sequence,row_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![r.id,r.correction_id,tag(&r.reference.kind)?,r.reference.original_id,r.reference.original_confirmation_id,r.previous_version_id,r.previous_original_version_id,seq(&r.reference.logical_seq)?,i64::try_from(r.reference.logical_position).map_err(unavailable)?,r.reference.recorded_at,seq(&r.reference.seq)?,json(r)?]).map_err(unavailable)?;
    }
    for r in &s.correction_event_heads {
        c.execute("INSERT INTO correction_event_heads(kind,original_id,current_version_id) VALUES(?1,?2,?3)",params![tag(&r.kind)?,r.original_id,r.current_version_id]).map_err(unavailable)?;
    }
    for r in &s.correction_projection_versions {
        c.execute("INSERT INTO correction_projection_versions(id,correction_id,kind,subject_id,previous_version_id,recorded_at,sequence,row_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![r.id,r.correction_id,r.projection.kind(),r.projection.subject_id(),r.previous_version_id,r.recorded_at,seq(&r.seq)?,json(r)?]).map_err(unavailable)?;
    }
    for r in &s.correction_projection_heads {
        c.execute("INSERT INTO correction_projection_heads(kind,subject_id,current_version_id) VALUES(?1,?2,?3)",params![r.kind.text(),r.subject_id,r.current_version_id]).map_err(unavailable)?;
    }
    for r in &s.correction_price_bases {
        c.execute("INSERT INTO correction_price_bases(id,correction_id,material_id,material_version_id,amount_micros,quantity_micros,logical_sequence,recorded_at,sequence,row_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![r.id,r.correction_id,r.material_id,r.material_version_id,money(&r.amount)?,q(&r.quantity)?,seq(&r.logical_seq)?,r.recorded_at,seq(&r.seq)?,json(r)?]).map_err(unavailable)?;
    }
    Ok(())
}
