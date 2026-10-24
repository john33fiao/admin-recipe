//! Immutable sale snapshots and exact revenue/consumption-input contracts.
use crate::{
    business::{normalize_draft_uuid, validate_draft_text},
    catalog::CatalogState,
    domain::{DomainError, Money, Quantity, ResolvedPrice},
    recipe::{ExactRatio, RecipeRatioUnavailable},
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftSaleV1 {
    pub material_id: Option<String>,
    pub material_version_id: Option<String>,
    pub recipe_version_id: Option<String>,
    pub count: String,
    pub note: String,
}

/// Structural normalization preserves incomplete input for DB-backed drafts.
pub fn normalize_sale_draft(data: &mut DraftSaleV1) -> Result<(), DomainError> {
    for (label, id) in [
        ("판매 품목 ID", &mut data.material_id),
        ("판매 품목 버전", &mut data.material_version_id),
        ("판매 레시피 버전", &mut data.recipe_version_id),
    ] {
        if let Some(id) = id {
            normalize_draft_uuid(id, label)?;
        }
    }
    data.note = data.note.replace("\r\n", "\n").replace('\r', "\n");
    validate_draft_text(&data.count, "판매 개수", 80, false)?;
    validate_draft_text(&data.note, "메모/정정 이유", 2000, true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SaleCount(u32);

impl SaleCount {
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        let raw = raw.trim();
        let invalid = || DomainError("판매 개수는 0~1000000000의 정수로 입력하세요".into());
        if raw.is_empty() || !raw.bytes().all(|value| value.is_ascii_digit()) {
            return Err(invalid());
        }
        raw.parse::<u32>()
            .ok()
            .filter(|value| *value <= 1_000_000_000)
            .map(Self)
            .ok_or_else(invalid)
    }

    pub fn value(self) -> u32 {
        self.0
    }

    pub fn decimal(self) -> String {
        self.0.to_string()
    }
}

pub fn calculate_sale_amount(
    count: SaleCount,
    selling_price: Option<Money>,
) -> Result<Option<Money>, DomainError> {
    selling_price
        .map(|price| {
            let overflow = || DomainError("판매금액이 계산 허용 범위를 벗어났습니다".into());
            let amount = i128::from(price.micros())
                .checked_mul(i128::from(count.value()))
                .ok_or_else(overflow)?;
            Money::from_micros(i64::try_from(amount).map_err(|_| overflow())?)
        })
        .transpose()
}

/// A list total is wider than one sale's Money bound. Missing amounts remain
/// explicitly counted, including a zero-count sale with no selling price.
pub fn sum_sale_amounts(amounts: &[Option<Money>]) -> Result<(String, usize), DomainError> {
    let overflow = || DomainError("판매금액 합계가 계산 허용 범위를 벗어났습니다".into());
    let mut total = 0_i128;
    let mut missing = 0_usize;
    for amount in amounts {
        match amount {
            Some(amount) => {
                total = total
                    .checked_add(i128::from(amount.micros()))
                    .ok_or_else(overflow)?;
            }
            None => missing = missing.checked_add(1).ok_or_else(overflow)?,
        }
    }
    let whole = total / 1_000_000;
    let fraction = total % 1_000_000;
    let text = if fraction == 0 {
        whole.to_string()
    } else {
        let fractional = format!("{fraction:06}");
        format!("{whole}.{}", fractional.trim_end_matches('0'))
    };
    Ok((text, missing))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SalePriceSource {
    HoldingAverage,
    LastValid,
    Preregistered,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SalePriceSnapshot {
    pub source: SalePriceSource,
    pub source_id: Option<String>,
    pub amount: Option<String>,
    pub quantity: Option<String>,
    pub reason: Option<String>,
}

impl SalePriceSnapshot {
    pub fn from_resolved(price: ResolvedPrice) -> Result<Self, DomainError> {
        let source = match price.source.as_str() {
            "holding_average" => SalePriceSource::HoldingAverage,
            "last_valid" => SalePriceSource::LastValid,
            "preregistered" => SalePriceSource::Preregistered,
            "missing" => SalePriceSource::Missing,
            _ => {
                return Err(DomainError(
                    "판매 원가의 가격 출처가 올바르지 않습니다".into(),
                ));
            }
        };
        let snapshot = Self {
            source,
            source_id: price.source_id,
            amount: price.amount,
            quantity: price.quantity,
            reason: price.reason,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    /// Check shape and exact values; the store additionally checks the real
    /// source record, material ownership and historical price precedence.
    pub fn validate(&self) -> Result<(), DomainError> {
        let invalid = || DomainError("판매 원가의 가격 근거를 확인해 주세요".into());
        if self.source == SalePriceSource::Missing {
            if self.source_id.is_some()
                || self.amount.is_some()
                || self.quantity.is_some()
                || self
                    .reason
                    .as_ref()
                    .is_none_or(|text| text.trim().is_empty())
            {
                return Err(invalid());
            }
            return Ok(());
        }
        let source = self.source_id.as_deref().ok_or_else(invalid)?;
        if Uuid::parse_str(source).map_err(|_| invalid())?.to_string() != source
            || self.reason.is_some()
        {
            return Err(invalid());
        }
        let amount_text = self.amount.as_deref().ok_or_else(invalid)?;
        let quantity_text = self.quantity.as_deref().ok_or_else(invalid)?;
        let amount = Money::parse(amount_text)?;
        let quantity = Quantity::parse(quantity_text)?;
        if quantity.micros() == 0
            || amount.decimal() != amount_text
            || quantity.decimal() != quantity_text
        {
            return Err(invalid());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaleRecipeCostRow {
    pub line_id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub numerator_quantity: String,
    pub basis_count: String,
    pub unit: String,
    pub price: SalePriceSnapshot,
    pub cost: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaleRecipeCost {
    pub recipe_version_id: String,
    pub basis_count: String,
    pub rows: Vec<SaleRecipeCostRow>,
    pub missing_price_line_ids: Vec<String>,
    pub known_subtotal: String,
    pub total_cost: Option<String>,
    pub selling_price: Option<String>,
    pub cost_ratio: Option<ExactRatio>,
    pub ratio_unavailable_reason: Option<RecipeRatioUnavailable>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaleOriginal {
    pub id: String,
    pub original_confirmation_id: String,
    pub business_record_id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub recipe_version_id: String,
    pub recipe_cost: SaleRecipeCost,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaleVersion {
    pub id: String,
    pub sale_id: String,
    pub previous_version_id: Option<String>,
    pub confirmation_id: String,
    pub business_record_id: String,
    pub count: String,
    pub amount: Option<String>,
    pub note: String,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaleProposal {
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub recipe_version_id: String,
    pub count: String,
    pub amount: Option<String>,
    pub recipe_cost: SaleRecipeCost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaleConsumptionStatus {
    Unconfirmed,
    PartiallySettled,
    Settled,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaleMaterialConsumptionStatus {
    Unconfirmed,
    Allocated,
    Unallocated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaleConsumptionReason {
    NotObserved,
    InitialBasisMissing,
    ZeroWeight,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaleMaterialConsumption {
    pub material_id: String,
    pub material_name: String,
    pub unit: String,
    pub status: SaleMaterialConsumptionStatus,
    pub stocktake_id: Option<String>,
    pub observation_id: Option<String>,
    pub observation_version_id: Option<String>,
    pub consumption_version_id: Option<String>,
    pub sale_version_id: Option<String>,
    pub quantity: Option<String>,
    pub value: Option<String>,
    pub reason: Option<SaleConsumptionReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaleSummary {
    pub id: String,
    pub material_id: String,
    pub material_name: String,
    pub recipe_version_id: String,
    pub current_version_id: String,
    pub count: String,
    pub selling_price: Option<String>,
    pub amount: Option<String>,
    pub original_seq: String,
    pub recorded_at: String,
    pub updated_at: String,
    pub consumption_status: SaleConsumptionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaleList {
    pub items: Vec<SaleSummary>,
    pub known_revenue: String,
    pub missing_revenue_count: usize,
    pub state: CatalogState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaleDetail {
    pub item: SaleOriginal,
    pub head: SaleVersion,
    pub versions: Vec<SaleVersion>,
    pub consumption_status: SaleConsumptionStatus,
    pub consumption: Vec<SaleMaterialConsumption>,
    pub state: CatalogState,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaleFilter {
    pub material_id: Option<String>,
}

/// Internal exact input after the store selects effective versions as of a
/// specified revision. This is not a persisted stocktake or allocation record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaleConsumptionSource {
    pub sale_id: String,
    pub sale_version_id: String,
    pub recipe_version_id: String,
    pub original_seq: i64,
    pub count: SaleCount,
    pub direct_material_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SaleInterval {
    pub material_id: String,
    pub after_seq: i64,
    pub before_seq: i64,
}

/// Keep direct references (including zero-quantity recipe lines) once per sale.
/// No recursive recipe expansion, inventory mutation or consumption estimate.
pub fn eligible_sales(
    interval: &SaleInterval,
    sources: &[SaleConsumptionSource],
) -> Result<Vec<SaleConsumptionSource>, DomainError> {
    let invalid = || DomainError("판매 소모 연결의 구간과 참조를 확인해 주세요".into());
    let target = Uuid::parse_str(&interval.material_id).map_err(|_| invalid())?;
    if interval.after_seq < 0 || interval.before_seq <= interval.after_seq {
        return Err(invalid());
    }
    let mut sale_ids = HashSet::new();
    let mut version_ids = HashSet::new();
    let mut selected = Vec::new();
    for source in sources {
        let sale = Uuid::parse_str(&source.sale_id).map_err(|_| invalid())?;
        let version = Uuid::parse_str(&source.sale_version_id).map_err(|_| invalid())?;
        let recipe = Uuid::parse_str(&source.recipe_version_id).map_err(|_| invalid())?;
        if source.original_seq <= 0 || !sale_ids.insert(sale) || !version_ids.insert(version) {
            return Err(invalid());
        }
        let mut direct = HashSet::new();
        let mut refs = Vec::new();
        for material in &source.direct_material_ids {
            let id = Uuid::parse_str(material).map_err(|_| invalid())?;
            if direct.insert(id) {
                refs.push(id.to_string());
            }
        }
        if interval.after_seq < source.original_seq
            && source.original_seq < interval.before_seq
            && direct.contains(&target)
        {
            selected.push(SaleConsumptionSource {
                sale_id: sale.to_string(),
                sale_version_id: version.to_string(),
                recipe_version_id: recipe.to_string(),
                original_seq: source.original_seq,
                count: source.count,
                direct_material_ids: refs,
            });
        }
    }
    selected.sort_by(|left, right| {
        left.original_seq
            .cmp(&right.original_seq)
            .then_with(|| left.sale_id.cmp(&right.sale_id))
    });
    Ok(selected)
}
