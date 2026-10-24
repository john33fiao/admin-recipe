use super::*;
use crate::store::{
    business, catalog, holding, production, purchase_order, purchase_return, receipt,
    reconciliation, sale, stocktake, waste,
};
use crate::{
    business::BusinessPayload,
    holding::HoldingBasisKind,
    production::{ProductionMaterialEffect, ProductionPreview},
    purchase_order::{OrderReceiptClosure, OrderStatus},
    sale::{SaleOriginal, SaleProposal},
    stocktake::{ConsumptionAllocation, StocktakeObservationRecord},
};
use chrono::NaiveDate;
use uuid::Uuid;

fn number(value: &str) -> Result<i64, StoreError> {
    let n = value.parse::<i64>().map_err(unavailable)?;
    if n < 0 || n.to_string() != value {
        return Err(unavailable("정정 이력 순번이 올바르지 않습니다"));
    }
    Ok(n)
}
fn latest_projection<'a>(
    s: &'a Validation<'_>,
    kind: &str,
    id: &str,
    at: i64,
) -> Option<&'a ProjectionVersionRow> {
    s.projection_versions
        .iter()
        .filter(|v| {
            v.projection.kind() == kind
                && v.projection.subject_id() == id
                && v.seq.parse::<i64>().is_ok_and(|n| n <= at)
        })
        .max_by_key(|v| v.seq.parse::<i64>().unwrap_or(-1))
}
fn resolved<T: Serialize + DeserializeOwned>(
    s: &Validation<'_>,
    correction_id: &str,
    value: &T,
) -> Result<T, StoreError> {
    let r = s
        .records
        .iter()
        .find(|r| r.id == correction_id)
        .ok_or_else(|| unavailable("정정 참조 원기록이 없습니다"))?;
    super::write::resolve_value(value, &r.resolved_refs)
}
fn input(value: &CorrectionEventValue) -> CorrectionReplacementDraft {
    match value {
        CorrectionEventValue::Receipt { input, .. } => CorrectionReplacementDraft::Receipt {
            data: input.clone(),
        },
        CorrectionEventValue::Production { input, .. } => CorrectionReplacementDraft::Production {
            data: input.clone(),
        },
        CorrectionEventValue::SaleCount { input, .. } => CorrectionReplacementDraft::SaleCount {
            data: input.clone(),
        },
        CorrectionEventValue::Waste { input, .. } => CorrectionReplacementDraft::Waste {
            data: input.clone(),
        },
        CorrectionEventValue::PurchaseReturn { input, .. } => {
            CorrectionReplacementDraft::PurchaseReturn {
                data: input.clone(),
            }
        }
        CorrectionEventValue::Stocktake { input } => CorrectionReplacementDraft::Stocktake {
            data: input.clone(),
        },
    }
}
fn reference(
    s: &Validation<'_>,
    confirmation: &str,
    kind: CorrectionSubjectKind,
    id: &str,
) -> Result<CorrectionEventRef, StoreError> {
    let c = s
        .confirmations
        .iter()
        .find(|c| c.id == confirmation)
        .ok_or_else(|| unavailable("정정 원확정이 없습니다"))?;
    Ok(CorrectionEventRef {
        kind,
        original_id: id.into(),
        original_confirmation_id: confirmation.into(),
        confirmation_id: confirmation.into(),
        version_id: id.into(),
        business_record_id: c.record_id.clone(),
        logical_seq: c.seq.clone(),
        logical_position: 0,
        original_recorded_at: c.recorded_at.clone(),
        recorded_at: c.recorded_at.clone(),
        seq: c.seq.clone(),
    })
}
fn payload<'a>(
    s: &'a Validation<'_>,
    confirmation: &str,
) -> Result<&'a BusinessPayload, StoreError> {
    let c = s
        .confirmations
        .iter()
        .find(|c| c.id == confirmation)
        .ok_or_else(|| unavailable("정정 원확정이 없습니다"))?;
    s.revisions
        .iter()
        .find(|r| r.id == c.revision_id && r.record_id == c.record_id)
        .map(|r| &r.payload)
        .ok_or_else(|| unavailable("정정 원입력이 없습니다"))
}
fn original_observations(
    s: &Validation<'_>,
    batch: &str,
) -> Result<Vec<StocktakeObservationRecord>, StoreError> {
    let mut result = vec![];
    for o in s.observations.iter().filter(|o| o.stocktake_id == batch) {
        let v = s
            .observation_versions
            .iter()
            .filter(|v| v.observation_id == o.id && v.previous_version_id.is_none())
            .min_by_key(|v| v.seq.parse::<i64>().unwrap_or(i64::MAX))
            .ok_or_else(|| unavailable("원관측 개정이 없습니다"))?;
        result.push(StocktakeObservationRecord {
            id: o.id.clone(),
            version_id: v.id.clone(),
            previous_version_id: None,
            stocktake_id: batch.into(),
            previous_observation_version_id: v.previous_observation_version_id.clone(),
            consumption_version_id: s
                .consumption_versions
                .iter()
                .find(|c| {
                    c.observation_version_id == v.id
                        && c.previous_version_id.is_none()
                        && c.correction_projection_id.is_none()
                })
                .map(|c| c.id.clone()),
            effect: v.effect.clone(),
            recorded_at: v.recorded_at.clone(),
            seq: v.seq.clone(),
        });
    }
    result.sort_by_key(|o| o.effect.position);
    Ok(result)
}
fn production_value(
    s: &Validation<'_>,
    id: &str,
    data: &crate::production::DraftProductionV1,
) -> Result<CorrectionEventValue, StoreError> {
    let p = s
        .productions
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| unavailable("원생산이 없습니다"))?;
    let mut materials = vec![];
    for e in s
        .production_effects
        .iter()
        .filter(|e| e.production_id == id)
    {
        let after = s
            .holdings
            .iter()
            .find(|h| h.production_effect_id.as_ref() == Some(&e.id))
            .ok_or_else(|| unavailable("원생산의 보유 결과가 없습니다"))?;
        let before = after
            .previous_version_id
            .as_ref()
            .map(|id| {
                s.holdings
                    .iter()
                    .find(|h| &h.id == id)
                    .ok_or_else(|| unavailable("생산 이전 보유가 없습니다"))
            })
            .transpose()?;
        let m = s
            .materials
            .iter()
            .find(|m| m.id == e.material_version_id)
            .ok_or_else(|| unavailable("원생산 품목 버전이 없습니다"))?;
        materials.push(ProductionMaterialEffect {
            material_id: e.material_id.clone(),
            material_version_id: e.material_version_id.clone(),
            unit: m.canonical_unit.clone(),
            before: before.map(holding::amounts),
            after: holding::amounts(after),
            basis_before: before.map(holding::basis).transpose()?,
            observation: holding::input_from_history(s.holdings, s.observation_versions, before)?
                .observation,
            input_quantity: e.input_quantity.clone(),
            input_value: e.input_value.clone(),
            output_quantity: e.output_quantity.clone(),
            output_value: e.output_value.clone(),
        });
    }
    Ok(CorrectionEventValue::Production {
        input: data.clone(),
        result: ProductionPreview {
            inputs: s
                .production_inputs
                .iter()
                .filter(|r| r.production_id == id)
                .map(|r| r.row.clone())
                .collect(),
            outputs: s
                .production_outputs
                .iter()
                .filter(|r| r.production_id == id)
                .map(|r| r.row.clone())
                .collect(),
            materials,
            total_input_value: p.total_input_value.clone(),
        },
    })
}
fn sale_value(original: &SaleOriginal, version: &crate::sale::SaleVersion) -> CorrectionEventValue {
    CorrectionEventValue::SaleCount {
        input: DraftSaleCountCorrectionV1 {
            count: version.count.clone(),
        },
        result: SaleProposal {
            material_id: original.material_id.clone(),
            material_version_id: original.material_version_id.clone(),
            material_name: original.material_name.clone(),
            recipe_version_id: original.recipe_version_id.clone(),
            count: version.count.clone(),
            amount: version.amount.clone(),
            recipe_cost: original.recipe_cost.clone(),
        },
    }
}
pub(in crate::store) fn from_snapshot(
    s: &Validation<'_>,
    at: i64,
    today: NaiveDate,
) -> Result<CorrectionHistory, StoreError> {
    let mut events = vec![];
    for row in s.receipts {
        if number(&row.seq)? > at {
            continue;
        }
        let BusinessPayload::Receipt { data, .. } = payload(s, &row.confirmation_id)? else {
            return Err(unavailable("원입고 입력 종류 불일치"));
        };
        let value = CorrectionEventValue::Receipt {
            input: data.clone(),
            reconciliation: s
                .reconciliations
                .iter()
                .find(|r| r.receipt_id == row.id)
                .map(|r| r.effect.clone()),
        };
        events.push(CorrectionHistoryEvent {
            reference: reference(
                s,
                &row.confirmation_id,
                CorrectionSubjectKind::Receipt,
                &row.id,
            )?,
            original_input: input(&value),
            effective_input: Some(input(&value)),
            effective_value: Some(value),
            observations: vec![],
        });
    }
    for row in s.productions {
        if number(&row.seq)? > at {
            continue;
        }
        let BusinessPayload::Production { data, .. } = payload(s, &row.confirmation_id)? else {
            return Err(unavailable("원생산 입력 종류 불일치"));
        };
        let value = production_value(s, &row.id, data)?;
        events.push(CorrectionHistoryEvent {
            reference: reference(
                s,
                &row.confirmation_id,
                CorrectionSubjectKind::Production,
                &row.id,
            )?,
            original_input: input(&value),
            effective_input: Some(input(&value)),
            effective_value: Some(value),
            observations: vec![],
        });
    }
    let mut sales = vec![];
    for row in s.sales {
        if number(&row.seq)? > at {
            continue;
        }
        let original = sale::full_original(
            row.clone(),
            s.sale_cost_rows
                .iter()
                .filter(|r| r.sale_id == row.id)
                .map(|r| r.row.clone())
                .collect(),
        );
        let first = s
            .sale_versions
            .iter()
            .find(|v| v.sale_id == row.id && v.previous_version_id.is_none())
            .ok_or_else(|| unavailable("원판매 개정이 없습니다"))?;
        let latest = s
            .sale_versions
            .iter()
            .filter(|v| v.sale_id == row.id && v.seq.parse::<i64>().is_ok_and(|n| n <= at))
            .max_by_key(|v| v.seq.parse::<i64>().unwrap_or(-1))
            .ok_or_else(|| unavailable("유효 판매 개정이 없습니다"))?;
        let value = sale_value(&original, latest);
        let mut r = reference(
            s,
            &row.original_confirmation_id,
            CorrectionSubjectKind::SaleCount,
            &row.id,
        )?;
        r.version_id = latest.id.clone();
        r.confirmation_id = latest.confirmation_id.clone();
        r.business_record_id = latest.business_record_id.clone();
        r.seq = latest.seq.clone();
        r.recorded_at = latest.recorded_at.clone();
        events.push(CorrectionHistoryEvent {
            reference: r,
            original_input: input(&sale_value(&original, first)),
            effective_input: Some(input(&value)),
            effective_value: Some(value),
            observations: vec![],
        });
        sales.push(original);
    }
    for row in s.purchase_returns {
        if number(&row.seq)? > at {
            continue;
        }
        let BusinessPayload::PurchaseReturn { data, .. } = payload(s, &row.confirmation_id)? else {
            return Err(unavailable("원반품 입력 종류 불일치"));
        };
        let value = CorrectionEventValue::PurchaseReturn {
            input: data.clone(),
            result: row.effect.clone(),
        };
        events.push(CorrectionHistoryEvent {
            reference: reference(
                s,
                &row.confirmation_id,
                CorrectionSubjectKind::PurchaseReturn,
                &row.id,
            )?,
            original_input: input(&value),
            effective_input: Some(input(&value)),
            effective_value: Some(value),
            observations: vec![],
        });
    }
    for row in s.wastes {
        if number(&row.seq)? > at {
            continue;
        }
        let BusinessPayload::Waste { data, .. } = payload(s, &row.confirmation_id)? else {
            return Err(unavailable("원폐기 입력 종류 불일치"));
        };
        let value = CorrectionEventValue::Waste {
            input: data.clone(),
            result: row.effect.clone(),
        };
        events.push(CorrectionHistoryEvent {
            reference: reference(
                s,
                &row.confirmation_id,
                CorrectionSubjectKind::Waste,
                &row.id,
            )?,
            original_input: input(&value),
            effective_input: Some(input(&value)),
            effective_value: Some(value),
            observations: vec![],
        });
    }
    for row in s.stocktake_batches {
        if number(&row.seq)? > at {
            continue;
        }
        let BusinessPayload::Stocktake { data, .. } = payload(s, &row.confirmation_id)? else {
            return Err(unavailable("원실사 입력 종류 불일치"));
        };
        let observations = original_observations(s, &row.id)?;
        let rows = observations
            .iter()
            .map(|o| {
                let raw = data
                    .rows
                    .get(o.effect.position)
                    .ok_or_else(|| unavailable("원실사 입력 위치가 없습니다"))?;
                Ok(StocktakeCorrectionRowDraft {
                    observation_id: Some(o.id.clone()),
                    expected_version_id: Some(o.version_id.clone()),
                    action: ObservationCorrectionActionDraft::Replace {
                        quantity: raw.quantity.clone(),
                        unit: raw.unit.clone(),
                        initial_value: raw.initial_value.clone(),
                        excess: raw.excess.clone(),
                    },
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;
        let value = CorrectionEventValue::Stocktake {
            input: DraftStocktakeCorrectionV1 { rows },
        };
        events.push(CorrectionHistoryEvent {
            reference: reference(
                s,
                &row.confirmation_id,
                CorrectionSubjectKind::Stocktake,
                &row.id,
            )?,
            original_input: input(&value),
            effective_input: Some(input(&value)),
            effective_value: Some(value),
            observations,
        });
    }
    for event in &mut events {
        if let Some(v) = s
            .event_versions
            .iter()
            .filter(|v| {
                v.reference.kind == event.reference.kind
                    && v.reference.original_id == event.reference.original_id
                    && v.reference.seq.parse::<i64>().is_ok_and(|n| n <= at)
            })
            .max_by_key(|v| v.reference.seq.parse::<i64>().unwrap_or(-1))
        {
            event.reference = v.reference.clone();
            let after: Option<CorrectionEventValue> = resolved(s, &v.correction_id, &v.after)?;
            event.effective_input = after.as_ref().map(input);
            event.effective_value = after;
        }
        for observation in &mut event.observations {
            if let Some(p) = latest_projection(s, "observation", &observation.id, at) {
                let Projection::Observation {
                    change,
                    observation_version_id,
                    ..
                } = &p.projection
                else {
                    unreachable!()
                };
                observation.previous_version_id = Some(observation.version_id.clone());
                observation.version_id = observation_version_id
                    .clone()
                    .unwrap_or_else(|| p.id.clone());
                if let Some(after) = &change.after {
                    observation.effect = resolved(s, &p.correction_id, after)?;
                }
                if let Some(CorrectionReplacementDraft::Stocktake { data }) =
                    &mut event.effective_input
                    && let Some(row) = data
                        .rows
                        .iter_mut()
                        .find(|r| r.observation_id.as_ref() == Some(&observation.id))
                {
                    row.expected_version_id = Some(observation.version_id.clone());
                    if change.after.is_none() {
                        row.action = ObservationCorrectionActionDraft::Void {};
                    }
                }
            }
        }
    }
    events.sort_by(|a, b| {
        number(&a.reference.logical_seq)
            .unwrap_or(-1)
            .cmp(&number(&b.reference.logical_seq).unwrap_or(-1))
            .then(
                a.reference
                    .logical_position
                    .cmp(&b.reference.logical_position),
            )
            .then(a.reference.original_id.cmp(&b.reference.original_id))
    });
    let mut actual_materials = vec![];
    for material in s.materials {
        if number(material.seq.as_deref().unwrap_or("0"))? > at
            || actual_materials
                .iter()
                .any(|m: &CorrectionMaterialSnapshot| m.material_id == material.material_id)
        {
            continue;
        }
        let latest = s
            .holdings
            .iter()
            .filter(|h| {
                h.material_id == material.material_id && h.seq.parse::<i64>().is_ok_and(|n| n <= at)
            })
            .max_by_key(|h| h.seq.parse::<i64>().unwrap_or(-1));
        let projection = latest_projection(s, "material", &material.material_id, at);
        let state = if let Some(p) = projection.filter(|p| {
            latest.is_none_or(|h| {
                h.seq.parse::<i64>().unwrap_or(-1) <= p.seq.parse::<i64>().unwrap_or(-1)
            })
        }) {
            let Projection::Material {
                change,
                holding_version_id,
                price_basis_id,
            } = &p.projection
            else {
                unreachable!()
            };
            let mut state = resolved(s, &p.correction_id, &change.after)?;
            if let CorrectionHoldingState::Known {
                amounts,
                projection_seq,
                last_valid_price,
                ..
            } = &mut state
            {
                amounts.source_id = holding_version_id.clone();
                *projection_seq = Some(p.seq.clone());
                if let Some(id) = price_basis_id {
                    let price = s
                        .price_bases
                        .iter()
                        .find(|x| &x.id == id)
                        .ok_or_else(|| unavailable("정정 단가 근거가 없습니다"))?;
                    *last_valid_price = Some(CorrectionPriceBasis {
                        amount: price.amount.clone(),
                        quantity: price.quantity.clone(),
                        source: CorrectionPlanRef::Existing {
                            entity: CorrectionRefKind::PriceBasis,
                            id: price.id.clone(),
                            version_id: price.id.clone(),
                        },
                        logical_seq: price.logical_seq.clone(),
                    });
                }
            }
            state
        } else {
            holding_state(s, &events, latest)?
        };
        actual_materials.push(CorrectionMaterialSnapshot {
            material_id: material.material_id.clone(),
            state,
        });
    }
    let mut actual_allocations = vec![];
    for observation in s.observations {
        if let Some(p) = latest_projection(s, "allocation", &observation.id, at) {
            let Projection::Allocation {
                change,
                consumption_version_id,
            } = &p.projection
            else {
                unreachable!()
            };
            if let Some(a) = &change.after {
                actual_allocations.push(CorrectionAllocationSnapshot {
                    observation_id: observation.id.clone(),
                    consumption_version_id: consumption_version_id
                        .clone()
                        .ok_or_else(|| unavailable("정정 배분 버전이 없습니다"))?,
                    allocation: resolved(s, &p.correction_id, a)?,
                });
            }
            continue;
        }
        if let Some(version) = s
            .consumption_versions
            .iter()
            .filter(|v| {
                // A corrected allocation's as_of_revision is the input
                // revision, not its publication time. Future projections
                // must not leak into replay of an earlier confirmation.
                v.correction_projection_id.is_none()
                    && s.observation_versions.iter().any(|o| {
                        o.id == v.observation_version_id
                            && o.observation_id == observation.id
                            && o.seq.parse::<i64>().is_ok_and(|n| n <= at)
                    })
                    && v.as_of_revision.parse::<i64>().is_ok_and(|n| n <= at)
            })
            .max_by_key(|v| v.as_of_revision.parse::<i64>().unwrap_or(-1))
        {
            actual_allocations.push(CorrectionAllocationSnapshot {
                observation_id: observation.id.clone(),
                consumption_version_id: version.id.clone(),
                allocation: allocation(s, version),
            });
        }
    }
    let mut existing_inclusions = vec![];
    for event in events
        .iter()
        .filter(|e| e.reference.kind == CorrectionSubjectKind::Receipt)
    {
        let original = s.reconciliations.iter().find(|r| {
            r.receipt_id == event.reference.original_id
                && r.seq.parse::<i64>().is_ok_and(|n| n <= at)
        });
        let r = if let Some(p) = latest_projection(s, "inclusion", &event.reference.original_id, at)
        {
            let Projection::Inclusion { change, links, .. } = &p.projection else {
                unreachable!()
            };
            let Some(after) = &change.after else { continue };
            let owner = s
                .records
                .iter()
                .find(|r| r.id == p.correction_id)
                .ok_or_else(|| unavailable("포함 정정 원기록이 없습니다"))?;
            crate::reconciliation::ReceiptReconciliation {
                id: p.id.clone(),
                receipt_id: event.reference.original_id.clone(),
                confirmation_id: owner.confirmation_id.clone(),
                business_record_id: owner.business_record_id.clone(),
                effect: resolved(s, &p.correction_id, after)?,
                selection_results: links
                    .iter()
                    .map(|l| crate::reconciliation::ReceiptInclusionResult {
                        selection_position: l.selection_position,
                        portion_position: l.portion_position,
                        link_id: l.link_id.clone(),
                        link_version_id: l.id.clone(),
                        reservation_id: l.reservation_id.clone(),
                    })
                    .collect(),
                material_results: vec![],
                recorded_at: p.recorded_at.clone(),
                seq: p.seq.clone(),
            }
        } else if let Some(r) = original {
            r.clone()
        } else {
            continue;
        };
        existing_inclusions.push(CorrectionEffectiveInclusion {
            receipt_id: r.receipt_id.clone(),
            effective_confirmation_id: event.reference.confirmation_id.clone(),
            reconciliation: r,
        });
    }
    let mut closures = vec![];
    for row in s.closures {
        let Some(active) = events.iter().find(|e| {
            e.reference.kind == CorrectionSubjectKind::Receipt
                && e.reference.original_id == row.receipt_id
        }) else {
            continue;
        };
        if !matches!(&active.effective_input,Some(CorrectionReplacementDraft::Receipt{data}) if matches!(&data.source,crate::receipt::ReceiptSourceDraft::Order{order_id:Some(id),..} if id==&row.order_id))
        {
            continue;
        }
        let original_seq = number(&active.reference.logical_seq)?;
        if let Some(p) = latest_projection(s, "order_closure", &row.order_id, at)
            .filter(|p| p.seq.parse::<i64>().is_ok_and(|n| n > original_seq))
        {
            let Projection::OrderClosure { change, .. } = &p.projection else {
                unreachable!()
            };
            if change.after.is_none() {
                continue;
            }
        }
        let Some(receipt) = s
            .receipts
            .iter()
            .find(|r| r.id == row.receipt_id && r.seq.parse::<i64>().is_ok_and(|n| n <= at))
        else {
            continue;
        };
        let first = s
            .orders
            .iter()
            .find(|o| o.id == row.initial_version_id)
            .ok_or_else(|| unavailable("원주문 최초 버전이 없습니다"))?;
        let order = s
            .orders
            .iter()
            .find(|o| o.id == row.ordered_version_id)
            .ok_or_else(|| unavailable("원주문 적용 버전이 없습니다"))?;
        let mut closure = OrderReceiptClosure {
            status: OrderStatus::Received,
            order_id: Uuid::parse_str(&row.order_id).map_err(unavailable)?,
            initial_order_version_id: Uuid::parse_str(&row.initial_version_id)
                .map_err(unavailable)?,
            initial_ordered_quantity: first.quantity.clone(),
            ordered_version_id: Uuid::parse_str(&row.ordered_version_id).map_err(unavailable)?,
            ordered_quantity: order.quantity.clone(),
            receipt_id: Uuid::parse_str(&row.receipt_id).map_err(unavailable)?,
            receipt_confirmation_id: Uuid::parse_str(&row.confirmation_id).map_err(unavailable)?,
            receipt_material_version_id: Uuid::parse_str(&receipt.material_version_id)
                .map_err(unavailable)?,
            received_quantity: row.received_quantity.clone(),
            difference: row.difference.clone(),
            unit: order.unit.clone(),
        };
        if let Some(p) = latest_projection(s, "order_closure", &row.order_id, at)
            .filter(|p| p.seq.parse::<i64>().is_ok_and(|n| n > original_seq))
        {
            let Projection::OrderClosure { change, .. } = &p.projection else {
                unreachable!()
            };
            if let Some(after) = &change.after {
                closure.received_quantity = after.received_quantity.clone();
                closure.difference = after.difference.clone();
            }
        }
        closures.push(closure);
    }
    Ok(CorrectionHistory {
        as_of_revision: at,
        today,
        events,
        materials: s
            .materials
            .iter()
            .filter(|m| {
                m.seq
                    .as_deref()
                    .unwrap_or("0")
                    .parse::<i64>()
                    .is_ok_and(|n| n <= at)
            })
            .cloned()
            .collect(),
        prices: s
            .prices
            .iter()
            .filter(|p| p.seq.parse::<i64>().is_ok_and(|n| n <= at))
            .cloned()
            .collect(),
        orders: s
            .orders
            .iter()
            .filter(|o| o.seq.parse::<i64>().is_ok_and(|n| n <= at))
            .cloned()
            .collect(),
        closures,
        receipts: s
            .receipts
            .iter()
            .filter(|r| r.seq.parse::<i64>().is_ok_and(|n| n <= at))
            .cloned()
            .collect(),
        sales,
        sale_versions: s
            .sale_versions
            .iter()
            .filter(|v| v.seq.parse::<i64>().is_ok_and(|n| n <= at))
            .cloned()
            .collect(),
        existing_inclusions,
        actual_materials,
        actual_allocations,
    })
}
fn allocation(s: &Validation<'_>, v: &stocktake::ConsumptionVersion) -> ConsumptionAllocation {
    ConsumptionAllocation {
        basis: v.basis.clone(),
        end_seq: v.end_seq.clone(),
        as_of_revision: v.as_of_revision.clone(),
        total_quantity: v.total_quantity.clone(),
        total_value: v.total_value.clone(),
        candidates: s
            .consumption_rows
            .iter()
            .filter(|r| r.consumption_version_id == v.id)
            .map(|r| r.row.clone())
            .collect(),
        unallocated_quantity: v.unallocated_quantity.clone(),
        unallocated_value: v.unallocated_value.clone(),
        reason: v.reason,
    }
}
fn holding_state(
    s: &Validation<'_>,
    events: &[CorrectionHistoryEvent],
    h: Option<&holding::HoldingVersion>,
) -> Result<CorrectionHoldingState, StoreError> {
    let Some(h) = h else {
        return Ok(CorrectionHoldingState::NotEstablished {});
    };
    let input = holding::input_at(
        s.holdings,
        s.observation_versions,
        s.projection_versions,
        &h.material_id,
        number(&h.seq)?,
    )?;
    let observation_boundary = if let Some(o) = &input.observation {
        // Allocation-only corrections retain the physical holding row while
        // publishing a new observation version. Follow the stable observation
        // identity instead of requiring that preserved baseline to be current.
        let original = s
            .observation_versions
            .iter()
            .find(|v| v.id == o.id)
            .ok_or_else(|| unavailable("보유 원관측 개정이 없습니다"))?;
        let observation = events
            .iter()
            .flat_map(|e| &e.observations)
            .find(|r| r.id == original.observation_id)
            .ok_or_else(|| unavailable("현재 관측 경계가 없습니다"))?;
        Some(CorrectionObservationBoundary {
            reference: CorrectionPlanRef::Existing {
                entity: CorrectionRefKind::Observation,
                id: observation.id.clone(),
                version_id: observation.version_id.clone(),
            },
            stocktake_id: observation.stocktake_id.clone(),
            logical_seq: observation.seq.clone(),
            observed_at: observation.recorded_at.clone(),
            quantity: observation.effect.measured_quantity.clone(),
        })
    } else {
        None
    };
    let initial_boundary = if observation_boundary.is_none() {
        let b = input
            .basis
            .as_ref()
            .ok_or_else(|| unavailable("현재 시작 기준이 없습니다"))?;
        let id =
            b.id.as_deref()
                .ok_or_else(|| unavailable("현재 시작 기준 ID가 없습니다"))?;
        let (entity, stable, version) = match b.kind {
            HoldingBasisKind::Receipt => (CorrectionRefKind::Receipt, id.to_owned(), id.to_owned()),
            HoldingBasisKind::Production => {
                let e = s
                    .production_effects
                    .iter()
                    .find(|e| e.id == id)
                    .ok_or_else(|| unavailable("생산 시작 기준이 없습니다"))?;
                (
                    CorrectionRefKind::Production,
                    e.production_id.clone(),
                    e.production_id.clone(),
                )
            }
            HoldingBasisKind::Reconciliation => {
                let r = s
                    .reconciliations
                    .iter()
                    .find(|r| r.material_results.iter().any(|m| m.effect_id == id))
                    .ok_or_else(|| unavailable("입고 보정 시작 기준이 없습니다"))?;
                (
                    CorrectionRefKind::Receipt,
                    r.receipt_id.clone(),
                    r.receipt_id.clone(),
                )
            }
            HoldingBasisKind::Correction => {
                let p = s
                    .projection_versions
                    .iter()
                    .find(|p| Some(&p.id) == b.id.as_ref())
                    .ok_or_else(|| unavailable("정정 시작 기준이 없습니다"))?;
                let Projection::Material { change, .. } = &p.projection else {
                    return Err(unavailable("정정 시작 근거 종류가 다릅니다"));
                };
                if let CorrectionHoldingState::Known {
                    initial_boundary: Some(initial),
                    ..
                } = &change.after
                {
                    let resolved: CorrectionInitialBoundary =
                        resolved(s, &p.correction_id, initial)?;
                    match resolved.reference {
                        CorrectionPlanRef::Existing {
                            entity,
                            id,
                            version_id,
                        } => (entity, id, version_id),
                        _ => return Err(unavailable("시작 근거 실제 버전이 없습니다")),
                    }
                } else {
                    return Err(unavailable("정정 시작 위치가 없습니다"));
                }
            }
            HoldingBasisKind::Observation => return Err(unavailable("관측 없는 관측 기준입니다")),
        };
        Some(CorrectionInitialBoundary {
            reference: CorrectionPlanRef::Existing {
                entity,
                id: stable,
                version_id: version,
            },
            logical_seq: b
                .seq
                .clone()
                .ok_or_else(|| unavailable("시작 순번이 없습니다"))?,
        })
    } else {
        None
    };
    let last_valid_price = input.last_valid.map(|p| CorrectionPriceBasis {
        amount: p.amount().decimal(),
        quantity: p.quantity().decimal(),
        source: CorrectionPlanRef::Existing {
            entity: if s.price_bases.iter().any(|b| b.id == p.source_id()) {
                CorrectionRefKind::PriceBasis
            } else {
                CorrectionRefKind::Material
            },
            id: if s.price_bases.iter().any(|b| b.id == p.source_id()) {
                p.source_id().into()
            } else {
                h.material_id.clone()
            },
            version_id: p.source_id().into(),
        },
        logical_seq: s
            .holdings
            .iter()
            .find(|v| v.id == p.source_id())
            .map(|v| v.seq.clone())
            .or_else(|| {
                s.price_bases
                    .iter()
                    .find(|b| b.id == p.source_id())
                    .map(|b| b.logical_seq.clone())
            })
            .unwrap_or_else(|| h.seq.clone()),
    });
    Ok(CorrectionHoldingState::Known {
        amounts: holding::amounts(h),
        projection_seq: Some(h.seq.clone()),
        last_valid_price,
        observation_boundary,
        initial_boundary,
    })
}

pub(in crate::store) fn load(
    connection: &Connection,
    today: NaiveDate,
) -> Result<CorrectionHistory, StoreError> {
    load_at(connection, today, None)
}

pub(super) fn load_at(
    connection: &Connection,
    today: NaiveDate,
    at: Option<i64>,
) -> Result<CorrectionHistory, StoreError> {
    let corrections = super::snapshot_rows(connection)?;
    let business = business::snapshot_rows(connection)?;
    let materials = catalog::versions(connection, None)?;
    let prices = catalog::prices(connection, None)?;
    let orders = purchase_order::read_versions(connection, None, None, false)?;
    let closures = receipt::closures(connection)?;
    let receipts = receipt::records(connection, None, None)?;
    let production = production::snapshot_rows(connection)?;
    let sale = sale::snapshot_rows(connection)?;
    let stocktake = stocktake::snapshot_rows(connection)?;
    let holdings = holding::versions(connection, None)?;
    let wastes = waste::snapshot_rows(connection)?;
    let purchase_returns = purchase_return::snapshot_rows(connection)?;
    let reconciliations = reconciliation::snapshot_rows(connection)?;
    let sequence = number(&catalog::state(connection)?.revision)?;
    let view = Validation {
        records: &corrections.records,
        event_versions: &corrections.event_versions,
        event_heads: &corrections.event_heads,
        projection_versions: &corrections.projection_versions,
        projection_heads: &corrections.projection_heads,
        price_bases: &corrections.price_bases,
        business_records: &business.records,
        revisions: &business.revisions,
        confirmations: &business.confirmations,
        materials: &materials,
        prices: &prices,
        orders: &orders,
        closures: &closures,
        receipts: &receipts,
        purchase_returns: &purchase_returns,
        productions: &production.records,
        production_inputs: &production.inputs,
        production_outputs: &production.outputs,
        production_effects: &production.effects,
        sales: &sale.records,
        sale_versions: &sale.versions,
        sale_cost_rows: &sale.cost_rows,
        stocktake_batches: &stocktake.batches,
        observations: &stocktake.observations,
        observation_versions: &stocktake.versions,
        consumption_versions: &stocktake.consumption_versions,
        consumption_rows: &stocktake.consumption_rows,
        holdings: &holdings,
        wastes: &wastes,
        reconciliations: &reconciliations.records,
        inclusion_links: &reconciliations.links,
        inclusion_link_versions: &reconciliations.versions,
        inclusion_reservations: &reconciliations.reservations,
        mutations: &[],
        sequence,
    };
    from_snapshot(&view, at.unwrap_or(sequence), today)
}
