use super::super::{SnapshotStateV14, StoreError, unavailable};
use super::common::{json, money, q, seq, tag};
use rusqlite::{Connection, params};

pub(super) fn insert(connection: &Connection, state: &SnapshotStateV14) -> Result<(), StoreError> {
    for r in &state.business_records {
        connection.execute("INSERT INTO business_records(id,kind,created_at,created_sequence,corrects_confirmation_id) VALUES(?1,?2,?3,?4,?5)",params![r.id,tag(&r.kind)?,r.created_at,seq(&r.created_sequence)?,r.corrects_confirmation_id]).map_err(unavailable)?;
    }
    for r in &state.business_revisions {
        connection.execute("INSERT INTO business_revisions(id,record_id,previous_revision_id,status,payload_version,payload_json,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![r.id,r.record_id,r.previous_revision_id,tag(&r.status)?,r.payload.version(),json(&r.payload)?,r.recorded_at,seq(&r.seq)?]).map_err(unavailable)?;
    }
    for r in &state.business_heads {
        connection
            .execute(
                "INSERT INTO business_heads(record_id,current_revision_id) VALUES(?1,?2)",
                params![r.record_id, r.current_revision_id],
            )
            .map_err(unavailable)?;
    }
    for r in &state.business_confirmations {
        connection.execute("INSERT INTO business_confirmations(id,record_id,revision_id,request_id,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6)",params![r.id,r.record_id,r.revision_id,r.request_id,r.recorded_at,seq(&r.seq)?]).map_err(unavailable)?;
    }
    for r in &state.business_catalog_price_results {
        connection.execute("INSERT INTO business_catalog_price_results(confirmation_id,price_id) VALUES(?1,?2)",params![r.confirmation_id,r.price_id]).map_err(unavailable)?;
    }
    for r in &state.business_corrections {
        connection.execute("INSERT INTO business_corrections(target_confirmation_id,replacement_confirmation_id) VALUES(?1,?2)",params![r.target_confirmation_id,r.replacement_confirmation_id]).map_err(unavailable)?;
    }
    for r in &state.purchase_orders {
        connection.execute("INSERT INTO purchase_orders(id,first_confirmation_id,created_at,sequence) VALUES(?1,?2,?3,?4)",params![r.id,r.first_confirmation_id,r.created_at,seq(&r.seq)?]).map_err(unavailable)?;
    }
    for r in &state.purchase_order_versions {
        connection.execute("INSERT INTO purchase_order_versions(id,order_id,previous_version_id,confirmation_id,action,status,material_id,material_version_id,quantity_micros,unit,input_quantity,input_unit,supplier,expected_date,estimated_amount_micros,note,recorded_at,sequence,correction_projection_id) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19)",params![r.id,r.order_id,r.previous_version_id,r.confirmation_id,tag(&r.action)?,tag(&r.status)?,r.material_id,r.material_version_id,q(&r.quantity)?,r.unit,r.input_quantity,r.input_unit,r.supplier,r.expected_date,r.estimated_amount.as_deref().map(money).transpose()?,r.note,r.recorded_at,seq(&r.seq)?,r.correction_projection_id]).map_err(unavailable)?;
    }
    for r in &state.purchase_order_heads {
        connection
            .execute(
                "INSERT INTO purchase_order_heads(order_id,current_version_id) VALUES(?1,?2)",
                params![r.order_id, r.current_version_id],
            )
            .map_err(unavailable)?;
    }
    Ok(())
}
