use super::{business, catalog, correction, stocktake};
use crate::{
    correction::{
        CorrectionEventValue, CorrectionHistory, CorrectionHistoryEvent,
        CorrectionReplacementDraft, CorrectionSubjectKind, ObservationCorrectionActionDraft,
    },
    domain::{Money, Quantity, Unit},
    receipt::ReceiptSourceDraft,
    statistics::*,
    stocktake::{
        ConsumptionAllocation, NoAllocationReason, StocktakeCalculation, StocktakeRowEffect,
    },
};
use chrono::NaiveDate;
use rusqlite::Connection;
use std::collections::{HashMap, HashSet};

pub(super) fn broken(message: impl std::fmt::Display) -> StatisticsError {
    StatisticsError::Unavailable(message.to_string())
}
pub(super) fn sequence(value: &str) -> Result<i64, StatisticsError> {
    let number = value.parse::<i64>().map_err(broken)?;
    if number < 0 || number.to_string() != value {
        return Err(broken("조회 근거 순번위치가 올바르지 않습니다"));
    }
    Ok(number)
}

pub(super) struct EffectiveObservation {
    pub origin: StatisticsOrigin,
    pub observation_id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub unit: String,
    pub position: usize,
    pub effective_observation_version_id: Option<String>,
    pub tombstone_projection_id: Option<String>,
    pub changes: Vec<StatisticsChangeRef>,
    pub status: StatisticsObservationStatus,
    pub effect: Option<StocktakeRowEffect>,
    pub consumption_version_id: Option<String>,
    pub allocation: Option<ConsumptionAllocation>,
}
pub(super) struct EffectiveReadView {
    pub state: crate::catalog::CatalogState,
    pub history: CorrectionHistory,
    pub stocktake: stocktake::SnapshotRows,
    pub corrections: correction::SnapshotRows,
    pub business: business::SnapshotRows,
}

impl EffectiveReadView {
    pub fn origin(
        &self,
        event: &CorrectionHistoryEvent,
    ) -> Result<StatisticsOrigin, StatisticsError> {
        let reference = &event.reference;
        let confirmation = self
            .business
            .confirmations
            .iter()
            .find(|row| row.id == reference.original_confirmation_id)
            .ok_or_else(|| broken("통계 원확정이 없습니다"))?;
        Ok(StatisticsOrigin {
            kind: reference.kind,
            id: reference.original_id.clone(),
            confirmation_id: confirmation.id.clone(),
            business_record_id: confirmation.record_id.clone(),
            recorded_at: confirmation.recorded_at.clone(),
            seq: confirmation.seq.clone(),
        })
    }

    pub fn latest_projection(
        &self,
        kind: &str,
        id: &str,
    ) -> Result<Option<&correction::ProjectionVersionRow>, StatisticsError> {
        let mut result = None;
        let mut latest = -1;
        for row in &self.corrections.projection_versions {
            if row.projection.kind() == kind && row.projection.subject_id() == id {
                let number = sequence(&row.seq)?;
                if number > latest && number <= self.history.as_of_revision {
                    result = Some(row);
                    latest = number;
                }
            }
        }
        Ok(result)
    }

