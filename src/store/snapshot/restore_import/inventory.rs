use super::super::{SnapshotStateV14, StoreError, unavailable};
use super::common::{bad, json, money, q, seq, tag};
use crate::{correction::CorrectionPlanRef, receipt::ReceiptSource};
use rusqlite::{Connection, params};

pub(super) fn insert(c: &Connection, s: &SnapshotStateV14) -> Result<(), StoreError> {
    for r in &s.receipts {
        let (kind, order, version) = match &r.source {
            ReceiptSource::Direct {} => ("direct", None, None),
            ReceiptSource::Order {
                order_id,
                ordered_version_id,
            } => ("order", Some(order_id), Some(ordered_version_id)),
        };
        c.execute("INSERT INTO receipts(id,confirmation_id,material_id,material_version_id,quantity_micros,unit,input_quantity,input_unit,amount_micros,receipt_date,note,recorded_at,sequence,source_kind,order_id,ordered_version_id) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",params![r.id,r.confirmation_id,r.material_id,r.material_version_id,q(&r.quantity)?,r.unit,r.input_quantity,r.input_unit,money(&r.amount)?,r.receipt_date,r.note,r.recorded_at,seq(&r.seq)?,kind,order,version]).map_err(unavailable)?;
    }
    for r in &s.order_receipt_closures {
        c.execute("INSERT INTO order_receipt_closures(order_id,receipt_id,confirmation_id,initial_version_id,ordered_version_id,received_version_id,received_quantity,difference) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![r.order_id,r.receipt_id,r.confirmation_id,r.initial_version_id,r.ordered_version_id,r.received_version_id,r.received_quantity,r.difference]).map_err(unavailable)?;
    }
    for r in &s.holding_versions {
        c.execute("INSERT INTO holding_versions(id,material_id,material_version_id,previous_version_id,receipt_id,baseline_receipt_id,baseline_sequence,quantity_micros,value_micros,recorded_at,sequence,production_effect_id,baseline_production_effect_id,observation_version_id,baseline_observation_version_id,waste_id,reconciliation_effect_id,baseline_reconciliation_effect_id,correction_projection_id,baseline_correction_projection_id,last_valid_correction_price_id,purchase_return_id) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)",params![r.id,r.material_id,r.material_version_id,r.previous_version_id,r.receipt_id,r.baseline_receipt_id,seq(&r.baseline_seq)?,q(&r.quantity)?,money(&r.value)?,r.recorded_at,seq(&r.seq)?,r.production_effect_id,r.baseline_production_effect_id,r.observation_version_id,r.baseline_observation_version_id,r.waste_id,r.reconciliation_effect_id,r.baseline_reconciliation_effect_id,r.correction_projection_id,r.baseline_correction_projection_id,r.last_valid_correction_price_id,r.purchase_return_id]).map_err(unavailable)?;
    }
    for r in &s.holding_heads {
        c.execute("INSERT INTO holding_heads(material_id,current_version_id,last_valid_version_id,unknown_correction_projection_id,last_valid_correction_price_id) VALUES(?1,?2,?3,?4,?5)",params![r.material_id,r.current_version_id,r.last_valid_version_id,r.unknown_correction_projection_id,r.last_valid_correction_price_id]).map_err(unavailable)?;
    }
    for r in &s.wastes {
        let mut e = r.effect.clone();
        e.after.source_id = None;
        c.execute("INSERT INTO wastes(id,confirmation_id,material_id,material_version_id,before_holding_version_id,quantity_micros,loss_value_micros,reason,note,effect_json,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![r.id,r.confirmation_id,e.material_id,e.material_version_id,e.before.source_id,q(&e.quantity)?,money(&e.loss_value)?,tag(&e.reason)?,r.note,json(&e)?,r.recorded_at,seq(&r.seq)?]).map_err(unavailable)?;
    }
    for r in &s.purchase_returns {
        let mut e = r.effect.clone();
        e.after.source_id = None;
        let (receipt, version) = match &e.source.reference {
            CorrectionPlanRef::Existing { id, version_id, .. } => (id, version_id),
            _ => return Err(bad("원반품에 실제 입고 참조가 필요합니다")),
        };
        c.execute("INSERT INTO purchase_returns(id,confirmation_id,receipt_id,receipt_version_id,material_id,material_version_id,before_holding_version_id,quantity_micros,deducted_value_micros,note,effect_json,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",params![r.id,r.confirmation_id,receipt,version,e.material_id,e.material_version_id,e.before.source_id,q(&e.quantity)?,money(&e.deducted_value)?,r.note,json(&e)?,r.recorded_at,seq(&r.seq)?]).map_err(unavailable)?;
    }
    Ok(())
}
