use super::super::{Projection, Validation};
use crate::{
    correction::*,
    domain::{Money, Quantity},
    reconciliation::SourceKind,
    store::{StoreError, unavailable},
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

fn invalid(message: &str) -> StoreError {
    unavailable(message)
}
fn check(condition: bool, message: &str) -> Result<(), StoreError> {
    if condition {
        Ok(())
    } else {
        Err(invalid(message))
    }
}
fn same<T: Serialize>(left: &T, right: &T) -> Result<bool, StoreError> {
    Ok(serde_json::to_value(left).map_err(unavailable)?
        == serde_json::to_value(right).map_err(unavailable)?)
}
fn seq(text: &str) -> Result<i64, StoreError> {
    text.parse::<i64>()
        .ok()
        .filter(|n| *n > 0 && n.to_string() == text)
        .ok_or_else(|| invalid("정정 결과 순번이 올바르지 않습니다"))
}
fn uuid(text: &str) -> Result<(), StoreError> {
    check(
        uuid::Uuid::parse_str(text).is_ok_and(|id| id.to_string() == text),
        "정정 결과 참조가 정규 UUID가 아닙니다",
    )
}
fn canonical_amount(value: &str) -> Result<(), StoreError> {
    check(
        Money::parse(value).map_err(unavailable)?.decimal() == value,
        "정정 금액이 정규 문자열이 아닙니다",
    )
}
fn canonical_quantity(value: &str) -> Result<(), StoreError> {
    check(
        Quantity::parse(value).map_err(unavailable)?.decimal() == value,
        "정정 수량이 정규 문자열이 아닙니다",
    )
}
fn ref_id(reference: &CorrectionPlanRef) -> &str {
    match reference {
        CorrectionPlanRef::Existing { id, .. } | CorrectionPlanRef::Proposed { id, .. } => id,
    }
}

fn resolve_token(r: &CorrectionRecord, entity: CorrectionRefKind, token: &str) -> String {
    r.resolved_refs
        .iter()
        .find_map(|v| match &v.reference {
            CorrectionPlanRef::Proposed {
                entity: e,
                expected_version_id: Some(old),
                ..
            } if *e == entity && old == token => Some(v.version_id.clone()),
            _ => None,
        })
        .unwrap_or_else(|| token.into())
}
fn resolve_basis(r: &CorrectionRecord, basis: &mut crate::holding::HoldingBasisRef) {
    let entity = match basis.kind {
        crate::holding::HoldingBasisKind::Receipt
        | crate::holding::HoldingBasisKind::Reconciliation => CorrectionRefKind::Receipt,
        crate::holding::HoldingBasisKind::Production => CorrectionRefKind::Production,
        crate::holding::HoldingBasisKind::Observation => CorrectionRefKind::Observation,
        crate::holding::HoldingBasisKind::Correction => CorrectionRefKind::Material,
    };
    if let Some(id) = &mut basis.id {
        *id = resolve_token(r, entity, id);
    }
}
fn resolve_allocation(r: &CorrectionRecord, value: &mut crate::stocktake::ConsumptionAllocation) {
    resolve_basis(r, &mut value.basis);
    for candidate in &mut value.candidates {
        candidate.sale_version_id =
            resolve_token(r, CorrectionRefKind::Sale, &candidate.sale_version_id);
    }
}
fn resolve_effect(r: &CorrectionRecord, value: &mut crate::stocktake::StocktakeRowEffect) {
    if let Some(basis) = &mut value.basis_before {
        resolve_basis(r, basis);
    }
    if let Some(observation) = &mut value.observation_before {
        observation.id = resolve_token(r, CorrectionRefKind::Observation, &observation.id);
    }
    if let crate::stocktake::StocktakeCalculation::Settlement { allocation, .. } =
        &mut value.calculation
    {
        resolve_allocation(r, allocation);
    }
}

fn expected_references(
    s: &Validation<'_>,
    r: &CorrectionRecord,
) -> Result<BTreeMap<String, String>, StoreError> {
    let mut result = BTreeMap::new();
    let mut add = |reference: CorrectionPlanRef, actual: &str| -> Result<(), StoreError> {
        let key = serde_json::to_string(&reference).map_err(unavailable)?;
        if let Some(old) = result.insert(key, actual.into()) {
            check(
                old == actual,
                "동일 계획 참조에 다른 실제 결과가 연결됐습니다",
            )?;
        }
        Ok(())
    };
    for change in &r.plan.events {
        let event = s
            .event_versions
            .iter()
            .find(|v| {
                v.correction_id == r.id
                    && v.reference.kind == change.reference.kind
                    && v.reference.original_id == change.reference.original_id
            })
            .ok_or_else(|| invalid("계획 사건의 실제 개정 없음"))?;
        let entity = match change.reference.kind {
            CorrectionSubjectKind::Receipt => CorrectionRefKind::Receipt,
            CorrectionSubjectKind::PurchaseReturn => CorrectionRefKind::PurchaseReturn,
            CorrectionSubjectKind::Production => CorrectionRefKind::Production,
            CorrectionSubjectKind::SaleCount => CorrectionRefKind::Sale,
            CorrectionSubjectKind::Waste => CorrectionRefKind::Waste,
            CorrectionSubjectKind::Stocktake => CorrectionRefKind::Stocktake,
        };
        add(
            CorrectionPlanRef::Proposed {
                entity,
                id: change.reference.original_id.clone(),
                expected_version_id: Some(change.reference.version_id.clone()),
            },
            event.sale_version_id.as_deref().unwrap_or(&event.id),
        )?;
        if let Some(actual) = &event.sale_version_id {
            let previous = s
                .sale_versions
                .iter()
                .filter(|v| {
                    v.sale_id == change.reference.original_id
                        && seq(&v.seq).is_ok_and(|n| n < seq(&r.seq).unwrap_or(0))
                })
                .max_by_key(|v| seq(&v.seq).unwrap_or(0))
                .ok_or_else(|| invalid("정정 전 실제 판매 개정 없음"))?;
            add(
                CorrectionPlanRef::Proposed {
                    entity: CorrectionRefKind::Sale,
                    id: change.reference.original_id.clone(),
                    expected_version_id: Some(previous.id.clone()),
                },
                actual,
            )?;
        }
    }
    for p in s
        .projection_versions
        .iter()
        .filter(|p| p.correction_id == r.id)
    {
        match &p.projection {
            Projection::Observation {
                change,
                observation_version_id: Some(id),
                ..
            } => add(change.reference.clone(), id)?,
            Projection::Material {
                change,
                holding_version_id: Some(id),
                ..
            } => add(
                CorrectionPlanRef::Proposed {
                    entity: CorrectionRefKind::Material,
                    id: change.material_id.clone(),
                    expected_version_id: match &change.before {
                        CorrectionHoldingState::Known { amounts, .. } => amounts.source_id.clone(),
                        _ => None,
                    },
                },
                id,
            )?,
            _ => {}
        }
    }
    Ok(result)
}

fn observation_owner<'a>(
    s: &'a Validation<'_>,
    id: &str,
) -> Result<&'a crate::store::stocktake::ObservationRow, StoreError> {
    s.observations
        .iter()
        .find(|o| o.id == id)
        .ok_or_else(|| invalid("정정 결과의 원관측이 없습니다"))
}

