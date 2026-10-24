pub use crate::purchase_order::DraftOrderActionV1;
use crate::{
    catalog::{CatalogState, MaterialPriceVersion, MutationMeta},
    domain::DomainError,
    holding::{HoldingAmounts, HoldingBasisRef, HoldingObservation},
    production::{
        DraftProductionV1, ProductionInputRow, ProductionMaterialEffect, ProductionOutputRow,
        normalize_production_draft,
    },
    purchase_order::{OrderAction, OrderProposal, OrderVersion, normalize_order_draft},
    receipt::{DraftReceiptV1, OrderReceiptPreview, normalize_receipt_draft},
};
use serde::{Deserialize, Deserializer, Serialize, de::Error};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BusinessKind {
    CatalogPrice,
    PurchaseOrder,
    Receipt,
    Production,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BusinessStatus {
    Draft,
    Confirmed,
    Archived,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BusinessStatusFilter {
    #[default]
    Draft,
    Confirmed,
    Archived,
    All,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessFilter {
    #[serde(default)]
    pub status: BusinessStatusFilter,
    #[serde(default)]
    pub kind: Option<BusinessKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessMeta {
    pub request_id: String,
    pub epoch: String,
    pub expected_revision: String,
}
impl From<BusinessMeta> for MutationMeta {
    fn from(value: BusinessMeta) -> Self {
        Self {
            request_id: value.request_id,
            epoch: value.epoch,
            expected_revision: value.expected_revision,
        }
    }
}

fn payload_version<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    match u32::deserialize(deserializer)? {
        1 => Ok(1),
        _ => Err(D::Error::custom("지원하지 않는 업무 입력 버전입니다")),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BusinessPayload {
    CatalogPrice {
        #[serde(deserialize_with = "payload_version")]
        version: u32,
        data: DraftPriceV1,
    },
    PurchaseOrder {
        #[serde(deserialize_with = "payload_version")]
        version: u32,
        data: DraftOrderActionV1,
    },
    Receipt {
        #[serde(deserialize_with = "payload_version")]
        version: u32,
        data: DraftReceiptV1,
    },
    Production {
        #[serde(deserialize_with = "payload_version")]
        version: u32,
        data: DraftProductionV1,
    },
}

impl BusinessPayload {
    pub fn kind(&self) -> BusinessKind {
        match self {
            Self::CatalogPrice { .. } => BusinessKind::CatalogPrice,
            Self::PurchaseOrder { .. } => BusinessKind::PurchaseOrder,
            Self::Receipt { .. } => BusinessKind::Receipt,
            Self::Production { .. } => BusinessKind::Production,
        }
    }
    pub fn version(&self) -> u32 {
        match self {
            Self::CatalogPrice { version, .. }
            | Self::PurchaseOrder { version, .. }
            | Self::Receipt { version, .. }
            | Self::Production { version, .. } => *version,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DraftPriceV1 {
    pub material_id: Option<String>,
    pub material_version_id: Option<String>,
    pub quantity: String,
    pub unit: String,
    pub amount: String,
    pub note: String,
}

/// Draft text may be incomplete. Validate its structure without manufacturing
/// valid quantities, prices or references. Confirmation validates their meaning.
pub fn normalize_business_payload(
    mut payload: BusinessPayload,
) -> Result<BusinessPayload, DomainError> {
    if payload.version() != 1 {
        return Err(DomainError("지원하지 않는 업무 입력 버전입니다".into()));
    }
    match &mut payload {
        BusinessPayload::CatalogPrice { data, .. } => normalize_price_draft(data)?,
        BusinessPayload::PurchaseOrder { data, .. } => normalize_order_draft(data)?,
        BusinessPayload::Receipt { data, .. } => normalize_receipt_draft(data)?,
        BusinessPayload::Production { data, .. } => normalize_production_draft(data)?,
    }
    Ok(payload)
}

pub(crate) fn normalize_draft_uuid(value: &mut String, label: &str) -> Result<(), DomainError> {
    *value = Uuid::parse_str(value)
        .map_err(|_| DomainError(format!("{label}가 올바른 UUID가 아닙니다")))?
        .to_string();
    Ok(())
}

pub(crate) fn validate_draft_text(
    text: &str,
    label: &str,
    limit: usize,
    multiline: bool,
) -> Result<(), DomainError> {
    if text.chars().count() > limit
        || text
            .chars()
            .any(|c| c.is_control() && !(multiline && c == '\n'))
    {
        return Err(DomainError(format!(
            "{label}은 {limit}자 이내이며 허용되지 않는 제어문자를 포함할 수 없습니다"
        )));
    }
    Ok(())
}

fn normalize_price_draft(data: &mut DraftPriceV1) -> Result<(), DomainError> {
    for (label, id) in [
        ("재료 ID", &mut data.material_id),
        ("재료 버전", &mut data.material_version_id),
    ] {
        if let Some(value) = id {
            normalize_draft_uuid(value, label)?;
        }
    }
    data.note = data.note.replace("\r\n", "\n").replace('\r', "\n");
    for (label, text, limit, multiline) in [
        ("수량", data.quantity.as_str(), 80, false),
        ("금액", data.amount.as_str(), 80, false),
        ("단위", data.unit.as_str(), 20, false),
        ("메모/정정 이유", data.note.as_str(), 2000, true),
    ] {
        validate_draft_text(text, label, limit, multiline)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateBusinessDraft {
    pub meta: BusinessMeta,
    pub payload: BusinessPayload,
    pub corrects_confirmation_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviseBusinessDraft {
    pub meta: BusinessMeta,
    pub expected_head_id: String,
    pub payload: BusinessPayload,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewBusiness {
    pub expected_head_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmBusiness {
    pub meta: BusinessMeta,
    pub expected_head_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArchiveBusiness {
    pub meta: BusinessMeta,
    pub expected_head_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessIssue {
    pub field: String,
    pub code: String,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessRevision {
    pub id: String,
    pub record_id: String,
    pub previous_revision_id: Option<String>,
    pub status: BusinessStatus,
    pub payload: BusinessPayload,
    pub recorded_at: String,
    pub seq: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BusinessResult {
    Production {
        production_id: String,
    },
    CatalogPrice {
        price_id: String,
        material_id: String,
        material_version_id: String,
    },
    PurchaseOrder {
        action: OrderAction,
        order_id: String,
        order_version_id: String,
        material_id: String,
        material_version_id: String,
    },
    Receipt {
        receipt_id: String,
        holding_version_id: String,
        material_id: String,
        material_version_id: String,
        order_id: Option<String>,
        order_version_id: Option<String>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessConfirmation {
    pub id: String,
    pub record_id: String,
    pub revision_id: String,
    pub request_id: String,
    pub recorded_at: String,
    pub seq: String,
    pub result: BusinessResult,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessCorrection {
    pub target_confirmation_id: String,
    pub replacement_confirmation_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessSummary {
    pub id: String,
    pub kind: BusinessKind,
    pub head_id: String,
    pub status: BusinessStatus,
    pub created_at: String,
    pub updated_at: String,
    pub corrects_confirmation_id: Option<String>,
    pub confirmation_id: Option<String>,
    pub effective_confirmation_id: Option<String>,
    pub issue_count: usize,
    pub material_name: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessDetail {
    pub item: BusinessSummary,
    pub head: BusinessRevision,
    pub revisions: Vec<BusinessRevision>,
    pub confirmations: Vec<BusinessConfirmation>,
    pub corrections: Vec<BusinessCorrection>,
    pub issues: Vec<BusinessIssue>,
    pub state: CatalogState,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessList {
    pub items: Vec<BusinessSummary>,
    pub state: CatalogState,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessWrite {
    pub item: BusinessSummary,
    pub head: BusinessRevision,
    pub confirmation: Option<BusinessConfirmation>,
    pub issues: Vec<BusinessIssue>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessMutation {
    pub result: BusinessWrite,
    pub state: CatalogState,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum BusinessEffect {
    Production {
        inputs: Vec<ProductionInputRow>,
        outputs: Vec<ProductionOutputRow>,
        materials: Vec<ProductionMaterialEffect>,
        total_input_value: String,
    },
    CatalogPrice {
        material_id: String,
        material_version_id: String,
        quantity: String,
        amount: String,
        unit: String,
        corrects_confirmation_id: Option<String>,
        previous_price: Option<MaterialPriceVersion>,
        corrected_price: Option<MaterialPriceVersion>,
    },
    PurchaseOrder {
        action: OrderAction,
        order_id: Option<String>,
        before: Option<OrderVersion>,
        after: OrderProposal,
    },
    Receipt {
        material_id: String,
        material_version_id: String,
        quantity: String,
        amount: String,
        unit: String,
        input_quantity: String,
        input_unit: String,
        receipt_date: String,
        before: Option<HoldingAmounts>,
        after: HoldingAmounts,
        basis_before: Option<HoldingBasisRef>,
        observation: Option<HoldingObservation>,
        order_before: Option<Box<OrderVersion>>,
        order_after: Option<OrderReceiptPreview>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BusinessPreview {
    pub record_id: String,
    pub head_id: String,
    pub state: CatalogState,
    pub can_confirm: bool,
    pub issues: Vec<BusinessIssue>,
    pub effect: Option<BusinessEffect>,
}