    pub fn changes(
        &self,
        event: &CorrectionHistoryEvent,
        observation: Option<&str>,
    ) -> Result<Vec<StatisticsChangeRef>, StatisticsError> {
        let mut correction_ids = HashSet::new();
        let mut latest = None;
        let mut latest_sequence = -1;
        for row in &self.corrections.event_versions {
            if row.reference.kind == event.reference.kind
                && row.reference.original_id == event.reference.original_id
            {
                let number = sequence(&row.reference.seq)?;
                if number > latest_sequence && number <= self.history.as_of_revision {
                    latest = Some(row);
                    latest_sequence = number;
                }
            }
        }
        if let Some(row) = latest {
            correction_ids.insert(row.correction_id.as_str());
        }
        if let Some(id) = observation {
            for kind in ["observation", "allocation"] {
                if let Some(row) = self.latest_projection(kind, id)? {
                    correction_ids.insert(row.correction_id.as_str());
                }
            }
        }
        let mut result = vec![];
        for id in correction_ids {
            let record = self
                .corrections
                .records
                .iter()
                .find(|r| r.id == id)
                .ok_or_else(|| broken("통계 정정 원기록이 없습니다"))?;
            result.push(StatisticsChangeRef {
                kind: StatisticsChangeKind::Correction,
                id: record.id.clone(),
                business_record_id: record.business_record_id.clone(),
                confirmation_id: record.confirmation_id.clone(),
                recorded_at: record.recorded_at.clone(),
                seq: record.seq.clone(),
            });
        }
        if latest.is_none() && event.reference.kind == CorrectionSubjectKind::SaleCount {
            let version = self
                .history
                .sale_versions
                .iter()
                .find(|v| v.id == event.reference.version_id)
                .ok_or_else(|| broken("통계 판매 개정이 없습니다"))?;
            if version.previous_version_id.is_some() {
                result.push(StatisticsChangeRef {
                    kind: StatisticsChangeKind::LegacySaleRevision,
                    id: version.id.clone(),
                    business_record_id: version.business_record_id.clone(),
                    confirmation_id: version.confirmation_id.clone(),
                    recorded_at: version.recorded_at.clone(),
                    seq: version.seq.clone(),
                });
            }
        }
        result.sort_by(|a, b| {
            sequence(&a.seq)
                .unwrap_or(-1)
                .cmp(&sequence(&b.seq).unwrap_or(-1))
                .then(a.id.cmp(&b.id))
        });
        Ok(result)
    }

    pub fn observations(&self) -> Result<Vec<EffectiveObservation>, StatisticsError> {
        let allocations: HashMap<_, _> = self
            .history
            .actual_allocations
            .iter()
            .map(|a| (a.observation_id.as_str(), a))
            .collect();
        let mut rows = vec![];
        for event in self
            .history
            .events
            .iter()
            .filter(|e| e.reference.kind == CorrectionSubjectKind::Stocktake)
        {
            for observation in &event.observations {
                let original = self
                    .stocktake
                    .versions
                    .iter()
                    .find(|v| v.observation_id == observation.id && v.previous_version_id.is_none())
                    .ok_or_else(|| broken("통계 최초 관측이 없습니다"))?;
                let input_row = match &event.effective_input {
                    Some(CorrectionReplacementDraft::Stocktake { data }) => data
                        .rows
                        .iter()
                        .find(|r| r.observation_id.as_deref() == Some(&observation.id)),
                    None => None,
                    _ => return Err(broken("통계 실사의 유효 입력 종류가 다릅니다")),
                };
                let active = input_row.is_some_and(|r| {
                    matches!(r.action, ObservationCorrectionActionDraft::Replace { .. })
                });
                let projection = self.latest_projection("observation", &observation.id)?;
                if let Some(p) = projection {
                    let correction::Projection::Observation { change, .. } = &p.projection else {
                        unreachable!()
                    };
                    if active != change.after.is_some() {
                        return Err(broken("통계 유효 관측 상태가 정정 결과와 다릅니다"));
                    }
                }
                let mut effect = active.then(|| observation.effect.clone());
                let allocation = allocations.get(observation.id.as_str()).copied();
                let status = match effect.as_ref().map(|e| &e.calculation) {
                    None => StatisticsObservationStatus::Void,
                    Some(StocktakeCalculation::Initial { .. }) => {
                        StatisticsObservationStatus::Initial
                    }
                    Some(StocktakeCalculation::Settlement { .. }) => {
                        StatisticsObservationStatus::Settlement
                    }
                };
                if (status == StatisticsObservationStatus::Settlement) != allocation.is_some() {
                    return Err(broken("통계 유효 실사와 소모 개정의 연결이 다릅니다"));
                }
                if let Some(a) = allocation {
                    validate_allocation(&a.allocation, effect.as_ref().unwrap())?;
                    if let Some(crate::stocktake::StocktakeRowEffect {
                        calculation: StocktakeCalculation::Settlement { allocation, .. },
                        ..
                    }) = &mut effect
                    {
                        **allocation = a.allocation.clone();
                    }
                }
                let tombstone = if active {
                    None
                } else {
                    projection.map(|p| p.id.clone())
                };
                if !active && tombstone.is_none() {
                    return Err(broken("취소된 관측의 정정 근거가 없습니다"));
                }
                rows.push(EffectiveObservation {
                    origin: self.origin(event)?,
                    observation_id: observation.id.clone(),
                    material_id: original.effect.material_id.clone(),
                    material_version_id: original.effect.material_version_id.clone(),
                    material_name: original.effect.material_name.clone(),
                    unit: original.effect.unit.clone(),
                    position: original.effect.position,
                    effective_observation_version_id: active
                        .then(|| observation.version_id.clone()),
                    tombstone_projection_id: tombstone,
                    changes: self.changes(event, Some(&observation.id))?,
                    status,
                    effect,
                    consumption_version_id: allocation.map(|a| a.consumption_version_id.clone()),
                    allocation: allocation.map(|a| a.allocation.clone()),
                });
            }
        }
        Ok(rows)
    }

