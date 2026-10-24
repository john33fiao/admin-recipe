use super::{Store, StoreError, business, catalog, correction, stocktake};
use crate::{
    correction::{
        CorrectionEventValue, CorrectionHistory, CorrectionHistoryEvent,
        CorrectionReplacementDraft, CorrectionSubjectKind, ObservationCorrectionActionDraft,
    },
    domain::{Money, Quantity, Unit},
    receipt::ReceiptSourceDraft,
    statistics::*,
    stocktake::StocktakeCalculation,
};
use std::collections::{HashMap, HashSet};

mod totals;

impl From<StoreError> for StatisticsError {
    fn from(_error: StoreError) -> Self {
        Self::Unavailable("통계 자료를 읽을 수 없습니다".into())
    }
}
fn broken(message: impl std::fmt::Display) -> StatisticsError {
    StatisticsError::Unavailable(message.to_string())
}
fn sequence(value: &str) -> Result<i64, StatisticsError> {
    let number = value.parse::<i64>().map_err(broken)?;
    if number < 0 || number.to_string() != value {
        return Err(broken("통계 근거 순번이 올바르지 않습니다"));
    }
    Ok(number)
}

struct ReadView {
    history: CorrectionHistory,
    stocktake: stocktake::SnapshotRows,
    corrections: correction::SnapshotRows,
    business: business::SnapshotRows,
}

impl ReadView {
    fn origin(&self, event: &CorrectionHistoryEvent) -> Result<StatisticsOrigin, StatisticsError> {
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

    fn latest_projection(
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

    fn changes(
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

    fn observations(
        &self,
        window: &StatisticsWindow,
    ) -> Result<Vec<StatisticsObservation>, StatisticsError> {
        let allocations: HashMap<_, _> = self
            .history
            .actual_allocations
            .iter()
            .map(|a| (a.observation_id.as_str(), a))
            .collect();
        let sales: HashMap<_, _> = self
            .history
            .sales
            .iter()
            .map(|s| (s.id.as_str(), s))
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
                let mut candidate_links = vec![];
                if let Some(a) = allocation {
                    totals::validate_allocation(&a.allocation, effect.as_ref().unwrap())?;
                    if let Some(crate::stocktake::StocktakeRowEffect {
                        calculation: StocktakeCalculation::Settlement { allocation, .. },
                        ..
                    }) = &mut effect
                    {
                        **allocation = a.allocation.clone();
                    }
                    for candidate in &a.allocation.candidates {
                        let sale = sales
                            .get(candidate.sale_id.as_str())
                            .ok_or_else(|| broken("통계 소모 후보의 원판매가 없습니다"))?;
                        candidate_links.push(StatisticsCandidateLink {
                            position: candidate.position,
                            sale_id: sale.id.clone(),
                            business_record_id: sale.business_record_id.clone(),
                            original_sale_recorded_at: sale.recorded_at.clone(),
                            sale_in_period: window.contains(&sale.recorded_at)?,
                        });
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
                rows.push(StatisticsObservation {
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
                    candidate_links,
                });
            }
        }
        Ok(rows)
    }

    fn receipt_basis(
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

fn sale_consumption(
    original: &crate::sale::SaleOriginal,
    observations: &[StatisticsObservation],
    window: &StatisticsWindow,
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
            row.observation_in_period = Some(window.contains(&observation.origin.recorded_at)?);
        }
        result.push(row);
    }
    Ok(result)
}

impl Store {
    pub fn statistics(&self, query: StatisticsQuery) -> Result<StatisticsReport, StatisticsError> {
        let window = StatisticsWindow::parse(&query)?;
        let state = catalog::state(&self.connection)?;
        let view = ReadView {
            history: correction::effective_history(&self.connection, (self.today)())?,
            stocktake: stocktake::snapshot_rows(&self.connection)?,
            corrections: correction::snapshot_rows(&self.connection)?,
            business: business::snapshot_rows(&self.connection)?,
        };
        if sequence(&state.revision)? != view.history.as_of_revision {
            return Err(broken("통계 조회 기준이 달라졌습니다"));
        }
        let all_observations = view.observations(&window)?;
        let mut sales = vec![];
        let mut operations = vec![];
        let mut all_sales = 0;
        let mut all_operations = 0;
        for event in &view.history.events {
            let origin = view.origin(event)?;
            let selected = window.contains(&origin.recorded_at)?;
            let status = if event.effective_value.is_some() {
                StatisticsStatus::Active
            } else {
                StatisticsStatus::Void
            };
            if event.reference.kind == CorrectionSubjectKind::SaleCount {
                all_sales += 1;
                if !selected {
                    continue;
                }
                let original = view
                    .history
                    .sales
                    .iter()
                    .find(|s| s.id == event.reference.original_id)
                    .ok_or_else(|| broken("통계 원판매가 없습니다"))?;
                let value = match &event.effective_value {
                    None => None,
                    Some(CorrectionEventValue::SaleCount { result, .. }) => {
                        Some(StatisticsSaleValue {
                            count: result.count.clone(),
                            amount: result.amount.clone(),
                        })
                    }
                    _ => return Err(broken("통계 판매의 유효 종류가 다릅니다")),
                };
                sales.push(StatisticsSale {
                    origin,
                    effective_ref: event.reference.clone(),
                    changes: view.changes(event, None)?,
                    status,
                    original: original.clone(),
                    value,
                    consumption: if status == StatisticsStatus::Active {
                        sale_consumption(original, &all_observations, &window)?
                    } else {
                        vec![]
                    },
                });
            } else {
                all_operations += 1;
                if !selected {
                    continue;
                }
                operations.push(StatisticsOperation {
                    origin,
                    effective_ref: event.reference.clone(),
                    changes: view.changes(event, None)?,
                    status,
                    value: event.effective_value.clone(),
                    receipt_basis: view.receipt_basis(event)?,
                    observation_ids: event.observations.iter().map(|o| o.id.clone()).collect(),
                });
            }
        }
        let all_observation_count = all_observations.len();
        let observations = all_observations
            .into_iter()
            .filter_map(|o| match window.contains(&o.origin.recorded_at) {
                Ok(true) => Some(Ok(o)),
                Ok(false) => None,
                Err(error) => Some(Err(error)),
            })
            .collect::<Result<Vec<_>, _>>()?;
        let report = StatisticsReport {
            range: window.range(),
            state,
            sales: StatisticsSales {
                coverage: StatisticsCoverage {
                    matching_records: sales.len(),
                    all_time_records: all_sales,
                },
                summary: totals::sales(&sales)?,
                rows: sales,
            },
            observations: StatisticsObservations {
                coverage: StatisticsCoverage {
                    matching_records: observations.len(),
                    all_time_records: all_observation_count,
                },
                summary_by_material: totals::observations(&observations, &view.history.materials)?,
                rows: observations,
            },
            operations: StatisticsOperations {
                coverage: StatisticsCoverage {
                    matching_records: operations.len(),
                    all_time_records: all_operations,
                },
                rows: operations,
            },
        };
        enforce_report_limit(&report)?;
        Ok(report)
    }
}
