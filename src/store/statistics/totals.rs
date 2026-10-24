use super::{broken, sequence};
use crate::{catalog::MaterialVersion, sale::SaleCount, statistics::*, stocktake::*};
use std::collections::BTreeMap;

pub(super) fn validate_allocation(
    allocation: &ConsumptionAllocation,
    effect: &StocktakeRowEffect,
) -> Result<(), StatisticsError> {
    let StocktakeCalculation::Settlement {
        consumption_quantity,
        consumption_value,
        ..
    } = &effect.calculation
    else {
        return Err(broken("시작 기준에는 소모 배분이 없습니다"));
    };
    if &allocation.total_quantity != consumption_quantity
        || &allocation.total_value != consumption_value
    {
        return Err(broken("실사 소모와 유효 배분의 합계가 다릅니다"));
    }
    let mut quantity = DecimalTotal::default();
    let mut value = DecimalTotal::default();
    quantity.add(&allocation.unallocated_quantity)?;
    value.add(&allocation.unallocated_value)?;
    for row in &allocation.candidates {
        match (&row.quantity, &row.value) {
            (Some(q), Some(v)) => {
                quantity.add(q)?;
                value.add(v)?;
            }
            (None, None) if allocation.reason == Some(NoAllocationReason::ZeroWeight) => {}
            _ => return Err(broken("판매별 배분의 미계산 상태가 올바르지 않습니다")),
        }
    }
    if quantity.decimal() != allocation.total_quantity || value.decimal() != allocation.total_value
    {
        return Err(broken("소모 총량은 배분과 미배분의 합이어야 합니다"));
    }
    Ok(())
}

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
