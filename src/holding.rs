use crate::{
    catalog::CatalogState,
    domain::{DomainError, HoldingBasis, Money, Quantity, ResolvedPrice, UnitPrice, resolve_price},
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoldingBasisKind {
    Receipt,
    Observation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoldingBasisRef {
    pub kind: HoldingBasisKind,
    pub id: Option<String>,
    pub seq: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoldingAmounts {
    pub quantity: String,
    pub value: Option<String>,
    pub source_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoldingObservation {
    pub id: String,
    pub quantity: String,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoldingChangeKind {
    Receipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoldingChange {
    pub kind: HoldingChangeKind,
    pub event_id: String,
    pub quantity: String,
    pub value: String,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PriceBasis {
    pub amount: String,
    pub quantity: String,
    pub source_id: String,
}

impl From<&UnitPrice> for PriceBasis {
    fn from(price: &UnitPrice) -> Self {
        Self {
            amount: price.amount().decimal(),
            quantity: price.quantity().decimal(),
            source_id: price.source_id().into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HoldingDetail {
    pub material_id: String,
    pub material_version_id: String,
    pub unit: String,
    pub basis: Option<HoldingBasisRef>,
    pub observation: Option<HoldingObservation>,
    pub settlement_basis: Option<HoldingAmounts>,
    pub last_valid_price: Option<PriceBasis>,
    pub confirmed_changes: Vec<HoldingChange>,
    pub price: ResolvedPrice,
    pub state: CatalogState,
}

/// Already settled quantities in the material's canonical unit. Absence of a
/// basis differs from a confirmed zero. A later receipt cannot fill an unknown
/// value by silently treating it as zero.
#[derive(Debug, Clone, Default)]
pub struct HoldingInput {
    pub basis: Option<HoldingBasisRef>,
    pub observation: Option<HoldingObservation>,
    pub quantity: Option<Quantity>,
    pub value: Option<Money>,
    pub source_id: Option<String>,
    pub last_valid: Option<UnitPrice>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoldingProjection {
    pub before: Option<HoldingAmounts>,
    pub after: HoldingAmounts,
    pub basis_before: Option<HoldingBasisRef>,
    pub observation: Option<HoldingObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoldingViewProjection {
    pub basis: Option<HoldingBasisRef>,
    pub observation: Option<HoldingObservation>,
    pub settlement_basis: Option<HoldingAmounts>,
    pub confirmed_changes: Vec<HoldingChange>,
}

fn invalid(message: &str) -> DomainError {
    DomainError(message.into())
}

fn valid_id(id: &str) -> Result<(), DomainError> {
    Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| invalid("보유 근거 ID가 올바르지 않습니다"))
}

fn sequence(text: &str) -> Result<i64, DomainError> {
    text.parse::<i64>()
        .ok()
        .filter(|n| *n > 0 && n.to_string() == text)
        .ok_or_else(|| invalid("보유 근거 순서가 올바르지 않습니다"))
}

fn timestamp(text: &str) -> Result<(), DomainError> {
    chrono::DateTime::parse_from_rfc3339(text)
        .map(|_| ())
        .map_err(|_| invalid("보유 근거 기록 시각이 올바르지 않습니다"))
}

pub fn validate_holding_input(input: &HoldingInput) -> Result<(), DomainError> {
    let Some(quantity) = input.quantity else {
        if input.basis.is_some()
            || input.observation.is_some()
            || input.value.is_some()
            || input.source_id.is_some()
            || input.last_valid.is_some()
        {
            return Err(invalid(
                "미확인 보유 기준에 수량·가치 근거를 혼합할 수 없습니다",
            ));
        }
        return Ok(());
    };
    let basis = input
        .basis
        .as_ref()
        .ok_or_else(|| invalid("보유 수량의 시작 근거가 없습니다"))?;
    let basis_id = basis
        .id
        .as_deref()
        .ok_or_else(|| invalid("보유 수량의 시작 근거 ID가 없습니다"))?;
    valid_id(basis_id)?;
    sequence(
        basis
            .seq
            .as_deref()
            .ok_or_else(|| invalid("보유 수량의 시작 순서가 없습니다"))?,
    )?;
    valid_id(
        input
            .source_id
            .as_deref()
            .ok_or_else(|| invalid("현재 보유 수량의 출처가 없습니다"))?,
    )?;
    if quantity.micros() == 0 && input.value.map(|v| v.micros()) != Some(0) {
        return Err(invalid("확인한 수량 0의 보유 가치도 0이어야 합니다"));
    }
    match (basis.kind, &input.observation) {
        (HoldingBasisKind::Receipt, None) => {}
        (HoldingBasisKind::Observation, Some(observation)) => {
            valid_id(&observation.id)?;
            sequence(&observation.seq)?;
            timestamp(&observation.recorded_at)?;
            Quantity::parse(&observation.quantity)?;
            if observation.id != basis_id || Some(&observation.seq) != basis.seq.as_ref() {
                return Err(invalid("마지막 관측과 정산 시작 근거가 다릅니다"));
            }
        }
        _ => return Err(invalid("관측 근거와 마지막 확인 기록이 일치하지 않습니다")),
    }
    if let Some(price) = &input.last_valid {
        valid_id(price.source_id())?;
    }
    Ok(())
}

fn amounts(input: &HoldingInput) -> Option<HoldingAmounts> {
    input.quantity.map(|quantity| HoldingAmounts {
        quantity: quantity.decimal(),
        value: input.value.map(|value| value.decimal()),
        source_id: input.source_id.clone(),
    })
}

fn add_receipt(
    current_quantity: Quantity,
    current_value: Money,
    quantity: Quantity,
    value: Money,
) -> Result<(Quantity, Money), DomainError> {
    if quantity.micros() == 0 {
        return Err(invalid("실입고 수량은 0보다 커야 합니다"));
    }
    let q = current_quantity
        .micros()
        .checked_add(quantity.micros())
        .ok_or_else(|| invalid("보유 수량 합계가 허용 범위를 넘습니다"))?;
    let v = current_value
        .micros()
        .checked_add(value.micros())
        .ok_or_else(|| invalid("보유 가치 합계가 허용 범위를 넘습니다"))?;
    Ok((Quantity::from_micros(q)?, Money::from_micros(v)?))
}

/// Preview one canonical receipt against the current settled Q/V. No event or
/// holding ID is allocated and the last measured observation remains unchanged.
pub fn project_receipt(
    before: &HoldingInput,
    quantity: Quantity,
    amount: Money,
) -> Result<HoldingProjection, DomainError> {
    validate_holding_input(before)?;
    let (current_q, current_v) = match (before.quantity, before.value) {
        (None, None) => (Quantity::from_micros(0)?, Money::from_micros(0)?),
        (Some(q), Some(v)) => (q, v),
        _ => return Err(invalid("기존 보유 수량의 가치를 먼저 확인해 주세요")),
    };
    let (q, v) = add_receipt(current_q, current_v, quantity, amount)?;
    Ok(HoldingProjection {
        before: amounts(before),
        after: HoldingAmounts {
            quantity: q.decimal(),
            value: Some(v.decimal()),
            source_id: None,
        },
        basis_before: before.basis.clone(),
        observation: before.observation.clone(),
    })
}

pub fn resolve_holding_price(
    input: &HoldingInput,
    preregistered: Option<UnitPrice>,
) -> Result<ResolvedPrice, DomainError> {
    validate_holding_input(input)?;
    let holding = match (input.quantity, input.value) {
        (None, None) => HoldingBasis::NotEstablished,
        (Some(quantity), Some(value)) => HoldingBasis::Known {
            quantity,
            value,
            source_id: input.source_id.clone().unwrap_or_default(),
        },
        (Some(quantity), None) => HoldingBasis::ValueUnknown { quantity },
        _ => return Err(invalid("보유 수량과 가치의 근거가 일치하지 않습니다")),
    };
    Ok(resolve_price(
        holding,
        input.last_valid.clone(),
        preregistered,
    ))
}

/// Reconstruct a view from the exact baseline Q/V and subsequent canonical
/// receipts. This input is not the already accumulated current holding head.
/// Older events are historical; the baseline receipt itself is checked and
/// excluded. A synthetic observation can exercise the same boundary contract.
pub fn project_holding(
    baseline: &HoldingInput,
    changes: &[HoldingChange],
) -> Result<HoldingViewProjection, DomainError> {
    validate_holding_input(baseline)?;
    if let Some(observation) = &baseline.observation
        && baseline.quantity != Some(Quantity::parse(&observation.quantity)?)
    {
        return Err(invalid("조회 시작 수량은 해당 관측 수량과 같아야 합니다"));
    }
    let mut basis = baseline.basis.clone();
    let boundary = basis
        .as_ref()
        .and_then(|value| value.seq.as_deref())
        .map(sequence)
        .transpose()?
        .unwrap_or(0);
    let mut quantity = baseline.quantity;
    let mut value = baseline.value;
    let mut subsequent = Vec::new();
    let mut prior_sequence = 0;
    let mut ids = HashSet::new();
    let mut applied = false;
    for change in changes {
        valid_id(&change.event_id)?;
        timestamp(&change.recorded_at)?;
        let seq = sequence(&change.seq)?;
        let q = Quantity::parse(&change.quantity)?;
        let v = Money::parse(&change.value)?;
        if seq <= prior_sequence || !ids.insert(&change.event_id) || q.micros() == 0 {
            return Err(invalid("확정 변동의 순서·중복·수량을 확인해 주세요"));
        }
        prior_sequence = seq;
        if baseline
            .basis
            .as_ref()
            .is_some_and(|b| b.id.as_deref() == Some(change.event_id.as_str()))
            && seq != boundary
        {
            return Err(invalid(
                "같은 시작 근거를 다른 입고 순서로 다시 적용할 수 없습니다",
            ));
        }
        if seq < boundary {
            continue;
        }
        if seq == boundary {
            if basis.as_ref().is_none_or(|b| {
                b.kind != HoldingBasisKind::Receipt
                    || b.id.as_deref() != Some(change.event_id.as_str())
            }) || quantity != Some(q)
                || value != Some(v)
            {
                return Err(invalid("시작 기준과 같은 순서의 입고 근거가 다릅니다"));
            }
            continue;
        }
        let (current_q, current_v) = match (quantity, value) {
            (None, None) => (Quantity::from_micros(0)?, Money::from_micros(0)?),
            (Some(q), Some(v)) => (q, v),
            _ => return Err(invalid("기존 보유 수량의 가치를 먼저 확인해 주세요")),
        };
        let (next_q, next_v) = add_receipt(current_q, current_v, q, v)?;
        quantity = Some(next_q);
        value = Some(next_v);
        applied = true;
        if basis.is_none() {
            basis = Some(HoldingBasisRef {
                kind: HoldingBasisKind::Receipt,
                id: Some(change.event_id.clone()),
                seq: Some(change.seq.clone()),
            });
        } else {
            subsequent.push(change.clone());
        }
    }
    Ok(HoldingViewProjection {
        basis,
        observation: baseline.observation.clone(),
        settlement_basis: quantity.map(|q| HoldingAmounts {
            quantity: q.decimal(),
            value: value.map(|v| v.decimal()),
            source_id: if applied {
                None
            } else {
                baseline.source_id.clone()
            },
        }),
        confirmed_changes: subsequent,
    })
}
