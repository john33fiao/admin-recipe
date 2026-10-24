//! Inclusion planning over the already recomputed physical timeline. Frozen
//! reservations keep their amounts; explicitly edited receipts get new shares.
//! This pass does not materialize holdings or allocate database identifiers.
use super::*;
use crate::{
    domain::{Money, Quantity, Unit},
    receipt::{ReceiptSourceDraft, ReceiptStockEffectDraft},
    reconciliation::{
        InclusionPortionEffect, InclusionSourceSelection, ReceiptInclusionEffect, SourceKind,
        ValueDelta, ValueReplacement, ValueReplayEvent,
        provenance::{
            InclusionRequest, ProvenancePortion, ReservedInclusion, allocate_share_amount,
            inclusion_availability, resolve_inclusions,
        },
        split_receipt_amount,
    },
    stocktake::StocktakeCalculation,
};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{ToPrimitive, Zero};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct CorrectionReplayObservation {
    pub id: String,
    pub expected_version_id: String,
    pub reference: CorrectionPlanRef,
    pub stocktake_id: String,
    pub logical_seq: i64,
    /// Value known at the original observation position, before replacements
    /// booked by later receipts. Earlier receipts' already known value stays.
    pub effect: StocktakeRowEffect,
}

pub struct CorrectionInclusionInput<'a> {
    pub history: &'a CorrectionHistory,
    pub events: &'a [CorrectionHistoryEvent],
    pub topology: &'a [ValueReplayEvent],
    pub observations: &'a [CorrectionReplayObservation],
    pub changed_receipt_ids: &'a [String],
    pub changed_observation_ids: &'a [String],
    /// Stable receipt ID and its actual draft field prefix, without a suffix.
    pub receipt_fields: &'a [(String, String)],
}

#[derive(Debug, Clone, Default)]
pub struct CorrectionInclusionOutput {
    /// Complete effective replacement set, not an additional set of deltas.
    pub replacements: Vec<ValueReplacement>,
    /// Source shares are complete; edited Bv/Ai must be refreshed in original
    /// receipt order against observations' then-known values. The caller also
    /// fills materials, trace and separate_adjustment before saving.
    pub receipts: Vec<(String, ReconciledReceiptEffect)>,
    /// These entries contain provisional zero Bv and cannot be persisted until
    /// refresh_correction_receipt_values succeeds at their logical positions.
    pub requires_value_refresh: Vec<String>,
    pub issues: Vec<BusinessIssue>,
    pub review_requests: Vec<CorrectionReviewRequest>,
}

struct EditedReceipt<'a> {
    event: &'a CorrectionHistoryEvent,
    draft: &'a DraftReceiptV1,
    material: &'a MaterialVersion,
    quantity: Quantity,
    amount: Money,
    split: crate::reconciliation::ReceiptAmountSplit,
    requests: Vec<usize>,
}

fn invalid(message: impl Into<String>) -> CorrectionError {
    CorrectionError::InvalidHistory(message.into())
}
fn range(error: impl std::fmt::Display) -> CorrectionError {
    CorrectionError::OutOfRange(error.to_string())
}
fn q(text: &str) -> Result<Quantity, CorrectionError> {
    Quantity::parse(text).map_err(range)
}
fn money(text: &str) -> Result<Money, CorrectionError> {
    Money::parse(text).map_err(range)
}
fn seq(text: &str) -> Result<i64, CorrectionError> {
    text.parse::<i64>()
        .map_err(|_| invalid("정정 사건 순번이 올바르지 않습니다"))
}
fn field(input: &CorrectionInclusionInput<'_>, id: &str) -> String {
    input
        .receipt_fields
        .iter()
        .find(|(key, _)| key == id)
        .map(|(_, value)| value.clone())
        .unwrap_or_else(|| "linked_receipt_changes".into())
}
fn issue(output: &mut CorrectionInclusionOutput, field: String, message: impl Into<String>) {
    output.issues.push(BusinessIssue {
        field,
        code: "inclusion_review_required".into(),
        message: message.into(),
    });
}
fn material<'a>(
    input: &'a CorrectionInclusionInput<'_>,
    data: &DraftReceiptV1,
) -> Result<&'a MaterialVersion, CorrectionError> {
    let version = match &data.source {
        ReceiptSourceDraft::Direct {
            material_version_id,
            ..
        }
        | ReceiptSourceDraft::Order {
            material_version_id,
            ..
        } => material_version_id.as_deref(),
    }
    .ok_or_else(|| range("입고 품목 버전을 선택하세요"))?;
    input
        .history
        .materials
        .iter()
        .find(|m| m.id == version)
        .ok_or_else(|| range("입고 품목 버전이 없습니다"))
}

