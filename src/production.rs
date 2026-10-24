use crate::{
    business::{normalize_draft_uuid, validate_draft_text},
    catalog::CatalogState,
    domain::{DomainError, Money, Quantity, Unit, allocate_largest_remainder, round_ratio},
    holding::{
        HoldingAmounts, HoldingBasisRef, HoldingInput, HoldingObservation, validate_holding_input,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftProductionInput {
    pub material_id: Option<String>,
    pub material_version_id: Option<String>,
    pub quantity: String,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftProductionOutput {
    pub material_id: Option<String>,
    pub material_version_id: Option<String>,
    pub quantity: String,
    pub unit: String,
    pub allocation_percent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftProductionV1 {
    pub inputs: Vec<DraftProductionInput>,
    pub outputs: Vec<DraftProductionOutput>,
    pub note: String,
}

fn normalize_row(
    material_id: &mut Option<String>,
    version_id: &mut Option<String>,
    quantity: &str,
    unit: &str,
) -> Result<(), DomainError> {
    if let Some(id) = material_id {
        normalize_draft_uuid(id, "생산 품목 ID")?;
    }
    if let Some(id) = version_id {
        normalize_draft_uuid(id, "생산 품목 버전 ID")?;
    }
    validate_draft_text(quantity, "실제 수량", 80, false)?;
    validate_draft_text(unit, "입력 단위", 20, false)
}

/// Keep incomplete measurements and percentages until preview/confirmation.
pub fn normalize_production_draft(data: &mut DraftProductionV1) -> Result<(), DomainError> {
    if data.inputs.len() > 100 || data.outputs.len() > 100 {
        return Err(invalid("투입과 결과는 각각 100행 이내로 입력하세요"));
    }
    for row in &mut data.inputs {
        normalize_row(
            &mut row.material_id,
            &mut row.material_version_id,
            &row.quantity,
            &row.unit,
        )?;
    }
    for row in &mut data.outputs {
        normalize_row(
            &mut row.material_id,
            &mut row.material_version_id,
            &row.quantity,
            &row.unit,
        )?;
        validate_draft_text(&row.allocation_percent, "원가 배분율", 80, false)?;
    }
    data.note = data.note.replace("\r\n", "\n").replace('\r', "\n");
    validate_draft_text(&data.note, "생산 메모", 2000, true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocationPercent(i64);

impl AllocationPercent {
    pub fn parse(text: &str) -> Result<Self, DomainError> {
        let value = Quantity::parse(text)?;
        if value.micros() > 100_000_000 {
            return Err(invalid("배분율은 0~100%로 입력하세요"));
        }
        Ok(Self(value.micros()))
    }
    pub fn micros(self) -> i64 {
        self.0
    }
    pub fn decimal(self) -> String {
        // The private constructor keeps this within the Quantity range.
        Quantity::from_micros(self.0).unwrap().decimal()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionInputRow {
    pub position: String,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub quantity: String,
    pub unit: String,
    pub input_quantity: String,
    pub input_unit: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionOutputRow {
    pub position: String,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub quantity: String,
    pub unit: String,
    pub input_quantity: String,
    pub input_unit: String,
    pub value: String,
    pub allocation_percent: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionMaterialEffect {
    pub material_id: String,
    pub material_version_id: String,
    pub unit: String,
    pub before: Option<HoldingAmounts>,
    pub after: HoldingAmounts,
    pub basis_before: Option<HoldingBasisRef>,
    pub observation: Option<HoldingObservation>,
    pub input_quantity: String,
    pub input_value: String,
    pub output_quantity: String,
    pub output_value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionPreview {
    pub inputs: Vec<ProductionInputRow>,
    pub outputs: Vec<ProductionOutputRow>,
    pub materials: Vec<ProductionMaterialEffect>,
    pub total_input_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionRecord {
    pub id: String,
    pub confirmation_id: String,
    pub business_record_id: String,
    pub note: String,
    pub recorded_at: String,
    pub seq: String,
    pub total_input_value: String,
    pub inputs: Vec<ProductionInputRow>,
    pub outputs: Vec<ProductionOutputRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionMaterialResult {
    pub id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub unit: String,
    pub before: Option<HoldingAmounts>,
    pub after: HoldingAmounts,
    pub basis_before: Option<HoldingBasisRef>,
    pub observation: Option<HoldingObservation>,
    pub input_quantity: String,
    pub input_value: String,
    pub output_quantity: String,
    pub output_value: String,
}

impl ProductionMaterialResult {
    pub fn new(id: String, effect: ProductionMaterialEffect) -> Self {
        Self {
            id,
            material_id: effect.material_id,
            material_version_id: effect.material_version_id,
            unit: effect.unit,
            before: effect.before,
            after: effect.after,
            basis_before: effect.basis_before,
            observation: effect.observation,
            input_quantity: effect.input_quantity,
            input_value: effect.input_value,
            output_quantity: effect.output_quantity,
            output_value: effect.output_value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionSummary {
    pub id: String,
    pub confirmation_id: String,
    pub business_record_id: String,
    pub first_output_name: String,
    pub input_count: usize,
    pub output_count: usize,
    pub total_input_value: String,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionFilter {
    pub material_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionList {
    pub items: Vec<ProductionSummary>,
    pub state: CatalogState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionDetail {
    pub item: ProductionRecord,
    pub materials: Vec<ProductionMaterialResult>,
    pub state: CatalogState,
}

/// One current material version and its settled state, read before any output.
#[derive(Debug, Clone)]
pub struct ProductionMaterialContext {
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub unit: String,
    pub holding: HoldingInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionConflictReason {
    UnknownQuantity,
    UnknownValue,
    InsufficientQuantity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionConflict {
    pub field: String,
    pub material_id: String,
    pub requested_quantity: String,
    pub available_quantity: Option<String>,
    pub source_id: Option<String>,
    pub reason: ProductionConflictReason,
}

impl ProductionConflict {
    pub fn message(&self) -> String {
        match self.reason {
            ProductionConflictReason::UnknownQuantity =>
                "생산 전 보유 수량이 미확인입니다. 실제 누락 입고·시작 기준·실사 또는 원기록 정정을 확인한 뒤 다시 미리보기하세요. 입력한 실제량은 유지됩니다.".into(),
            ProductionConflictReason::UnknownValue =>
                "생산 전 보유 가치가 미확인입니다. 실제 입고금액·시작 가치·실사 또는 원기록 정정 근거를 확인한 뒤 다시 미리보기하세요. 입력한 실제량은 유지됩니다.".into(),
            ProductionConflictReason::InsufficientQuantity => format!(
                "투입 합계 {}가 확인한 보유 수량 {}를 넘습니다(품목 기준 단위). 누락 입고·시작 기준·실사 또는 원기록 정정을 확인한 뒤 다시 미리보기하세요. 입력한 실제량은 유지됩니다.",
                self.requested_quantity, self.available_quantity.as_deref().unwrap_or("미확인")
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProductionCheck {
    Ready(ProductionPreview),
    NeedsReconciliation(Vec<ProductionConflict>),
}

fn invalid(message: &str) -> DomainError {
    DomainError(message.into())
}

#[derive(Default)]
struct Group {
    context: usize,
    inputs: Vec<usize>,
    outputs: Vec<usize>,
    input_quantity: i64,
    output_quantity: i64,
    input_value: i64,
    output_value: i64,
}

fn sum_quantity(left: i64, right: i64) -> Result<i64, DomainError> {
    Quantity::from_micros(
        left.checked_add(right)
            .ok_or_else(|| invalid("품목별 실제 수량 합계가 허용 범위를 넘습니다"))?,
    )
    .map(Quantity::micros)
}

fn sum_value(left: i64, right: i64) -> Result<i64, DomainError> {
    Money::from_micros(
        left.checked_add(right)
            .ok_or_else(|| invalid("생산 원가 합계가 허용 범위를 넘습니다"))?,
    )
    .map(Money::micros)
}

fn resolved_row(
    material_id: Option<&str>,
    version_id: Option<&str>,
    quantity: &str,
    unit: &str,
    contexts: &[ProductionMaterialContext],
    lookup: &HashMap<&str, usize>,
) -> Result<(usize, Quantity, Unit), DomainError> {
    let id = material_id.ok_or_else(|| invalid("생산의 모든 행에서 품목을 선택하세요"))?;
    let context = *lookup
        .get(id)
        .ok_or_else(|| invalid("생산 품목의 현재 보유 근거를 찾을 수 없습니다"))?;
    if version_id != Some(contexts[context].material_version_id.as_str()) {
        return Err(invalid("생산 품목의 현재 버전을 다시 확인하세요"));
    }
    let input_unit = Unit::parse(unit)?;
    let material_unit = Unit::parse(&contexts[context].unit)?;
    if !input_unit.same_dimension(&material_unit) {
        return Err(invalid("생산 입력 단위가 품목의 단위 차원과 다릅니다"));
    }
    let quantity = input_unit.to_canonical(Quantity::parse(quantity)?)?;
    if quantity.micros() == 0 {
        return Err(invalid("생산 투입과 결과의 각 수량은 0보다 커야 합니다"));
    }
    Ok((context, quantity, input_unit))
}

fn group_index(
    groups: &mut Vec<Group>,
    lookup: &mut HashMap<usize, usize>,
    context: usize,
) -> usize {
    *lookup.entry(context).or_insert_with(|| {
        groups.push(Group {
            context,
            ..Group::default()
        });
        groups.len() - 1
    })
}

/// Evaluate each material once against its pre-production state. No quantities
/// are inferred from recipes, future output, prices or expected sales.
pub fn project_production(
    data: &DraftProductionV1,
    contexts: &[ProductionMaterialContext],
) -> Result<ProductionCheck, DomainError> {
    let mut data = data.clone();
    normalize_production_draft(&mut data)?;
    if data.inputs.is_empty() || data.outputs.is_empty() {
        return Err(invalid("생산에는 투입과 결과가 각각 한 행 이상 필요합니다"));
    }
    let mut context_lookup = HashMap::new();
    for (position, context) in contexts.iter().enumerate() {
        for id in [&context.material_id, &context.material_version_id] {
            uuid::Uuid::parse_str(id)
                .map_err(|_| invalid("생산 품목 참조가 올바른 UUID가 아닙니다"))?;
        }
        if context_lookup
            .insert(context.material_id.as_str(), position)
            .is_some()
        {
            return Err(invalid("같은 품목의 생산 전 보유 근거가 중복되었습니다"));
        }
        validate_holding_input(&context.holding)?;
    }
    let mut inputs = Vec::with_capacity(data.inputs.len());
    let mut outputs = Vec::with_capacity(data.outputs.len());
    let mut groups = Vec::new();
    let mut group_lookup = HashMap::new();
    let mut input_quantities = Vec::with_capacity(data.inputs.len());
    let mut percentages = Vec::with_capacity(data.outputs.len());
    for (position, row) in data.inputs.iter().enumerate() {
        let (index, quantity, unit) = resolved_row(
            row.material_id.as_deref(),
            row.material_version_id.as_deref(),
            &row.quantity,
            &row.unit,
            contexts,
            &context_lookup,
        )?;
        let context = &contexts[index];
        let group_position = group_index(&mut groups, &mut group_lookup, index);
        let group = &mut groups[group_position];
        group.inputs.push(position);
        group.input_quantity = sum_quantity(group.input_quantity, quantity.micros())?;
        input_quantities.push(quantity.micros());
        inputs.push(ProductionInputRow {
            position: position.to_string(),
            material_id: context.material_id.clone(),
            material_version_id: context.material_version_id.clone(),
            material_name: context.material_name.clone(),
            quantity: quantity.decimal(),
            unit: unit.canonical().into(),
            input_quantity: row.quantity.trim().into(),
            input_unit: row.unit.trim().into(),
            value: "0".into(),
        });
    }
    for (position, row) in data.outputs.iter().enumerate() {
        let (index, quantity, unit) = resolved_row(
            row.material_id.as_deref(),
            row.material_version_id.as_deref(),
            &row.quantity,
            &row.unit,
            contexts,
            &context_lookup,
        )?;
        let context = &contexts[index];
        let percentage = AllocationPercent::parse(&row.allocation_percent)?;
        let group_position = group_index(&mut groups, &mut group_lookup, index);
        let group = &mut groups[group_position];
        group.outputs.push(position);
        group.output_quantity = sum_quantity(group.output_quantity, quantity.micros())?;
        percentages.push(percentage.micros());
        outputs.push(ProductionOutputRow {
            position: position.to_string(),
            material_id: context.material_id.clone(),
            material_version_id: context.material_version_id.clone(),
            material_name: context.material_name.clone(),
            quantity: quantity.decimal(),
            unit: unit.canonical().into(),
            input_quantity: row.quantity.trim().into(),
            input_unit: row.unit.trim().into(),
            value: "0".into(),
            allocation_percent: percentage.decimal(),
        });
    }
    let percentage_total = percentages
        .iter()
        .try_fold(0_i64, |total, value| total.checked_add(*value))
        .ok_or_else(|| invalid("배분율 합계가 허용 범위를 넘습니다"))?;
    if percentage_total != 100_000_000 {
        return Err(invalid("결과물의 원가 배분율 합계를 100%로 맞추세요"));
    }

    let mut conflicts = Vec::new();
    for group in &groups {
        let context = &contexts[group.context];
        let holding = &context.holding;
        let reason = match (holding.quantity, holding.value) {
            (None, _) if group.input_quantity > 0 => {
                Some(ProductionConflictReason::UnknownQuantity)
            }
            (Some(_), None) => Some(ProductionConflictReason::UnknownValue),
            (Some(quantity), _) if group.input_quantity > quantity.micros() => {
                Some(ProductionConflictReason::InsufficientQuantity)
            }
            _ => None,
        };
        if let Some(reason) = reason {
            let field = if let Some(position) = group.inputs.first() {
                format!("inputs.{position}.quantity")
            } else {
                format!("outputs.{}.quantity", group.outputs[0])
            };
            conflicts.push(ProductionConflict {
                field,
                material_id: context.material_id.clone(),
                requested_quantity: Quantity::from_micros(group.input_quantity)?.decimal(),
                available_quantity: holding.quantity.map(Quantity::decimal),
                source_id: holding.source_id.clone(),
                reason,
            });
        }
    }
    if !conflicts.is_empty() {
        return Ok(ProductionCheck::NeedsReconciliation(conflicts));
    }

    let mut total_value = 0;
    for group in &mut groups {
        if group.inputs.is_empty() {
            continue;
        }
        let before = &contexts[group.context].holding;
        let quantity = before
            .quantity
            .ok_or_else(|| invalid("투입 보유 수량을 확인하세요"))?
            .micros();
        let value = before
            .value
            .ok_or_else(|| invalid("투입 보유 가치를 확인하세요"))?
            .micros();
        let input_value = if group.input_quantity == quantity {
            value
        } else {
            let numerator = i128::from(value)
                .checked_mul(i128::from(group.input_quantity))
                .ok_or_else(|| invalid("투입 원가 계산이 허용 범위를 넘습니다"))?;
            i64::try_from(round_ratio(numerator, i128::from(quantity))?)
                .map_err(|_| invalid("투입 원가 계산이 허용 범위를 넘습니다"))?
        };
        if !(0..=value).contains(&input_value) {
            return Err(invalid("투입 가치가 생산 전 보유 가치를 벗어났습니다"));
        }
        group.input_value = input_value;
        total_value = sum_value(total_value, input_value)?;
        let weights: Vec<_> = group
            .inputs
            .iter()
            .map(|&position| input_quantities[position])
            .collect();
        let values = allocate_largest_remainder(input_value, &weights)?;
        for (&position, value) in group.inputs.iter().zip(values) {
            inputs[position].value = Money::from_micros(value)?.decimal();
        }
    }
    let output_values = allocate_largest_remainder(total_value, &percentages)?;
    for (row, &value) in outputs.iter_mut().zip(&output_values) {
        row.value = Money::from_micros(value)?.decimal();
    }
    let mut materials = Vec::with_capacity(groups.len());
    for group in &mut groups {
        for &position in &group.outputs {
            group.output_value = sum_value(group.output_value, output_values[position])?;
        }
        let context = &contexts[group.context];
        let before = &context.holding;
        let before_q = before.quantity.map(Quantity::micros).unwrap_or(0);
        let before_v = before.value.map(Money::micros).unwrap_or(0);
        let after_q = sum_quantity(before_q - group.input_quantity, group.output_quantity)?;
        let after_v = sum_value(before_v - group.input_value, group.output_value)?;
        if after_q == 0 && after_v != 0 {
            return Err(invalid("수량 0인 생산 결과에 보유 가치가 남을 수 없습니다"));
        }
        materials.push(ProductionMaterialEffect {
            material_id: context.material_id.clone(),
            material_version_id: context.material_version_id.clone(),
            unit: Unit::parse(&context.unit)?.canonical().into(),
            before: before.quantity.map(|quantity| HoldingAmounts {
                quantity: quantity.decimal(),
                value: before.value.map(Money::decimal),
                source_id: before.source_id.clone(),
            }),
            after: HoldingAmounts {
                quantity: Quantity::from_micros(after_q)?.decimal(),
                value: Some(Money::from_micros(after_v)?.decimal()),
                source_id: None,
            },
            basis_before: before.basis.clone(),
            observation: before.observation.clone(),
            input_quantity: Quantity::from_micros(group.input_quantity)?.decimal(),
            input_value: Money::from_micros(group.input_value)?.decimal(),
            output_quantity: Quantity::from_micros(group.output_quantity)?.decimal(),
            output_value: Money::from_micros(group.output_value)?.decimal(),
        });
    }
    Ok(ProductionCheck::Ready(ProductionPreview {
        inputs,
        outputs,
        materials,
        total_input_value: Money::from_micros(total_value)?.decimal(),
    }))
}
