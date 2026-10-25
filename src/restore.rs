use crate::{business::BusinessMeta, catalog::CatalogState};
use serde::{Deserialize, Serialize};

pub const RESTORE_TARGET_PAGE_SIZE: usize = 25;
pub const RESTORE_PHOTO_EXAMPLE_LIMIT: usize = 20;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreTargetSource {
    #[default]
    Journal,
    Import,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreSourceKind {
    Journal,
    BeforeRestore,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreTargetQuery {
    #[serde(default)]
    pub source: RestoreTargetSource,
    pub cursor: Option<String>,
}

impl RestoreTargetQuery {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(cursor) = &self.cursor {
            match self.source {
                RestoreTargetSource::Journal => {
                    positive_sequence(cursor)?;
                }
                RestoreTargetSource::Import => validate_restore_sha256(cursor)?,
            }
        }
        Ok(())
    }
}

/// A closed lookup key, never a filesystem path supplied by the caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RestoreTargetId {
    Journal(i64),
    Import(String),
}

impl RestoreTargetId {
    pub fn parse(input: &str) -> Result<Self, String> {
        if let Some(sequence) = input.strip_prefix("journal:") {
            return positive_sequence(sequence).map(Self::Journal);
        }
        if let Some(hash) = input.strip_prefix("import:") {
            validate_restore_sha256(hash)?;
            return Ok(Self::Import(hash.to_owned()));
        }
        Err("서버에서 제공한 복원 지점을 선택해 주세요.".into())
    }
}

pub fn validate_restore_sha256(hash: &str) -> Result<(), String> {
    if hash.len() != 64
        || !hash
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    {
        return Err("복원 자료의 SHA-256 형식이 올바르지 않습니다.".into());
    }
    Ok(())
}

fn positive_sequence(value: &str) -> Result<i64, String> {
    let parsed = value
        .parse::<i64>()
        .ok()
        .filter(|v| *v > 0 && v.to_string() == value);
    parsed.ok_or_else(|| "변경 기록 순번은 정규 양의 정수여야 합니다.".into())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreTarget {
    pub target_id: String,
    pub source: RestoreTargetSource,
    pub source_kind: RestoreSourceKind,
    pub sha256: String,
    pub byte_length: String,
    pub format_version: u32,
    pub schema_version: Option<u32>,
    pub sequence: String,
    pub request_id: Option<String>,
    pub recorded_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreTargetPage {
    pub state: CatalogState,
    pub source: RestoreTargetSource,
    pub rows: Vec<RestoreTarget>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewRestore {
    pub target_id: String,
    pub expected_sha256: String,
}

impl PreviewRestore {
    pub fn validate(&self) -> Result<RestoreTargetId, String> {
        validate_restore_sha256(&self.expected_sha256)?;
        let target = RestoreTargetId::parse(&self.target_id)?;
        if let RestoreTargetId::Import(hash) = &target
            && hash != &self.expected_sha256
        {
            return Err("선택한 복원 자료와 SHA-256이 일치하지 않습니다.".into());
        }
        Ok(target)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmRestore {
    pub meta: BusinessMeta,
    pub target_id: String,
    pub expected_sha256: String,
}

impl ConfirmRestore {
    pub fn target(&self) -> Result<RestoreTargetId, String> {
        PreviewRestore {
            target_id: self.target_id.clone(),
            expected_sha256: self.expected_sha256.clone(),
        }
        .validate()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreArea {
    Materials,
    Recipes,
    Business,
    Orders,
    Receipts,
    Productions,
    Sales,
    Stocktakes,
    Allocations,
    Wastes,
    PurchaseReturns,
    Reconciliations,
    Corrections,
    Profiles,
    Photos,
}

pub const RESTORE_AREAS: [RestoreArea; 15] = [
    RestoreArea::Materials,
    RestoreArea::Recipes,
    RestoreArea::Business,
    RestoreArea::Orders,
    RestoreArea::Receipts,
    RestoreArea::Productions,
    RestoreArea::Sales,
    RestoreArea::Stocktakes,
    RestoreArea::Allocations,
    RestoreArea::Wastes,
    RestoreArea::PurchaseReturns,
    RestoreArea::Reconciliations,
    RestoreArea::Corrections,
    RestoreArea::Profiles,
    RestoreArea::Photos,
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreAreaCount {
    pub area: RestoreArea,
    pub records: usize,
    pub versions: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreSummary {
    pub areas: Vec<RestoreAreaCount>,
    pub drafts: usize,
    pub photo_bytes: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestorePhotoProblemKind {
    Missing,
    Corrupt,
    UnsafePath,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestorePhotoProblem {
    pub sha256: String,
    pub problem: RestorePhotoProblemKind,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestorePhotoCheck {
    pub required: usize,
    pub verified: usize,
    pub missing: usize,
    pub corrupt: usize,
    pub unsafe_path: usize,
    pub examples: Vec<RestorePhotoProblem>,
    pub examples_truncated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreBlockerCode {
    MissingAssets,
    InvalidAssets,
    RequestConflict,
    SequenceExhausted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreBlocker {
    pub code: RestoreBlockerCode,
    pub count: usize,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestorePreview {
    pub target: RestoreTarget,
    pub state: CatalogState,
    pub before: RestoreSummary,
    pub after: RestoreSummary,
    pub photos: RestorePhotoCheck,
    pub blockers: Vec<RestoreBlocker>,
    pub can_restore: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreResult {
    pub restore_id: String,
    pub target: RestoreTarget,
    pub previous_target: RestoreTarget,
    pub recorded_at: String,
    pub state: CatalogState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreRequestStatus {
    AlreadyProcessed,
    NotProcessed,
}