fn source_version(input: &CorrectionInclusionInput<'_>, key: &SourceKey) -> Option<String> {
    match key.kind {
        SourceKind::ReceiptInflow => input.topology.iter().find_map(|event| match event {
            ValueReplayEvent::Inflow { event, .. } if event.id == key.id => Some(event.id.clone()),
            _ => None,
        }),
        SourceKind::ObservationInitial | SourceKind::ObservationExcess => input
            .observations
            .iter()
            .find(|o| o.id == key.id)
            .map(|o| o.expected_version_id.clone()),
    }
}

fn source_ref(input: &CorrectionInclusionInput<'_>, key: &SourceKey) -> Option<CorrectionPlanRef> {
    match key.kind {
        SourceKind::ReceiptInflow => input
            .events
            .iter()
            .find(|e| e.reference.original_id == key.id)
            .map(|e| {
                if input.changed_receipt_ids.contains(&key.id) {
                    CorrectionPlanRef::Proposed {
                        entity: CorrectionRefKind::Receipt,
                        id: key.id.clone(),
                        expected_version_id: Some(e.reference.version_id.clone()),
                    }
                } else {
                    CorrectionPlanRef::Existing {
                        entity: CorrectionRefKind::Receipt,
                        id: key.id.clone(),
                        version_id: e.reference.version_id.clone(),
                    }
                }
            }),
        SourceKind::ObservationInitial | SourceKind::ObservationExcess => input
            .observations
            .iter()
            .find(|o| o.id == key.id)
            .map(|o| o.reference.clone()),
    }
}

fn reservations(
    id: &str,
    effect: &ReconciledReceiptEffect,
) -> Result<Vec<ReservedInclusion>, CorrectionError> {
    effect
        .selections
        .iter()
        .map(|s| {
            Ok(ReservedInclusion {
                // Deterministic local calculation key, never a persisted identifier.
                id: format!("{id}:{}", s.position),
                observation_version_id: s.observation_version_id.clone(),
                material_id: effect.material_id.clone(),
                quantity: q(&s.included_quantity)?,
                portions: s
                    .portions
                    .iter()
                    .map(|p| ProvenancePortion {
                        source_key: p.source_key.clone(),
                        source_version_id: p.source_version_id.clone(),
                        included_share: p.included_share.clone(),
                        reserved_source_share: p.reserved_source_share.clone(),
                    })
                    .collect(),
            })
        })
        .collect()
}

fn remap_frozen(
    input: &CorrectionInclusionInput<'_>,
    old: &ReconciledReceiptEffect,
) -> Option<ReconciledReceiptEffect> {
    let mut effect = old.clone();
    for selection in &mut effect.selections {
        let observation = input
            .observations
            .iter()
            .find(|o| o.id == selection.observation_id)?;
        if observation.effect.material_id != effect.material_id {
            return None;
        }
        selection.observation_version_id = observation.expected_version_id.clone();
        for portion in &mut selection.portions {
            portion.source_version_id = source_version(input, &portion.source_key)?;
        }
    }
    Some(effect)
}

