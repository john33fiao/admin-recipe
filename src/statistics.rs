//! Read-only statistics preserve original timestamps and current effective evidence.
use crate::{
    catalog::CatalogState,
    correction::{CorrectionEventRef, CorrectionEventValue, CorrectionSubjectKind},
    domain::Money,
    sale::SaleOriginal,
    stocktake::{ConsumptionAllocation, StocktakeRowEffect},
};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

pub const MAX_REPORT_ROWS: usize = 10_000;

#[derive(Debug)]
pub enum StatisticsError {
    Validation(String),
    ReportTooLarge,
    Unavailable(String),
}
impl std::fmt::Display for StatisticsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Validation(message) | Self::Unavailable(message) => f.write_str(message),
            Self::ReportTooLarge => {
                f.write_str("조회 결과가 많습니다. 기간을 좁혀 다시 조회하세요")
            }
        }
    }
}
impl std::error::Error for StatisticsError {}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsQuery {
    pub from: String,
    pub until: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsRange {
    pub from: String,
    pub until: String,
    pub time_basis: String,
    pub correction_basis: String,
}

pub struct StatisticsWindow {
    from: DateTime<Utc>,
    until: DateTime<Utc>,
}
impl StatisticsWindow {
    pub fn parse(query: &StatisticsQuery) -> Result<Self, StatisticsError> {
        let parse = |text: &str| {
            if text.split_once('.').is_some_and(|(_, fraction)| {
                fraction.bytes().take_while(u8::is_ascii_digit).count() > 9
            }) {
                return Err(StatisticsError::Validation(
                    "기간의 소수 초는 최대 9자리로 입력하세요".into(),
                ));
            }
            DateTime::parse_from_rfc3339(text)
                .map(|value| value.with_timezone(&Utc))
                .map_err(|_| {
                    StatisticsError::Validation(
                        "기간은 시간대가 포함된 RFC3339 시각으로 입력하세요".into(),
                    )
                })
        };
        let from = parse(&query.from)?;
        let until = parse(&query.until)?;
        if from >= until {
            return Err(StatisticsError::Validation(
                "기간 끝은 시작보다 뒤여야 합니다".into(),
            ));
        }
        Ok(Self { from, until })
    }
    pub fn contains(&self, timestamp: &str) -> Result<bool, StatisticsError> {
        let time = DateTime::parse_from_rfc3339(timestamp)
            .map_err(|_| {
                StatisticsError::Unavailable("저장된 입력 시각이 올바르지 않습니다".into())
            })?
            .with_timezone(&Utc);
        Ok(self.from <= time && time < self.until)
    }
    pub fn range(&self) -> StatisticsRange {
        StatisticsRange {
            from: self.from.to_rfc3339_opts(SecondsFormat::AutoSi, true),
            until: self.until.to_rfc3339_opts(SecondsFormat::AutoSi, true),
            time_basis: "original_recorded_at".into(),
            correction_basis: "current".into(),
        }
    }
}

#[derive(Default)]
pub struct DecimalTotal(i128);
impl DecimalTotal {
    pub fn add(&mut self, value: &str) -> Result<(), StatisticsError> {
        let value = Money::parse(value).map_err(|e| StatisticsError::Unavailable(e.to_string()))?;
        self.0 = self
            .0
            .checked_add(i128::from(value.micros()))
            .ok_or_else(|| StatisticsError::Unavailable("집계 합계 범위를 넘습니다".into()))?;
        Ok(())
    }
    pub fn decimal(&self) -> String {
        let whole = self.0 / 1_000_000;
        let fraction = self.0 % 1_000_000;
        if fraction == 0 {
            whole.to_string()
        } else {
            format!("{whole}.{}", format!("{fraction:06}").trim_end_matches('0'))
        }
    }
}

/// Count every serialized array entry, including nested evidence and links.
pub fn enforce_report_limit(value: &impl Serialize) -> Result<(), StatisticsError> {
    fn count(value: &serde_json::Value, total: &mut usize) -> Result<(), StatisticsError> {
        match value {
            serde_json::Value::Array(rows) => {
                *total = total
                    .checked_add(rows.len())
                    .ok_or(StatisticsError::ReportTooLarge)?;
                if *total > MAX_REPORT_ROWS {
                    return Err(StatisticsError::ReportTooLarge);
                }
                for row in rows {
                    count(row, total)?;
                }
            }
            serde_json::Value::Object(fields) => {
                for child in fields.values() {
                    count(child, total)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    let value =
        serde_json::to_value(value).map_err(|e| StatisticsError::Unavailable(e.to_string()))?;
    count(&value, &mut 0)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsCoverage {
    pub matching_records: usize,
    pub all_time_records: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsOrigin {
    pub kind: CorrectionSubjectKind,
    pub id: String,
    pub confirmation_id: String,
    pub business_record_id: String,
    pub recorded_at: String,
    pub seq: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatisticsChangeKind {
    Correction,
    LegacySaleRevision,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsChangeRef {
    pub kind: StatisticsChangeKind,
    pub id: String,
    pub business_record_id: String,
    pub confirmation_id: String,
    pub recorded_at: String,
    pub seq: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatisticsStatus {
    Active,
    Void,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsSaleValue {
    pub count: String,
    pub amount: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatisticsConsumptionStatus {
    Unconfirmed,
    Allocated,
    Unallocated,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatisticsConsumptionReason {
    NoEffectiveAllocation,
    InitialBasisMissing,
    ZeroWeight,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsSaleConsumption {
    pub material_id: String,
    pub material_name: String,
    pub unit: String,
    pub status: StatisticsConsumptionStatus,
    pub observation_id: Option<String>,
    pub observation_version_id: Option<String>,
    pub stocktake_id: Option<String>,
    pub consumption_version_id: Option<String>,
    pub sale_version_id: Option<String>,
    pub quantity: Option<String>,
    pub value: Option<String>,
    pub reason: Option<StatisticsConsumptionReason>,
    pub observation_recorded_at: Option<String>,
    pub observation_in_period: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsSale {
    pub origin: StatisticsOrigin,
    pub effective_ref: CorrectionEventRef,
    pub changes: Vec<StatisticsChangeRef>,
    pub status: StatisticsStatus,
    pub original: SaleOriginal,
    pub value: Option<StatisticsSaleValue>,
    pub consumption: Vec<StatisticsSaleConsumption>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SalesTotals {
    pub active_records: usize,
    pub void_records: usize,
    pub count: String,
    pub known_revenue: String,
    pub missing_revenue_records: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsSales {
    pub coverage: StatisticsCoverage,
    pub summary: SalesTotals,
    pub rows: Vec<StatisticsSale>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatisticsObservationStatus {
    Initial,
    Settlement,
    Void,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsCandidateLink {
    pub position: usize,
    pub sale_id: String,
    pub business_record_id: String,
    pub original_sale_recorded_at: String,
    pub sale_in_period: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsObservation {
    pub origin: StatisticsOrigin,
    pub observation_id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub unit: String,
    pub position: usize,
    pub effective_observation_version_id: Option<String>,
    pub tombstone_projection_id: Option<String>,
    pub changes: Vec<StatisticsChangeRef>,
    pub status: StatisticsObservationStatus,
    pub effect: Option<StocktakeRowEffect>,
    pub consumption_version_id: Option<String>,
    pub allocation: Option<ConsumptionAllocation>,
    pub candidate_links: Vec<StatisticsCandidateLink>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialConsumptionTotals {
    pub material_id: String,
    pub material_name: String,
    pub label_material_version_id: String,
    pub unit: String,
    pub initial_records: usize,
    pub settlement_records: usize,
    pub void_records: usize,
    pub consumption_quantity: String,
    pub consumption_value: String,
    pub allocated_quantity: String,
    pub allocated_value: String,
    pub unallocated_quantity: String,
    pub unallocated_value: String,
    pub no_sales_records: usize,
    pub zero_weight_records: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsObservations {
    pub coverage: StatisticsCoverage,
    pub summary_by_material: Vec<MaterialConsumptionTotals>,
    pub rows: Vec<StatisticsObservation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsReceiptBasis {
    pub material_id: String,
    pub material_version_id: String,
    pub material_name: String,
    pub quantity: String,
    pub unit: String,
    pub amount: String,
    pub receipt_date: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsReceiptDisplay {
    pub original: StatisticsReceiptBasis,
    pub effective: Option<StatisticsReceiptBasis>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsOperation {
    pub origin: StatisticsOrigin,
    pub effective_ref: CorrectionEventRef,
    pub changes: Vec<StatisticsChangeRef>,
    pub status: StatisticsStatus,
    pub value: Option<CorrectionEventValue>,
    pub receipt_basis: Option<StatisticsReceiptDisplay>,
    pub observation_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsOperations {
    pub coverage: StatisticsCoverage,
    pub rows: Vec<StatisticsOperation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatisticsReport {
    pub range: StatisticsRange,
    pub state: CatalogState,
    pub sales: StatisticsSales,
    pub observations: StatisticsObservations,
    pub operations: StatisticsOperations,
}
