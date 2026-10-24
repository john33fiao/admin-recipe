use crate::{
    business::{normalize_draft_uuid, validate_draft_text},
    catalog::CatalogState,
    domain::{
        DomainError, Money, Quantity, ResolvedPrice, Unit, UnitPrice, allocate_largest_remainder,
        round_ratio,
    },
    holding::{
        HoldingAmounts, HoldingBasisRef, HoldingInput, HoldingObservation, resolve_holding_price,
        validate_holding_input,
    },
    sale::{
        SaleConsumptionSource, SaleInterval, SalePriceSnapshot, SalePriceSource, eligible_sales,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftStocktakeV1 {
    pub rows: Vec<DraftStocktakeRowV1>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftStocktakeRowV1 {
    pub material_id: Option<String>,
    pub material_version_id: Option<String>,
    pub quantity: String,
    pub unit: String,
    pub initial_value: Option<StocktakeValueDraft>,
    pub excess: Option<StocktakeExcessDraft>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StocktakeValueDraft {
    ExistingPrice {},
    ManualAmount { amount: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StocktakeExcessDraft {
    pub reviewed: bool,
    pub reason: String,
    pub value: Option<StocktakeValueDraft>,
}

fn normalize_value(value: &Option<StocktakeValueDraft>) -> Result<(), DomainError> {
    if let Some(StocktakeValueDraft::ManualAmount { amount }) = value {
        validate_draft_text(amount, "확인 금액", 80, false)?;
    }
    Ok(())
}

pub fn normalize_stocktake_draft(data: &mut DraftStocktakeV1) -> Result<(), DomainError> {
    if data.rows.len() > 100 {
        return Err(DomainError("실사는 100행 이내로 입력하세요".into()));
    }
    for row in &mut data.rows {
        if let Some(id) = &mut row.material_id {
            normalize_draft_uuid(id, "실사 품목 ID")?;
        }
        if let Some(id) = &mut row.material_version_id {
            normalize_draft_uuid(id, "실사 품목 버전 ID")?;
        }
        validate_draft_text(&row.quantity, "실측 수량", 80, false)?;
        validate_draft_text(&row.unit, "입력 단위", 20, false)?;
        normalize_value(&row.initial_value)?;
        if let Some(excess) = &mut row.excess {
            excess.reason = excess.reason.replace("\r\n", "\n").replace('\r', "\n");
            validate_draft_text(&excess.reason, "초과 확인 이유", 2000, true)?;
            normalize_value(&excess.value)?;
        }
    }
    data.note = data.note.replace("\r\n", "\n").replace('\r', "\n");
    validate_draft_text(&data.note, "실사 메모", 2000, true)
}

#[derive(Debug, Clone)]
pub struct StocktakeMaterialContext {
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub unit: String,
    pub holding: HoldingInput,
    pub price: ResolvedPrice,
    pub sales: Vec<SaleConsumptionSource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StocktakeIssue {
    pub field: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StocktakeCheck {
    Ready(Box<StocktakePreview>),
    NeedsConfirmation(Vec<StocktakeIssue>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StocktakePreview {
    pub rows: Vec<StocktakeRowEffect>,
    pub unchecked_positions: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StocktakeRowEffect {
    pub position: usize,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub unit: String,
    pub input_quantity: String,
    pub input_unit: String,
    pub measured_quantity: String,
    pub before: Option<HoldingAmounts>,
    pub after: HoldingAmounts,
    pub basis_before: Option<HoldingBasisRef>,
    pub observation_before: Option<HoldingObservation>,
    pub calculation: StocktakeCalculation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum StocktakeCalculation {
    Initial {
        value: String,
        valuation: StocktakeValuation,
    },
    Settlement {
        consumption_quantity: String,
        consumption_value: String,
        excess_quantity: String,
        excess_value: String,
        valuation: Option<StocktakeValuation>,
        allocation: Box<ConsumptionAllocation>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StocktakeValuation {
    ExistingPrice {
        source: StocktakePriceSource,
        source_id: String,
        amount: String,
        quantity: String,
    },
    ManualAmount {
        amount: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StocktakePriceSource {
    HoldingAverage,
    LastValid,
    Preregistered,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsumptionAllocation {
    pub basis: HoldingBasisRef,
    pub end_seq: String,
    pub as_of_revision: String,
    pub total_quantity: String,
    pub total_value: String,
    pub candidates: Vec<SaleAllocationProposal>,
    pub unallocated_quantity: String,
    pub unallocated_value: String,
    pub reason: Option<NoAllocationReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoAllocationReason {
    NoSales,
    ZeroWeight,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaleAllocationProposal {
    pub sale_id: String,
    pub sale_version_id: String,
    pub recipe_version_id: String,
    pub original_seq: String,
    pub count: String,
    pub position: usize,
    pub quantity: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StocktakeRecord {
    pub id: String,
    pub confirmation_id: String,
    pub business_record_id: String,
    pub note: String,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StocktakeObservationRecord {
    pub id: String,
    pub version_id: String,
    pub previous_version_id: Option<String>,
    pub stocktake_id: String,
    pub previous_observation_version_id: Option<String>,
    pub consumption_version_id: Option<String>,
    pub effect: StocktakeRowEffect,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StocktakeSummary {
    pub id: String,
    pub confirmation_id: String,
    pub business_record_id: String,
    pub first_material_name: String,
    pub confirmed_count: usize,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StocktakeFilter {
    pub material_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StocktakeList {
    pub items: Vec<StocktakeSummary>,
    pub state: CatalogState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StocktakeDetail {
    pub item: StocktakeRecord,
    pub observations: Vec<StocktakeObservationRecord>,
    pub state: CatalogState,
}

fn invalid(message: &str) -> DomainError {
    DomainError(message.into())
}

fn issue(position: usize, field: &str, code: &str, message: &str) -> StocktakeIssue {
    StocktakeIssue {
        field: format!("rows.{position}.{field}"),
        code: code.into(),
        message: message.into(),
    }
}

fn exact_money(value: i128) -> Result<Money, DomainError> {
    Money::from_micros(
        i64::try_from(value).map_err(|_| invalid("실사 금액이 허용 범위를 넘습니다"))?,
    )
}

fn checked_price(context: &StocktakeMaterialContext) -> Result<ResolvedPrice, DomainError> {
    let parsed = SalePriceSnapshot::from_resolved(context.price.clone())?;
    let preregistered = if parsed.source == SalePriceSource::Preregistered {
        Some(UnitPrice::new(
            Money::parse(
                parsed
                    .amount
                    .as_deref()
                    .ok_or_else(|| invalid("가격 금액이 없습니다"))?,
            )?,
            Quantity::parse(
                parsed
                    .quantity
                    .as_deref()
                    .ok_or_else(|| invalid("가격 수량이 없습니다"))?,
            )?,
            parsed
                .source_id
                .ok_or_else(|| invalid("가격 출처가 없습니다"))?,
        )?)
    } else {
        None
    };
    let expected = resolve_holding_price(&context.holding, preregistered)?;
    if expected != context.price {
        return Err(invalid("실사 가격 출처가 현재 보유 기준과 다릅니다"));
    }
    Ok(expected)
}

fn confirmed_value(
    choice: Option<&StocktakeValueDraft>,
    context: &StocktakeMaterialContext,
    quantity: Quantity,
    allow_manual_with_price: bool,
) -> Result<Option<(Money, StocktakeValuation)>, DomainError> {
    let price = checked_price(context)?;
    match choice {
        Some(StocktakeValueDraft::ManualAmount { amount }) => {
            if !allow_manual_with_price && price.source != "missing" {
                return Ok(None);
            }
            let amount = Money::parse(amount)?;
            Ok(Some((
                amount,
                StocktakeValuation::ManualAmount {
                    amount: amount.decimal(),
                },
            )))
        }
        Some(StocktakeValueDraft::ExistingPrice {}) if price.source != "missing" => {
            let amount = Money::parse(
                price
                    .amount
                    .as_deref()
                    .ok_or_else(|| invalid("가격 금액이 없습니다"))?,
            )?;
            let basis = Quantity::parse(
                price
                    .quantity
                    .as_deref()
                    .ok_or_else(|| invalid("가격 기준 수량이 없습니다"))?,
            )?;
            let value = exact_money(round_ratio(
                i128::from(amount.micros())
                    .checked_mul(i128::from(quantity.micros()))
                    .ok_or_else(|| invalid("실사 가치 계산 범위를 넘습니다"))?,
                i128::from(basis.micros()),
            )?)?;
            let source = match price.source.as_str() {
                "holding_average" => StocktakePriceSource::HoldingAverage,
                "last_valid" => StocktakePriceSource::LastValid,
                "preregistered" => StocktakePriceSource::Preregistered,
                _ => return Err(invalid("지원하지 않는 실사 가격 출처입니다")),
            };
            Ok(Some((
                value,
                StocktakeValuation::ExistingPrice {
                    source,
                    source_id: price
                        .source_id
                        .ok_or_else(|| invalid("가격 출처가 없습니다"))?,
                    amount: amount.decimal(),
                    quantity: basis.decimal(),
                },
            )))
        }
        _ => Ok(None),
    }
}

fn allocate(
    context: &StocktakeMaterialContext,
    basis: &HoldingBasisRef,
    end_seq: i64,
    as_of_revision: i64,
    quantity: Quantity,
    value: Money,
) -> Result<ConsumptionAllocation, DomainError> {
    let after_seq = basis
        .seq
        .as_deref()
        .ok_or_else(|| invalid("실사 시작 순번이 없습니다"))?
        .parse::<i64>()
        .map_err(|_| invalid("실사 시작 순번이 올바르지 않습니다"))?;
    let sales = eligible_sales(
        &SaleInterval {
            material_id: context.material_id.clone(),
            after_seq,
            before_seq: end_seq,
        },
        &context.sales,
    )?;
    let weights = sales
        .iter()
        .map(|sale| i64::from(sale.count.value()))
        .collect::<Vec<_>>();
    let weight = weights.iter().try_fold(0_i128, |sum, n| {
        sum.checked_add(i128::from(*n))
            .ok_or_else(|| invalid("판매 개수 합계 범위를 넘습니다"))
    })?;
    let (quantities, values) = if weight == 0 {
        (None, None)
    } else {
        (
            Some(allocate_largest_remainder(quantity.micros(), &weights)?),
            Some(allocate_largest_remainder(value.micros(), &weights)?),
        )
    };
    let candidates = sales
        .into_iter()
        .enumerate()
        .map(|(position, sale)| {
            Ok(SaleAllocationProposal {
                sale_id: sale.sale_id,
                sale_version_id: sale.sale_version_id,
                recipe_version_id: sale.recipe_version_id,
                original_seq: sale.original_seq.to_string(),
                count: sale.count.decimal(),
                position,
                quantity: quantities
                    .as_ref()
                    .map(|rows| Quantity::from_micros(rows[position]).map(|q| q.decimal()))
                    .transpose()?,
                value: values
                    .as_ref()
                    .map(|rows| Money::from_micros(rows[position]).map(|v| v.decimal()))
                    .transpose()?,
            })
        })
        .collect::<Result<Vec<_>, DomainError>>()?;
    let reason = if candidates.is_empty() {
        Some(NoAllocationReason::NoSales)
    } else if weight == 0 {
        Some(NoAllocationReason::ZeroWeight)
    } else {
        None
    };
    Ok(ConsumptionAllocation {
        basis: basis.clone(),
        end_seq: end_seq.to_string(),
        as_of_revision: as_of_revision.to_string(),
        total_quantity: quantity.decimal(),
        total_value: value.decimal(),
        candidates,
        unallocated_quantity: if weight == 0 {
            quantity.decimal()
        } else {
            "0".into()
        },
        unallocated_value: if weight == 0 {
            value.decimal()
        } else {
            "0".into()
        },
        reason,
    })
}

fn calculate_row(
    row: &DraftStocktakeRowV1,
    position: usize,
    context: &StocktakeMaterialContext,
    end_seq: i64,
    as_of_revision: i64,
) -> Result<Result<StocktakeRowEffect, StocktakeIssue>, DomainError> {
    validate_holding_input(&context.holding)?;
    let unit = Unit::parse(&row.unit)?;
    let canonical = Unit::parse(&context.unit)?;
    if !unit.same_dimension(&canonical) {
        return Err(invalid("실사 수량의 단위 차원을 확인하세요"));
    }
    let measured = unit.to_canonical(Quantity::parse(&row.quantity)?)?;
    let zero_q = Quantity::from_micros(0)?;
    let zero_v = Money::from_micros(0)?;
    let before = context.holding.quantity.map(|q| HoldingAmounts {
        quantity: q.decimal(),
        value: context.holding.value.map(|v| v.decimal()),
        source_id: context.holding.source_id.clone(),
    });
    let (after_value, calculation) = if let Some(quantity) = context.holding.quantity {
        let Some(value) = context.holding.value else {
            return Ok(Err(issue(
                position,
                "quantity",
                "needs_reconciliation",
                "보유 가치를 먼저 확인한 뒤 실사를 다시 검토하세요. 입력은 유지됩니다.",
            )));
        };
        if row.initial_value.is_some() {
            return Ok(Err(issue(
                position,
                "initial_value",
                "confirmation_required",
                "이미 시작 기준이 있으므로 초기 가치 확인을 다시 검토하세요.",
            )));
        }
        let basis = context
            .holding
            .basis
            .as_ref()
            .ok_or_else(|| invalid("실사 시작 근거가 없습니다"))?;
        let (consumption, consumed_value, excess, excess_value, valuation, after_value) =
            if measured <= quantity {
                if row.excess.is_some() {
                    return Ok(Err(issue(
                        position,
                        "excess",
                        "confirmation_required",
                        "초과가 없는 현재 기준에 맞춰 초과 확인을 다시 검토하세요.",
                    )));
                }
                let consumed = Quantity::from_micros(quantity.micros() - measured.micros())?;
                let cost = if consumed.micros() == 0 {
                    zero_v
                } else if consumed == quantity {
                    value
                } else {
                    exact_money(round_ratio(
                        i128::from(value.micros()) * i128::from(consumed.micros()),
                        i128::from(quantity.micros()),
                    )?)?
                };
                (
                    consumed,
                    cost,
                    zero_q,
                    zero_v,
                    None,
                    Money::from_micros(value.micros() - cost.micros())?,
                )
            } else {
                let excess = Quantity::from_micros(measured.micros() - quantity.micros())?;
                let Some(confirmation) = row
                    .excess
                    .as_ref()
                    .filter(|c| c.reviewed && !c.reason.trim().is_empty())
                else {
                    return Ok(Err(issue(
                        position,
                        "excess",
                        "confirmation_required",
                        "누락 입고·생산·오입력을 먼저 확인하고 초과 이유를 남겨 주세요.",
                    )));
                };
                let Some((addition, valuation)) =
                    confirmed_value(confirmation.value.as_ref(), context, excess, false)?
                else {
                    return Ok(Err(issue(
                        position,
                        "excess.value",
                        "confirmation_required",
                        "사용 가능한 기존 단가를 확인하거나 단가가 없으면 초과분 금액을 확인하세요.",
                    )));
                };
                let after =
                    exact_money(i128::from(value.micros()) + i128::from(addition.micros()))?;
                (zero_q, zero_v, excess, addition, Some(valuation), after)
            };
        let allocation = allocate(
            context,
            basis,
            end_seq,
            as_of_revision,
            consumption,
            consumed_value,
        )?;
        (
            after_value,
            StocktakeCalculation::Settlement {
                consumption_quantity: consumption.decimal(),
                consumption_value: consumed_value.decimal(),
                excess_quantity: excess.decimal(),
                excess_value: excess_value.decimal(),
                valuation,
                allocation: Box::new(allocation),
            },
        )
    } else {
        if row.excess.is_some() {
            return Ok(Err(issue(
                position,
                "excess",
                "confirmation_required",
                "시작 기준이 없으므로 초과가 아닌 실제 시작 수량과 가치를 확인하세요.",
            )));
        }
        let Some((value, valuation)) =
            confirmed_value(row.initial_value.as_ref(), context, measured, true)?
        else {
            return Ok(Err(issue(
                position,
                "initial_value",
                "confirmation_required",
                "실제 시작 수량과 가치 또는 적용 단가를 확인하세요.",
            )));
        };
        if measured.micros() == 0 && value.micros() != 0 {
            return Err(invalid("실제 시작 수량 0의 가치는 0이어야 합니다"));
        }
        (
            value,
            StocktakeCalculation::Initial {
                value: value.decimal(),
                valuation,
            },
        )
    };
    Ok(Ok(StocktakeRowEffect {
        position,
        material_id: context.material_id.clone(),
        material_version_id: context.material_version_id.clone(),
        material_name: context.material_name.clone(),
        unit: canonical.canonical().into(),
        input_quantity: row.quantity.clone(),
        input_unit: row.unit.clone(),
        measured_quantity: measured.decimal(),
        before,
        after: HoldingAmounts {
            quantity: measured.decimal(),
            value: Some(after_value.decimal()),
            source_id: None,
        },
        basis_before: context.holding.basis.clone(),
        observation_before: context.holding.observation.clone(),
        calculation,
    }))
}

/// Evaluate every explicitly measured row against one immutable pre-confirmation state.
/// Blank rows have no effect, and no future observation or holding ID is allocated.
pub fn calculate_stocktake(
    data: &DraftStocktakeV1,
    contexts: &[StocktakeMaterialContext],
    end_seq: i64,
    as_of_revision: i64,
) -> Result<StocktakeCheck, DomainError> {
    if as_of_revision < 0 || as_of_revision.checked_add(1) != Some(end_seq) {
        return Err(invalid("실사 미리보기와 확정 순번이 일치하지 않습니다"));
    }
    let mut normalized = data.clone();
    normalize_stocktake_draft(&mut normalized)?;
    let mut indexed = HashMap::new();
    for context in contexts {
        for id in [&context.material_id, &context.material_version_id] {
            if !uuid::Uuid::parse_str(id).is_ok_and(|parsed| parsed.to_string() == id.as_str()) {
                return Err(invalid("실사 품목 근거 ID가 올바르지 않습니다"));
            }
        }
        if indexed
            .insert(context.material_id.as_str(), context)
            .is_some()
        {
            return Err(invalid("실사 품목 기준이 중복되었습니다"));
        }
    }
    let mut seen = HashSet::new();
    let mut rows = vec![];
    let mut unchecked_positions = vec![];
    let mut issues = vec![];
    for (position, row) in normalized.rows.iter().enumerate() {
        if row.quantity.trim().is_empty() {
            unchecked_positions.push(position);
            continue;
        }
        let Some(material) = row.material_id.as_deref() else {
            issues.push(issue(
                position,
                "material_id",
                "missing_reference",
                "실사 품목을 선택하세요.",
            ));
            continue;
        };
        if !seen.insert(material) {
            issues.push(issue(
                position,
                "material_id",
                "invalid_input",
                "한 품목은 한 번만 확인하세요.",
            ));
            continue;
        }
        let Some(context) = indexed.get(material) else {
            issues.push(issue(
                position,
                "material_id",
                "missing_reference",
                "실사 품목을 다시 선택하세요.",
            ));
            continue;
        };
        if row.material_version_id.as_deref() != Some(context.material_version_id.as_str()) {
            issues.push(issue(
                position,
                "material_version_id",
                "stale_reference",
                "현재 품목 버전을 다시 확인하세요.",
            ));
            continue;
        }
        match calculate_row(row, position, context, end_seq, as_of_revision) {
            Ok(Ok(effect)) => rows.push(effect),
            Ok(Err(problem)) => issues.push(problem),
            Err(error) => issues.push(issue(position, "quantity", "invalid_input", &error.0)),
        }
    }
    if rows.is_empty() && issues.is_empty() {
        issues.push(StocktakeIssue {
            field: "rows".into(),
            code: "invalid_input".into(),
            message: "실제 수량을 확인한 품목을 한 개 이상 입력하세요.".into(),
        });
    }
    if !issues.is_empty() {
        return Ok(StocktakeCheck::NeedsConfirmation(issues));
    }
    Ok(StocktakeCheck::Ready(Box::new(StocktakePreview {
        rows,
        unchecked_positions,
    })))
}