fn candidates(
    input: &CorrectionInclusionInput<'_>,
    reserved: &[ReservedInclusion],
    event: &CorrectionHistoryEvent,
) -> Result<Vec<CorrectionInclusionCandidate>, CorrectionError> {
    let Some(CorrectionReplacementDraft::Receipt { data }) = &event.effective_input else {
        return Ok(vec![]);
    };
    let mat = match material(input, data) {
        Ok(m) => m,
        Err(_) => return Ok(vec![]),
    };
    let views =
        inclusion_availability(input.topology, reserved, &mat.material_id).map_err(range)?;
    let before = seq(&event.reference.logical_seq)?;
    let mut result = Vec::new();
    for view in views {
        let o = input
            .observations
            .iter()
            .find(|o| o.expected_version_id == view.observation_version_id)
            .ok_or_else(|| invalid("후보 관측 참조가 없습니다"))?;
        if o.logical_seq >= before {
            continue;
        }
        result.push(CorrectionInclusionCandidate {
            observation: o.reference.clone(),
            stocktake_id: o.stocktake_id.clone(),
            material_id: o.effect.material_id.clone(),
            material_version_id: o.effect.material_version_id.clone(),
            material_name: o.effect.material_name.clone(),
            unit: o.effect.unit.clone(),
            quantity: o.effect.measured_quantity.clone(),
            value: o
                .effect
                .after
                .value
                .clone()
                .ok_or_else(|| invalid("포함 관측 가치가 없습니다"))?,
            available_share: view.available_share,
            available_quantity: view.available_quantity.decimal(),
            logical_seq: o.logical_seq.to_string(),
            sources: view
                .sources
                .into_iter()
                .map(|s| {
                    let source = source_ref(input, &s.source_key)
                        .ok_or_else(|| invalid("포함 출처 참조가 없습니다"))?;
                    let name = input
                        .history
                        .materials
                        .iter()
                        .rev()
                        .find(|m| m.material_id == s.material_id)
                        .map(|m| m.name.clone())
                        .ok_or_else(|| invalid("포함 출처 품목이 없습니다"))?;
                    Ok(CorrectionInclusionSourceCandidate {
                        source_key: s.source_key,
                        source_ref: source,
                        source_material_id: s.material_id,
                        source_material_name: name,
                        source_unit: s.unit,
                        source_quantity: s.quantity.decimal(),
                        reserved_source_share: s.reserved_share,
                        remaining_source_share: s.remaining_share,
                        available_share: s.available_share,
                        available_quantity: s.available_quantity.decimal(),
                    })
                })
                .collect::<Result<_, CorrectionError>>()?,
        });
    }
    Ok(result)
}

fn review(
    input: &CorrectionInclusionInput<'_>,
    reserved: &[ReservedInclusion],
    output: &mut CorrectionInclusionOutput,
    event: &CorrectionHistoryEvent,
    message: impl Into<String>,
) -> Result<(), CorrectionError> {
    let message = message.into();
    let id = &event.reference.original_id;
    if output.review_requests.iter().any(|r| {
        matches!(r,
        CorrectionReviewRequest::ReceiptInclusions { receipt_id, .. } if receipt_id == id)
    }) {
        return Ok(());
    }
    let path = format!("{}.stock_effect", field(input, id));
    issue(output, path.clone(), message.clone());
    output
        .review_requests
        .push(CorrectionReviewRequest::ReceiptInclusions {
            field: path,
            receipt_id: id.clone(),
            expected_version_id: event.reference.version_id.clone(),
            candidates: candidates(input, reserved, event)?,
            message,
        });
    Ok(())
}

fn basis(
    effect: &StocktakeRowEffect,
    source: &InclusionSourceSelection,
    observation_id: &str,
) -> Result<(Quantity, Money), CorrectionError> {
    match (source, &effect.calculation) {
        (
            InclusionSourceSelection::Source { source_key },
            StocktakeCalculation::Settlement {
                excess_quantity,
                excess_value,
                ..
            },
        ) if source_key.kind == SourceKind::ObservationExcess
            && source_key.id == observation_id =>
        {
            Ok((q(excess_quantity)?, money(excess_value)?))
        }
        _ => Ok((
            q(&effect.measured_quantity)?,
            money(
                effect
                    .after
                    .value
                    .as_deref()
                    .ok_or_else(|| invalid("포함 관측 가치가 없습니다"))?,
            )?,
        )),
    }
}
fn exact(
    effect: &StocktakeRowEffect,
    source: &InclusionSourceSelection,
    observation_id: &str,
    quantity: Quantity,
) -> Result<BigRational, CorrectionError> {
    if quantity.micros() == 0 {
        return Ok(BigRational::zero());
    }
    let (base_q, base_v) = basis(effect, source, observation_id)?;
    if base_q.micros() == 0 {
        return Err(range("포함 가치의 기준 수량이 0입니다"));
    }
    Ok(BigRational::new(
        BigInt::from(base_v.micros()) * BigInt::from(quantity.micros()),
        BigInt::from(base_q.micros()),
    ))
}

