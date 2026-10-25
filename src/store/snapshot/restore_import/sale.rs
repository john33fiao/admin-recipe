use super::super::{SnapshotStateV14, StoreError, unavailable};
use super::common::{json, money, nonnegative, q, tag};
use crate::sale::SalePriceSource;
use rusqlite::{Connection, params};

pub(super) fn insert(c: &Connection, s: &SnapshotStateV14) -> Result<(), StoreError> {
    for r in &s.sales {
        c.execute("INSERT INTO sales(id,original_confirmation_id,material_id,material_version_id,recipe_version_id,basis_count,selling_price_micros,known_subtotal_micros,total_cost_micros,ratio_json,ratio_reason_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![r.id,r.original_confirmation_id,r.material_id,r.material_version_id,r.recipe_version_id,nonnegative(&r.basis_count)?,r.selling_price.as_deref().map(money).transpose()?,money(&r.known_subtotal)?,r.total_cost.as_deref().map(money).transpose()?,json(&r.cost_ratio)?,json(&r.ratio_unavailable_reason)?]).map_err(unavailable)?;
    }
    for r in &s.sale_versions {
        c.execute("INSERT INTO sale_versions(id,sale_id,previous_version_id,confirmation_id,count,amount_micros,note) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![r.id,r.sale_id,r.previous_version_id,r.confirmation_id,nonnegative(&r.count)?,r.amount.as_deref().map(money).transpose()?,r.note]).map_err(unavailable)?;
    }
    for r in &s.sale_heads {
        c.execute(
            "INSERT INTO sale_heads(sale_id,current_version_id) VALUES(?1,?2)",
            params![r.sale_id, r.current_version_id],
        )
        .map_err(unavailable)?;
    }
    for r in &s.sale_cost_rows {
        let row = &r.row;
        let basis = row
            .price
            .source_id
            .as_deref()
            .filter(|id| s.correction_price_bases.iter().any(|b| b.id == *id));
        let holding = if matches!(
            row.price.source,
            SalePriceSource::HoldingAverage | SalePriceSource::LastValid
        ) && basis.is_none()
        {
            row.price.source_id.as_deref()
        } else {
            None
        };
        let preregistered = (row.price.source == SalePriceSource::Preregistered)
            .then_some(row.price.source_id.as_deref())
            .flatten();
        c.execute("INSERT INTO sale_cost_rows(sale_id,position,recipe_line_id,source_kind,holding_version_id,material_price_id,price_amount_micros,price_quantity_micros,missing_reason,cost_micros,correction_price_basis_id) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![r.sale_id,nonnegative(&r.position)?,row.line_id,tag(&row.price.source)?,holding,preregistered,row.price.amount.as_deref().map(money).transpose()?,row.price.quantity.as_deref().map(q).transpose()?,row.price.reason,row.cost.as_deref().map(money).transpose()?,basis]).map_err(unavailable)?;
    }
    Ok(())
}