    pub fn receipt_basis(
        &self,
        event: &CorrectionHistoryEvent,
    ) -> Result<Option<StatisticsReceiptDisplay>, StatisticsError> {
        if event.reference.kind != CorrectionSubjectKind::Receipt {
            return Ok(None);
        }
        let original = self
            .history
            .receipts
            .iter()
            .find(|r| r.id == event.reference.original_id)
            .ok_or_else(|| broken("통계 원입고가 없습니다"))?;
        let original = StatisticsReceiptBasis {
            material_id: original.material_id.clone(),
            material_version_id: original.material_version_id.clone(),
            material_name: original.material_name.clone(),
            quantity: original.quantity.clone(),
            unit: original.unit.clone(),
            amount: original.amount.clone(),
            receipt_date: original.receipt_date.clone(),
        };
        let effective = match &event.effective_value {
            None => None,
            Some(CorrectionEventValue::Receipt { input, .. }) => {
                let (id, version) = match &input.source {
                    ReceiptSourceDraft::Direct {
                        material_id: Some(id),
                        material_version_id: Some(version),
                    } => (id.as_str(), version.as_str()),
                    ReceiptSourceDraft::Order {
                        order_id: Some(order),
                        material_version_id: Some(version),
                        ..
                    } => {
                        let order = self
                            .history
                            .orders
                            .iter()
                            .find(|o| &o.order_id == order)
                            .ok_or_else(|| broken("통계 입고 주문이 없습니다"))?;
                        (order.material_id.as_str(), version.as_str())
                    }
                    _ => return Err(broken("통계 입고 품목 참조가 없습니다")),
                };
                let material = self
                    .history
                    .materials
                    .iter()
                    .find(|m| m.material_id == id && m.id == version)
                    .ok_or_else(|| broken("통계 입고 품목 버전이 없습니다"))?;
                let unit = Unit::parse(&input.unit).map_err(broken)?;
                if !unit.same_dimension(&Unit::parse(&material.unit).map_err(broken)?) {
                    return Err(broken("통계 입고 단위가 다릅니다"));
                }
                Some(StatisticsReceiptBasis {
                    material_id: material.material_id.clone(),
                    material_version_id: material.id.clone(),
                    material_name: material.name.clone(),
                    quantity: unit
                        .to_canonical(Quantity::parse(&input.quantity).map_err(broken)?)
                        .map_err(broken)?
                        .decimal(),
                    unit: material.canonical_unit.clone(),
                    amount: Money::parse(&input.amount).map_err(broken)?.decimal(),
                    receipt_date: input.receipt_date.clone(),
                })
            }
            _ => return Err(broken("통계 입고의 유효 종류가 다릅니다")),
        };
        Ok(Some(StatisticsReceiptDisplay {
            original,
            effective,
        }))
    }
}

