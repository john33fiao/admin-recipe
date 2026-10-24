use crate::{
    catalog::{CatalogState, MaterialPriceVersion, MutationMeta},
    domain::DomainError,
};
use serde::{Deserialize, Deserializer, Serialize, de::Error};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BusinessKind {
    CatalogPrice,
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
#[serde(deny_unknown_fields)]
pub struct BusinessPayload {
    pub kind: BusinessKind,
    #[serde(deserialize_with = "payload_version")]
    pub version: u32,
    pub data: DraftPriceV1,
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
    if payload.version != 1 {
        return Err(DomainError("지원하지 않는 업무 입력 버전입니다".into()));
    }
    let data = &mut payload.data;
    for (label, id) in [
        ("재료 ID", &mut data.material_id),
        ("재료 버전", &mut data.material_version_id),
    ] {
        if let Some(value) = id {
            *value = Uuid::parse_str(value)
                .map_err(|_| DomainError(format!("{label}가 올바른 UUID가 아닙니다")))?
                .to_string();
        }
    }
    data.note = data.note.replace("\r\n", "\n").replace('\r', "\n");
    for (label, text, limit, multiline) in [
        ("수량", data.quantity.as_str(), 80, false),
        ("금액", data.amount.as_str(), 80, false),
        ("단위", data.unit.as_str(), 20, false),
        ("메모/정정 이유", data.note.as_str(), 2000, true),
    ] {
        if text.chars().count() > limit
            || text
                .chars()
                .any(|c| c.is_control() && !(multiline && c == '\n'))
        {
            return Err(DomainError(format!(
                "{label}은 {limit}자 이내이며 허용되지 않는 제어문자를 포함할 수 없습니다"
            )));
        }
    }
    Ok(payload)
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
#[serde(deny_unknown_fields)]
pub struct BusinessResult {
    pub kind: BusinessKind,
    pub price_id: String,
    pub material_id: String,
    pub material_version_id: String,
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
#[serde(deny_unknown_fields)]
pub struct BusinessEffect {
    pub kind: BusinessKind,
    pub material_id: String,
    pub material_version_id: String,
    pub quantity: String,
    pub amount: String,
    pub unit: String,
    pub corrects_confirmation_id: Option<String>,
    pub previous_price: Option<MaterialPriceVersion>,
    pub corrected_price: Option<MaterialPriceVersion>,
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
