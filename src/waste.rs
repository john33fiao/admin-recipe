use crate::{
    business::{normalize_draft_uuid, validate_draft_text},
    catalog::CatalogState,
    domain::{DomainError, Money, Quantity, Unit},
    holding::{
        HoldingAmounts, HoldingBasisRef, HoldingInput, HoldingObservation, validate_holding_input,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WasteReason {
    Discard,
    StaffMeal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftWasteV1 {
    pub material_id: Option<String>,
    pub material_version_id: Option<String>,
    pub quantity: String,
    pub unit: String,
    pub reason: WasteReason,
    pub note: String,
}

pub fn normalize_waste_draft(data: &mut DraftWasteV1) -> Result<(), DomainError> {
    if let Some(id) = &mut data.material_id {
        normalize_draft_uuid(id, "폐기 품목 ID")?;
    }
    if let Some(id) = &mut data.material_version_id {
        normalize_draft_uuid(id, "폐기 품목 버전 ID")?;
    }
    validate_draft_text(&data.quantity, "실제 폐기 수량", 80, false)?;
    validate_draft_text(&data.unit, "입력 단위", 20, false)?;
    data.note = data.note.replace("\r\n", "\n").replace('\r', "\n");
    validate_draft_text(&data.note, "폐기 메모", 2000, true)
}

#[derive(Debug, Clone)]
pub struct WasteMaterialContext {
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub unit: String,
    pub holding: HoldingInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasteConflict {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasteCheck {
    Ready(Box<WasteEffect>),
    NeedsReconciliation(WasteConflict),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WasteEffect {
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub unit: String,
    pub input_quantity: String,
    pub input_unit: String,
    pub quantity: String,
    pub reason: WasteReason,
    pub before: HoldingAmounts,
    pub after: HoldingAmounts,
    pub basis_before: HoldingBasisRef,
    pub observation: Option<HoldingObservation>,
    pub loss_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WasteRecord {
    pub id: String,
    pub confirmation_id: String,
    pub business_record_id: String,
    pub effect: WasteEffect,
    pub note: String,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WasteFilter {
    pub material_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WasteList {
    pub items: Vec<WasteRecord>,
    pub state: CatalogState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WasteDetail {
    pub item: WasteRecord,
    pub state: CatalogState,
}

/// Explicit waste retains value in a positive remainder and ends all value only
/// when the entire quantity is discarded. It never creates a new observation.
pub fn calculate_waste(
    data: &DraftWasteV1,
    context: &WasteMaterialContext,
) -> Result<WasteCheck, DomainError> {
    let mut normalized = data.clone();
    normalize_waste_draft(&mut normalized)?;
    if normalized.material_id.as_deref() != Some(context.material_id.as_str())
        || normalized.material_version_id.as_deref() != Some(context.material_version_id.as_str())
    {
        return Err(DomainError("폐기 품목과 현재 버전을 확인하세요".into()));
    }
    validate_holding_input(&context.holding)?;
    let unit = Unit::parse(&normalized.unit)?;
    let canonical = Unit::parse(&context.unit)?;
    if !unit.same_dimension(&canonical) {
        return Err(DomainError("폐기 수량의 단위 차원을 확인하세요".into()));
    }
    let quantity = unit.to_canonical(Quantity::parse(&normalized.quantity)?)?;
    if quantity.micros() == 0 {
        return Err(DomainError("실제 폐기 수량은 0보다 커야 합니다".into()));
    }
    let (Some(before_q), Some(before_v)) = (context.holding.quantity, context.holding.value) else {
        return Ok(WasteCheck::NeedsReconciliation(WasteConflict {
            field: "quantity".into(),
            message: "폐기 전 수량과 가치를 먼저 확인하세요. 입력한 실제량은 유지됩니다.".into(),
        }));
    };
    if quantity > before_q {
        return Ok(WasteCheck::NeedsReconciliation(WasteConflict{field:"quantity".into(),message:"실제 폐기량이 현재 정산 기준을 넘습니다. 누락 입고·시작 기준·실사 또는 원기록을 확인하세요. 입력은 유지됩니다.".into()}));
    }
    let remaining = Quantity::from_micros(before_q.micros() - quantity.micros())?;
    let (after_v, loss) = if remaining.micros() == 0 {
        (Money::from_micros(0)?, before_v)
    } else {
        (before_v, Money::from_micros(0)?)
    };
    Ok(WasteCheck::Ready(Box::new(WasteEffect {
        material_id: context.material_id.clone(),
        material_version_id: context.material_version_id.clone(),
        material_name: context.material_name.clone(),
        unit: canonical.canonical().into(),
        input_quantity: normalized.quantity,
        input_unit: normalized.unit,
        quantity: quantity.decimal(),
        reason: normalized.reason,
        before: HoldingAmounts {
            quantity: before_q.decimal(),
            value: Some(before_v.decimal()),
            source_id: context.holding.source_id.clone(),
        },
        after: HoldingAmounts {
            quantity: remaining.decimal(),
            value: Some(after_v.decimal()),
            source_id: None,
        },
        basis_before: context
            .holding
            .basis
            .clone()
            .ok_or_else(|| DomainError("폐기 전 시작 근거가 없습니다".into()))?,
        observation: context.holding.observation.clone(),
        loss_value: loss.decimal(),
    })))
}
