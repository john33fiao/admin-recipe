use super::*;
use crate::{
    domain::{Money, Quantity, Unit, UnitPrice, allocate_largest_remainder, round_ratio},
    holding::{
        HoldingBasisKind, HoldingBasisRef, HoldingInput, HoldingObservation, resolve_holding_price,
    },
    production::{
        AllocationPercent, ProductionCheck, ProductionMaterialContext, project_production,
    },
    purchase_order::OrderStatus,
    receipt::{ReceiptSourceDraft, ReceiptStockEffectDraft, parse_receipt_date},
    reconciliation::{
        ReconciliationMaterialEffect, ValueActualAmounts, ValueDelta, ValueEventRef, ValueMaterial,
        ValueObservation, ValueProductionInput, ValueProductionOutput, ValueReplacement,
        ValueReplayEvent, replay_effective_values, split_receipt_amount,
    },
    sale::{SaleConsumptionSource, SaleCount, SaleInterval, calculate_sale_amount, eligible_sales},
    stocktake::{
        NoAllocationReason, SaleAllocationProposal, StocktakeCalculation, StocktakePriceSource,
        StocktakeValuation,
    },
    waste::{WasteCheck, WasteMaterialContext, calculate_waste},
};
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, CorrectionError>;
fn bad(message: impl Into<String>) -> CorrectionError {
    CorrectionError::InvalidHistory(message.into())
}
fn range(error: DomainError) -> CorrectionError {
    CorrectionError::OutOfRange(error.0)
}
fn q(value: &str) -> Result<Quantity> {
    Quantity::parse(value).map_err(range)
}
fn money(value: &str) -> Result<Money> {
    Money::parse(value).map_err(range)
}
fn number(value: &str) -> Result<i64> {
    value
        .parse::<i64>()
        .ok()
        .filter(|n| *n > 0 && n.to_string() == value)
        .ok_or_else(|| bad("기록 순번이 올바르지 않습니다"))
}
fn id(value: &str) -> Result<()> {
    uuid::Uuid::parse_str(value)
        .ok()
        .filter(|v| v.to_string() == value)
        .map(|_| ())
        .ok_or_else(|| bad("기록 참조가 정규 UUID가 아닙니다"))
}
fn issue(field: impl Into<String>, code: &str, message: impl Into<String>) -> BusinessIssue {
    BusinessIssue {
        field: field.into(),
        code: code.into(),
        message: message.into(),
    }
}
fn same<T: Serialize>(a: &T, b: &T) -> bool {
    serde_json::to_value(a).ok() == serde_json::to_value(b).ok()
}
fn plan_id(reference: &CorrectionPlanRef) -> &str {
    match reference {
        CorrectionPlanRef::Existing { version_id, .. } => version_id,
        CorrectionPlanRef::Proposed {
            id,
            expected_version_id,
            ..
        } => expected_version_id.as_deref().unwrap_or(id),
    }
}
fn entity(kind: CorrectionSubjectKind) -> CorrectionRefKind {
    match kind {
        CorrectionSubjectKind::Receipt => CorrectionRefKind::Receipt,
        CorrectionSubjectKind::Production => CorrectionRefKind::Production,
        CorrectionSubjectKind::SaleCount => CorrectionRefKind::Sale,
        CorrectionSubjectKind::Waste => CorrectionRefKind::Waste,
        CorrectionSubjectKind::Stocktake => CorrectionRefKind::Stocktake,
    }
}
fn event_ref(event: &CorrectionHistoryEvent, proposed: bool) -> CorrectionPlanRef {
    if proposed {
        CorrectionPlanRef::Proposed {
            entity: entity(event.reference.kind),
            id: event.reference.original_id.clone(),
            expected_version_id: Some(event.reference.version_id.clone()),
        }
    } else {
        CorrectionPlanRef::Existing {
            entity: entity(event.reference.kind),
            id: event.reference.original_id.clone(),
            version_id: event.reference.version_id.clone(),
        }
    }
}
fn observation_ref(row: &StocktakeObservationRecord, proposed: bool) -> CorrectionPlanRef {
    if proposed {
        CorrectionPlanRef::Proposed {
            entity: CorrectionRefKind::Observation,
            id: row.id.clone(),
            expected_version_id: Some(row.version_id.clone()),
        }
    } else {
        CorrectionPlanRef::Existing {
            entity: CorrectionRefKind::Observation,
            id: row.id.clone(),
            version_id: row.version_id.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
struct Pass {
    materials: BTreeMap<String, CorrectionHoldingState>,
    sources: BTreeMap<String, String>,
    values: BTreeMap<String, CorrectionEventValue>,
    observations: BTreeMap<String, StocktakeRowEffect>,
    observation_prices: BTreeMap<String, CorrectionPriceBasis>,
    topology: Vec<ValueReplayEvent>,
    issues: Vec<BusinessIssue>,
    reviews: Vec<CorrectionReviewRequest>,
    closures: BTreeMap<String, OrderReceiptPreview>,
}

fn amounts(state: &CorrectionHoldingState) -> Result<Option<(Quantity, Money)>> {
    match state {
        CorrectionHoldingState::NotEstablished {} => Ok(None),
        CorrectionHoldingState::Known { amounts, .. } => {
            let value = amounts
                .value
                .as_deref()
                .ok_or_else(|| bad("정정 원장의 보유 가치가 없습니다"))?;
            let result = (q(&amounts.quantity)?, money(value)?);
            if result.0.micros() == 0 && result.1.micros() != 0 {
                return Err(bad("0수량의 가치가 0이 아닙니다"));
            }
            Ok(Some(result))
        }
    }
}

fn holding(pass: &Pass, material: &str) -> Result<HoldingInput> {
    let Some(CorrectionHoldingState::Known {
        amounts,
        last_valid_price,
        observation_boundary,
        initial_boundary,
        ..
    }) = pass.materials.get(material)
    else {
        return Ok(HoldingInput::default());
    };
    let (basis, observation) = if let Some(o) = observation_boundary {
        (
            HoldingBasisRef {
                kind: HoldingBasisKind::Observation,
                id: Some(plan_id(&o.reference).into()),
                seq: Some(o.logical_seq.clone()),
            },
            Some(HoldingObservation {
                id: plan_id(&o.reference).into(),
                quantity: o.quantity.clone(),
                recorded_at: o.observed_at.clone(),
                seq: o.logical_seq.clone(),
            }),
        )
    } else {
        let initial = initial_boundary
            .as_ref()
            .ok_or_else(|| bad("유효 시작 근거가 없습니다"))?;
        let (entity, original_id) = match &initial.reference {
            CorrectionPlanRef::Existing { entity, id, .. }
            | CorrectionPlanRef::Proposed { entity, id, .. } => (entity, id),
        };
        let kind = match entity {
            CorrectionRefKind::Production => HoldingBasisKind::Production,
            CorrectionRefKind::Receipt if matches!(pass.values.get(original_id),Some(CorrectionEventValue::Receipt{input,..}) if matches!(input.stock_effect,ReceiptStockEffectDraft::Reconciled{..})) => {
                HoldingBasisKind::Reconciliation
            }
            CorrectionRefKind::Receipt => HoldingBasisKind::Receipt,
            _ => return Err(bad("유효 시작 근거의 사건 종류가 다릅니다")),
        };
        (
            HoldingBasisRef {
                kind,
                id: Some(plan_id(&initial.reference).into()),
                seq: Some(initial.logical_seq.clone()),
            },
            None,
        )
    };
    let last_valid = last_valid_price
        .as_ref()
        .map(|p| {
            UnitPrice::new(
                money(&p.amount)?,
                q(&p.quantity)?,
                plan_id(&p.source).into(),
            )
            .map_err(range)
        })
        .transpose()?;
    Ok(HoldingInput {
        basis: Some(basis),
        observation,
        quantity: Some(q(&amounts.quantity)?),
        value: Some(money(
            amounts
                .value
                .as_deref()
                .ok_or_else(|| bad("보유 가치 없음"))?,
        )?),
        source_id: pass.sources.get(material).cloned(),
        last_valid,
    })
}

fn material<'a>(
    history: &'a CorrectionHistory,
    material_id: &str,
    version_id: &str,
    at: i64,
) -> Result<&'a MaterialVersion> {
    history
        .materials
        .iter()
        .find(|m| {
            m.id == version_id
                && m.material_id == material_id
                && m.seq
                    .as_ref()
                    .is_none_or(|s| number(s).is_ok_and(|n| n <= at))
        })
        .ok_or_else(|| bad("품목 버전이 유효 시점에 존재하지 않습니다"))
}
fn value_material(m: &MaterialVersion) -> Result<ValueMaterial> {
    Ok(ValueMaterial {
        material_id: m.material_id.clone(),
        material_version_id: m.id.clone(),
        unit: Unit::parse(&m.canonical_unit).map_err(range)?,
    })
}
fn canonical(quantity: &str, unit: &str, m: &MaterialVersion) -> Result<Quantity> {
    let input = Unit::parse(unit).map_err(range)?;
    let expected = Unit::parse(&m.canonical_unit).map_err(range)?;
    if !input.same_dimension(&expected) {
        return Err(bad("품목과 입력 단위 차원이 다릅니다"));
    }
    input.to_canonical(q(quantity)?).map_err(range)
}

fn write_state(
    pass: &mut Pass,
    material: &str,
    quantity: Quantity,
    value: Money,
    reference: CorrectionPlanRef,
    logical_seq: &str,
    observed: Option<CorrectionObservationBoundary>,
) -> Result<()> {
    if quantity.micros() == 0 && value.micros() != 0 {
        return Err(bad("정정 후 0수량에 가치가 남습니다"));
    }
    let old = pass.materials.remove(material);
    let (mut last_valid, old_observation, initial) = match old {
        Some(CorrectionHoldingState::Known {
            last_valid_price,
            observation_boundary,
            initial_boundary,
            ..
        }) => (last_valid_price, observation_boundary, initial_boundary),
        _ => (
            None,
            None,
            Some(CorrectionInitialBoundary {
                reference: reference.clone(),
                logical_seq: logical_seq.into(),
            }),
        ),
    };
    if quantity.micros() > 0 {
        last_valid = Some(CorrectionPriceBasis {
            amount: value.decimal(),
            quantity: quantity.decimal(),
            source: reference.clone(),
            logical_seq: logical_seq.into(),
        });
    }
    pass.sources
        .insert(material.into(), plan_id(&reference).into());
    pass.materials.insert(
        material.into(),
        CorrectionHoldingState::Known {
            amounts: HoldingAmounts {
                quantity: quantity.decimal(),
                value: Some(value.decimal()),
                source_id: None,
            },
            projection_seq: None,
            last_valid_price: last_valid,
            observation_boundary: observed.or(old_observation),
            initial_boundary: initial,
        },
    );
    Ok(())
}

fn current_price(
    history: &CorrectionHistory,
    pass: &Pass,
    material_id: &str,
    logical_seq: i64,
) -> Result<ResolvedPrice> {
    let price = history
        .prices
        .iter()
        .filter(|p| p.material_id == material_id && number(&p.seq).is_ok_and(|n| n < logical_seq))
        .max_by_key(|p| number(&p.seq).unwrap_or(0))
        .map(|p| UnitPrice::new(money(&p.amount)?, q(&p.quantity)?, p.id.clone()).map_err(range))
        .transpose()?;
    resolve_holding_price(&holding(pass, material_id)?, price).map_err(range)
}

fn sale_sources(
    history: &CorrectionHistory,
    events: &[CorrectionHistoryEvent],
) -> Result<Vec<SaleConsumptionSource>> {
    let mut result = vec![];
    for original in &history.sales {
        let Some(event) = events.iter().find(|e| {
            e.reference.kind == CorrectionSubjectKind::SaleCount
                && e.reference.original_id == original.id
        }) else {
            continue;
        };
        let Some(CorrectionReplacementDraft::SaleCount { data }) = &event.effective_input else {
            continue;
        };
        result.push(SaleConsumptionSource {
            sale_id: original.id.clone(),
            sale_version_id: history
                .sale_versions
                .iter()
                .filter(|v| {
                    v.sale_id == original.id
                        && number(&v.seq).is_ok_and(|n| n <= history.as_of_revision)
                })
                .max_by_key(|v| number(&v.seq).unwrap_or(0))
                .ok_or_else(|| bad("판매의 실제 개정이 없습니다"))?
                .id
                .clone(),
            recipe_version_id: original.recipe_version_id.clone(),
            original_seq: number(&original.seq)?,
            count: SaleCount::parse(&data.count).map_err(range)?,
            direct_material_ids: original
                .recipe_cost
                .rows
                .iter()
                .map(|r| r.material_id.clone())
                .collect(),
        });
    }
    Ok(result)
}

fn allocation(
    history: &CorrectionHistory,
    sources: &[SaleConsumptionSource],
    material_id: &str,
    basis: &HoldingBasisRef,
    end: i64,
    quantity: Quantity,
    value: Money,
) -> Result<ConsumptionAllocation> {
    let selected = eligible_sales(
        &SaleInterval {
            material_id: material_id.into(),
            after_seq: number(
                basis
                    .seq
                    .as_deref()
                    .ok_or_else(|| bad("배분 시작 근거 없음"))?,
            )?,
            before_seq: end,
        },
        sources,
    )
    .map_err(range)?;
    let weights = selected
        .iter()
        .map(|s| i64::from(s.count.value()))
        .collect::<Vec<_>>();
    let positive = weights.iter().any(|w| *w > 0);
    let quantities = if positive {
        Some(allocate_largest_remainder(quantity.micros(), &weights).map_err(range)?)
    } else {
        None
    };
    let values = if positive {
        Some(allocate_largest_remainder(value.micros(), &weights).map_err(range)?)
    } else {
        None
    };
    let candidates = selected
        .into_iter()
        .enumerate()
        .map(|(position, s)| {
            Ok(SaleAllocationProposal {
                sale_id: s.sale_id,
                sale_version_id: s.sale_version_id,
                recipe_version_id: s.recipe_version_id,
                original_seq: s.original_seq.to_string(),
                count: s.count.decimal(),
                position,
                quantity: quantities
                    .as_ref()
                    .map(|v| Quantity::from_micros(v[position]).map(|n| n.decimal()))
                    .transpose()
                    .map_err(range)?,
                value: values
                    .as_ref()
                    .map(|v| Money::from_micros(v[position]).map(|n| n.decimal()))
                    .transpose()
                    .map_err(range)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let reason = if candidates.is_empty() {
        Some(NoAllocationReason::NoSales)
    } else if !positive {
        Some(NoAllocationReason::ZeroWeight)
    } else {
        None
    };
    Ok(ConsumptionAllocation {
        basis: basis.clone(),
        end_seq: end.to_string(),
        as_of_revision: history.as_of_revision.to_string(),
        total_quantity: quantity.decimal(),
        total_value: value.decimal(),
        candidates,
        unallocated_quantity: if positive {
            "0".into()
        } else {
            quantity.decimal()
        },
        unallocated_value: if positive {
            "0".into()
        } else {
            value.decimal()
        },
        reason,
    })
}

fn valuation(
    choice: Option<&StocktakeValueDraft>,
    price: &ResolvedPrice,
    quantity: Quantity,
    initial: bool,
) -> Result<Option<(Money, StocktakeValuation)>> {
    match choice {
        Some(StocktakeValueDraft::ManualAmount { amount })
            if initial || price.source == "missing" =>
        {
            let value = money(amount)?;
            Ok(Some((
                value,
                StocktakeValuation::ManualAmount {
                    amount: value.decimal(),
                },
            )))
        }
        Some(StocktakeValueDraft::ExistingPrice {}) if price.source != "missing" => {
            let amount = money(
                price
                    .amount
                    .as_deref()
                    .ok_or_else(|| bad("단가 금액 없음"))?,
            )?;
            let basis = q(price
                .quantity
                .as_deref()
                .ok_or_else(|| bad("단가 수량 없음"))?)?;
            if basis.micros() == 0 {
                return Err(bad("단가 기준량은 양수여야 합니다"));
            }
            let value = Money::from_micros(
                i64::try_from(
                    round_ratio(
                        i128::from(amount.micros()) * i128::from(quantity.micros()),
                        i128::from(basis.micros()),
                    )
                    .map_err(range)?,
                )
                .map_err(|_| bad("단가 계산 범위"))?,
            )
            .map_err(range)?;
            let source = match price.source.as_str() {
                "holding_average" => StocktakePriceSource::HoldingAverage,
                "last_valid" => StocktakePriceSource::LastValid,
                "preregistered" => StocktakePriceSource::Preregistered,
                _ => return Err(bad("알 수 없는 단가 출처")),
            };
            Ok(Some((
                value,
                StocktakeValuation::ExistingPrice {
                    source,
                    source_id: price
                        .source_id
                        .clone()
                        .ok_or_else(|| bad("단가 참조 없음"))?,
                    amount: amount.decimal(),
                    quantity: basis.decimal(),
                },
            )))
        }
        _ => Ok(None),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "One observation combines immutable event evidence, explicit review choices, and replay-pass state without owning them"
)]
fn observation(
    history: &CorrectionHistory,
    pass: &mut Pass,
    event: &CorrectionHistoryEvent,
    row: &StocktakeObservationRecord,
    action: &ObservationCorrectionActionDraft,
    review: Option<&ObservationReviewDraft>,
    sources: &[SaleConsumptionSource],
    deltas: &BTreeMap<String, i64>,
    frozen: Option<&StocktakeRowEffect>,
    changed: bool,
    quantity_only: bool,
) -> Result<()> {
    let ObservationCorrectionActionDraft::Replace {
        quantity,
        unit,
        initial_value,
        excess,
    } = action
    else {
        return Ok(());
    };
    if quantity.trim().is_empty() {
        return Err(bad("실사 대체 수량을 입력하거나 명시 취소하세요"));
    }
    let at = number(&event.reference.logical_seq)?;
    let m = material(
        history,
        &row.effect.material_id,
        &row.effect.material_version_id,
        history.as_of_revision,
    )?;
    let measured = canonical(quantity, unit, m)?;
    let input = holding(pass, &m.material_id)?;
    let price = current_price(history, pass, &m.material_id, at)?;
    let reference = observation_ref(row, changed || review.is_some());
    let old = frozen.unwrap_or(&row.effect);
    let explicit = (changed || review.is_some()) && frozen.is_none();
    let initial_choice = review
        .map(|r| r.initial_value.as_ref())
        .unwrap_or(initial_value.as_ref());
    let excess_choice = review.map(|r| r.excess.as_ref()).unwrap_or(excess.as_ref());
    let zero_q = Quantity::from_micros(0).map_err(range)?;
    let zero_v = Money::from_micros(0).map_err(range)?;
    let before = input.quantity.map(|v| HoldingAmounts {
        quantity: v.decimal(),
        value: input.value.map(|v| v.decimal()),
        source_id: input.source_id.clone(),
    });
    let mut request = None;
    let needs_mode_review = frozen.is_none()
        && !explicit
        && match (&old.calculation, input.quantity) {
            (StocktakeCalculation::Initial { .. }, Some(_)) => true,
            (StocktakeCalculation::Settlement { .. }, None) => true,
            (
                StocktakeCalculation::Settlement {
                    excess_quantity, ..
                },
                Some(before_q),
            ) => q(excess_quantity)?.micros() > 0 && measured <= before_q,
            _ => false,
        };
    let computed = if quantity_only {
        // Build source topology without choosing a monetary valuation. The
        // knowledge-time pass will emit any required review with real Q/V and
        // price context; scratch zeroes are never user-visible plan results.
        let calculation = if let Some(before_q) = input.quantity {
            let basis = input
                .basis
                .as_ref()
                .ok_or_else(|| bad("관측 전 기준 없음"))?;
            let consumption = Quantity::from_micros((before_q.micros() - measured.micros()).max(0))
                .map_err(range)?;
            let excess = Quantity::from_micros((measured.micros() - before_q.micros()).max(0))
                .map_err(range)?;
            StocktakeCalculation::Settlement {
                consumption_quantity: consumption.decimal(),
                consumption_value: "0".into(),
                excess_quantity: excess.decimal(),
                excess_value: "0".into(),
                valuation: (excess.micros() > 0)
                    .then(|| StocktakeValuation::ManualAmount { amount: "0".into() }),
                allocation: Box::new(allocation(
                    history,
                    sources,
                    &m.material_id,
                    basis,
                    at,
                    consumption,
                    zero_v,
                )?),
            }
        } else {
            StocktakeCalculation::Initial {
                value: "0".into(),
                valuation: StocktakeValuation::ManualAmount { amount: "0".into() },
            }
        };
        Some((zero_v, calculation))
    } else if needs_mode_review {
        request = Some((
            if input.quantity.is_none() {
                CorrectionValuationMode::Initial
            } else {
                CorrectionValuationMode::Excess
            },
            None,
        ));
        None
    } else if let (Some(before_q), Some(before_v)) = (input.quantity, input.value) {
        let basis = input
            .basis
            .as_ref()
            .ok_or_else(|| bad("관측 전 기준 없음"))?;
        if measured <= before_q {
            if explicit && (initial_choice.is_some() || excess_choice.is_some()) {
                request = Some((CorrectionValuationMode::Excess, None));
                None
            } else {
                let consumed =
                    Quantity::from_micros(before_q.micros() - measured.micros()).map_err(range)?;
                let cost = if consumed.micros() == 0 {
                    zero_v
                } else if consumed == before_q {
                    before_v
                } else {
                    Money::from_micros(
                        i64::try_from(
                            round_ratio(
                                i128::from(before_v.micros()) * i128::from(consumed.micros()),
                                i128::from(before_q.micros()),
                            )
                            .map_err(range)?,
                        )
                        .map_err(|_| bad("소모 가치 범위"))?,
                    )
                    .map_err(range)?
                };
                let allocated =
                    allocation(history, sources, &m.material_id, basis, at, consumed, cost)?;
                Some((
                    Money::from_micros(before_v.micros() - cost.micros()).map_err(range)?,
                    StocktakeCalculation::Settlement {
                        consumption_quantity: consumed.decimal(),
                        consumption_value: cost.decimal(),
                        excess_quantity: "0".into(),
                        excess_value: "0".into(),
                        valuation: None,
                        allocation: Box::new(allocated),
                    },
                ))
            }
        } else {
            let added =
                Quantity::from_micros(measured.micros() - before_q.micros()).map_err(range)?;
            let preserved = if !explicit {
                match &old.calculation {
                    StocktakeCalculation::Settlement {
                        excess_quantity,
                        excess_value,
                        valuation: Some(v),
                        ..
                    } if q(excess_quantity)? == added => Some((money(excess_value)?, v.clone())),
                    _ => None,
                }
            } else {
                None
            };
            let chosen = if let Some(v) = preserved {
                Some(v)
            } else if let Some(c) =
                excess_choice.filter(|c| c.reviewed && !c.reason.trim().is_empty())
            {
                valuation(c.value.as_ref(), &price, added, false)?
            } else {
                None
            };
            if let Some((added_value, val)) = chosen
                && (frozen.is_some() || initial_choice.is_none())
            {
                let allocated =
                    allocation(history, sources, &m.material_id, basis, at, zero_q, zero_v)?;
                Some((
                    Money::from_micros(
                        before_v
                            .micros()
                            .checked_add(added_value.micros())
                            .ok_or_else(|| bad("초과 가치 범위"))?,
                    )
                    .map_err(range)?,
                    StocktakeCalculation::Settlement {
                        consumption_quantity: "0".into(),
                        consumption_value: "0".into(),
                        excess_quantity: added.decimal(),
                        excess_value: added_value.decimal(),
                        valuation: Some(val),
                        allocation: Box::new(allocated),
                    },
                ))
            } else {
                request = Some((CorrectionValuationMode::Excess, Some(added.decimal())));
                None
            }
        }
    } else {
        let preserved = if !explicit {
            match &old.calculation {
                StocktakeCalculation::Initial { value, valuation }
                    if q(&old.measured_quantity)? == measured =>
                {
                    Some((money(value)?, valuation.clone()))
                }
                _ => None,
            }
        } else {
            None
        };
        let chosen = match preserved {
            Some(v) => Some(v),
            None => valuation(initial_choice, &price, measured, true)?,
        };
        if let Some((value, val)) = chosen
            && (frozen.is_some() || excess_choice.is_none())
        {
            if measured.micros() == 0 && value.micros() != 0 {
                return Err(bad("초기 수량 0의 가치는 0이어야 합니다"));
            }
            Some((
                value,
                StocktakeCalculation::Initial {
                    value: value.decimal(),
                    valuation: val,
                },
            ))
        } else {
            request = Some((CorrectionValuationMode::Initial, None));
            None
        }
    };
    let Some((value, calculation)) = computed else {
        let (mode, excess_quantity) = request.expect("review request");
        let field = format!("observations.{}.valuation", row.id);
        pass.issues.push(issue(
            &field,
            "confirmation_required",
            "정정 후 관측의 초기·초과 가치 확인을 다시 검토하세요.",
        ));
        pass.reviews
            .push(CorrectionReviewRequest::ObservationValuation {
                field,
                observation: reference,
                material_id: m.material_id.clone(),
                material_name: m.name.clone(),
                unit: m.canonical_unit.clone(),
                measured_quantity: measured.decimal(),
                proposed_before: pass
                    .materials
                    .get(&m.material_id)
                    .cloned()
                    .unwrap_or(CorrectionHoldingState::NotEstablished {}),
                proposed_price: price,
                mode,
                excess_quantity,
                message: "해당 논리 시점의 시작·초과 가치 입력을 확인하세요.".into(),
            });
        return Ok(());
    };
    let calc = match &calculation {
        StocktakeCalculation::Initial { value, .. } => ValueObservation::Initial {
            value: money(value)?,
        },
        StocktakeCalculation::Settlement {
            consumption_quantity,
            excess_quantity,
            excess_value,
            ..
        } => ValueObservation::Settlement {
            consumption: q(consumption_quantity)?,
            excess: q(excess_quantity)?,
            excess_value: money(excess_value)?,
        },
    };
    let valuation = match &calculation {
        StocktakeCalculation::Initial { valuation, .. } => Some(valuation),
        StocktakeCalculation::Settlement { valuation, .. } => valuation.as_ref(),
    };
    if let Some(StocktakeValuation::ExistingPrice {
        source,
        source_id,
        amount,
        quantity,
    }) = valuation
        && *source != StocktakePriceSource::Preregistered
        && price.source_id.as_deref() == Some(source_id.as_str())
        && let Some(CorrectionHoldingState::Known {
            last_valid_price: Some(basis),
            ..
        }) = pass.materials.get(&m.material_id)
    {
        if basis.amount != *amount || basis.quantity != *quantity {
            return Err(bad("관측 단가와 해당 시점의 가격 근거가 다릅니다"));
        }
        pass.observation_prices
            .insert(row.id.clone(), basis.clone());
    }
    pass.topology.push(ValueReplayEvent::Observation {
        event: ValueEventRef {
            id: row.version_id.clone(),
            seq: at,
        },
        observation_id: row.id.clone(),
        material: value_material(m)?,
        measured,
        calculation: calc,
    });
    pass.observations.insert(
        row.id.clone(),
        StocktakeRowEffect {
            position: row.effect.position,
            material_id: m.material_id.clone(),
            material_version_id: m.id.clone(),
            material_name: m.name.clone(),
            unit: m.canonical_unit.clone(),
            input_quantity: quantity.clone(),
            input_unit: unit.clone(),
            measured_quantity: measured.decimal(),
            before,
            after: HoldingAmounts {
                quantity: measured.decimal(),
                value: Some(value.decimal()),
                source_id: None,
            },
            basis_before: input.basis,
            observation_before: input.observation,
            calculation,
        },
    );
    let adjusted = Money::from_micros(
        value
            .micros()
            .checked_add(*deltas.get(&row.version_id).unwrap_or(&0))
            .ok_or_else(|| bad("포함 보정 가치 범위"))?,
    )
    .map_err(range)?;
    write_state(
        pass,
        &m.material_id,
        measured,
        if quantity_only { zero_v } else { adjusted },
        reference.clone(),
        &at.to_string(),
        Some(CorrectionObservationBoundary {
            reference,
            stocktake_id: row.stocktake_id.clone(),
            logical_seq: at.to_string(),
            observed_at: event.reference.original_recorded_at.clone(),
            quantity: measured.decimal(),
        }),
    )
}

fn receipt_material<'a>(
    history: &'a CorrectionHistory,
    data: &DraftReceiptV1,
) -> Result<&'a MaterialVersion> {
    let (mid, vid) = match &data.source {
        ReceiptSourceDraft::Direct {
            material_id,
            material_version_id,
        } => (material_id.as_deref(), material_version_id.as_deref()),
        ReceiptSourceDraft::Order {
            order_id,
            expected_order_version_id,
            material_version_id,
        } => {
            let order = history
                .orders
                .iter()
                .find(|o| {
                    Some(o.order_id.as_str()) == order_id.as_deref()
                        && Some(o.id.as_str()) == expected_order_version_id.as_deref()
                })
                .ok_or_else(|| bad("주문 버전을 확인하세요"))?;
            (
                Some(order.material_id.as_str()),
                material_version_id.as_deref(),
            )
        }
    };
    material(
        history,
        mid.ok_or_else(|| bad("입고 품목을 선택하세요"))?,
        vid.ok_or_else(|| bad("입고 품목 버전을 선택하세요"))?,
        history.as_of_revision,
    )
}

fn run(
    history: &CorrectionHistory,
    events: &[CorrectionHistoryEvent],
    reviews: &[ObservationReviewDraft],
    deltas: &BTreeMap<String, i64>,
    quantity_only: bool,
    changed: &BTreeSet<String>,
    receipt_deltas: &BTreeMap<String, BTreeMap<String, i64>>,
) -> Result<Pass> {
    let mut pass = Pass::default();
    let sales = sale_sources(history, events)?;
    for event in events {
        let Some(data) = &event.effective_input else {
            continue;
        };
        let seq = number(&event.reference.logical_seq)?;
        let event_changed = changed.contains(&event.reference.original_id);
        let reference = event_ref(event, event_changed);
        let result = (|| -> Result<CorrectionEventValue> {
            match data {
                CorrectionReplacementDraft::Receipt { data } => {
                    let m = receipt_material(history, data)?;
                    parse_receipt_date(&data.receipt_date, history.today).map_err(range)?;
                    let quantity = canonical(&data.quantity, &data.unit, m)?;
                    if quantity.micros() == 0 {
                        return Err(bad("실입고 수량은 양수여야 합니다"));
                    }
                    let amount = money(&data.amount)?;
                    let (inflow, value) = match &data.stock_effect {
                        ReceiptStockEffectDraft::Unreflected {} => (quantity, amount),
                        ReceiptStockEffectDraft::NeedsReconciliation { .. } => {
                            return Err(bad("포함 근거를 명시적으로 선택하세요"));
                        }
                        ReceiptStockEffectDraft::Reconciled { selections } => {
                            let parts = selections
                                .iter()
                                .map(|s| q(&s.included_quantity))
                                .collect::<Result<Vec<_>>>()?;
                            let split =
                                split_receipt_amount(quantity, amount, &parts).map_err(range)?;
                            (split.unreflected_quantity, split.unreflected_amount)
                        }
                    };
                    if inflow.micros() > 0 {
                        let current = pass
                            .materials
                            .get(&m.material_id)
                            .map(amounts)
                            .transpose()?
                            .flatten();
                        let (q0, v0) = current.unwrap_or((
                            Quantity::from_micros(0).map_err(range)?,
                            Money::from_micros(0).map_err(range)?,
                        ));
                        let next_q = Quantity::from_micros(
                            q0.micros()
                                .checked_add(inflow.micros())
                                .ok_or_else(|| bad("보유량 범위"))?,
                        )
                        .map_err(range)?;
                        // Provenance depends on quantities, never the monetary
                        // balance before the later knowledge-time D is applied.
                        // This scratch value cannot enter the final plan.
                        let next_v = if quantity_only {
                            Money::from_micros(0).map_err(range)?
                        } else {
                            Money::from_micros(
                                v0.micros()
                                    .checked_add(value.micros())
                                    .ok_or_else(|| bad("보유 가치 범위"))?,
                            )
                            .map_err(range)?
                        };
                        write_state(
                            &mut pass,
                            &m.material_id,
                            next_q,
                            next_v,
                            reference.clone(),
                            &seq.to_string(),
                            None,
                        )?;
                        pass.topology.push(ValueReplayEvent::Inflow {
                            event: ValueEventRef {
                                id: event.reference.original_id.clone(),
                                seq,
                            },
                            material: value_material(m)?,
                            quantity: inflow,
                            amount: value,
                        });
                    }
                    if let Some(adjustments) = receipt_deltas.get(&event.reference.original_id) {
                        for (mid, delta) in adjustments {
                            if *delta == 0 {
                                continue;
                            }
                            let (current_q, current_v) = pass
                                .materials
                                .get(mid)
                                .map(amounts)
                                .transpose()?
                                .flatten()
                                .ok_or_else(|| bad("포함 가치 보정의 현재 보유 근거가 없습니다"))?;
                            let value = Money::from_micros(
                                current_v
                                    .micros()
                                    .checked_add(*delta)
                                    .ok_or_else(|| bad("포함 현재 가치 범위"))?,
                            )
                            .map_err(range)?;
                            write_state(
                                &mut pass,
                                mid,
                                current_q,
                                value,
                                reference.clone(),
                                &seq.to_string(),
                                None,
                            )?;
                        }
                    }
                    if let ReceiptSourceDraft::Order {
                        order_id,
                        expected_order_version_id,
                        ..
                    } = &data.source
                    {
                        let order = history
                            .orders
                            .iter()
                            .find(|o| {
                                Some(o.id.as_str()) == expected_order_version_id.as_deref()
                                    && Some(o.order_id.as_str()) == order_id.as_deref()
                            })
                            .ok_or_else(|| bad("주문 연결 없음"))?;
                        if pass.closures.contains_key(&order.order_id) {
                            return Err(bad("같은 주문에 유효 입고가 두 개입니다"));
                        }
                        let initial = history
                            .orders
                            .iter()
                            .filter(|o| o.order_id == order.order_id)
                            .min_by_key(|o| number(&o.seq).unwrap_or(i64::MAX))
                            .ok_or_else(|| bad("원발주 없음"))?;
                        let difference = quantity.micros() - q(&order.quantity)?.micros();
                        pass.closures.insert(
                            order.order_id.clone(),
                            OrderReceiptPreview {
                                status: OrderStatus::Received,
                                order_id: order.order_id.clone(),
                                initial_order_version_id: initial.id.clone(),
                                initial_ordered_quantity: initial.quantity.clone(),
                                ordered_version_id: order.id.clone(),
                                ordered_quantity: order.quantity.clone(),
                                received_quantity: quantity.decimal(),
                                difference: ValueDelta::from_micros(difference)
                                    .map_err(range)?
                                    .decimal(),
                                unit: m.canonical_unit.clone(),
                            },
                        );
                    }
                    Ok(CorrectionEventValue::Receipt {
                        input: data.clone(),
                        reconciliation: None,
                    })
                }
                CorrectionReplacementDraft::Production { data } => {
                    let mut selected = BTreeMap::new();
                    for (mid, vid) in data
                        .inputs
                        .iter()
                        .map(|r| (&r.material_id, &r.material_version_id))
                        .chain(
                            data.outputs
                                .iter()
                                .map(|r| (&r.material_id, &r.material_version_id)),
                        )
                    {
                        let m = material(
                            history,
                            mid.as_deref().ok_or_else(|| bad("생산 품목 없음"))?,
                            vid.as_deref().ok_or_else(|| bad("생산 버전 없음"))?,
                            history.as_of_revision,
                        )?;
                        if selected
                            .insert(m.material_id.clone(), m)
                            .is_some_and(|old| old.id != m.id)
                        {
                            return Err(bad("같은 품목의 생산 버전이 다릅니다"));
                        }
                    }
                    let contexts = selected
                        .values()
                        .map(|m| {
                            Ok(ProductionMaterialContext {
                                material_id: m.material_id.clone(),
                                material_version_id: m.id.clone(),
                                material_name: m.name.clone(),
                                unit: m.canonical_unit.clone(),
                                holding: holding(&pass, &m.material_id)?,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    let result = match project_production(data, &contexts).map_err(range)? {
                        ProductionCheck::Ready(v) => v,
                        ProductionCheck::NeedsReconciliation(conflicts) => {
                            return Err(bad(conflicts
                                .iter()
                                .map(|c| c.message())
                                .collect::<Vec<_>>()
                                .join(" / ")));
                        }
                    };
                    for effect in &result.materials {
                        let after_q = q(&effect.after.quantity)?;
                        let after_v = money(
                            effect
                                .after
                                .value
                                .as_deref()
                                .ok_or_else(|| bad("생산 가치 없음"))?,
                        )?;
                        write_state(
                            &mut pass,
                            &effect.material_id,
                            after_q,
                            after_v,
                            reference.clone(),
                            &seq.to_string(),
                            None,
                        )?;
                    }
                    let inputs = result
                        .inputs
                        .iter()
                        .map(|r| {
                            Ok(ValueProductionInput {
                                position: r.position.parse().map_err(|_| bad("생산 행 위치"))?,
                                material: value_material(selected[&r.material_id])?,
                                quantity: q(&r.quantity)?,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    let outputs = result
                        .outputs
                        .iter()
                        .map(|r| {
                            Ok(ValueProductionOutput {
                                position: r.position.parse().map_err(|_| bad("생산 행 위치"))?,
                                material: value_material(selected[&r.material_id])?,
                                quantity: q(&r.quantity)?,
                                allocation: AllocationPercent::parse(&r.allocation_percent)
                                    .map_err(range)?,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    pass.topology.push(ValueReplayEvent::Production {
                        event: ValueEventRef {
                            id: event.reference.original_id.clone(),
                            seq,
                        },
                        inputs,
                        outputs,
                    });
                    Ok(CorrectionEventValue::Production {
                        input: data.clone(),
                        result,
                    })
                }
                CorrectionReplacementDraft::Waste { data } => {
                    let m = material(
                        history,
                        data.material_id
                            .as_deref()
                            .ok_or_else(|| bad("폐기 품목 없음"))?,
                        data.material_version_id
                            .as_deref()
                            .ok_or_else(|| bad("폐기 버전 없음"))?,
                        history.as_of_revision,
                    )?;
                    let context = WasteMaterialContext {
                        material_id: m.material_id.clone(),
                        material_version_id: m.id.clone(),
                        material_name: m.name.clone(),
                        unit: m.canonical_unit.clone(),
                        holding: holding(&pass, &m.material_id)?,
                    };
                    let result = match calculate_waste(data, &context).map_err(range)? {
                        WasteCheck::Ready(v) => *v,
                        WasteCheck::NeedsReconciliation(c) => return Err(bad(c.message)),
                    };
                    write_state(
                        &mut pass,
                        &m.material_id,
                        q(&result.after.quantity)?,
                        money(
                            result
                                .after
                                .value
                                .as_deref()
                                .ok_or_else(|| bad("폐기 후 가치 없음"))?,
                        )?,
                        reference.clone(),
                        &seq.to_string(),
                        None,
                    )?;
                    pass.topology.push(ValueReplayEvent::Waste {
                        event: ValueEventRef {
                            id: event.reference.original_id.clone(),
                            seq,
                        },
                        material: value_material(m)?,
                        quantity: q(&result.quantity)?,
                    });
                    Ok(CorrectionEventValue::Waste {
                        input: data.clone(),
                        result,
                    })
                }
                CorrectionReplacementDraft::SaleCount { data } => {
                    let original = history
                        .sales
                        .iter()
                        .find(|s| s.id == event.reference.original_id)
                        .ok_or_else(|| bad("판매 원본 없음"))?;
                    let count = SaleCount::parse(&data.count).map_err(range)?;
                    let selling = original
                        .recipe_cost
                        .selling_price
                        .as_deref()
                        .map(money)
                        .transpose()?;
                    let amount = calculate_sale_amount(count, selling)
                        .map_err(range)?
                        .map(|m| m.decimal());
                    Ok(CorrectionEventValue::SaleCount {
                        input: data.clone(),
                        result: SaleProposal {
                            material_id: original.material_id.clone(),
                            material_version_id: original.material_version_id.clone(),
                            material_name: original.material_name.clone(),
                            recipe_version_id: original.recipe_version_id.clone(),
                            count: count.decimal(),
                            amount,
                            recipe_cost: original.recipe_cost.clone(),
                        },
                    })
                }
                CorrectionReplacementDraft::Stocktake { data } => {
                    for row in &event.observations {
                        let action = data
                            .rows
                            .iter()
                            .find(|r| r.observation_id.as_deref() == Some(row.id.as_str()))
                            .ok_or_else(|| bad("유효 실사 행 입력이 없습니다"))?;
                        let review = reviews
                            .iter()
                            .find(|r| r.observation_id.as_deref() == Some(row.id.as_str()));
                        let row_changed = changed.contains(&row.id);
                        observation(
                            history,
                            &mut pass,
                            event,
                            row,
                            &action.action,
                            review,
                            &sales,
                            deltas,
                            None,
                            row_changed,
                            quantity_only,
                        )?;
                        if !pass.issues.is_empty() {
                            break;
                        }
                    }
                    Ok(CorrectionEventValue::Stocktake {
                        input: data.clone(),
                    })
                }
            }
        })();
        match result {
            Ok(value) => {
                pass.values
                    .insert(event.reference.original_id.clone(), value);
            }
            Err(error) => {
                pass.issues.push(issue(
                    format!("events.{}", event.reference.original_id),
                    "invalid_input",
                    error.to_string(),
                ));
            }
        }
        if !pass.issues.is_empty() {
            break;
        }
    }
    Ok(pass)
}

fn validate_history(history: &CorrectionHistory) -> Result<Vec<CorrectionHistoryEvent>> {
    if history.as_of_revision < 0 {
        return Err(bad("음수 기준 순번"));
    }
    let mut originals = BTreeSet::new();
    let mut confirmations = BTreeSet::new();
    let mut versions = BTreeSet::new();
    let mut observations = BTreeSet::new();
    let mut observation_versions = BTreeSet::new();
    let mut result = history.events.clone();
    for e in &result {
        let r = &e.reference;
        for value in [
            &r.original_id,
            &r.original_confirmation_id,
            &r.confirmation_id,
            &r.version_id,
            &r.business_record_id,
        ] {
            id(value)?;
        }
        let logical = number(&r.logical_seq)?;
        let actual = number(&r.seq)?;
        if logical > actual
            || actual > history.as_of_revision
            || !originals.insert(r.original_id.clone())
            || !confirmations.insert(r.original_confirmation_id.clone())
            || !versions.insert(r.version_id.clone())
        {
            return Err(bad(
                "정정 사건의 원본/버전/순번이 중복되거나 일치하지 않습니다",
            ));
        }
        for time in [&r.original_recorded_at, &r.recorded_at] {
            chrono::DateTime::parse_from_rfc3339(time).map_err(|_| bad("정정 기록 시각 형식"))?;
        }
        if e.original_input.kind() != r.kind
            || e.effective_input
                .as_ref()
                .is_some_and(|i| i.kind() != r.kind)
        {
            return Err(bad("정정 사건의 종류가 다릅니다"));
        }
        for o in &e.observations {
            id(&o.id)?;
            id(&o.version_id)?;
            if r.kind != CorrectionSubjectKind::Stocktake
                || o.stocktake_id != r.original_id
                || !observations.insert(o.id.clone())
                || !observation_versions.insert(o.version_id.clone())
            {
                return Err(bad("실사 원본 관측의 연결이 잘못됐습니다"));
            }
        }
    }
    let mut actuals = BTreeSet::new();
    for current in &history.actual_materials {
        id(&current.material_id)?;
        if !actuals.insert(&current.material_id) {
            return Err(bad("실제 품목 projection 중복"));
        }
        amounts(&current.state)?;
    }
    result.sort_by(|a, b| {
        number(&a.reference.logical_seq)
            .unwrap_or(0)
            .cmp(&number(&b.reference.logical_seq).unwrap_or(0))
            .then(
                a.reference
                    .logical_position
                    .cmp(&b.reference.logical_position),
            )
            .then(a.reference.original_id.cmp(&b.reference.original_id))
    });
    Ok(result)
}

fn empty_plan(target: CorrectionEventRef) -> CorrectionPlan {
    CorrectionPlan {
        target,
        events: vec![],
        materials: vec![],
        observations: vec![],
        allocations: vec![],
        inclusions: vec![],
        separate_adjustments: vec![],
        order_closures: vec![],
        review_requests: vec![],
    }
}

fn replace_event(
    event: &mut CorrectionHistoryEvent,
    replacement: &CorrectionReplacementDraft,
    changed: &mut BTreeSet<String>,
    issues: &mut Vec<BusinessIssue>,
    field: &str,
) {
    if replacement.kind() != event.reference.kind {
        issues.push(issue(
            field,
            "invalid_input",
            "원사건과 대체 입력의 종류가 다릅니다.",
        ));
        return;
    }
    if let CorrectionReplacementDraft::Stocktake { data } = replacement {
        let mut rows = match &event.effective_input {
            Some(CorrectionReplacementDraft::Stocktake { data }) => data.rows.clone(),
            _ => match &event.original_input {
                CorrectionReplacementDraft::Stocktake { data } => data
                    .rows
                    .iter()
                    .map(|r| StocktakeCorrectionRowDraft {
                        observation_id: r.observation_id.clone(),
                        expected_version_id: r.expected_version_id.clone(),
                        action: ObservationCorrectionActionDraft::Void {},
                    })
                    .collect(),
                _ => vec![],
            },
        };
        let mut seen = BTreeSet::new();
        for (position, row) in data.rows.iter().enumerate() {
            let path = format!("{field}.rows[{position}]");
            let Some(observation_id) = row.observation_id.as_deref() else {
                issues.push(issue(
                    &path,
                    "missing_reference",
                    "정정할 기존 관측을 선택하세요.",
                ));
                continue;
            };
            let Some(original) = event.observations.iter().find(|o| o.id == observation_id) else {
                issues.push(issue(
                    &path,
                    "missing_reference",
                    "이 실사에 속한 기존 관측을 선택하세요.",
                ));
                continue;
            };
            if row.expected_version_id.as_deref() != Some(original.version_id.as_str())
                || !seen.insert(observation_id)
            {
                issues.push(issue(
                    &path,
                    "stale_reference",
                    "관측의 현재 버전과 중복 선택을 확인하세요.",
                ));
                continue;
            }
            if let Some(existing) = rows
                .iter_mut()
                .find(|r| r.observation_id == row.observation_id)
            {
                *existing = row.clone();
            } else {
                rows.push(row.clone());
            }
            changed.insert(observation_id.into());
        }
        event.effective_input = Some(CorrectionReplacementDraft::Stocktake {
            data: DraftStocktakeCorrectionV1 { rows },
        });
    } else {
        event.effective_input = Some(replacement.clone());
    }
    changed.insert(event.reference.original_id.clone());
}

fn select_changes(
    history: &CorrectionHistory,
    events: &mut [CorrectionHistoryEvent],
    target_index: usize,
    draft: &DraftCorrectionV1,
) -> (BTreeSet<String>, Vec<BusinessIssue>, Vec<(String, String)>) {
    let mut changed = BTreeSet::new();
    let mut issues = vec![];
    let mut fields = vec![];
    if draft.reason.trim().is_empty() {
        issues.push(issue("reason", "invalid_input", "정정 이유를 입력하세요."));
    }
    if draft.expected_target_version_id.as_deref()
        != Some(events[target_index].reference.version_id.as_str())
    {
        issues.push(issue(
            "expected_target_version_id",
            "stale_reference",
            "현재 유효 개정을 다시 검토하세요.",
        ));
        return (changed, issues, fields);
    }
    let target = &mut events[target_index];
    match &draft.action {
        CorrectionActionDraft::Void {} => {
            target.effective_input = None;
            changed.insert(target.reference.original_id.clone());
            for o in &target.observations {
                changed.insert(o.id.clone());
            }
        }
        CorrectionActionDraft::Replace { replacement } => replace_event(
            target,
            replacement,
            &mut changed,
            &mut issues,
            "action.replacement.data",
        ),
    }
    if target.reference.kind == CorrectionSubjectKind::Receipt {
        fields.push((
            target.reference.original_id.clone(),
            "action.replacement.data".into(),
        ));
    }
    let target_id = target.reference.original_id.clone();
    let mut receipts = BTreeSet::new();
    for (position, row) in draft.linked_receipt_changes.iter().enumerate() {
        let field = format!("linked_receipt_changes[{position}]");
        let Some(receipt) = row.receipt_id.as_deref() else {
            issues.push(issue(
                &field,
                "missing_reference",
                "관련 입고를 선택하세요.",
            ));
            continue;
        };
        let Some(event) = events.iter_mut().find(|e| {
            e.reference.kind == CorrectionSubjectKind::Receipt && e.reference.original_id == receipt
        }) else {
            issues.push(issue(
                &field,
                "missing_reference",
                "관련 입고 원본이 없습니다.",
            ));
            continue;
        };
        if receipt == target_id
            || !receipts.insert(receipt)
            || row.expected_version_id.as_deref() != Some(event.reference.version_id.as_str())
        {
            issues.push(issue(
                &field,
                "stale_reference",
                "관련 입고의 현재 버전과 중복 선택을 확인하세요.",
            ));
            continue;
        }
        if !history
            .existing_inclusions
            .iter()
            .any(|r| r.receipt_id == receipt)
        {
            issues.push(issue(
                &field,
                "invalid_input",
                "포함 근거가 영향을 받는 기존 입고만 함께 변경할 수 있습니다.",
            ));
            continue;
        }
        match &row.action {
            LinkedReceiptActionDraft::Void {} => {
                event.effective_input = None;
                changed.insert(receipt.into());
            }
            LinkedReceiptActionDraft::Replace { data } => replace_event(
                event,
                &CorrectionReplacementDraft::Receipt { data: data.clone() },
                &mut changed,
                &mut issues,
                &format!("{field}.action.data"),
            ),
        }
        fields.push((receipt.into(), format!("{field}.action.data")));
    }
    let mut reviews = BTreeSet::new();
    for (position, row) in draft.observation_reviews.iter().enumerate() {
        let field = format!("observation_reviews[{position}]");
        let Some(o) = events
            .iter()
            .flat_map(|e| &e.observations)
            .find(|o| Some(o.id.as_str()) == row.observation_id.as_deref())
        else {
            issues.push(issue(
                &field,
                "missing_reference",
                "검토할 기존 관측을 선택하세요.",
            ));
            continue;
        };
        if row.expected_version_id.as_deref() != Some(o.version_id.as_str())
            || !reviews.insert(&o.id)
        {
            issues.push(issue(
                &field,
                "stale_reference",
                "검토 관측의 현재 버전과 중복 선택을 확인하세요.",
            ));
            continue;
        }
        changed.insert(o.id.clone());
    }
    (changed, issues, fields)
}

fn validate_base(history: &CorrectionHistory, base: &Pass) -> Result<()> {
    if !base.issues.is_empty() {
        return Err(bad(format!(
            "기존 유효 원장 재생 실패: {}",
            base.issues[0].message
        )));
    }
    let mut actual = BTreeMap::new();
    for current in &history.actual_materials {
        actual.insert(current.material_id.as_str(), &current.state);
    }
    for (mid, state) in &base.materials {
        let expected = actual
            .get(mid.as_str())
            .ok_or_else(|| bad(format!("현재 projection이 없는 품목: {mid}")))?;
        if amounts(state)? != amounts(expected)? {
            return Err(bad("기존 유효 재생과 실제 현재 수량/가치가 다릅니다"));
        }
        if let (
            CorrectionHoldingState::Known {
                last_valid_price: l,
                observation_boundary: o,
                ..
            },
            CorrectionHoldingState::Known {
                last_valid_price: r,
                observation_boundary: p,
                ..
            },
        ) = (state, *expected)
            && (l.as_ref().map(|v| (&v.amount, &v.quantity))
                != r.as_ref().map(|v| (&v.amount, &v.quantity))
                || o.as_ref().map(|v| (&v.logical_seq, &v.quantity))
                    != p.as_ref().map(|v| (&v.logical_seq, &v.quantity)))
        {
            return Err(bad(
                "기존 유효 마지막 단가/관측 구간이 실제 현재와 다릅니다",
            ));
        }
    }
    for current in &history.actual_materials {
        if !base.materials.contains_key(&current.material_id) && amounts(&current.state)?.is_some()
        {
            return Err(bad("실제 보유량의 유효 수량 원천이 없습니다"));
        }
    }
    for current in &history.actual_allocations {
        let row = base
            .observations
            .get(&current.observation_id)
            .ok_or_else(|| bad("현재 소모 배분의 유효 관측이 없습니다"))?;
        let StocktakeCalculation::Settlement { allocation, .. } = &row.calculation else {
            return Err(bad("초기 관측에 소모 배분이 연결됐습니다"));
        };
        let mut expected = current.allocation.clone();
        expected.as_of_revision = allocation.as_of_revision.clone();
        // A projection can carry a new observation/basis UUID; the interval and
        // all candidate versions/amounts must still reproduce exactly.
        expected.basis.id = allocation.basis.id.clone();
        if expected.basis.kind == HoldingBasisKind::Correction {
            // Stored projections refer to their actual correction row. Replay
            // uses the original logical source, with the same interval start.
            expected.basis.kind = allocation.basis.kind;
        }
        if !same(&expected, allocation.as_ref()) {
            return Err(bad("기존 소모 배분을 재현할 수 없습니다"));
        }
    }
    Ok(())
}

fn inclusion_observations(
    events: &[CorrectionHistoryEvent],
    pass: &Pass,
    changed: &BTreeSet<String>,
) -> Result<Vec<super::inclusion::CorrectionReplayObservation>> {
    let mut rows = vec![];
    for event in events {
        for row in &event.observations {
            if let Some(effect) = pass.observations.get(&row.id) {
                rows.push(super::inclusion::CorrectionReplayObservation {
                    id: row.id.clone(),
                    expected_version_id: row.version_id.clone(),
                    reference: observation_ref(row, changed.contains(&row.id)),
                    stocktake_id: row.stocktake_id.clone(),
                    logical_seq: number(&event.reference.logical_seq)?,
                    effect: effect.clone(),
                });
            }
        }
    }
    Ok(rows)
}

fn signed(value: i128) -> Result<String> {
    ValueDelta::from_micros(i64::try_from(value).map_err(|_| bad("차이 합계 범위"))?)
        .map(ValueDelta::decimal)
        .map_err(range)
}

fn allocation_of(effect: Option<&StocktakeRowEffect>) -> Option<ConsumptionAllocation> {
    match effect.map(|e| &e.calculation) {
        Some(StocktakeCalculation::Settlement { allocation, .. }) => {
            Some(allocation.as_ref().clone())
        }
        _ => None,
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "Plan assembly explicitly compares base and proposed events and passes while retaining the target and inclusion evidence"
)]
fn make_plan(
    history: &CorrectionHistory,
    base_events: &[CorrectionHistoryEvent],
    events: &[CorrectionHistoryEvent],
    base: &Pass,
    next: &Pass,
    target: CorrectionEventRef,
    changed: &BTreeSet<String>,
    receipts: &[(String, ReconciledReceiptEffect)],
) -> Result<CorrectionPlan> {
    let mut plan = empty_plan(target);
    for e in events {
        let before = base.values.get(&e.reference.original_id).cloned();
        let after = next.values.get(&e.reference.original_id).cloned();
        if changed.contains(&e.reference.original_id) || !same(&before, &after) {
            plan.events.push(CorrectionEventChange {
                reference: e.reference.clone(),
                before,
                after,
            });
        }
    }
    let mids = history
        .actual_materials
        .iter()
        .map(|m| m.material_id.clone())
        .chain(next.materials.keys().cloned())
        .collect::<BTreeSet<_>>();
    for mid in mids {
        let before = history
            .actual_materials
            .iter()
            .find(|m| m.material_id == mid)
            .map(|m| m.state.clone())
            .unwrap_or(CorrectionHoldingState::NotEstablished {});
        let after = next
            .materials
            .get(&mid)
            .cloned()
            .unwrap_or(CorrectionHoldingState::NotEstablished {});
        let b = amounts(&before)?;
        let a = amounts(&after)?;
        let unchanged = b == a
            && match (&before, &after) {
                (
                    CorrectionHoldingState::NotEstablished {},
                    CorrectionHoldingState::NotEstablished {},
                ) => true,
                (
                    CorrectionHoldingState::Known {
                        last_valid_price: b,
                        observation_boundary: c,
                        initial_boundary: d,
                        ..
                    },
                    CorrectionHoldingState::Known {
                        last_valid_price: e,
                        observation_boundary: f,
                        initial_boundary: g,
                        ..
                    },
                ) => {
                    b.as_ref().map(|p| (&p.amount, &p.quantity, &p.logical_seq))
                        == e.as_ref().map(|p| (&p.amount, &p.quantity, &p.logical_seq))
                        && c.as_ref().map(|o| (&o.logical_seq, &o.quantity))
                            == f.as_ref().map(|o| (&o.logical_seq, &o.quantity))
                        // Once an observation exists, that logical boundary
                        // supersedes the initial one. Storage need not repeat
                        // the earlier origin in the current holding snapshot.
                        && (c.is_some()
                            || d.as_ref().map(|x| &x.logical_seq)
                                == g.as_ref().map(|x| &x.logical_seq))
                }
                _ => false,
            };
        if unchanged {
            continue;
        }
        let m = history
            .materials
            .iter()
            .filter(|m| m.material_id == mid)
            .max_by_key(|m| m.seq.as_deref().and_then(|s| number(s).ok()).unwrap_or(0))
            .ok_or_else(|| bad("변경 품목 정보 없음"))?;
        plan.materials.push(CorrectionMaterialChange {
            material_id: mid,
            material_version_id: m.id.clone(),
            material_name: m.name.clone(),
            unit: m.canonical_unit.clone(),
            before,
            after,
            quantity_delta: signed(
                i128::from(a.map(|v| v.0.micros()).unwrap_or(0))
                    - i128::from(b.map(|v| v.0.micros()).unwrap_or(0)),
            )?,
            value_delta: signed(
                i128::from(a.map(|v| v.1.micros()).unwrap_or(0))
                    - i128::from(b.map(|v| v.1.micros()).unwrap_or(0)),
            )?,
        });
    }
    for e in events {
        for o in &e.observations {
            let before = base.observations.get(&o.id).cloned();
            let after = next.observations.get(&o.id).cloned();
            if changed.contains(&o.id) || !same(&before, &after) {
                plan.observations.push(CorrectionObservationChange {
                    reference: observation_ref(o, true),
                    stocktake_id: o.stocktake_id.clone(),
                    logical_seq: e.reference.logical_seq.clone(),
                    before: before.clone(),
                    after: after.clone(),
                    valuation_basis: next.observation_prices.get(&o.id).cloned(),
                });
            }
            let old = allocation_of(before.as_ref());
            let new = allocation_of(after.as_ref());
            if !same(&old, &new) {
                plan.allocations.push(CorrectionAllocationChange {
                    observation: observation_ref(o, true),
                    expected_consumption_version_id: history
                        .actual_allocations
                        .iter()
                        .find(|a| a.observation_id == o.id)
                        .map(|a| a.consumption_version_id.clone()),
                    before: old,
                    after: new,
                });
            }
        }
    }
    let receipt_ids = history
        .existing_inclusions
        .iter()
        .map(|r| r.receipt_id.clone())
        .chain(receipts.iter().map(|r| r.0.clone()))
        .collect::<BTreeSet<_>>();
    for rid in receipt_ids {
        let old = history
            .existing_inclusions
            .iter()
            .find(|r| r.receipt_id == rid)
            .map(|r| r.reconciliation.effect.clone());
        let new = receipts.iter().find(|r| r.0 == rid).map(|r| r.1.clone());
        let base_effect = match base.values.get(&rid) {
            Some(CorrectionEventValue::Receipt { reconciliation, .. }) => reconciliation.clone(),
            _ => old.clone(),
        };
        if changed.contains(&rid) || !same(&base_effect, &new) {
            let event = base_events
                .iter()
                .find(|e| e.reference.original_id == rid)
                .ok_or_else(|| bad("포함 변경 원입고 없음"))?;
            let before = old
                .as_ref()
                .map(|v| v.separate_adjustment.as_str())
                .unwrap_or("0");
            let after = new
                .as_ref()
                .map(|v| v.separate_adjustment.as_str())
                .unwrap_or("0");
            let parse_signed = |v: &str| -> Result<i64> {
                let m = money(v.strip_prefix('-').unwrap_or(v))?.micros();
                Ok(if v.starts_with('-') { -m } else { m })
            };
            if before != after {
                plan.separate_adjustments
                    .push(CorrectionSeparateAdjustmentChange {
                        receipt_id: rid.clone(),
                        before: before.into(),
                        after: after.into(),
                        delta: signed(
                            i128::from(parse_signed(after)?) - i128::from(parse_signed(before)?),
                        )?,
                    });
            }
            plan.inclusions.push(CorrectionInclusionChange {
                receipt_id: rid,
                expected_version_id: event.reference.version_id.clone(),
                before: old,
                after: new,
            });
        }
    }
    let orders = base
        .closures
        .keys()
        .chain(next.closures.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for order_id in orders {
        let before = base.closures.get(&order_id).cloned();
        let after = next.closures.get(&order_id).cloned();
        if !same(&before, &after) {
            let expected = history
                .orders
                .iter()
                .filter(|o| o.order_id == order_id)
                .max_by_key(|o| number(&o.seq).unwrap_or(0))
                .ok_or_else(|| bad("주문 현재 버전 없음"))?;
            plan.order_closures.push(CorrectionOrderClosureChange {
                order_id,
                expected_order_version_id: expected.id.clone(),
                before,
                after,
            });
        }
    }
    Ok(plan)
}

fn clear_amount_references(value: &mut CorrectionEventValue) {
    let clear = |amounts: &mut HoldingAmounts| {
        amounts.source_id = None;
    };
    match value {
        CorrectionEventValue::Production { result, .. } => {
            for material in &mut result.materials {
                if let Some(before) = &mut material.before {
                    clear(before);
                }
                clear(&mut material.after);
            }
        }
        CorrectionEventValue::Waste { result, .. } => {
            clear(&mut result.before);
            clear(&mut result.after);
        }
        CorrectionEventValue::Receipt {
            reconciliation: Some(effect),
            ..
        } => {
            for material in &mut effect.materials {
                if let Some(before) = &mut material.before {
                    clear(before);
                }
                clear(&mut material.after);
            }
        }
        _ => {}
    }
}

fn sanitize_plan(plan: &mut CorrectionPlan) {
    for event in &mut plan.events {
        if let Some(before) = &mut event.before {
            clear_amount_references(before);
        }
        if let Some(after) = &mut event.after {
            clear_amount_references(after);
        }
    }
    for observation in &mut plan.observations {
        for effect in [&mut observation.before, &mut observation.after]
            .into_iter()
            .flatten()
        {
            if let Some(before) = &mut effect.before {
                before.source_id = None;
            }
            effect.after.source_id = None;
        }
        if observation.valuation_basis.is_some()
            && let Some(effect) = &mut observation.after
        {
            let valuation = match &mut effect.calculation {
                StocktakeCalculation::Initial { valuation, .. } => Some(valuation),
                StocktakeCalculation::Settlement { valuation, .. } => valuation.as_mut(),
            };
            if let Some(StocktakeValuation::ExistingPrice { source_id, .. }) = valuation {
                // Only the persisted derived row may contain an actual price ID.
                source_id.clear();
            }
        }
    }
    for inclusion in &mut plan.inclusions {
        if let Some(effect) = &mut inclusion.after {
            for material in &mut effect.materials {
                if let Some(before) = &mut material.before {
                    before.source_id = None;
                }
                material.after.source_id = None;
            }
        }
    }
    // Material.before is the actual snapshot head and deliberately retains its
    // real holding reference. All derived intermediate IDs are plan references.
}

fn effect_replacements(
    effect: &ReconciledReceiptEffect,
    events: &[CorrectionHistoryEvent],
) -> Result<Vec<ValueReplacement>> {
    let mut rows = vec![];
    for selection in &effect.selections {
        let observation = events
            .iter()
            .flat_map(|e| &e.observations)
            .find(|o| o.id == selection.observation_id)
            .ok_or_else(|| bad("포함 관측이 없습니다"))?;
        for p in &selection.portions {
            rows.push(ValueReplacement {
                observation_version_id: observation.version_id.clone(),
                material_id: effect.material_id.clone(),
                source_key: p.source_key.clone(),
                selection_position: selection.position,
                portion_position: p.position,
                booked_value: money(&p.booked_value)?,
                actual_included_value: money(&p.actual_included_value)?,
            });
        }
    }
    Ok(rows)
}

fn latest_material<'a>(history: &'a CorrectionHistory, mid: &str) -> Result<&'a MaterialVersion> {
    history
        .materials
        .iter()
        .filter(|m| m.material_id == mid)
        .max_by_key(|m| m.seq.as_deref().and_then(|s| number(s).ok()).unwrap_or(0))
        .ok_or_else(|| bad("품목 정보가 없습니다"))
}

/// Knowledge-time pass: a late receipt adjusts the then-current ledger once.
/// Its backward shadow replay does not rewrite already confirmed K/production
/// values. Later observations consume the adjusted current value normally.
fn run_known(
    history: &CorrectionHistory,
    events: &[CorrectionHistoryEvent],
    reviews: &[ObservationReviewDraft],
    changed: &BTreeSet<String>,
    resolved: &[(String, ReconciledReceiptEffect)],
) -> Result<(Pass, Vec<(String, ReconciledReceiptEffect)>)> {
    use super::inclusion::refresh_correction_receipt_values;
    let mut receipt_deltas = BTreeMap::<String, BTreeMap<String, i64>>::new();
    let mut previous = vec![];
    let mut effective = vec![];
    let changed_receipts = events
        .iter()
        .filter(|e| {
            e.reference.kind == CorrectionSubjectKind::Receipt
                && changed.contains(&e.reference.original_id)
        })
        .map(|e| e.reference.original_id.clone())
        .collect::<Vec<_>>();
    for (position, event) in events.iter().enumerate() {
        let Some(CorrectionReplacementDraft::Receipt { data }) = &event.effective_input else {
            continue;
        };
        if !matches!(
            data.stock_effect,
            ReceiptStockEffectDraft::Reconciled { .. }
        ) {
            continue;
        }
        let before = run(
            history,
            &events[..position],
            reviews,
            &BTreeMap::new(),
            false,
            changed,
            &receipt_deltas,
        )?;
        if !before.issues.is_empty() {
            return Ok((before, previous));
        }
        let provisional = resolved
            .iter()
            .find(|(id, _)| id == &event.reference.original_id)
            .map(|(_, e)| e)
            .ok_or_else(|| bad("유효 보정 입고의 포함 계획이 없습니다"))?;
        let observations = inclusion_observations(events, &before, changed)?;
        let mut effect = refresh_correction_receipt_values(
            history,
            &event.reference.original_id,
            provisional,
            &observations,
            &previous,
            &changed_receipts,
        )?;
        let proposed = effect_replacements(&effect, events)?;
        let mut actual = vec![];
        for (mid, state) in &before.materials {
            if let Some((quantity, value)) = amounts(state)? {
                actual.push(ValueActualAmounts {
                    material: value_material(latest_material(history, mid)?)?,
                    quantity,
                    value,
                });
            }
        }
        let comparison = replay_effective_values(
            number(&event.reference.logical_seq)? - 1,
            &before.topology,
            &effective,
            &proposed,
            &actual,
        )
        .map_err(|e| bad(e.to_string()))?;
        effect.trace = comparison.trace;
        effect.separate_adjustment = comparison.separate_adjustment.decimal();
        effect.materials.clear();
        let mut adjustments = BTreeMap::new();
        let inflow_q = q(&effect.unreflected_quantity)?;
        let inflow_v = money(&effect.unreflected_amount)?;
        let mut material_ids = comparison
            .materials
            .iter()
            .filter(|m| m.delta.micros() != 0)
            .map(|m| m.material_id.clone())
            .collect::<BTreeSet<_>>();
        if inflow_q.micros() > 0 {
            material_ids.insert(effect.material_id.clone());
        }
        for delta in &comparison.materials {
            adjustments.insert(delta.material_id.clone(), delta.delta.micros());
        }
        for mid in material_ids {
            let old = before
                .materials
                .get(&mid)
                .map(amounts)
                .transpose()?
                .flatten();
            let (old_q, old_v) = old.unwrap_or((
                Quantity::from_micros(0).map_err(range)?,
                Money::from_micros(0).map_err(range)?,
            ));
            let iq = if mid == effect.material_id {
                inflow_q.micros()
            } else {
                0
            };
            let iv = if mid == effect.material_id {
                inflow_v.micros()
            } else {
                0
            };
            let d = *adjustments.get(&mid).unwrap_or(&0);
            let after_q = Quantity::from_micros(
                old_q
                    .micros()
                    .checked_add(iq)
                    .ok_or_else(|| bad("입고 후 수량 범위"))?,
            )
            .map_err(range)?;
            let after_v = Money::from_micros(
                old_v
                    .micros()
                    .checked_add(iv)
                    .and_then(|v| v.checked_add(d))
                    .ok_or_else(|| bad("입고 후 가치 범위"))?,
            )
            .map_err(range)?;
            let m = latest_material(history, &mid)?;
            let context = holding(&before, &mid)?;
            effect.materials.push(ReconciliationMaterialEffect {
                material_id: mid.clone(),
                material_version_id: m.id.clone(),
                material_name: m.name.clone(),
                unit: m.canonical_unit.clone(),
                before: old.map(|(q, v)| HoldingAmounts {
                    quantity: q.decimal(),
                    value: Some(v.decimal()),
                    source_id: context.source_id.clone(),
                }),
                after: HoldingAmounts {
                    quantity: after_q.decimal(),
                    value: Some(after_v.decimal()),
                    source_id: None,
                },
                basis_before: context.basis,
                observation: context.observation,
                value_delta: signed(i128::from(d))?,
                inflow_quantity: Quantity::from_micros(iq).map_err(range)?.decimal(),
                inflow_value: Money::from_micros(iv).map_err(range)?.decimal(),
            });
        }
        receipt_deltas.insert(event.reference.original_id.clone(), adjustments);
        effective.extend(proposed);
        previous.push((event.reference.original_id.clone(), effect));
    }
    let mut result = run(
        history,
        events,
        reviews,
        &BTreeMap::new(),
        false,
        changed,
        &receipt_deltas,
    )?;
    for (id, effect) in &previous {
        if let Some(CorrectionEventValue::Receipt { reconciliation, .. }) =
            result.values.get_mut(id)
        {
            *reconciliation = Some(effect.clone());
        }
    }
    Ok((result, previous))
}

/// Original position controls quantities and allocations. Actual correction
/// time controls version CAS; receipt knowledge time controls D versus closed K.
pub fn build_correction_plan(
    history: &CorrectionHistory,
    target_confirmation_id: &str,
    draft: &DraftCorrectionV1,
) -> Result<CorrectionCheck> {
    use super::inclusion::{CorrectionInclusionInput, resolve_correction_inclusions};
    let original = validate_history(history)?;
    let target_index = original
        .iter()
        .position(|e| e.reference.original_confirmation_id == target_confirmation_id)
        .ok_or_else(|| bad("정정 원사건을 찾을 수 없습니다"))?;
    let target = original[target_index].reference.clone();
    let old_effects = history
        .existing_inclusions
        .iter()
        .map(|r| (r.receipt_id.clone(), r.reconciliation.effect.clone()))
        .collect::<Vec<_>>();
    let (base, _) = run_known(history, &original, &[], &BTreeSet::new(), &old_effects)?;
    validate_base(history, &base)?;
    let mut normalized = draft.clone();
    if let Err(error) = normalize_correction_draft(&mut normalized) {
        return Ok(CorrectionCheck {
            plan: empty_plan(target),
            issues: vec![issue("action", "invalid_input", error.0)],
        });
    }
    let mut proposed = original.clone();
    let (changed, mut issues, fields) =
        select_changes(history, &mut proposed, target_index, &normalized);
    if !issues.is_empty() {
        return Ok(CorrectionCheck {
            plan: empty_plan(target),
            issues,
        });
    }
    // Only quantities/source capacities are used from this scratch pass. Full
    // values and valuation decisions are checked again by run_known below.
    let raw = run(
        history,
        &proposed,
        &normalized.observation_reviews,
        &BTreeMap::new(),
        true,
        &changed,
        &BTreeMap::new(),
    )?;
    if !raw.issues.is_empty() {
        let mut plan = empty_plan(target);
        plan.review_requests = raw.reviews;
        return Ok(CorrectionCheck {
            plan,
            issues: raw.issues,
        });
    }
    let observations = inclusion_observations(&proposed, &raw, &changed)?;
    let receipt_ids = proposed
        .iter()
        .filter(|e| {
            e.reference.kind == CorrectionSubjectKind::Receipt
                && changed.contains(&e.reference.original_id)
        })
        .map(|e| e.reference.original_id.clone())
        .collect::<Vec<_>>();
    let observation_ids = observations
        .iter()
        .filter(|o| changed.contains(&o.id))
        .map(|o| o.id.clone())
        .collect::<Vec<_>>();
    let inclusion = resolve_correction_inclusions(&CorrectionInclusionInput {
        history,
        events: &proposed,
        topology: &raw.topology,
        observations: &observations,
        changed_receipt_ids: &receipt_ids,
        changed_observation_ids: &observation_ids,
        receipt_fields: &fields,
    })?;
    if !inclusion.issues.is_empty() {
        let mut plan = empty_plan(target);
        plan.review_requests = inclusion.review_requests;
        return Ok(CorrectionCheck {
            plan,
            issues: inclusion.issues,
        });
    }
    let (next, receipts) = match run_known(
        history,
        &proposed,
        &normalized.observation_reviews,
        &changed,
        &inclusion.receipts,
    ) {
        Ok(result) => result,
        Err(error) => {
            return Ok(CorrectionCheck {
                plan: empty_plan(target),
                issues: vec![issue(
                    "linked_receipt_changes",
                    "invalid_input",
                    error.to_string(),
                )],
            });
        }
    };
    if !next.issues.is_empty() {
        let mut plan = empty_plan(target);
        plan.review_requests = next.reviews;
        return Ok(CorrectionCheck {
            plan,
            issues: next.issues,
        });
    }
    let mut plan = make_plan(
        history, &original, &proposed, &base, &next, target, &changed, &receipts,
    )?;
    plan.review_requests.extend(inclusion.review_requests);
    for (position, row) in normalized.linked_receipt_changes.iter().enumerate() {
        if let Some(id) = &row.receipt_id {
            let old = history
                .existing_inclusions
                .iter()
                .find(|r| &r.receipt_id == id)
                .ok_or_else(|| bad("연결 입고 원본 없음"))?;
            let affected = old.reconciliation.effect.selections.iter().any(|s| {
                changed.contains(&s.observation_id)
                    || s.portions
                        .iter()
                        .any(|p| changed.contains(&p.source_key.id))
                    || base
                        .observations
                        .get(&s.observation_id)
                        .zip(next.observations.get(&s.observation_id))
                        .is_none_or(|(a, b)| !same(a, b))
            });
            if !affected {
                issues.push(issue(
                    format!("linked_receipt_changes[{position}]"),
                    "invalid_input",
                    "정정으로 실제 근거가 영향을 받는 입고만 함께 변경하세요.",
                ));
            }
        }
    }
    sanitize_plan(&mut plan);
    Ok(CorrectionCheck { plan, issues })
}

/// Feed ordinary post-correction receipt/provenance queries from the effective
/// ledger. This is a pure adapter, not an additional correction or a new writer.
pub fn effective_correction_replay(
    history: &CorrectionHistory,
) -> Result<CorrectionEffectiveReplay> {
    use crate::reconciliation::{
        SourceKind,
        provenance::{ProvenancePortion, ReservedInclusion},
    };
    let events = validate_history(history)?;
    let old = history
        .existing_inclusions
        .iter()
        .map(|r| (r.receipt_id.clone(), r.reconciliation.effect.clone()))
        .collect::<Vec<_>>();
    let (pass, receipts) = run_known(history, &events, &[], &BTreeSet::new(), &old)?;
    validate_base(history, &pass)?;
    let observations = inclusion_observations(&events, &pass, &BTreeSet::new())?;
    let mut source_refs = vec![];
    for e in &events {
        if e.effective_input.is_none() {
            continue;
        }
        if e.reference.kind == CorrectionSubjectKind::Receipt {
            source_refs.push(CorrectionSourceVersion {
                source_key: SourceKey {
                    kind: SourceKind::ReceiptInflow,
                    id: e.reference.original_id.clone(),
                    position: 0,
                },
                calculation_version_id: e.reference.original_id.clone(),
                actual_version_id: e.reference.version_id.clone(),
            });
        }
        for o in &e.observations {
            if !pass.observations.contains_key(&o.id) {
                continue;
            }
            for kind in [
                SourceKind::ObservationInitial,
                SourceKind::ObservationExcess,
            ] {
                source_refs.push(CorrectionSourceVersion {
                    source_key: SourceKey {
                        kind,
                        id: o.id.clone(),
                        position: 0,
                    },
                    calculation_version_id: o.version_id.clone(),
                    actual_version_id: o.version_id.clone(),
                });
            }
        }
    }
    let mut replacements = vec![];
    let mut reservations = vec![];
    for (rid, effect) in receipts {
        replacements.extend(effect_replacements(&effect, &events)?);
        let record = history
            .existing_inclusions
            .iter()
            .find(|r| r.receipt_id == rid)
            .ok_or_else(|| bad("유효 포함 기록이 없습니다"))?;
        for selection in &effect.selections {
            let quantity = q(&selection.included_quantity)?;
            if quantity.micros() == 0 {
                continue;
            }
            let identity = record
                .reconciliation
                .selection_results
                .iter()
                .find(|r| r.selection_position == selection.position)
                .ok_or_else(|| bad("유효 포함 예약의 실제 ID가 없습니다"))?;
            let observation = observations
                .iter()
                .find(|o| o.id == selection.observation_id)
                .ok_or_else(|| bad("예약 관측이 없습니다"))?;
            let portions = selection
                .portions
                .iter()
                .map(|p| {
                    let source = source_refs
                        .iter()
                        .find(|s| s.source_key == p.source_key)
                        .ok_or_else(|| bad("예약 원천이 없습니다"))?;
                    if p.source_version_id != source.actual_version_id
                        && p.source_version_id != source.calculation_version_id
                    {
                        return Err(bad("예약 원천의 실제 유효 버전이 다릅니다"));
                    }
                    Ok(ProvenancePortion {
                        source_key: p.source_key.clone(),
                        source_version_id: source.calculation_version_id.clone(),
                        included_share: p.included_share.clone(),
                        reserved_source_share: p.reserved_source_share.clone(),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            reservations.push(ReservedInclusion {
                id: identity.reservation_id.clone(),
                observation_version_id: observation.expected_version_id.clone(),
                material_id: effect.material_id.clone(),
                quantity,
                portions,
            });
        }
    }
    Ok(CorrectionEffectiveReplay {
        events: pass.topology,
        replacements,
        reservations,
        observations,
        source_refs,
    })
}