/// Only unchanged valuation bases share a rounding remainder. A frozen old
/// basis never silently acquires the new observation's price.
fn booked_values(
    input: &CorrectionInclusionInput<'_>,
    fixed: &[(String, ReconciledReceiptEffect)],
    requests: &[InclusionRequest],
) -> Result<Vec<Money>, CorrectionError> {
    let mut result = vec![Money::from_micros(0).map_err(range)?; requests.len()];
    let mut groups = BTreeMap::<&str, Vec<&InclusionRequest>>::new();
    for request in requests {
        groups
            .entry(&request.observation_version_id)
            .or_default()
            .push(request);
    }
    for (anchor, selected) in groups {
        let observation = input
            .observations
            .iter()
            .find(|o| o.expected_version_id == anchor)
            .ok_or_else(|| invalid("포함 관측 근거가 없습니다"))?;
        let old_observation = input
            .history
            .events
            .iter()
            .flat_map(|e| &e.observations)
            .find(|o| o.id == observation.id);
        let mut total = BigRational::zero();
        let mut prior_compatible = 0i64;
        for (receipt_id, selection) in fixed
            .iter()
            .flat_map(|(id, e)| e.selections.iter().map(move |s| (id, s)))
            .filter(|(_, s)| s.observation_id == observation.id)
        {
            let booked = money(&selection.booked_value)?.micros();
            let compatible = input.changed_receipt_ids.contains(receipt_id)
                || old_observation.is_some_and(|old| {
                    basis(&old.effect, &selection.source, &observation.id).ok()
                        == basis(&observation.effect, &selection.source, &observation.id).ok()
                });
            if compatible {
                total += exact(
                    &observation.effect,
                    &selection.source,
                    &observation.id,
                    q(&selection.included_quantity)?,
                )?;
                prior_compatible = prior_compatible
                    .checked_add(booked)
                    .ok_or_else(|| range("포함 가치 합계 범위"))?;
            }
        }
        let mut weights = Vec::new();
        for request in &selected {
            let value = exact(
                &observation.effect,
                &request.source,
                &observation.id,
                request.quantity,
            )?;
            total += &value;
            let weight = value / BigInt::from(1_000_000);
            weights.push(SourceShare {
                numerator: weight.numer().to_string(),
                denominator: weight.denom().to_string(),
                unit: "원".into(),
            });
        }
        let rounded = ((total.numer() * 2u32 + total.denom()) / (total.denom() * 2u32))
            .to_i64()
            .ok_or_else(|| range("포함 가치 합계 범위"))?;
        let next = rounded
            .checked_sub(prior_compatible)
            .ok_or_else(|| range("포함 가치 잔여가 기존 확정액보다 작습니다"))?;
        let cap = money(
            observation
                .effect
                .after
                .value
                .as_deref()
                .ok_or_else(|| invalid("관측 가치 누락"))?,
        )?
        .micros();
        if next < 0 || rounded > cap {
            return Err(range(
                "같은 가치 근거의 이전 연결과 새 포함 가치가 관측 가치를 넘습니다. 관련 입고를 함께 정정하세요",
            ));
        }
        let allocated = allocate_share_amount(Money::from_micros(next).map_err(range)?, &weights)
            .map_err(range)?;
        for (request, value) in selected.into_iter().zip(allocated) {
            result[request.position] = value;
        }
    }
    Ok(result)
}