pub(super) fn sale_consumption(
    original: &crate::sale::SaleOriginal,
    observations: &[EffectiveObservation],
) -> Result<Vec<StatisticsSaleConsumption>, StatisticsError> {
    let mut result = vec![];
    let mut seen = HashSet::new();
    for direct in &original.recipe_cost.rows {
        if !seen.insert(direct.material_id.as_str()) {
            continue;
        }
        let mut linked = None;
        for observation in observations
            .iter()
            .filter(|o| o.material_id == direct.material_id)
        {
            if let Some(candidate) = observation
                .allocation
                .as_ref()
                .and_then(|a| a.candidates.iter().find(|c| c.sale_id == original.id))
            {
                if linked.is_some() {
                    return Err(broken(
                        "판매가 같은 품목의 여러 유효 소모에 중복 연결됐습니다",
                    ));
                }
                linked = Some((observation, candidate));
            }
        }
        let mut row = StatisticsSaleConsumption {
            material_id: direct.material_id.clone(),
            material_name: direct.material_name.clone(),
            unit: direct.unit.clone(),
            status: StatisticsConsumptionStatus::Unconfirmed,
            observation_id: None,
            observation_version_id: None,
            stocktake_id: None,
            consumption_version_id: None,
            sale_version_id: None,
            quantity: None,
            value: None,
            reason: Some(StatisticsConsumptionReason::NoEffectiveAllocation),
            observation_recorded_at: None,
            observation_in_period: None,
        };
        let anchor = if let Some((observation, candidate)) = linked {
            row.status = if candidate.quantity.is_some() {
                StatisticsConsumptionStatus::Allocated
            } else {
                StatisticsConsumptionStatus::Unallocated
            };
            row.consumption_version_id = observation.consumption_version_id.clone();
            row.sale_version_id = Some(candidate.sale_version_id.clone());
            row.quantity = candidate.quantity.clone();
            row.value = candidate.value.clone();
            row.reason = candidate
                .quantity
                .is_none()
                .then_some(StatisticsConsumptionReason::ZeroWeight);
            Some(observation)
        } else {
            let sale_seq = sequence(&original.seq)?;
            let mut initial = None;
            for observation in observations.iter().filter(|o| {
                o.material_id == direct.material_id
                    && o.status == StatisticsObservationStatus::Initial
            }) {
                if sequence(&observation.origin.seq)? > sale_seq {
                    initial = Some(observation);
                    break;
                }
            }
            if initial.is_some() {
                row.reason = Some(StatisticsConsumptionReason::InitialBasisMissing);
            }
            initial
        };
        if let Some(observation) = anchor {
            row.observation_id = Some(observation.observation_id.clone());
            row.observation_version_id = observation.effective_observation_version_id.clone();
            row.stocktake_id = Some(observation.origin.id.clone());
            row.observation_recorded_at = Some(observation.origin.recorded_at.clone());
        }
        result.push(row);
    }
    Ok(result)
}

impl EffectiveReadView {
    pub fn load(connection: &Connection, today: NaiveDate) -> Result<Self, StatisticsError> {
        let state = catalog::state(connection)?;
        let view = Self {
            state,
            history: correction::effective_history(connection, today)?,
            stocktake: stocktake::snapshot_rows(connection)?,
            corrections: correction::snapshot_rows(connection)?,
            business: business::snapshot_rows(connection)?,
        };
        if sequence(&view.state.revision)? != view.history.as_of_revision {
            return Err(broken("조회 기준이 달라졌습니다"));
        }
        Ok(view)
    }
}
fn validate_allocation(
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
