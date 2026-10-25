use super::{broken, sequence};
use crate::{catalog::MaterialVersion, sale::SaleCount, statistics::*, stocktake::*};
use std::collections::BTreeMap;

pub(super) fn sales(rows: &[StatisticsSale]) -> Result<SalesTotals, StatisticsError> {
    let mut count = 0i128;
    let mut amount = DecimalTotal::default();
    let mut result = SalesTotals {
        active_records: 0,
        void_records: 0,
        count: "0".into(),
        known_revenue: "0".into(),
        missing_revenue_records: 0,
    };
    for row in rows {
        let Some(value) = &row.value else {
            result.void_records += 1;
            continue;
        };
        result.active_records += 1;
        count = count
            .checked_add(i128::from(
                SaleCount::parse(&value.count).map_err(broken)?.value(),
            ))
            .ok_or_else(|| broken("판매 개수 합계 범위를 넘습니다"))?;
        if let Some(value) = &value.amount {
            amount.add(value)?;
        } else {
            result.missing_revenue_records += 1;
        }
    }
    result.count = count.to_string();
    result.known_revenue = amount.decimal();
    Ok(result)
}

#[derive(Default)]
struct Accumulator {
    initial: usize,
    settlement: usize,
    void: usize,
    no_sales: usize,
    zero_weight: usize,
    q: DecimalTotal,
    v: DecimalTotal,
    aq: DecimalTotal,
    av: DecimalTotal,
    uq: DecimalTotal,
    uv: DecimalTotal,
}

pub(super) fn observations(
    rows: &[StatisticsObservation],
    materials: &[MaterialVersion],
) -> Result<Vec<MaterialConsumptionTotals>, StatisticsError> {
    let mut groups: BTreeMap<(&str, &str), Accumulator> = BTreeMap::new();
    for row in rows {
        let group = groups.entry((&row.material_id, &row.unit)).or_default();
        match row.status {
            StatisticsObservationStatus::Initial => group.initial += 1,
            StatisticsObservationStatus::Void => group.void += 1,
            StatisticsObservationStatus::Settlement => {
                group.settlement += 1;
                let allocation = row
                    .allocation
                    .as_ref()
                    .ok_or_else(|| broken("실사 소모 배분이 없습니다"))?;
                group.q.add(&allocation.total_quantity)?;
                group.v.add(&allocation.total_value)?;
                group.uq.add(&allocation.unallocated_quantity)?;
                group.uv.add(&allocation.unallocated_value)?;
                for candidate in &allocation.candidates {
                    if let (Some(q), Some(v)) = (&candidate.quantity, &candidate.value) {
                        group.aq.add(q)?;
                        group.av.add(v)?;
                    }
                }
                match allocation.reason {
                    Some(NoAllocationReason::NoSales) => group.no_sales += 1,
                    Some(NoAllocationReason::ZeroWeight) => group.zero_weight += 1,
                    None => {}
                }
            }
        }
    }
    let mut result = vec![];
    for ((material_id, unit), group) in groups {
        let mut latest = None;
        let mut last_sequence = -1;
        for material in materials.iter().filter(|m| m.material_id == material_id) {
            let seq = sequence(material.seq.as_deref().unwrap_or("0"))?;
            if seq > last_sequence {
                latest = Some(material);
                last_sequence = seq;
            }
        }
        let material = latest.ok_or_else(|| broken("소모 집계 품목이 없습니다"))?;
        result.push(MaterialConsumptionTotals {
            material_id: material_id.into(),
            material_name: material.name.clone(),
            label_material_version_id: material.id.clone(),
            unit: unit.into(),
            initial_records: group.initial,
            settlement_records: group.settlement,
            void_records: group.void,
            consumption_quantity: group.q.decimal(),
            consumption_value: group.v.decimal(),
            allocated_quantity: group.aq.decimal(),
            allocated_value: group.av.decimal(),
            unallocated_quantity: group.uq.decimal(),
            unallocated_value: group.uv.decimal(),
            no_sales_records: group.no_sales,
            zero_weight_records: group.zero_weight,
        });
    }
    Ok(result)
}