/// A projection retains the exact preview change; actual IDs are separate and
/// must point to rows produced by this same correction, not a later mutation.
pub(super) fn validate_projections(
    s: &Validation<'_>,
    r: &CorrectionRecord,
) -> Result<(), StoreError> {
    let at = seq(&r.seq)?;
    let rows = s
        .projection_versions
        .iter()
        .filter(|p| p.correction_id == r.id)
        .collect::<Vec<_>>();
    check(
        rows.len()
            == r.plan.materials.len()
                + r.plan.observations.len()
                + r.plan.allocations.len()
                + r.plan.inclusions.len()
                + r.plan.order_closures.len(),
        "정정 계획과 실제 결과 관계 수가 다릅니다",
    )?;
    let mut subjects = BTreeSet::new();
    let mut actual_ids = BTreeSet::new();
    for p in rows {
        uuid(&p.id)?;
        check(
            p.seq == r.seq
                && p.recorded_at == r.recorded_at
                && subjects.insert((p.projection.kind(), p.projection.subject_id())),
            "정정 결과의 소속·시점·중복이 올바르지 않습니다",
        )?;
        match &p.projection {
            Projection::Material {
                change,
                holding_version_id,
                price_basis_id,
            } => {
                let planned = r
                    .plan
                    .materials
                    .iter()
                    .find(|v| v.material_id == change.material_id)
                    .ok_or_else(|| invalid("계획에 없는 품목 결과"))?;
                check(
                    same(change, planned)?,
                    "품목 결과가 정정 미리보기와 다릅니다",
                )?;
                let material = s
                    .materials
                    .iter()
                    .find(|m| {
                        m.id == change.material_version_id && m.material_id == change.material_id
                    })
                    .ok_or_else(|| invalid("정정 품목 버전의 소속이 다릅니다"))?;
                check(
                    material
                        .seq
                        .as_deref()
                        .map(seq)
                        .transpose()?
                        .is_none_or(|v| v < at),
                    "미래 품목 버전을 정정 결과에 사용할 수 없습니다",
                )?;
                match &change.after {
                    CorrectionHoldingState::NotEstablished {} => check(
                        holding_version_id.is_none() && price_basis_id.is_none(),
                        "미확인 결과에 실제0 보유량이나 가격을 만들 수 없습니다",
                    )?,
                    CorrectionHoldingState::Known {
                        amounts,
                        last_valid_price,
                        observation_boundary,
                        initial_boundary,
                        ..
                    } => {
                        canonical_quantity(&amounts.quantity)?;
                        canonical_amount(
                            amounts
                                .value
                                .as_deref()
                                .ok_or_else(|| invalid("보유 가치 없음"))?,
                        )?;
                        let hid = holding_version_id
                            .as_deref()
                            .ok_or_else(|| invalid("확인된 보유 결과의 실제 버전이 없습니다"))?;
                        uuid(hid)?;
                        check(
                            actual_ids.insert(hid),
                            "정정 실제 결과 ID를 중복 사용했습니다",
                        )?;
                        let h = s
                            .holdings
                            .iter()
                            .find(|h| h.id == hid)
                            .ok_or_else(|| invalid("정정 보유 버전이 없습니다"))?;
                        check(
                            h.material_id == change.material_id
                                && h.material_version_id == change.material_version_id
                                && h.seq == r.seq
                                && h.recorded_at == r.recorded_at
                                && h.correction_projection_id.as_deref() == Some(p.id.as_str())
                                && h.quantity == amounts.quantity
                                && Some(h.value.as_str()) == amounts.value.as_deref(),
                            "정정 보유 결과의 소속·수량·가치·순번이 다릅니다",
                        )?;
                        let previous = match &change.before {
                            CorrectionHoldingState::Known { amounts, .. } => {
                                amounts.source_id.as_deref()
                            }
                            _ => None,
                        };
                        check(
                            h.previous_version_id.as_deref() == previous,
                            "정정 보유량의 실제 직전 버전이 다릅니다",
                        )?;
                        let boundary = observation_boundary
                            .as_ref()
                            .map(|b| b.logical_seq.as_str())
                            .or_else(|| initial_boundary.as_ref().map(|b| b.logical_seq.as_str()))
                            .ok_or_else(|| invalid("확인된 결과의 논리 시작 경계가 없습니다"))?;
                        check(
                            h.baseline_seq == boundary && seq(boundary)? <= at,
                            "정정 물질화 시점이 실제 구간 경계를 덮었습니다",
                        )?;
                        match (last_valid_price, price_basis_id) {
                            (None, None) => {}
                            (Some(expected), Some(pid)) => {
                                uuid(pid)?;
                                check(actual_ids.insert(pid), "정정 가격 근거 중복")?;
                                let price =
                                    s.price_bases.iter().find(|v| v.id == *pid).ok_or_else(
                                        || invalid("정정 마지막 단가 근거가 없습니다"),
                                    )?;
                                check(
                                    price.correction_id == r.id
                                        && price.material_id == change.material_id
                                        && price.material_version_id == change.material_version_id
                                        && price.seq == r.seq
                                        && price.recorded_at == r.recorded_at
                                        && price.amount == expected.amount
                                        && price.quantity == expected.quantity
                                        && price.logical_seq == expected.logical_seq
                                        && same(&price.source, &expected.source)?,
                                    "정정 마지막 단가의 근거·수치·시점이 다릅니다",
                                )?;
                                canonical_amount(&price.amount)?;
                                canonical_quantity(&price.quantity)?;
                                check(
                                    Quantity::parse(&price.quantity)
                                        .map_err(unavailable)?
                                        .micros()
                                        > 0
                                        && seq(&price.logical_seq)? <= at
                                        && h.last_valid_correction_price_id.as_deref()
                                            == Some(pid.as_str()),
                                    "정정 단가의 양수 기준량과 보유 참조를 확인하세요",
                                )?;
                            }
                            _ => {
                                return Err(invalid(
                                    "정정 마지막 단가와 실제 가격 근거가 일치하지 않습니다",
                                ));
                            }
                        }
                    }
                }
            }
            Projection::Observation {
                change,
                observation_version_id,
                price_basis_id,
            } => {
                let oid = ref_id(&change.reference);
                let planned = r
                    .plan
                    .observations
                    .iter()
                    .find(|v| ref_id(&v.reference) == oid)
                    .ok_or_else(|| invalid("계획에 없는 관측 결과"))?;
                check(
                    same(change, planned)?,
                    "관측 결과가 정정 미리보기와 다릅니다",
                )?;
                let original = observation_owner(s, oid)?;
                check(
                    original.stocktake_id == change.stocktake_id && seq(&change.logical_seq)? < at,
                    "관측 원본/논리 위치가 다릅니다",
                )?;
                match (&change.after, observation_version_id) {
                    (None, None) => check(
                        change.valuation_basis.is_none() && price_basis_id.is_none(),
                        "취소 관측에 단가 근거를 만들 수 없습니다",
                    )?,
                    (Some(effect), Some(vid)) => {
                        uuid(vid)?;
                        check(actual_ids.insert(vid), "관측 실제 결과 중복")?;
                        let row = s
                            .observation_versions
                            .iter()
                            .find(|o| o.id == *vid)
                            .ok_or_else(|| invalid("관측 실제 버전이 없습니다"))?;
                        check(
                            row.observation_id == oid
                                && row.seq == r.seq
                                && row.recorded_at == r.recorded_at
                                && row.correction_projection_id.as_deref() == Some(p.id.as_str())
                                && row.logical_seq.as_deref() == Some(change.logical_seq.as_str())
                                && original.material_id == effect.material_id
                                && original.position == effect.position,
                            "관측 개정 소속/순번/품목이 다릅니다",
                        )?;
                        let previous = s
                            .observation_versions
                            .iter()
                            .filter(|v| {
                                v.observation_id == oid && seq(&v.seq).is_ok_and(|n| n < at)
                            })
                            .max_by_key(|v| seq(&v.seq).unwrap_or(0));
                        check(
                            row.previous_version_id.as_deref() == previous.map(|v| v.id.as_str()),
                            "관측 실제 직전 버전이 다릅니다",
                        )?;
                        // The store may fill actual holding references, but it
                        // cannot alter measured amounts, calculation or units.
                        let mut expected = effect.clone();
                        resolve_effect(r, &mut expected);
                        if let (Some(basis), Some(pid)) = (&change.valuation_basis, price_basis_id)
                        {
                            let price = s
                                .price_bases
                                .iter()
                                .find(|b| b.id == *pid)
                                .ok_or_else(|| invalid("관측 정정 단가 근거 없음"))?;
                            uuid(pid)?;
                            check(actual_ids.insert(pid.as_str()), "관측 단가 근거 중복")?;
                            check(
                                price.correction_id == r.id
                                    && price.material_id == effect.material_id
                                    && price.material_version_id == effect.material_version_id
                                    && price.seq == r.seq
                                    && price.recorded_at == r.recorded_at
                                    && price.amount == basis.amount
                                    && price.quantity == basis.quantity
                                    && price.logical_seq == basis.logical_seq
                                    && same(&price.source, &basis.source)?,
                                "관측 단가의 수치/근거/시점이 다릅니다",
                            )?;
                            let valuation = match &mut expected.calculation {
                                crate::stocktake::StocktakeCalculation::Initial {
                                    valuation,
                                    ..
                                } => Some(valuation),
                                crate::stocktake::StocktakeCalculation::Settlement {
                                    valuation,
                                    ..
                                } => valuation.as_mut(),
                            };
                            match valuation {
                                Some(crate::stocktake::StocktakeValuation::ExistingPrice {
                                    source_id,
                                    amount,
                                    quantity,
                                    ..
                                }) if source_id.is_empty()
                                    && *amount == basis.amount
                                    && *quantity == basis.quantity =>
                                {
                                    *source_id = pid.clone()
                                }
                                _ => {
                                    return Err(invalid("관측 단가의 별도 근거와 계산이 다릅니다"));
                                }
                            }
                        } else if change.valuation_basis.is_some() || price_basis_id.is_some() {
                            return Err(invalid("관측 단가 근거의 저장 여부가 다릅니다"));
                        }
                        check(
                            same(&row.effect, &expected)?,
                            "관측 실제 수량/가치 계산이 계획과 다릅니다",
                        )?;
                    }
                    _ => return Err(invalid("관측 취소/확인의 실제 버전이 다릅니다")),
                }
            }
            Projection::Allocation {
                change,
                consumption_version_id,
            } => {
                let oid = ref_id(&change.observation);
                let planned = r
                    .plan
                    .allocations
                    .iter()
                    .find(|v| ref_id(&v.observation) == oid)
                    .ok_or_else(|| invalid("계획에 없는 소모 배분"))?;
                check(
                    same(change, planned)?,
                    "소모 배분이 정정 미리보기와 다릅니다",
                )?;
                observation_owner(s, oid)?;
                match (&change.after, consumption_version_id) {
                    (None, None) => {}
                    (Some(allocation), Some(cid)) => {
                        let mut allocation = allocation.clone();
                        resolve_allocation(r, &mut allocation);
                        uuid(cid)?;
                        check(actual_ids.insert(cid), "소모 개정 ID 중복")?;
                        let row = s
                            .consumption_versions
                            .iter()
                            .find(|c| c.id == *cid)
                            .ok_or_else(|| invalid("소모 실제 개정이 없습니다"))?;
                        let observation = s
                            .observation_versions
                            .iter()
                            .find(|o| o.id == row.observation_version_id)
                            .ok_or_else(|| invalid("소모 개정 관측이 없습니다"))?;
                        check(
                            observation.observation_id == oid
                                && seq(&observation.seq)? <= at
                                && row.previous_version_id
                                    == change.expected_consumption_version_id
                                && row.end_seq == allocation.end_seq
                                && row.total_quantity == allocation.total_quantity
                                && row.total_value == allocation.total_value
                                && row.unallocated_quantity == allocation.unallocated_quantity
                                && row.unallocated_value == allocation.unallocated_value
                                && row.reason == allocation.reason
                                && same(&row.basis, &allocation.basis)?
                                && row.correction_projection_id.as_deref() == Some(p.id.as_str())
                                && row.seq.as_deref() == Some(r.seq.as_str()),
                            "소모 개정의 구간·양·가치·직전 버전이 다릅니다",
                        )?;
                        check(
                            row.as_of_revision == allocation.as_of_revision
                                && seq(&row.as_of_revision)? < at,
                            "소모 개정의 실제 선택 시점이 다릅니다",
                        )?;
                        let mut candidates = s
                            .consumption_rows
                            .iter()
                            .filter(|c| c.consumption_version_id == *cid)
                            .map(|c| c.row.clone())
                            .collect::<Vec<_>>();
                        candidates.sort_by_key(|c| c.position);
                        check(
                            same(&candidates, &allocation.candidates)?,
                            "소모 후보와 실제 배분 값이 다릅니다",
                        )?;
                    }
                    _ => return Err(invalid("소모 배분 취소/개정 참조가 다릅니다")),
                }
            }
            Projection::Inclusion {
                change,
                active_link_version_ids,
                links,
            } => {
                let planned = r
                    .plan
                    .inclusions
                    .iter()
                    .find(|v| v.receipt_id == change.receipt_id)
                    .ok_or_else(|| invalid("계획에 없는 포함 개정"))?;
                check(same(change, planned)?, "포함 개정이 미리보기와 다릅니다")?;
                let expected = change
                    .after
                    .as_ref()
                    .map(|e| e.selections.iter().map(|v| v.portions.len()).sum::<usize>())
                    .unwrap_or(0);
                check(
                    links.len() == expected && active_link_version_ids.len() == expected,
                    "포함 개정의 실제 몫 수가 다릅니다",
                )?;
                let active = active_link_version_ids.iter().collect::<BTreeSet<_>>();
                check(active.len() == expected, "활성 포함 버전 중복")?;
                let mut positions = BTreeSet::new();
                let mut reservations = BTreeSet::new();
                let mut stable_links = BTreeSet::new();
                for link in links {
                    for v in [
                        &link.id,
                        &link.link_id,
                        &link.reservation_id,
                        &link.observation_version_id,
                    ] {
                        uuid(v)?;
                    }
                    check(
                        actual_ids.insert(&link.id)
                            && active.contains(&link.id)
                            && positions.insert((link.selection_position, link.portion_position))
                            && reservations.insert(&link.reservation_id)
                            && stable_links.insert(&link.link_id),
                        "포함 개정 ID/몫 위치가 중복됐습니다",
                    )?;
                    let selection = change
                        .after
                        .as_ref()
                        .and_then(|e| {
                            e.selections
                                .iter()
                                .find(|v| v.position == link.selection_position)
                        })
                        .ok_or_else(|| invalid("실제 포함 선택이 계획에 없습니다"))?;
                    let portion = selection
                        .portions
                        .iter()
                        .find(|v| v.position == link.portion_position)
                        .ok_or_else(|| invalid("실제 포함 몫이 계획에 없습니다"))?;
                    let observation = s
                        .observation_versions
                        .iter()
                        .find(|o| o.id == link.observation_version_id)
                        .ok_or_else(|| invalid("포함 관측 실제 버전이 없습니다"))?;
                    check(
                        observation.observation_id == selection.observation_id
                            && seq(&observation.seq)? <= at
                            && link.observation_version_id
                                == resolve_token(
                                    r,
                                    CorrectionRefKind::Observation,
                                    &selection.observation_version_id,
                                ),
                        "포함 관측의 소속 또는 미래 참조가 잘못됐습니다",
                    )?;
                    let mut actual = link.portion.clone();
                    actual.source_version_id = portion.source_version_id.clone();
                    check(
                        same(&actual, portion)?,
                        "포함 실제 몫의 양/가치가 계획과 다릅니다",
                    )?;
                    validate_source(
                        s,
                        r,
                        &link.portion.source_key,
                        &link.portion.source_version_id,
                    )?;
                    let source_entity = match portion.source_key.kind {
                        SourceKind::ReceiptInflow => CorrectionRefKind::Receipt,
                        _ => CorrectionRefKind::Observation,
                    };
                    check(
                        link.portion.source_version_id
                            == resolve_token(r, source_entity, &portion.source_version_id),
                        "포함 원천의 실제 버전 해결이 다릅니다",
                    )?;
                    if let Some(previous) = &link.previous_version_id {
                        uuid(previous)?;
                        let original = s.inclusion_link_versions.iter().find(|v| {
                            v.id == *previous
                                && v.link_id == link.link_id
                                && seq(&v.seq).is_ok_and(|n| n < at)
                                && s.inclusion_links.iter().any(|l| {
                                    l.id == v.link_id
                                        && l.selection_position == link.selection_position
                                        && s.reconciliations.iter().any(|rec| {
                                            rec.id == l.reconciliation_id
                                                && rec.receipt_id == change.receipt_id
                                        })
                                })
                                && v.portion.source_key == link.portion.source_key
                        });
                        let revised=s.projection_versions.iter().filter(|p|seq(&p.seq).is_ok_and(|n|n<at)).any(|p|matches!(&p.projection,Projection::Inclusion {change:prior,links,..} if prior.receipt_id==change.receipt_id && links.iter().any(|v|v.id==*previous&&v.link_id==link.link_id&&v.selection_position==link.selection_position&&v.portion.source_key==link.portion.source_key)));
                        check(
                            original.is_some() || revised,
                            "포함 직전 버전이 같은 링크의 과거 개정이 아닙니다",
                        )?;
                        let latest = s
                            .projection_versions
                            .iter()
                            .filter(|p| seq(&p.seq).is_ok_and(|n| n < at))
                            .filter_map(|p| match &p.projection {
                                Projection::Inclusion { links, .. } => links
                                    .iter()
                                    .find(|l| l.link_id == link.link_id)
                                    .map(|l| (seq(&p.seq).unwrap_or(0), l.id.as_str())),
                                _ => None,
                            })
                            .max_by_key(|(n, _)| *n);
                        check(
                            latest.is_none_or(|(_, id)| id == previous),
                            "포함 직전 개정을 건너뛰었습니다",
                        )?;
                    } else {
                        check(!s.inclusion_links.iter().any(|l|l.id==link.link_id)
                            && !s.projection_versions.iter().filter(|p|seq(&p.seq).is_ok_and(|n|n<at)).any(|p|matches!(&p.projection,Projection::Inclusion{links,..} if links.iter().any(|l|l.link_id==link.link_id))),"새 포함 링크가 기존 링크 ID를 재사용했습니다")?;
                    }
                }
            }
            Projection::OrderClosure {
                change,
                order_version_id,
            } => {
                let planned = r
                    .plan
                    .order_closures
                    .iter()
                    .find(|v| v.order_id == change.order_id)
                    .ok_or_else(|| invalid("계획에 없는 주문 종료 결과"))?;
                check(
                    same(change, planned)?,
                    "주문 종료 결과가 미리보기와 다릅니다",
                )?;
                if let Some(vid) = order_version_id {
                    uuid(vid)?;
                    check(actual_ids.insert(vid), "주문 실제 개정 중복")?;
                    let order = s
                        .orders
                        .iter()
                        .find(|v| v.id == *vid)
                        .ok_or_else(|| invalid("주문 실제 개정이 없습니다"))?;
                    check(
                        order.order_id == change.order_id
                            && order.seq == r.seq
                            && order.recorded_at == r.recorded_at
                            && order.confirmation_id == r.confirmation_id,
                        "주문 개정의 소속/시점이 다릅니다",
                    )?;
                    if let Some(after) = &change.after {
                        check(
                            order.status == after.status
                                && order.quantity == after.ordered_quantity
                                && order.unit == after.unit,
                            "주문 개정의 실제 값이 다릅니다",
                        )?;
                    } else {
                        check(
                            order.status == crate::purchase_order::OrderStatus::Open,
                            "취소된 입고의 주문이 미입고 상태가 아닙니다",
                        )?;
                    }
                } else {
                    return Err(invalid("주문 종료 정정에 실제 버전이 없습니다"));
                }
            }
        }
    }
    let mut refs = BTreeSet::new();
    let expected_refs = expected_references(s, r)?;
    for resolved in &r.resolved_refs {
        uuid(&resolved.version_id)?;
        let key = serde_json::to_string(&resolved.reference).map_err(unavailable)?;
        check(refs.insert(key), "계획 참조의 실제 해결이 중복됐습니다")?;
        check(
            expected_refs.get(&serde_json::to_string(&resolved.reference).map_err(unavailable)?)
                == Some(&resolved.version_id),
            "계획에 없는 또는 잘못 해결된 참조입니다",
        )?;
        check(
            actual_ids.contains(resolved.version_id.as_str())
                || s.event_versions.iter().any(|e| {
                    (e.id == resolved.version_id
                        || e.sale_version_id.as_deref() == Some(resolved.version_id.as_str()))
                        && e.correction_id == r.id
                }),
            "해결된 참조가 이 정정의 실제 결과가 아닙니다",
        )?;
        validate_resolved(s, r, &resolved.reference, &resolved.version_id)?;
    }
    check(
        refs.len() == expected_refs.len(),
        "계획의 실제 참조 해결이 빠졌습니다",
    )?;
    Ok(())
}

