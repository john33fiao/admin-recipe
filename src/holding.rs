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
    Production,
    Observation,
    Reconciliation,
    Correction,
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
    Production,
    Waste,
    Reconciliation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HoldingChange {
    Reconciliation {
        event_id: String,
        reconciliation_id: String,
        receipt_id: String,
        inflow_quantity: String,
        inflow_value: String,
        value_delta: String,
        recorded_at: String,
        seq: String,
    },
    Receipt {
        event_id: String,
        quantity: String,
        value: String,
        recorded_at: String,
        seq: String,
    },
    Production {
        event_id: String,
        production_id: String,
        input_quantity: String,
        input_value: String,
        output_quantity: String,
        output_value: String,
        recorded_at: String,
        seq: String,
    },
    Waste {
        event_id: String,
        quantity: String,
        loss_value: String,
        recorded_at: String,
        seq: String,
    },
}

impl HoldingChange {
    pub fn kind(&self) -> HoldingChangeKind {
        match self {
            Self::Receipt { .. } => HoldingChangeKind::Receipt,
            Self::Production { .. } => HoldingChangeKind::Production,
            Self::Waste { .. } => HoldingChangeKind::Waste,
            Self::Reconciliation { .. } => HoldingChangeKind::Reconciliation,
        }
    }
    pub fn event_id(&self) -> &str {
        match self {
            Self::Receipt { event_id, .. }
            | Self::Production { event_id, .. }
            | Self::Waste { event_id, .. } => event_id,
            Self::Reconciliation { event_id, .. } => event_id,
        }
    }
    pub fn recorded_at(&self) -> &str {
        match self {
            Self::Receipt { recorded_at, .. }
            | Self::Production { recorded_at, .. }
            | Self::Waste { recorded_at, .. } => recorded_at,
            Self::Reconciliation { recorded_at, .. } => recorded_at,
        }
    }
    pub fn seq(&self) -> &str {
        match self {
            Self::Receipt { seq, .. } | Self::Production { seq, .. } | Self::Waste { seq, .. } => {
                seq
            }
            Self::Reconciliation { seq, .. } => seq,
        }
    }
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
pub struct HoldingCorrection {
    pub id: String,
    pub recorded_at: String,
    pub seq: String,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correction: Option<HoldingCorrection>,
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
        (
            HoldingBasisKind::Receipt
            | HoldingBasisKind::Production
            | HoldingBasisKind::Reconciliation
            | HoldingBasisKind::Correction,
            None,
        ) => {}
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

fn change_amounts(
    change: &HoldingChange,
) -> Result<(Quantity, Money, Quantity, Money), DomainError> {
    let zero_q = Quantity::from_micros(0)?;
    let zero_v = Money::from_micros(0)?;
    match change {
        HoldingChange::Reconciliation {
            reconciliation_id,
            receipt_id,
            inflow_quantity,
            inflow_value,
            value_delta,
            ..
        } => {
            valid_id(reconciliation_id)?;
            valid_id(receipt_id)?;
            let q = Quantity::parse(inflow_quantity)?;
            let inflow = Money::parse(inflow_value)?;
            let negative = value_delta.starts_with('-');
            let delta = Money::parse(value_delta.strip_prefix('-').unwrap_or(value_delta))?;
            if (q.micros() == 0 && inflow.micros() != 0) || (negative && delta.micros() == 0) {
                return Err(invalid("보정 유입 수량·가치와 부호를 확인해 주세요"));
            }
            let out = if negative {
                inflow
            } else {
                Money::from_micros(
                    inflow
                        .micros()
                        .checked_add(delta.micros())
                        .ok_or_else(|| invalid("보정 가치 범위를 넘습니다"))?,
                )?
            };
            Ok((zero_q, if negative { delta } else { zero_v }, q, out))
        }
        HoldingChange::Waste {
            quantity,
            loss_value,
            ..
        } => {
            let q = Quantity::parse(quantity)?;
            if q.micros() == 0 {
                return Err(invalid("명시 폐기 수량은 0보다 커야 합니다"));
            }
            Ok((q, Money::parse(loss_value)?, zero_q, zero_v))
        }
        HoldingChange::Receipt {
            quantity, value, ..
        } => {
            let q = Quantity::parse(quantity)?;
            if q.micros() == 0 {
                return Err(invalid("실입고 수량은 0보다 커야 합니다"));
            }
            Ok((zero_q, zero_v, q, Money::parse(value)?))
        }
        HoldingChange::Production {
            production_id,
            input_quantity,
            input_value,
            output_quantity,
            output_value,
            ..
        } => {
            valid_id(production_id)?;
            let iq = Quantity::parse(input_quantity)?;
            let iv = Money::parse(input_value)?;
            let oq = Quantity::parse(output_quantity)?;
            let ov = Money::parse(output_value)?;
            if (iq.micros() == 0 && iv.micros() != 0)
                || (oq.micros() == 0 && ov.micros() != 0)
                || (iq.micros() == 0 && oq.micros() == 0)
            {
                return Err(invalid(
                    "생산 변동의 투입·결과 수량과 가치가 일치하지 않습니다",
                ));
            }
            Ok((iq, iv, oq, ov))
        }
    }
}

fn apply_change(
    current_q: Quantity,
    current_v: Money,
    input_q: Quantity,
    input_v: Money,
    output_q: Quantity,
    output_v: Money,
) -> Result<(Quantity, Money), DomainError> {
    if input_q.micros() > current_q.micros() || input_v.micros() > current_v.micros() {
        return Err(invalid("생산 투입이 입력 전 보유 수량·가치를 넘습니다"));
    }
    if input_q.micros() > 0 {
        let expected = crate::domain::round_ratio(
            i128::from(current_v.micros()) * i128::from(input_q.micros()),
            i128::from(current_q.micros()),
        )?;
        if i128::from(input_v.micros()) != expected {
            return Err(invalid(
                "생산 투입 가치가 입력 전 보유 평균 원가와 다릅니다",
            ));
        }
    }
    let q = (current_q.micros() - input_q.micros())
        .checked_add(output_q.micros())
        .ok_or_else(|| invalid("보유 수량 합계가 허용 범위를 넘습니다"))?;
    let v = (current_v.micros() - input_v.micros())
        .checked_add(output_v.micros())
        .ok_or_else(|| invalid("보유 가치 합계가 허용 범위를 넘습니다"))?;
    if q == 0 && v != 0 {
        return Err(invalid("확인한 수량 0의 보유 가치도 0이어야 합니다"));
    }
    Ok((Quantity::from_micros(q)?, Money::from_micros(v)?))
}

/// Reconstruct a view from the exact baseline Q/V and subsequent confirmed
/// receipts, production effects and explicit waste. This is not a current head.
/// The event establishing the baseline is checked, then excluded from changes.
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
        valid_id(change.event_id())?;
        timestamp(change.recorded_at())?;
        let seq = sequence(change.seq())?;
        let (input_q, input_v, output_q, output_v) = change_amounts(change)?;
        if seq <= prior_sequence || !ids.insert(change.event_id()) {
            return Err(invalid("확정 변동의 순서·중복·수량을 확인해 주세요"));
        }
        prior_sequence = seq;
        if baseline
            .basis
            .as_ref()
            .is_some_and(|b| b.id.as_deref() == Some(change.event_id()))
            && seq != boundary
        {
            return Err(invalid(
                "같은 시작 근거를 다른 확정 순서로 다시 적용할 수 없습니다",
            ));
        }
        if seq < boundary {
            continue;
        }
        if seq == boundary {
            let kind = match change.kind() {
                HoldingChangeKind::Receipt => HoldingBasisKind::Receipt,
                HoldingChangeKind::Production => HoldingBasisKind::Production,
                HoldingChangeKind::Reconciliation => HoldingBasisKind::Reconciliation,
                HoldingChangeKind::Waste => {
                    return Err(invalid("폐기는 시작 기준을 만들 수 없습니다"));
                }
            };
            if basis
                .as_ref()
                .is_none_or(|b| b.kind != kind || b.id.as_deref() != Some(change.event_id()))
                || input_q.micros() != 0
                || input_v.micros() != 0
                || quantity != Some(output_q)
                || value != Some(output_v)
            {
                return Err(invalid("시작 기준과 같은 순서의 확정 근거가 다릅니다"));
            }
            continue;
        }
        let (current_q, current_v) = match (quantity, value) {
            (None, None) => (Quantity::from_micros(0)?, Money::from_micros(0)?),
            (Some(q), Some(v)) => (q, v),
            _ => return Err(invalid("기존 보유 수량의 가치를 먼저 확인해 주세요")),
        };
        let (next_q, next_v) = if change.kind() == HoldingChangeKind::Waste {
            if input_q.micros() > current_q.micros() {
                return Err(invalid("폐기 수량이 확인한 정산 기준을 넘습니다"));
            }
            let expected_loss = if input_q == current_q {
                current_v.micros()
            } else {
                0
            };
            if input_v.micros() != expected_loss {
                return Err(invalid("부분 폐기 가치와 전량 폐기 손실의 근거가 다릅니다"));
            }
            (
                Quantity::from_micros(current_q.micros() - input_q.micros())?,
                Money::from_micros(current_v.micros() - expected_loss)?,
            )
        } else {
            apply_change(current_q, current_v, input_q, input_v, output_q, output_v)?
        };
        quantity = Some(next_q);
        value = Some(next_v);
        applied = true;
        if basis.is_none() {
            basis = Some(HoldingBasisRef {
                kind: match change.kind() {
                    HoldingChangeKind::Receipt => HoldingBasisKind::Receipt,
                    HoldingChangeKind::Production => HoldingBasisKind::Production,
                    HoldingChangeKind::Reconciliation => HoldingBasisKind::Reconciliation,
                    HoldingChangeKind::Waste => {
                        return Err(invalid("폐기는 시작 기준을 만들 수 없습니다"));
                    }
                },
                id: Some(change.event_id().into()),
                seq: Some(change.seq().into()),
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
