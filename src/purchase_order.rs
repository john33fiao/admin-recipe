use crate::{
    business::{normalize_draft_uuid, validate_draft_text},
    catalog::CatalogState,
    domain::{DomainError, Quantity, Unit},
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderAction {
    Place,
    Revise,
    Cancel,
    Receive,
    Correct,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum DraftOrderActionV1 {
    Place {
        material_id: Option<String>,
        material_version_id: Option<String>,
        quantity: String,
        unit: String,
        supplier: String,
        expected_date: String,
        estimated_amount: String,
        note: String,
    },
    Revise {
        order_id: String,
        expected_order_version_id: String,
        material_version_id: Option<String>,
        quantity: String,
        unit: String,
        supplier: String,
        expected_date: String,
        estimated_amount: String,
        note: String,
    },
    Cancel {
        order_id: String,
        expected_order_version_id: String,
        note: String,
    },
}

impl DraftOrderActionV1 {
    pub fn action(&self) -> OrderAction {
        match self {
            Self::Place { .. } => OrderAction::Place,
            Self::Revise { .. } => OrderAction::Revise,
            Self::Cancel { .. } => OrderAction::Cancel,
        }
    }
}

/// Check draft structure while retaining incomplete numbers and dates.
pub fn normalize_order_draft(data: &mut DraftOrderActionV1) -> Result<(), DomainError> {
    match data {
        DraftOrderActionV1::Place {
            material_id,
            material_version_id,
            ..
        } => {
            if let Some(id) = material_id {
                normalize_draft_uuid(id, "재료 ID")?;
            }
            if let Some(id) = material_version_id {
                normalize_draft_uuid(id, "재료 버전")?;
            }
        }
        DraftOrderActionV1::Revise {
            order_id,
            expected_order_version_id,
            material_version_id,
            ..
        } => {
            normalize_draft_uuid(order_id, "주문 ID")?;
            normalize_draft_uuid(expected_order_version_id, "주문 버전")?;
            if let Some(id) = material_version_id {
                normalize_draft_uuid(id, "재료 버전")?;
            }
        }
        DraftOrderActionV1::Cancel {
            order_id,
            expected_order_version_id,
            ..
        } => {
            normalize_draft_uuid(order_id, "주문 ID")?;
            normalize_draft_uuid(expected_order_version_id, "주문 버전")?;
        }
    }
    let note = match data {
        DraftOrderActionV1::Place {
            quantity,
            unit,
            supplier,
            expected_date,
            estimated_amount,
            note,
            ..
        }
        | DraftOrderActionV1::Revise {
            quantity,
            unit,
            supplier,
            expected_date,
            estimated_amount,
            note,
            ..
        } => {
            for (text, label, limit) in [
                (quantity.as_str(), "주문 수량", 80),
                (unit.as_str(), "주문 단위", 20),
                (supplier.as_str(), "거래처명", 200),
                (expected_date.as_str(), "입고 예정일", 32),
                (estimated_amount.as_str(), "예상 금액", 80),
            ] {
                validate_draft_text(text, label, limit, false)?;
            }
            note
        }
        DraftOrderActionV1::Cancel { note, .. } => note,
    };
    *note = note.replace("\r\n", "\n").replace('\r', "\n");
    validate_draft_text(note, "메모", 2000, true)
}

pub fn parse_expected_date(text: &str) -> Result<Option<String>, DomainError> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let bytes = text.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| index != 4 && index != 7 && !byte.is_ascii_digit())
        || &bytes[..4] == b"0000"
        || NaiveDate::parse_from_str(text, "%Y-%m-%d").is_err()
    {
        return Err(DomainError(
            "입고 예정일은 YYYY-MM-DD 형식의 유효한 날짜로 입력하세요".into(),
        ));
    }
    Ok(Some(text.into()))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Open,
    Received,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatusFilter {
    #[default]
    Open,
    Received,
    Cancelled,
    All,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderFilter {
    #[serde(default)]
    pub status: OrderStatusFilter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderProposal {
    pub status: OrderStatus,
    pub material_id: String,
    pub material_version_id: String,
    pub quantity: String,
    pub unit: String,
    pub input_quantity: String,
    pub input_unit: String,
    pub supplier: Option<String>,
    pub expected_date: Option<String>,
    pub estimated_amount: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderVersion {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correction_projection_id: Option<String>,
    pub id: String,
    pub order_id: String,
    pub previous_version_id: Option<String>,
    pub confirmation_id: String,
    pub business_record_id: String,
    pub action: OrderAction,
    pub status: OrderStatus,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub quantity: String,
    pub unit: String,
    pub input_quantity: String,
    pub input_unit: String,
    pub supplier: Option<String>,
    pub expected_date: Option<String>,
    pub estimated_amount: Option<String>,
    pub note: String,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderSummary {
    pub id: String,
    pub head_id: String,
    pub status: OrderStatus,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub quantity: String,
    pub unit: String,
    pub input_quantity: String,
    pub input_unit: String,
    pub supplier: Option<String>,
    pub expected_date: Option<String>,
    pub estimated_amount: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderList {
    pub items: Vec<OrderSummary>,
    pub state: CatalogState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderDetail {
    pub item: OrderSummary,
    pub head: OrderVersion,
    pub versions: Vec<OrderVersion>,
    pub state: CatalogState,
}

/// Identifiers supplied by the future receipt adapter. This type does not
/// establish that any referenced event exists or has been committed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptReferences {
    pub receipt_id: Uuid,
    pub confirmation_id: Uuid,
    pub order_id: Uuid,
    pub expected_order_version_id: Uuid,
    pub material_id: Uuid,
    pub material_version_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct ReceiptResult {
    references: ReceiptReferences,
    quantity: Quantity,
    unit: Unit,
}

impl ReceiptResult {
    /// Convert a positive measured quantity exactly. Database existence and
    /// receipt/order atomic persistence remain the T-007 adapter's concern.
    pub fn new(
        references: ReceiptReferences,
        quantity: Quantity,
        input_unit: Unit,
    ) -> Result<Self, DomainError> {
        positive(quantity)?;
        let quantity = input_unit.to_canonical(quantity)?;
        let unit = Unit::parse(input_unit.canonical())?;
        Ok(Self {
            references,
            quantity,
            unit,
        })
    }

    pub fn references(&self) -> &ReceiptReferences {
        &self.references
    }
    pub fn quantity(&self) -> Quantity {
        self.quantity
    }
    pub fn unit(&self) -> &Unit {
        &self.unit
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderReferences {
    pub order_id: Uuid,
    pub material_id: Uuid,
    pub initial_order_version_id: Uuid,
    pub current_order_version_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct OrderForReceipt {
    references: OrderReferences,
    state: OrderStatus,
    initial_quantity: Quantity,
    ordered_quantity: Quantity,
    canonical_unit: Unit,
}

impl OrderForReceipt {
    /// Both quantities are already in the canonical unit. Reject display units
    /// here so a caller cannot accidentally apply their multiplier twice.
    pub fn new(
        references: OrderReferences,
        state: OrderStatus,
        initial_quantity: Quantity,
        ordered_quantity: Quantity,
        canonical_unit: Unit,
    ) -> Result<Self, DomainError> {
        positive(initial_quantity)?;
        positive(ordered_quantity)?;
        if canonical_unit != Unit::parse(canonical_unit.canonical())? {
            return Err(DomainError(
                "주문 기준 수량에는 기준 단위를 사용하세요".into(),
            ));
        }
        Ok(Self {
            references,
            state,
            initial_quantity,
            ordered_quantity,
            canonical_unit,
        })
    }

    pub fn references(&self) -> &OrderReferences {
        &self.references
    }
    pub fn state(&self) -> OrderStatus {
        self.state
    }
    pub fn initial_quantity(&self) -> Quantity {
        self.initial_quantity
    }
    pub fn ordered_quantity(&self) -> Quantity {
        self.ordered_quantity
    }
    pub fn canonical_unit(&self) -> &Unit {
        &self.canonical_unit
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderReceiptClosure {
    pub status: OrderStatus,
    pub order_id: Uuid,
    pub initial_order_version_id: Uuid,
    pub initial_ordered_quantity: String,
    pub ordered_version_id: Uuid,
    pub ordered_quantity: String,
    pub receipt_id: Uuid,
    pub receipt_confirmation_id: Uuid,
    pub receipt_material_version_id: Uuid,
    pub received_quantity: String,
    pub difference: String,
    pub unit: String,
}

fn positive(quantity: Quantity) -> Result<(), DomainError> {
    if quantity.micros() == 0 {
        return Err(DomainError("주문·실입고 수량은 0보다 커야 합니다".into()));
    }
    Ok(())
}

/// Compute a one-receipt closure without changing the order or writing any
/// receipt, inventory, price, or journal. Retries belong to the mutation layer.
pub fn close_order(
    order: &OrderForReceipt,
    receipt: &ReceiptResult,
) -> Result<OrderReceiptClosure, DomainError> {
    if order.state != OrderStatus::Open {
        return Err(DomainError(
            "미입고 주문만 입고로 종료할 수 있습니다".into(),
        ));
    }
    if order.references.order_id != receipt.references.order_id
        || order.references.current_order_version_id != receipt.references.expected_order_version_id
        || order.references.material_id != receipt.references.material_id
    {
        return Err(DomainError(
            "주문·품목·현재 주문 버전이 입고 참조와 다릅니다".into(),
        ));
    }
    if !order.canonical_unit.same_dimension(&receipt.unit) {
        return Err(DomainError(
            "입고 수량은 주문과 같은 차원 단위를 사용하세요".into(),
        ));
    }
    let difference = receipt
        .quantity
        .micros()
        .checked_sub(order.ordered_quantity.micros())
        .ok_or_else(|| DomainError("입고 수량 차이가 허용 범위를 넘습니다".into()))?;
    let magnitude = difference
        .checked_abs()
        .ok_or_else(|| DomainError("입고 수량 차이가 허용 범위를 넘습니다".into()))?;
    let magnitude = Quantity::from_micros(magnitude)?.decimal();
    let difference = if difference < 0 {
        format!("-{magnitude}")
    } else {
        magnitude
    };
    Ok(OrderReceiptClosure {
        status: OrderStatus::Received,
        order_id: order.references.order_id,
        initial_order_version_id: order.references.initial_order_version_id,
        initial_ordered_quantity: order.initial_quantity.decimal(),
        ordered_version_id: order.references.current_order_version_id,
        ordered_quantity: order.ordered_quantity.decimal(),
        receipt_id: receipt.references.receipt_id,
        receipt_confirmation_id: receipt.references.confirmation_id,
        receipt_material_version_id: receipt.references.material_version_id,
        received_quantity: receipt.quantity.decimal(),
        difference,
        unit: order.canonical_unit.canonical().into(),
    })
}
