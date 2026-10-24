//! Recipe input contracts and exact direct-component cost calculations.
use crate::{
    business::{BusinessMeta, normalize_draft_uuid, validate_draft_text},
    catalog::{ArchiveFilter, CatalogState, MaterialSummary},
    domain::{DomainError, Money, Quantity, ResolvedPrice, Unit, UnitPrice, round_ratio},
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeReferenceInput {
    pub quantity: String,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeLineInput {
    pub material_id: String,
    pub material_version_id: String,
    pub quantity: String,
    pub unit: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeDataInput {
    pub basis_count: String,
    pub reference_output: Option<RecipeReferenceInput>,
    pub selling_price: Option<String>,
    pub cooking_description: String,
    pub process: String,
    pub reference_time: String,
    pub lines: Vec<RecipeLineInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum RecipeOwnerInput {
    Existing {
        material_id: String,
        expected_material_version_id: String,
    },
    New {
        name: String,
        unit: String,
        kind: String,
        description: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateRecipe {
    pub meta: BusinessMeta,
    pub owner: RecipeOwnerInput,
    pub data: RecipeDataInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviseRecipe {
    pub meta: BusinessMeta,
    pub expected_material_version_id: String,
    pub expected_recipe_version_id: String,
    pub data: RecipeDataInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CopyRecipe {
    pub meta: BusinessMeta,
    pub expected_material_version_id: String,
    pub source_recipe_version_id: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeFilter {
    #[serde(default)]
    pub query: String,
    #[serde(default)]
    pub archive: ArchiveFilter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeReferenceOutput {
    pub quantity: String,
    pub unit: String,
    pub input_quantity: String,
    pub input_unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeLine {
    pub id: String,
    pub position: String,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub quantity: String,
    pub unit: String,
    pub input_quantity: String,
    pub input_unit: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeVersion {
    pub id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub material_unit: String,
    pub previous_version_id: Option<String>,
    pub copied_from_version_id: Option<String>,
    pub basis_count: String,
    pub reference_output: Option<RecipeReferenceOutput>,
    pub selling_price: Option<String>,
    pub cooking_description: String,
    pub process: String,
    pub reference_time: String,
    pub lines: Vec<RecipeLine>,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeVersionSummary {
    pub id: String,
    pub material_version_id: String,
    pub previous_version_id: Option<String>,
    pub copied_from_version_id: Option<String>,
    pub basis_count: String,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeSummary {
    pub item: MaterialSummary,
    pub recipe_version_id: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeList {
    pub items: Vec<RecipeSummary>,
    pub state: CatalogState,
}

pub type RecipeUses = RecipeList;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeDetail {
    pub item: MaterialSummary,
    pub head: RecipeVersion,
    pub versions: Vec<RecipeVersionSummary>,
    pub state: CatalogState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeMutation {
    pub item: MaterialSummary,
    pub version: RecipeVersion,
    pub state: CatalogState,
}

#[derive(Debug, Clone)]
pub struct RecipeCostLineInput {
    pub numerator: Quantity,
    pub price: Option<UnitPrice>,
}

#[derive(Debug, Clone)]
pub struct RecipeCostInput {
    pub basis_count: u32,
    pub rows: Vec<RecipeCostLineInput>,
    pub selling_price: Option<Money>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExactRatio {
    pub numerator: String,
    pub denominator: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecipeRatioUnavailable {
    MissingSellingPrice,
    ZeroSellingPrice,
    MissingCost,
}

#[derive(Debug, Clone)]
pub struct RecipeCostValue {
    pub row_costs: Vec<Option<Money>>,
    pub known_subtotal: Money,
    pub total_cost: Option<Money>,
    pub cost_ratio: Option<ExactRatio>,
    pub ratio_unavailable_reason: Option<RecipeRatioUnavailable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeCostRow {
    pub line_id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub numerator_quantity: String,
    pub basis_count: String,
    pub unit: String,
    pub price: ResolvedPrice,
    pub cost: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecipeCost {
    pub recipe_version_id: String,
    pub basis_count: String,
    pub price_mode: String,
    pub rows: Vec<RecipeCostRow>,
    pub missing_price_line_ids: Vec<String>,
    pub known_subtotal: String,
    pub total_cost: Option<String>,
    pub selling_price: Option<String>,
    pub cost_ratio: Option<ExactRatio>,
    pub ratio_unavailable_reason: Option<RecipeRatioUnavailable>,
    pub state: CatalogState,
}

pub fn parse_basis_count(text: &str) -> Result<u32, DomainError> {
    let text = text.trim();
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(DomainError(
            "기준 개수는 1~1000000000의 정수로 입력하세요".into(),
        ));
    }
    text.parse::<u32>()
        .ok()
        .filter(|n| (1..=1_000_000_000).contains(n))
        .ok_or_else(|| DomainError("기준 개수는 1~1000000000의 정수로 입력하세요".into()))
}

fn normalize_text(
    text: &mut String,
    label: &str,
    limit: usize,
    multiline: bool,
) -> Result<(), DomainError> {
    if multiline {
        *text = text.replace("\r\n", "\n").replace('\r', "\n");
    }
    validate_draft_text(text, label, limit, multiline)
}

/// Preserve entered quantities; canonical unit conversion belongs to the store's
/// referenced material-version check. Zero is explicit, never an empty fallback.
pub fn normalize_recipe_data(data: &mut RecipeDataInput) -> Result<(), DomainError> {
    data.basis_count = parse_basis_count(&data.basis_count)?.to_string();
    if data.lines.len() > 100 {
        return Err(DomainError("레시피 구성은 100행 이내로 입력하세요".into()));
    }
    if let Some(reference) = &mut data.reference_output {
        if data.basis_count != "1" {
            return Err(DomainError(
                "참고 생산량은 기준 개수가 1일 때 기록할 수 있습니다".into(),
            ));
        }
        reference.quantity = reference.quantity.trim().to_owned();
        reference.unit = reference.unit.trim().to_owned();
        if Quantity::parse(&reference.quantity)?.micros() == 0 {
            return Err(DomainError("참고 생산량은 0보다 커야 합니다".into()));
        }
        Unit::parse(&reference.unit)?;
    }
    if let Some(price) = &mut data.selling_price {
        *price = price.trim().to_owned();
        Money::parse(price)?;
    }
    normalize_text(&mut data.cooking_description, "조리 설명", 2000, true)?;
    normalize_text(&mut data.process, "조리 공정", 4000, true)?;
    normalize_text(&mut data.reference_time, "참고 시간", 200, false)?;
    for line in &mut data.lines {
        normalize_draft_uuid(&mut line.material_id, "구성 품목 ID")?;
        normalize_draft_uuid(&mut line.material_version_id, "구성 품목 버전 ID")?;
        line.quantity = line.quantity.trim().to_owned();
        line.unit = line.unit.trim().to_owned();
        Quantity::parse(&line.quantity)?;
        Unit::parse(&line.unit)?;
        normalize_text(&mut line.note, "구성 메모", 200, true)?;
    }
    Ok(())
}

/// No holdings, recursive recipe expansion, or display-rounded unit prices enter
/// this calculation. Each direct row is rounded once at the money boundary.
pub fn cost_recipe(input: &RecipeCostInput) -> Result<RecipeCostValue, DomainError> {
    if !(1..=1_000_000_000).contains(&input.basis_count) {
        return Err(DomainError(
            "계산 기준 개수가 허용 범위를 벗어났습니다".into(),
        ));
    }
    let overflow = || DomainError("레시피 원가가 계산 허용 범위를 벗어났습니다".into());
    let zero = Money::from_micros(0)?;
    let mut known_subtotal = zero;
    let mut row_costs = Vec::with_capacity(input.rows.len());
    let mut complete = true;
    for row in &input.rows {
        let cost = if row.numerator.micros() == 0 {
            Some(zero)
        } else if let Some(price) = &row.price {
            let numerator = i128::from(price.amount().micros())
                .checked_mul(i128::from(row.numerator.micros()))
                .ok_or_else(overflow)?;
            let denominator = i128::from(price.quantity().micros())
                .checked_mul(i128::from(input.basis_count))
                .ok_or_else(overflow)?;
            let rounded = round_ratio(numerator, denominator)?;
            Some(Money::from_micros(
                i64::try_from(rounded).map_err(|_| overflow())?,
            )?)
        } else {
            complete = false;
            None
        };
        if let Some(amount) = cost {
            known_subtotal = Money::from_micros(
                known_subtotal
                    .micros()
                    .checked_add(amount.micros())
                    .ok_or_else(overflow)?,
            )?;
        }
        row_costs.push(cost);
    }
    let total_cost = complete.then_some(known_subtotal);
    let (cost_ratio, ratio_unavailable_reason) = match (input.selling_price, total_cost) {
        (None, _) => (None, Some(RecipeRatioUnavailable::MissingSellingPrice)),
        (Some(price), _) if price.micros() == 0 => {
            (None, Some(RecipeRatioUnavailable::ZeroSellingPrice))
        }
        (_, None) => (None, Some(RecipeRatioUnavailable::MissingCost)),
        (Some(price), Some(cost)) => (
            Some(ExactRatio {
                numerator: cost.micros().to_string(),
                denominator: price.micros().to_string(),
            }),
            None,
        ),
    };
    Ok(RecipeCostValue {
        row_costs,
        known_subtotal,
        total_cost,
        cost_ratio,
        ratio_unavailable_reason,
    })
}

/// Use every current material name, including archived items, as `occupied`.
/// The original is never renamed; reserve Unicode scalar capacity for the suffix.
pub fn next_copy_name(original: &str, occupied: &HashSet<String>) -> Result<String, DomainError> {
    if original.trim().is_empty()
        || original.chars().count() > 100
        || original.chars().any(char::is_control)
    {
        return Err(DomainError(
            "복제할 이름은 제어문자 없이 1~100자여야 합니다".into(),
        ));
    }
    // At most occupied.len() candidates can collide. The next must be free.
    for number in 2..=occupied.len().saturating_add(2) {
        let suffix = format!(" ({number})");
        let prefix: String = original
            .chars()
            .take(100 - suffix.chars().count())
            .collect();
        let candidate = format!("{prefix}{suffix}");
        if !occupied.contains(&candidate) {
            return Ok(candidate);
        }
    }
    Err(DomainError("복제 이름을 만들 수 없습니다".into()))
}
