use crate::{
    business::{normalize_draft_uuid, validate_draft_text},
    catalog::CatalogState,
    domain::DomainError,
    purchase_order::{OrderReceiptClosure, OrderStatus},
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReceiptSourceDraft {
    Direct {
        material_id: Option<String>,
        material_version_id: Option<String>,
    },
    Order {
        order_id: Option<String>,
        expected_order_version_id: Option<String>,
        material_version_id: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReceiptStockEffectDraft {
    Unreflected {},
    NeedsReconciliation {
        basis_id: Option<String>,
        included_quantity: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftReceiptV1 {
    pub source: ReceiptSourceDraft,
    pub quantity: String,
    pub unit: String,
    pub amount: String,
    pub receipt_date: String,
    pub note: String,
    pub stock_effect: ReceiptStockEffectDraft,
}

/// Keep incomplete business values; only normalize identifiers and note lines.
pub fn normalize_receipt_draft(data: &mut DraftReceiptV1) -> Result<(), DomainError> {
    let references = match &mut data.source {
        ReceiptSourceDraft::Direct {
            material_id,
            material_version_id,
        } => {
            vec![("재료 ID", material_id), ("재료 버전", material_version_id)]
        }
        ReceiptSourceDraft::Order {
            order_id,
            expected_order_version_id,
            material_version_id,
        } => vec![
            ("주문 ID", order_id),
            ("주문 버전", expected_order_version_id),
            ("재료 버전", material_version_id),
        ],
    };
    for (label, reference) in references {
        if let Some(id) = reference {
            normalize_draft_uuid(id, label)?;
        }
    }
    if let ReceiptStockEffectDraft::NeedsReconciliation {
        basis_id,
        included_quantity,
    } = &mut data.stock_effect
    {
        if let Some(id) = basis_id {
            normalize_draft_uuid(id, "포함 근거 ID")?;
        }
        validate_draft_text(included_quantity, "이미 포함된 수량", 80, false)?;
    }
    data.note = data.note.replace("\r\n", "\n").replace('\r', "\n");
    for (label, text, limit, multiline) in [
        ("실입고 수량", data.quantity.as_str(), 80, false),
        ("실제 구입금액", data.amount.as_str(), 80, false),
        ("입고 단위", data.unit.as_str(), 20, false),
        ("입고일", data.receipt_date.as_str(), 32, false),
        ("입고 메모", data.note.as_str(), 2000, true),
    ] {
        validate_draft_text(text, label, limit, multiline)?;
    }
    Ok(())
}

/// The caller supplies the server's local day once per preview/confirmation.
/// UTC recording time and the user's receipt day have separate meanings.
pub fn parse_receipt_date(text: &str, today: NaiveDate) -> Result<NaiveDate, DomainError> {
    let text = text.trim();
    let bytes = text.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || &bytes[..4] == b"0000"
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| index != 4 && index != 7 && !byte.is_ascii_digit())
    {
        return Err(DomainError(
            "입고일은 YYYY-MM-DD 형식의 실제 날짜로 입력하세요".into(),
        ));
    }
    let date = NaiveDate::parse_from_str(text, "%Y-%m-%d")
        .map_err(|_| DomainError("입고일은 실제 달력에 있는 날짜로 입력하세요".into()))?;
    if date > today {
        return Err(DomainError(
            "미래 예정일은 발주에 기록하고 실제 입고일을 입력하세요".into(),
        ));
    }
    Ok(date)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReceiptSource {
    Direct {},
    Order {
        order_id: String,
        ordered_version_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptRecord {
    pub id: String,
    pub confirmation_id: String,
    pub business_record_id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub quantity: String,
    pub unit: String,
    pub input_quantity: String,
    pub input_unit: String,
    pub amount: String,
    pub receipt_date: String,
    pub note: String,
    pub recorded_at: String,
    pub seq: String,
    pub source: ReceiptSource,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptFilter {
    pub material_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptList {
    pub items: Vec<ReceiptRecord>,
    pub state: CatalogState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReceiptDetail {
    pub item: ReceiptRecord,
    pub holding_version_id: String,
    pub order_closure: Option<OrderReceiptClosure>,
    pub state: CatalogState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderReceiptPreview {
    pub status: OrderStatus,
    pub order_id: String,
    pub initial_order_version_id: String,
    pub initial_ordered_quantity: String,
    pub ordered_version_id: String,
    pub ordered_quantity: String,
    pub received_quantity: String,
    pub difference: String,
    pub unit: String,
}