fn validate_source(
    s: &Validation<'_>,
    r: &CorrectionRecord,
    key: &crate::reconciliation::SourceKey,
    version: &str,
) -> Result<(), StoreError> {
    uuid(&key.id)?;
    uuid(version)?;
    check(key.position == 0, "지원하지 않는 원천 위치")?;
    let at = seq(&r.seq)?;
    let valid = match key.kind {
        SourceKind::ReceiptInflow => {
            s.receipts
                .iter()
                .any(|v| v.id == key.id && v.id == version && seq(&v.seq).is_ok_and(|n| n < at))
                || s.event_versions.iter().any(|v| {
                    v.id == version
                        && v.reference.original_id == key.id
                        && v.reference.kind == CorrectionSubjectKind::Receipt
                        && v.after.is_some()
                        && seq(&v.reference.seq).is_ok_and(|n| n <= at)
                })
        }
        SourceKind::ObservationInitial | SourceKind::ObservationExcess => {
            s.observation_versions.iter().any(|v| {
                v.id == version && v.observation_id == key.id && seq(&v.seq).is_ok_and(|n| n <= at)
            })
        }
    };
    check(valid, "포함 원천의 실제 버전 소속 또는 시점이 잘못됐습니다")
}

fn validate_resolved(
    s: &Validation<'_>,
    r: &CorrectionRecord,
    reference: &CorrectionPlanRef,
    version: &str,
) -> Result<(), StoreError> {
    let CorrectionPlanRef::Proposed { entity, id, .. } = reference else {
        return Err(invalid(
            "실제 결과 해결에는 proposed 참조만 사용할 수 있습니다",
        ));
    };
    let valid=match entity{
        CorrectionRefKind::Material=>s.projection_versions.iter().any(|p|p.correction_id==r.id&&matches!(&p.projection,Projection::Material {change,holding_version_id:Some(v),..} if change.material_id==*id&&v==version)),
        CorrectionRefKind::Observation=>s.projection_versions.iter().any(|p|p.correction_id==r.id&&matches!(&p.projection,Projection::Observation {change,observation_version_id:Some(v),..} if ref_id(&change.reference)==id&&v==version)),
        CorrectionRefKind::Sale=>s.event_versions.iter().any(|v|v.correction_id==r.id&&v.reference.original_id==*id&&v.reference.kind==CorrectionSubjectKind::SaleCount&&(v.sale_version_id.as_deref().unwrap_or(&v.id)==version)),
        CorrectionRefKind::PriceBasis=>s.price_bases.iter().any(|p|p.correction_id==r.id&&p.material_id==*id&&p.id==version),
        CorrectionRefKind::InclusionLink=>s.projection_versions.iter().any(|p|p.correction_id==r.id&&matches!(&p.projection,Projection::Inclusion {links,..} if links.iter().any(|l|l.link_id==*id&&l.id==version))),
        _=>s.event_versions.iter().any(|v|v.correction_id==r.id&&v.reference.original_id==*id&&v.id==version&&matches!((entity,v.reference.kind),(CorrectionRefKind::Receipt,CorrectionSubjectKind::Receipt)|(CorrectionRefKind::PurchaseReturn,CorrectionSubjectKind::PurchaseReturn)|(CorrectionRefKind::Production,CorrectionSubjectKind::Production)|(CorrectionRefKind::Sale,CorrectionSubjectKind::SaleCount)|(CorrectionRefKind::Waste,CorrectionSubjectKind::Waste)|(CorrectionRefKind::Stocktake,CorrectionSubjectKind::Stocktake))),
    };
    check(
        valid,
        "계획 참조가 다른 종류/원사건의 실제 결과를 가리킵니다",
    )
}
