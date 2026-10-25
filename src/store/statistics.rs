use super::{
    Store, StoreError,
    effective_read::{EffectiveObservation, EffectiveReadView, broken, sale_consumption, sequence},
};
use crate::{
    correction::{CorrectionEventValue, CorrectionSubjectKind},
    statistics::*,
};
use std::collections::HashMap;
mod totals;
impl From<StoreError> for StatisticsError {
    fn from(_error: StoreError) -> Self {
        Self::Unavailable("통계 자료를 읽을 수 없습니다".into())
    }
}
fn statistics_observations(
    view: &EffectiveReadView,
    observations: &[EffectiveObservation],
    window: &StatisticsWindow,
) -> Result<Vec<StatisticsObservation>, StatisticsError> {
    let sales = view
        .history
        .sales
        .iter()
        .map(|sale| (sale.id.as_str(), sale))
        .collect::<HashMap<_, _>>();
    observations
        .iter()
        .map(|o| {
            let mut candidate_links = vec![];
            if let Some(a) = &o.allocation {
                for candidate in &a.candidates {
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
            Ok(StatisticsObservation {
                origin: o.origin.clone(),
                observation_id: o.observation_id.clone(),
                material_id: o.material_id.clone(),
                material_version_id: o.material_version_id.clone(),
                material_name: o.material_name.clone(),
                unit: o.unit.clone(),
                position: o.position,
                effective_observation_version_id: o.effective_observation_version_id.clone(),
                tombstone_projection_id: o.tombstone_projection_id.clone(),
                changes: o.changes.clone(),
                status: o.status,
                effect: o.effect.clone(),
                consumption_version_id: o.consumption_version_id.clone(),
                allocation: o.allocation.clone(),
                candidate_links,
            })
        })
        .collect()
}
impl Store {
    pub fn statistics(&self, query: StatisticsQuery) -> Result<StatisticsReport, StatisticsError> {
        let window = StatisticsWindow::parse(&query)?;
        let view = EffectiveReadView::load(&self.connection, (self.today)())?;
        let state = view.state.clone();
        let selected_observations = view.observations()?;
        let all_observations = statistics_observations(&view, &selected_observations, &window)?;
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
                        let mut rows = sale_consumption(original, &selected_observations)?;
                        for row in &mut rows {
                            row.observation_in_period = row
                                .observation_recorded_at
                                .as_deref()
                                .map(|at| window.contains(at))
                                .transpose()?;
                        }
                        rows
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
