use crate::{
    business::{normalize_draft_uuid, validate_draft_text},
    catalog::CatalogState,
    correction::{CorrectionPlanRef, CorrectionRefKind},
    domain::{DomainError, Money, Quantity, Unit, round_ratio},
    holding::{
        HoldingAmounts, HoldingBasisRef, HoldingInput, HoldingObservation, validate_holding_input,
    },
    receipt::ReceiptRecord,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftPurchaseReturnV1 {
    pub receipt_id: Option<String>,
    pub expected_receipt_version_id: Option<String>,
    pub quantity: String,
    pub unit: String,
    pub note: String,
}
pub fn normalize_purchase_return_draft(
    data: &mut DraftPurchaseReturnV1,
) -> Result<(), DomainError> {
    if let Some(id) = &mut data.receipt_id {
        normalize_draft_uuid(id, "원입고 ID")?;
    }
    if let Some(id) = &mut data.expected_receipt_version_id {
        normalize_draft_uuid(id, "원입고 버전")?;
    }
    validate_draft_text(&data.quantity, "실제 반품 수량", 80, false)?;
    validate_draft_text(&data.unit, "반품 입력 단위", 20, false)?;
    data.note = data.note.replace("\r\n", "\n").replace('\r', "\n");
    validate_draft_text(&data.note, "반품 메모", 2000, true)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PurchaseReturnSourceState {
    pub reference: CorrectionPlanRef,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub unit: String,
    pub quantity: String,
    pub amount: String,
    pub returned_quantity: String,
    pub remaining_quantity: String,
    pub logical_seq: String,
    pub seq: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PurchaseReturnSource {
    pub original: ReceiptRecord,
    pub current: Option<PurchaseReturnSourceState>,
    pub state: CatalogState,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PurchaseReturnReceiptBasis {
    pub reference: CorrectionPlanRef,
    pub original_quantity: String,
    pub original_amount: String,
    pub original_unit: String,
    pub quantity: String,
    pub amount: String,
    pub returned_quantity: String,
    pub remaining_quantity: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PurchaseReturnEffect {
    pub source: PurchaseReturnReceiptBasis,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub unit: String,
    pub input_quantity: String,
    pub input_unit: String,
    pub quantity: String,
    pub before: HoldingAmounts,
    pub after: HoldingAmounts,
    pub basis_before: HoldingBasisRef,
    pub observation: Option<HoldingObservation>,
    pub deducted_value: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PurchaseReturnRecord {
    pub id: String,
    pub confirmation_id: String,
    pub business_record_id: String,
    pub effect: PurchaseReturnEffect,
    pub note: String,
    pub recorded_at: String,
    pub seq: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PurchaseReturnEffectiveState {
    Active {
        version_id: String,
        effect: Box<PurchaseReturnEffect>,
        note: String,
    },
    Void {
        version_id: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PurchaseReturnSummary {
    pub item: PurchaseReturnRecord,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective: Option<PurchaseReturnEffectiveState>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PurchaseReturnList {
    pub items: Vec<PurchaseReturnSummary>,
    pub state: CatalogState,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PurchaseReturnDetail {
    pub item: PurchaseReturnRecord,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective: Option<PurchaseReturnEffectiveState>,
    pub state: CatalogState,
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PurchaseReturnFilter {
    pub receipt_id: Option<String>,
    pub material_id: Option<String>,
}
#[derive(Debug, Clone)]
pub struct PurchaseReturnContext {
    pub original: ReceiptRecord,
    pub source: PurchaseReturnSourceState,
    pub holding: HoldingInput,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseReturnConflict {
    pub field: String,
    pub message: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurchaseReturnCheck {
    Ready(Box<PurchaseReturnEffect>),
    NeedsReconciliation(PurchaseReturnConflict),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurchaseReturnProjection {
    pub before: HoldingAmounts,
    pub after: HoldingAmounts,
    pub basis_before: HoldingBasisRef,
    pub observation: Option<HoldingObservation>,
    pub deducted_value: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PurchaseReturnProjectionCheck {
    Ready(Box<PurchaseReturnProjection>),
    NeedsReconciliation(PurchaseReturnConflict),
}

/// Withdraw current average value; unlike partial waste, value leaves with Q.
/// Historical replay uses this boundary without rechecking an old request CAS.
pub fn project_purchase_return(
    before: &HoldingInput,
    quantity: Quantity,
) -> Result<PurchaseReturnProjectionCheck, DomainError> {
    validate_holding_input(before)?;
    if quantity.micros() == 0 {
        return Err(DomainError("실제 반품 수량은 0보다 커야 합니다".into()));
    }
    let (Some(q), Some(v)) = (before.quantity, before.value) else {
        return Ok(PurchaseReturnProjectionCheck::NeedsReconciliation(
            PurchaseReturnConflict {
                field: "quantity".into(),
                message: "반품 전 수량과 가치를 먼저 확인하세요. 입력한 실제량은 유지됩니다."
                    .into(),
            },
        ));
    };
    if quantity > q {
        return Ok(PurchaseReturnProjectionCheck::NeedsReconciliation(PurchaseReturnConflict{field:"quantity".into(),message:"실제 반품량이 현재 정산 기준을 넘습니다. 누락 입고·실사 또는 원기록을 확인하세요. 입력은 유지됩니다.".into()}));
    }
    let deducted = if quantity == q {
        v
    } else {
        Money::from_micros(
            i64::try_from(round_ratio(
                i128::from(v.micros()) * i128::from(quantity.micros()),
                i128::from(q.micros()),
            )?)
            .map_err(|_| DomainError("반품 가치가 허용 범위를 넘습니다".into()))?,
        )?
    };
    Ok(PurchaseReturnProjectionCheck::Ready(Box::new(
        PurchaseReturnProjection {
            before: HoldingAmounts {
                quantity: q.decimal(),
                value: Some(v.decimal()),
                source_id: before.source_id.clone(),
            },
            after: HoldingAmounts {
                quantity: Quantity::from_micros(q.micros() - quantity.micros())?.decimal(),
                value: Some(Money::from_micros(v.micros() - deducted.micros())?.decimal()),
                source_id: None,
            },
            basis_before: before
                .basis
                .clone()
                .ok_or_else(|| DomainError("반품 전 시작 근거가 없습니다".into()))?,
            observation: before.observation.clone(),
            deducted_value: deducted.decimal(),
        },
    )))
}

pub fn calculate_purchase_return(
    data: &DraftPurchaseReturnV1,
    context: &PurchaseReturnContext,
) -> Result<PurchaseReturnCheck, DomainError> {
    let mut normalized = data.clone();
    normalize_purchase_return_draft(&mut normalized)?;
    let (receipt, version) = match &context.source.reference {
        CorrectionPlanRef::Existing {
            entity: CorrectionRefKind::Receipt,
            id,
            version_id,
        } => (id, Some(version_id)),
        CorrectionPlanRef::Proposed {
            entity: CorrectionRefKind::Receipt,
            id,
            expected_version_id,
        } => (id, expected_version_id.as_ref()),
        _ => return Err(DomainError("반품 근거는 실제 원입고여야 합니다".into())),
    };
    if normalized.receipt_id.as_ref() != Some(receipt)
        || context.original.id != *receipt
        || normalized.expected_receipt_version_id.as_ref() != version
        || version.is_none()
    {
        return Err(DomainError("반품 원입고와 확인한 버전을 확인하세요".into()));
    }
    let unit = Unit::parse(&normalized.unit)?;
    let canonical = Unit::parse(&context.source.unit)?;
    if canonical.canonical() != context.source.unit || !unit.same_dimension(&canonical) {
        return Err(DomainError("반품 수량의 단위 차원을 확인하세요".into()));
    }
    let quantity = unit.to_canonical(Quantity::parse(&normalized.quantity)?)?;
    if quantity.micros() == 0 {
        return Err(DomainError("실제 반품 수량은 0보다 커야 합니다".into()));
    }
    let received = Quantity::parse(&context.source.quantity)?;
    let returned = Quantity::parse(&context.source.returned_quantity)?;
    let remaining = Quantity::parse(&context.source.remaining_quantity)?;
    if received.micros() == 0
        || returned > received
        || remaining.micros() != received.micros() - returned.micros()
    {
        return Err(DomainError(
            "원입고의 유효 반품 한도가 일치하지 않습니다".into(),
        ));
    }
    if quantity > remaining {
        return Err(DomainError(
            "반품 수량이 원입고의 미반환 한도를 넘습니다".into(),
        ));
    }
    let original_q = Quantity::parse(&context.original.quantity)?;
    let original_a = Money::parse(&context.original.amount)?;
    let amount = Money::parse(&context.source.amount)?;
    Unit::parse(&context.original.unit)?;
    let projection = match project_purchase_return(&context.holding, quantity)? {
        PurchaseReturnProjectionCheck::Ready(p) => p,
        PurchaseReturnProjectionCheck::NeedsReconciliation(c) => {
            return Ok(PurchaseReturnCheck::NeedsReconciliation(c));
        }
    };
    Ok(PurchaseReturnCheck::Ready(Box::new(PurchaseReturnEffect {
        source: PurchaseReturnReceiptBasis {
            reference: context.source.reference.clone(),
            original_quantity: original_q.decimal(),
            original_amount: original_a.decimal(),
            original_unit: context.original.unit.clone(),
            quantity: received.decimal(),
            amount: amount.decimal(),
            returned_quantity: returned.decimal(),
            remaining_quantity: remaining.decimal(),
        },
        material_id: context.source.material_id.clone(),
        material_version_id: context.source.material_version_id.clone(),
        material_name: context.source.material_name.clone(),
        unit: canonical.canonical().into(),
        input_quantity: normalized.quantity,
        input_unit: normalized.unit,
        quantity: quantity.decimal(),
        before: projection.before,
        after: projection.after,
        basis_before: projection.basis_before,
        observation: projection.observation,
        deducted_value: projection.deducted_value,
    })))
}