/// Revalue one explicitly edited receipt only when the known-value replay
/// reaches its original position. Future receipts cannot change its rounding
/// remainder; its old D is absent from the observation value supplied here.
pub fn refresh_correction_receipt_values(
    history: &CorrectionHistory,
    receipt_id: &str,
    effect: &ReconciledReceiptEffect,
    observations: &[CorrectionReplayObservation],
    previous: &[(String, ReconciledReceiptEffect)],
    changed_receipt_ids: &[String],
) -> Result<ReconciledReceiptEffect, CorrectionError> {
    if !changed_receipt_ids.iter().any(|id| id == receipt_id) {
        return Ok(effect.clone());
    }
    if previous.iter().any(|(id, _)| id == receipt_id) {
        return Err(invalid(
            "포함 입고의 자기 이전 차액을 다시 가치 근거로 사용할 수 없습니다",
        ));
    }
    let requests = effect
        .selections
        .iter()
        .enumerate()
        .map(|(position, s)| {
            Ok(InclusionRequest {
                position,
                observation_version_id: s.observation_version_id.clone(),
                material_id: effect.material_id.clone(),
                source: s.source.clone(),
                quantity: q(&s.included_quantity)?,
            })
        })
        .collect::<Result<Vec<_>, CorrectionError>>()?;
    let input = CorrectionInclusionInput {
        history,
        events: &[],
        topology: &[],
        observations,
        changed_receipt_ids,
        changed_observation_ids: &[],
        receipt_fields: &[],
    };
    let booked = booked_values(&input, previous, &requests)?;
    let split = split_receipt_amount(
        q(&effect.quantity)?,
        money(&effect.amount)?,
        &requests.iter().map(|r| r.quantity).collect::<Vec<_>>(),
    )
    .map_err(range)?;
    let mut refreshed = effect.clone();
    let mut total = 0i64;
    for (position, selection) in refreshed.selections.iter_mut().enumerate() {
        let weights = selection
            .portions
            .iter()
            .map(|p| p.included_share.clone())
            .collect::<Vec<_>>();
        let booked_parts = allocate_share_amount(booked[position], &weights).map_err(range)?;
        let actual = split.selection_amounts[position];
        let actual_parts = allocate_share_amount(actual, &weights).map_err(range)?;
        for (i, portion) in selection.portions.iter_mut().enumerate() {
            portion.booked_value = booked_parts[i].decimal();
            portion.actual_included_value = actual_parts[i].decimal();
        }
        selection.booked_value = booked[position].decimal();
        selection.actual_included_value = actual.decimal();
        selection.value_difference =
            ValueDelta::from_micros(actual.micros() - booked[position].micros())
                .map_err(range)?
                .decimal();
        total = total
            .checked_add(booked[position].micros())
            .ok_or_else(|| range("포함 가치 합계 범위"))?;
    }
    refreshed.included_quantity = split.included_quantity.decimal();
    refreshed.unreflected_quantity = split.unreflected_quantity.decimal();
    refreshed.included_amount = split.included_amount.decimal();
    refreshed.unreflected_amount = split.unreflected_amount.decimal();
    refreshed.booked_value = Money::from_micros(total).map_err(range)?.decimal();
    refreshed.value_difference = ValueDelta::from_micros(split.included_amount.micros() - total)
        .map_err(range)?
        .decimal();
    refreshed.materials.clear();
    refreshed.trace.clear();
    refreshed.separate_adjustment = "0".into();
    Ok(refreshed)
}

pub fn resolve_correction_inclusions(
    input: &CorrectionInclusionInput<'_>,
) -> Result<CorrectionInclusionOutput, CorrectionError> {
    let mut output = CorrectionInclusionOutput::default();
    // Validate physical topology independently, before attributing a failure to
    // unchanged reservations that may no longer fit the proposed quantities.
    resolve_inclusions(input.topology, &[], &[]).map_err(|e| invalid(e.to_string()))?;
    let changed = input
        .changed_receipt_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut events = input
        .events
        .iter()
        .filter(|e| {
            matches!(
                e.effective_input,
                Some(CorrectionReplacementDraft::Receipt { .. })
            )
        })
        .collect::<Vec<_>>();
    events.sort_by(|a, b| {
        seq(&a.reference.logical_seq)
            .unwrap_or(0)
            .cmp(&seq(&b.reference.logical_seq).unwrap_or(0))
            .then_with(|| a.reference.original_id.cmp(&b.reference.original_id))
    });
    let mut fixed = Vec::<(String, ReconciledReceiptEffect)>::new();
    let mut reserved = Vec::<ReservedInclusion>::new();
    let mut incompatible = Vec::new();
    for event in &events {
        let id = &event.reference.original_id;
        let Some(CorrectionReplacementDraft::Receipt { data }) = &event.effective_input else {
            continue;
        };
        if changed.contains(id.as_str())
            || !matches!(
                data.stock_effect,
                ReceiptStockEffectDraft::Reconciled { .. }
            )
        {
            continue;
        }
        let old = input
            .history
            .existing_inclusions
            .iter()
            .find(|r| r.receipt_id == *id)
            .ok_or_else(|| invalid("유효 포함 입고의 이전 연결 기록이 없습니다"))?;
        if let Some(effect) = remap_frozen(input, &old.reconciliation.effect) {
            let rows = reservations(id, &effect)?;
            let mut proposed = reserved.clone();
            proposed.extend(rows);
            if resolve_inclusions(input.topology, &proposed, &[]).is_ok() {
                reserved = proposed;
                fixed.push((id.clone(), effect));
                continue;
            }
        }
        incompatible.push(*event);
    }
    for event in incompatible {
        review(
            input,
            &reserved,
            &mut output,
            event,
            "원천 또는 관측의 정정으로 기존 포함 연결을 유지할 수 없습니다. 해당 입고의 연결을 함께 확인하세요",
        )?;
    }
    let mut requests = Vec::<InclusionRequest>::new();
    let mut edited = Vec::<EditedReceipt<'_>>::new();
    for event in &events {
        let id = &event.reference.original_id;
        let Some(CorrectionReplacementDraft::Receipt { data }) = &event.effective_input else {
            continue;
        };
        if !changed.contains(id.as_str()) {
            continue;
        }
        let ReceiptStockEffectDraft::Reconciled { selections } = &data.stock_effect else {
            continue;
        };
        let prefix = field(input, id);
        let parsed = (|| {
            let m = material(input, data)?;
            let unit = Unit::parse(&data.unit).map_err(range)?;
            if unit.canonical() != m.canonical_unit {
                return Err(range("입고와 품목의 단위가 다릅니다"));
            }
            Ok((
                m,
                unit.to_canonical(q(&data.quantity)?).map_err(range)?,
                money(&data.amount)?,
            ))
        })();
        let (mat, quantity, amount) = match parsed {
            Ok(v) => v,
            Err(e) => {
                issue(&mut output, prefix.clone(), e.to_string());
                continue;
            }
        };
        let mut local = Vec::new();
        let mut bad = false;
        for (position, selection) in selections.iter().enumerate() {
            let path = format!("{prefix}.stock_effect.selections[{position}]");
            let Some(observation) = input.observations.iter().find(|o| {
                Some(o.expected_version_id.as_str()) == selection.observation_version_id.as_deref()
            }) else {
                issue(
                    &mut output,
                    format!("{path}.observation_version_id"),
                    "현재 유효 관측을 선택하세요",
                );
                bad = true;
                continue;
            };
            if observation.effect.material_id != mat.material_id
                || observation.logical_seq >= seq(&event.reference.logical_seq)?
            {
                issue(
                    &mut output,
                    format!("{path}.observation_version_id"),
                    "입고 이전의 같은 품목 관측을 선택하세요",
                );
                bad = true;
                continue;
            }
            let Some(source) = selection.source.clone() else {
                issue(
                    &mut output,
                    format!("{path}.source"),
                    "포함 원천을 선택하세요",
                );
                bad = true;
                continue;
            };
            let quantity = match q(&selection.included_quantity) {
                Ok(q) => q,
                Err(e) => {
                    issue(
                        &mut output,
                        format!("{path}.included_quantity"),
                        e.to_string(),
                    );
                    bad = true;
                    continue;
                }
            };
            local.push(InclusionRequest {
                position: requests.len() + local.len(),
                observation_version_id: observation.expected_version_id.clone(),
                material_id: mat.material_id.clone(),
                source,
                quantity,
            });
        }
        if bad {
            review(
                input,
                &reserved,
                &mut output,
                event,
                "포함 근거 입력을 확인하세요",
            )?;
            continue;
        }
        let split = match split_receipt_amount(
            quantity,
            amount,
            &local.iter().map(|r| r.quantity).collect::<Vec<_>>(),
        ) {
            Ok(split) => split,
            Err(e) => {
                issue(&mut output, format!("{prefix}.stock_effect"), e.to_string());
                continue;
            }
        };
        edited.push(EditedReceipt {
            event,
            draft: data,
            material: mat,
            quantity,
            amount,
            split,
            requests: local.iter().map(|r| r.position).collect(),
        });
        requests.extend(local);
    }
    if !output.issues.is_empty() {
        output.receipts = fixed;
        return Ok(output);
    }
    let resolved = match resolve_inclusions(input.topology, &reserved, &requests) {
        Ok(resolved) => resolved,
        Err(error) => {
            for receipt in &edited {
                review(
                    input,
                    &reserved,
                    &mut output,
                    receipt.event,
                    error.to_string(),
                )?;
            }
            output.receipts = fixed;
            return Ok(output);
        }
    };
    // This first pass owns quantities and exact source shares. The actual
    // observation valuation may include receipts before that observation,
    // so it is established only by the chronological known-value pass.
    let booked = vec![Money::from_micros(0).map_err(range)?; requests.len()];
    output.receipts = fixed;
    for receipt in edited {
        output
            .requires_value_refresh
            .push(receipt.event.reference.original_id.clone());
        let mut selections = Vec::new();
        let mut total_booked = 0i64;
        for (position, global) in receipt.requests.iter().enumerate() {
            let request = &requests[*global];
            let observation = input
                .observations
                .iter()
                .find(|o| o.expected_version_id == request.observation_version_id)
                .ok_or_else(|| invalid("포함 관측이 없습니다"))?;
            let resolved = resolved
                .iter()
                .find(|r| r.position == *global)
                .ok_or_else(|| invalid("포함 출처 계산 누락"))?;
            let weights = resolved
                .portions
                .iter()
                .map(|p| p.included_share.clone())
                .collect::<Vec<_>>();
            let booked_parts = allocate_share_amount(booked[*global], &weights).map_err(range)?;
            let actual = receipt.split.selection_amounts[position];
            let actual_parts = allocate_share_amount(actual, &weights).map_err(range)?;
            total_booked = total_booked
                .checked_add(booked[*global].micros())
                .ok_or_else(|| range("포함 가치 합계 범위"))?;
            let portions = resolved
                .portions
                .iter()
                .enumerate()
                .map(|(i, p)| InclusionPortionEffect {
                    position: i,
                    source_key: p.source_key.clone(),
                    source_version_id: p.source_version_id.clone(),
                    included_share: p.included_share.clone(),
                    reserved_source_share: p.reserved_source_share.clone(),
                    booked_value: booked_parts[i].decimal(),
                    actual_included_value: actual_parts[i].decimal(),
                })
                .collect();
            selections.push(ReceiptInclusionEffect {
                position,
                stocktake_id: observation.stocktake_id.clone(),
                observation_id: observation.id.clone(),
                observation_version_id: observation.expected_version_id.clone(),
                source: request.source.clone(),
                included_quantity: request.quantity.decimal(),
                unit: observation.effect.unit.clone(),
                booked_value: booked[*global].decimal(),
                actual_included_value: actual.decimal(),
                value_difference: ValueDelta::from_micros(
                    actual.micros() - booked[*global].micros(),
                )
                .map_err(range)?
                .decimal(),
                portions,
            });
        }
        let old = input
            .history
            .existing_inclusions
            .iter()
            .find(|r| r.receipt_id == receipt.event.reference.original_id)
            .map(|r| &r.reconciliation.effect);
        let difference = receipt.split.included_amount.micros() - total_booked;
        output.receipts.push((
            receipt.event.reference.original_id.clone(),
            ReconciledReceiptEffect {
                material_id: receipt.material.material_id.clone(),
                material_version_id: receipt.material.id.clone(),
                material_name: receipt.material.name.clone(),
                quantity: receipt.quantity.decimal(),
                amount: receipt.amount.decimal(),
                unit: receipt.material.canonical_unit.clone(),
                input_quantity: receipt.draft.quantity.clone(),
                input_unit: receipt.draft.unit.clone(),
                receipt_date: receipt.draft.receipt_date.clone(),
                included_quantity: receipt.split.included_quantity.decimal(),
                unreflected_quantity: receipt.split.unreflected_quantity.decimal(),
                included_amount: receipt.split.included_amount.decimal(),
                unreflected_amount: receipt.split.unreflected_amount.decimal(),
                booked_value: Money::from_micros(total_booked).map_err(range)?.decimal(),
                value_difference: ValueDelta::from_micros(difference)
                    .map_err(range)?
                    .decimal(),
                separate_adjustment: "0".into(),
                selections,
                materials: vec![],
                trace: vec![],
                order_before: old.and_then(|e| e.order_before.clone()),
                order_after: old.and_then(|e| e.order_after.clone()),
            },
        ));
    }
    // Unique positions belong to this entire plan, not each individual receipt.
    let mut selection_position = 0;
    for (_, effect) in &output.receipts {
        for selection in &effect.selections {
            for portion in &selection.portions {
                output.replacements.push(ValueReplacement {
                    observation_version_id: selection.observation_version_id.clone(),
                    material_id: effect.material_id.clone(),
                    source_key: portion.source_key.clone(),
                    selection_position,
                    portion_position: portion.position,
                    booked_value: money(&portion.booked_value)?,
                    actual_included_value: money(&portion.actual_included_value)?,
                });
            }
            selection_position += 1;
        }
    }
    Ok(output)
}
