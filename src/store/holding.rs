use super::{Store, StoreError, catalog, production, stocktake, unavailable};
use crate::{
    domain::{Money, Quantity, ResolvedPrice, UnitPrice},
    holding::*,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LegacyHoldingVersion {
    pub id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub previous_version_id: Option<String>,
    pub receipt_id: String,
    pub baseline_receipt_id: String,
    pub baseline_seq: String,
    pub quantity: String,
    pub value: String,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HoldingVersion {
    pub correction_projection_id: Option<String>,
    pub baseline_correction_projection_id: Option<String>,
    pub last_valid_correction_price_id: Option<String>,
    pub correction_last_valid_price: Option<PriceBasis>,
    pub reconciliation_effect_id: Option<String>,
    pub baseline_reconciliation_effect_id: Option<String>,
    pub id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub previous_version_id: Option<String>,
    pub receipt_id: Option<String>,
    pub baseline_receipt_id: Option<String>,
    pub production_effect_id: Option<String>,
    pub baseline_production_effect_id: Option<String>,
    pub observation_version_id: Option<String>,
    pub baseline_observation_version_id: Option<String>,
    pub waste_id: Option<String>,
    pub purchase_return_id: Option<String>,
    pub baseline_seq: String,
    pub quantity: String,
    pub value: String,
    pub recorded_at: String,
    pub seq: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HoldingHead {
    pub material_id: String,
    pub current_version_id: Option<String>,
    pub last_valid_version_id: Option<String>,
    pub unknown_correction_projection_id: Option<String>,
    pub last_valid_correction_price_id: Option<String>,
}

pub(super) fn versions(
    connection: &Connection,
    material: Option<&str>,
) -> Result<Vec<HoldingVersion>, StoreError> {
    read_versions(connection, material, None)
}

fn read_versions(
    connection: &Connection,
    material: Option<&str>,
    id: Option<&str>,
) -> Result<Vec<HoldingVersion>, StoreError> {
    let rows=connection.prepare("SELECT id,material_id,material_version_id,previous_version_id,receipt_id,baseline_receipt_id,baseline_sequence,quantity_micros,value_micros,recorded_at,sequence,production_effect_id,baseline_production_effect_id,observation_version_id,baseline_observation_version_id,waste_id,reconciliation_effect_id,baseline_reconciliation_effect_id,correction_projection_id,baseline_correction_projection_id,last_valid_correction_price_id,purchase_return_id FROM holding_versions WHERE (?1 IS NULL OR material_id=?1) AND (?2 IS NULL OR id=?2) ORDER BY sequence,id").map_err(unavailable)?
        .query_map(params![material,id], |r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,Option<String>>(3)?,r.get::<_,Option<String>>(4)?,r.get::<_,Option<String>>(5)?,r.get::<_,i64>(6)?,r.get::<_,i64>(7)?,r.get::<_,i64>(8)?,r.get::<_,String>(9)?,r.get::<_,i64>(10)?,r.get::<_,Option<String>>(11)?,r.get::<_,Option<String>>(12)?,r.get::<_,Option<String>>(13)?,r.get::<_,Option<String>>(14)?,r.get::<_,Option<String>>(15)?,r.get::<_,Option<String>>(16)?,r.get::<_,Option<String>>(17)?,r.get::<_,Option<String>>(18)?,r.get::<_,Option<String>>(19)?,r.get::<_,Option<String>>(20)?,r.get::<_,Option<String>>(21)?)))
        .map_err(unavailable)?.collect::<rusqlite::Result<Vec<_>>>().map_err(unavailable)?;
    rows.into_iter()
        .map(
            |(
                id,
                material_id,
                material_version_id,
                previous_version_id,
                receipt_id,
                baseline_receipt_id,
                baseline_seq,
                q,
                v,
                recorded_at,
                seq,
                production_effect_id,
                baseline_production_effect_id,
                observation_version_id,
                baseline_observation_version_id,
                waste_id,
                reconciliation_effect_id,
                baseline_reconciliation_effect_id,
                correction_projection_id,
                baseline_correction_projection_id,
                last_valid_correction_price_id,
                purchase_return_id,
            )| {
                let correction_last_valid_price = last_valid_correction_price_id
                    .as_deref()
                    .map(|price| {
                        super::correction::price_basis(connection, &material_id, price)
                            .map(|p| PriceBasis::from(&p))
                    })
                    .transpose()?;
                Ok(HoldingVersion {
                    correction_projection_id,
                    baseline_correction_projection_id,
                    last_valid_correction_price_id,
                    correction_last_valid_price,
                    id,
                    material_id,
                    material_version_id,
                    previous_version_id,
                    receipt_id,
                    baseline_receipt_id,
                    production_effect_id,
                    baseline_production_effect_id,
                    observation_version_id,
                    baseline_observation_version_id,
                    waste_id,
                    purchase_return_id,
                    reconciliation_effect_id,
                    baseline_reconciliation_effect_id,
                    baseline_seq: baseline_seq.to_string(),
                    quantity: Quantity::from_micros(q).map_err(unavailable)?.decimal(),
                    value: Money::from_micros(v).map_err(unavailable)?.decimal(),
                    recorded_at,
                    seq: seq.to_string(),
                })
            },
        )
        .collect()
}
pub(super) fn heads(connection: &Connection) -> Result<Vec<HoldingHead>, StoreError> {
    connection.prepare("SELECT material_id,current_version_id,last_valid_version_id,unknown_correction_projection_id,last_valid_correction_price_id FROM holding_heads ORDER BY material_id").map_err(unavailable)?
        .query_map([],|r|Ok(HoldingHead{material_id:r.get(0)?,current_version_id:r.get(1)?,last_valid_version_id:r.get(2)?,unknown_correction_projection_id:r.get(3)?,last_valid_correction_price_id:r.get(4)?})).map_err(unavailable)?
        .collect::<rusqlite::Result<_>>().map_err(unavailable)
}

pub(super) fn current_input(
    connection: &Connection,
    material_id: &str,
) -> Result<HoldingInput, StoreError> {
    let row=connection.query_row("SELECT current_version_id,last_valid_version_id,last_valid_correction_price_id,unknown_correction_projection_id FROM holding_heads WHERE material_id=?1",[material_id],|r|Ok((r.get::<_,Option<String>>(0)?,r.get::<_,Option<String>>(1)?,r.get::<_,Option<String>>(2)?,r.get::<_,Option<String>>(3)?))).optional().map_err(unavailable)?;
    let Some((id, last, correction_price, unknown)) = row else {
        return Ok(HoldingInput::default());
    };
    let Some(id) = id else {
        if unknown.is_none() || last.is_some() || correction_price.is_some() {
            return Err(unavailable("미확인 보유의 정정 근거가 올바르지 않습니다"));
        }
        return Ok(HoldingInput::default());
    };
    let version = read_versions(connection, Some(material_id), Some(&id))?
        .pop()
        .ok_or_else(|| unavailable("현재 보유 버전이 없습니다"))?;
    let last = last
        .as_deref()
        .map(|id| {
            read_versions(connection, Some(material_id), Some(id))?
                .pop()
                .ok_or_else(|| unavailable("마지막 유효 보유 버전이 없습니다"))
        })
        .transpose()?;
    let input = HoldingInput {
        basis: Some(basis(&version)?),
        observation: version
            .baseline_observation_version_id
            .as_ref()
            .map(|id| {
                stocktake::versions(connection)?
                    .iter()
                    .find(|v| &v.id == id)
                    .map(stocktake::observation)
                    .ok_or_else(|| unavailable("보유 관측 기준이 없습니다"))
            })
            .transpose()?,
        quantity: Some(Quantity::parse(&version.quantity).map_err(unavailable)?),
        value: Some(Money::parse(&version.value).map_err(unavailable)?),
        source_id: Some(id),
        last_valid: if let Some(price) = correction_price {
            Some(super::correction::price_basis(
                connection,
                material_id,
                &price,
            )?)
        } else {
            last.map(|last| {
                UnitPrice::new(
                    Money::parse(&last.value).map_err(unavailable)?,
                    Quantity::parse(&last.quantity).map_err(unavailable)?,
                    last.id,
                )
                .map_err(unavailable)
            })
            .transpose()?
        },
    };
    validate_holding_input(&input).map_err(unavailable)?;
    Ok(input)
}

pub(super) fn basis(version: &HoldingVersion) -> Result<HoldingBasisRef, StoreError> {
    let (kind, id) = match (
        &version.baseline_receipt_id,
        &version.baseline_production_effect_id,
        &version.baseline_observation_version_id,
        &version.baseline_reconciliation_effect_id,
        &version.baseline_correction_projection_id,
    ) {
        (Some(id), None, None, None, None) => (HoldingBasisKind::Receipt, id),
        (None, Some(id), None, None, None) => (HoldingBasisKind::Production, id),
        (None, None, Some(id), None, None) => (HoldingBasisKind::Observation, id),
        (None, None, None, Some(id), None) => (HoldingBasisKind::Reconciliation, id),
        (None, None, None, None, Some(id)) => (HoldingBasisKind::Correction, id),
        _ => return Err(unavailable("보유 시작 기준의 원천이 올바르지 않습니다")),
    };
    Ok(HoldingBasisRef {
        kind,
        id: Some(id.clone()),
        seq: Some(version.baseline_seq.clone()),
    })
}

pub(super) fn amounts(version: &HoldingVersion) -> HoldingAmounts {
    HoldingAmounts {
        quantity: version.quantity.clone(),
        value: Some(version.value.clone()),
        source_id: Some(version.id.clone()),
    }
}

pub(super) fn input_from_history(
    versions: &[HoldingVersion],
    observations: &[stocktake::VersionRow],
    version: Option<&HoldingVersion>,
) -> Result<HoldingInput, StoreError> {
    let Some(version) = version else {
        return Ok(HoldingInput::default());
    };
    let seq = version.seq.parse::<i64>().map_err(unavailable)?;
    let last = versions
        .iter()
        .filter(|v| {
            v.material_id == version.material_id
                && v.seq.parse::<i64>().is_ok_and(|s| s <= seq)
                && Quantity::parse(&v.quantity).is_ok_and(|q| q.micros() > 0)
        })
        .max_by_key(|v| v.seq.parse::<i64>().unwrap_or(0));
    let input = HoldingInput {
        basis: Some(basis(version)?),
        observation: version
            .baseline_observation_version_id
            .as_ref()
            .map(|id| {
                observations
                    .iter()
                    .find(|v| &v.id == id)
                    .map(stocktake::observation)
                    .ok_or_else(|| unavailable("보유 관측 기준이 없습니다"))
            })
            .transpose()?,
        quantity: Some(Quantity::parse(&version.quantity).map_err(unavailable)?),
        value: Some(Money::parse(&version.value).map_err(unavailable)?),
        source_id: Some(version.id.clone()),
        last_valid: if Quantity::parse(&version.quantity)
            .map_err(unavailable)?
            .micros()
            > 0
        {
            Some(
                UnitPrice::new(
                    Money::parse(&version.value).map_err(unavailable)?,
                    Quantity::parse(&version.quantity).map_err(unavailable)?,
                    version.id.clone(),
                )
                .map_err(unavailable)?,
            )
        } else if let Some(price) = &version.correction_last_valid_price {
            if version.last_valid_correction_price_id.as_ref() != Some(&price.source_id) {
                return Err(unavailable("정정 마지막 단가의 근거가 다릅니다"));
            }
            Some(
                UnitPrice::new(
                    Money::parse(&price.amount).map_err(unavailable)?,
                    Quantity::parse(&price.quantity).map_err(unavailable)?,
                    price.source_id.clone(),
                )
                .map_err(unavailable)?,
            )
        } else {
            last.map(|v| {
                UnitPrice::new(
                    Money::parse(&v.value)?,
                    Quantity::parse(&v.quantity)?,
                    v.id.clone(),
                )
            })
            .transpose()
            .map_err(unavailable)?
        },
    };
    validate_holding_input(&input).map_err(unavailable)?;
    Ok(input)
}

/// Effective as-of state. A tombstone is an unknown baseline, never an old
/// positive holding resurrected by a maximum-sequence lookup.
pub(super) fn input_at(
    versions: &[HoldingVersion],
    observations: &[stocktake::VersionRow],
    projections: &[super::correction::ProjectionVersionRow],
    material: &str,
    at: i64,
) -> Result<HoldingInput, StoreError> {
    let latest = versions
        .iter()
        .filter(|h| h.material_id == material && h.seq.parse::<i64>().is_ok_and(|n| n <= at))
        .max_by_key(|h| h.seq.parse::<i64>().unwrap_or(-1));
    let correction=projections.iter().filter(|p|matches!(&p.projection,super::correction::Projection::Material{change,..} if change.material_id==material)&&p.seq.parse::<i64>().is_ok_and(|n|n<=at)).max_by_key(|p|p.seq.parse::<i64>().unwrap_or(-1));
    if let Some(p) = correction {
        if latest
            .is_none_or(|h| h.seq.parse::<i64>().unwrap_or(-1) < p.seq.parse::<i64>().unwrap_or(-1))
        {
            return Ok(HoldingInput::default());
        }
        let history = versions
            .iter()
            .filter(|h| h.seq.parse::<i64>().unwrap_or(-1) >= p.seq.parse::<i64>().unwrap_or(-1))
            .cloned()
            .collect::<Vec<_>>();
        return input_from_history(&history, observations, latest);
    }
    input_from_history(versions, observations, latest)
}

pub(super) fn effect_holdings(
    connection: &Connection,
    effect_id: &str,
) -> Result<(Option<HoldingVersion>, HoldingVersion), StoreError> {
    let id: String = connection
        .query_row(
            "SELECT id FROM holding_versions WHERE production_effect_id=?1",
            [effect_id],
            |r| r.get(0),
        )
        .map_err(unavailable)?;
    let after = read_versions(connection, None, Some(&id))?
        .pop()
        .ok_or_else(|| unavailable("생산 결과 보유 버전이 없습니다"))?;
    let before = after
        .previous_version_id
        .as_deref()
        .map(|id| read_versions(connection, None, Some(id)))
        .transpose()?
        .and_then(|mut rows| rows.pop());
    if before.is_none() && after.previous_version_id.is_some() {
        return Err(unavailable("생산 전 보유 버전이 없습니다"));
    }
    Ok((before, after))
}

pub(super) fn price(
    connection: &Connection,
    material_id: &str,
    registered: Option<UnitPrice>,
) -> Result<ResolvedPrice, StoreError> {
    resolve_holding_price(&current_input(connection, material_id)?, registered).map_err(unavailable)
}

pub(super) struct ReceiptHoldingInput<'a> {
    pub receipt_id: &'a str,
    pub material_id: &'a str,
    pub material_version_id: &'a str,
    pub quantity: Quantity,
    pub amount: Money,
    pub seq: i64,
    pub time: &'a str,
}
pub(super) fn apply_receipt(
    connection: &Connection,
    input: ReceiptHoldingInput<'_>,
) -> Result<String, StoreError> {
    let ReceiptHoldingInput {
        receipt_id,
        material_id,
        material_version_id,
        quantity,
        amount,
        seq,
        time,
    } = input;
    let before = current_input(connection, material_id)?;
    let projected = project_receipt(&before, quantity, amount)
        .map_err(|e| StoreError::Validation(e.to_string()))?;
    append(
        connection,
        AppendHolding {
            material_id,
            material_version_id,
            receipt_id: Some(receipt_id),
            production_effect_id: None,
            waste_id: None,
            purchase_return_id: None,
            observation_version_id: None,
            reconciliation_effect_id: None,
            before: &before,
            after: &projected.after,
            seq,
            time,
        },
    )
}

pub(super) struct AppendHolding<'a> {
    pub material_id: &'a str,
    pub material_version_id: &'a str,
    pub receipt_id: Option<&'a str>,
    pub production_effect_id: Option<&'a str>,
    pub waste_id: Option<&'a str>,
    pub purchase_return_id: Option<&'a str>,
    pub observation_version_id: Option<&'a str>,
    pub reconciliation_effect_id: Option<&'a str>,
    pub before: &'a HoldingInput,
    pub after: &'a HoldingAmounts,
    pub seq: i64,
    pub time: &'a str,
}

pub(super) fn append(
    connection: &Connection,
    input: AppendHolding<'_>,
) -> Result<String, StoreError> {
    let AppendHolding {
        material_id,
        material_version_id,
        receipt_id,
        production_effect_id,
        waste_id,
        purchase_return_id,
        observation_version_id,
        reconciliation_effect_id,
        before,
        after,
        seq,
        time,
    } = input;
    let id = Uuid::new_v4().to_string();
    let (
        baseline_receipt,
        baseline_production,
        baseline_observation,
        baseline_reconciliation,
        baseline_correction,
        boundary,
    ) = if let Some(observation) = observation_version_id {
        (None, None, Some(observation), None, None, seq)
    } else {
        match before.basis.as_ref() {
            Some(b) => {
                let source =
                    b.id.as_deref()
                        .ok_or_else(|| unavailable("보유 기준 ID가 없습니다"))?;
                let boundary = b
                    .seq
                    .as_deref()
                    .ok_or_else(|| unavailable("보유 기준 순번이 없습니다"))?
                    .parse::<i64>()
                    .map_err(unavailable)?;
                match b.kind {
                    HoldingBasisKind::Receipt => (Some(source), None, None, None, None, boundary),
                    HoldingBasisKind::Production => {
                        (None, Some(source), None, None, None, boundary)
                    }
                    HoldingBasisKind::Observation => {
                        (None, None, Some(source), None, None, boundary)
                    }
                    HoldingBasisKind::Reconciliation => {
                        (None, None, None, Some(source), None, boundary)
                    }
                    HoldingBasisKind::Correction => {
                        (None, None, None, None, Some(source), boundary)
                    }
                }
            }
            None => (
                receipt_id,
                production_effect_id,
                None,
                reconciliation_effect_id,
                None,
                seq,
            ),
        }
    };
    let q = Quantity::parse(&after.quantity).map_err(unavailable)?;
    let v = Money::parse(
        after
            .value
            .as_deref()
            .ok_or_else(|| unavailable("보유 가치가 없습니다"))?,
    )
    .map_err(unavailable)?;
    let last = if q.micros() > 0 {
        Some(id.as_str())
    } else if before.quantity.as_ref().is_some_and(|q| q.micros() > 0) {
        Some(
            before
                .source_id
                .as_deref()
                .ok_or_else(|| unavailable("현재 양수 보유의 근거가 없습니다"))?,
        )
    } else {
        before.last_valid.as_ref().map(UnitPrice::source_id)
    };
    let last_is_correction=last.map(|last|connection.query_row("SELECT EXISTS(SELECT 1 FROM correction_price_bases WHERE id=?1 AND material_id=?2)",params![last,material_id],|r|r.get::<_,bool>(0)).map_err(unavailable)).transpose()?.unwrap_or(false);
    let last_price = if last_is_correction { last } else { None };
    let last_holding = if last_is_correction { None } else { last };
    connection.execute("INSERT INTO holding_versions(id,material_id,material_version_id,previous_version_id,receipt_id,baseline_receipt_id,baseline_sequence,quantity_micros,value_micros,recorded_at,sequence,production_effect_id,baseline_production_effect_id,observation_version_id,baseline_observation_version_id,waste_id,reconciliation_effect_id,baseline_reconciliation_effect_id,baseline_correction_projection_id,last_valid_correction_price_id,purchase_return_id) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21)",params![id,material_id,material_version_id,before.source_id,receipt_id,baseline_receipt,boundary,q.micros(),v.micros(),time,seq,production_effect_id,baseline_production,observation_version_id,baseline_observation,waste_id,reconciliation_effect_id,baseline_reconciliation,baseline_correction,last_price,purchase_return_id]).map_err(unavailable)?;
    connection.execute("INSERT INTO holding_heads(material_id,current_version_id,last_valid_version_id,last_valid_correction_price_id) VALUES(?1,?2,?3,?4) ON CONFLICT(material_id) DO UPDATE SET current_version_id=excluded.current_version_id,last_valid_version_id=excluded.last_valid_version_id,last_valid_correction_price_id=excluded.last_valid_correction_price_id,unknown_correction_projection_id=NULL",params![material_id,id,last_holding,last_price]).map_err(unavailable)?;
    Ok(id)
}

impl Store {
    pub fn holding_detail(&self, material_id: &str) -> Result<Option<HoldingDetail>, StoreError> {
        let id = catalog::uuid(material_id, "재료 ID")?;
        let Some(material) = catalog::current_version(&self.connection, &id)? else {
            return Ok(None);
        };
        let current = current_input(&self.connection, &id)?;
        let mut view = project_holding(&HoldingInput::default(), &[]).map_err(unavailable)?;
        let history = versions(&self.connection, Some(&id))?;
        let observations = stocktake::versions(&self.connection)?;
        let reconciliations = super::reconciliation::records(&self.connection)?;
        let corrections = super::correction::snapshot_rows(&self.connection)?;
        let latest_correction = corrections
            .projection_versions
            .iter()
            .filter(|p| p.projection.kind() == "material" && p.projection.subject_id() == id)
            .max_by_key(|p| p.seq.parse::<i64>().unwrap_or(-1));
        let correction_boundary = latest_correction
            .and_then(|p| p.seq.parse::<i64>().ok())
            .unwrap_or(0);
        let correction = latest_correction.map(|p| HoldingCorrection {
            id: p.correction_id.clone(),
            recorded_at: p.recorded_at.clone(),
            seq: p.seq.clone(),
        });
        let corrected_holding = latest_correction
            .and_then(|p| match &p.projection {
                super::correction::Projection::Material {
                    holding_version_id, ..
                } => holding_version_id.as_deref(),
                _ => None,
            })
            .and_then(|id| history.iter().find(|h| h.id == id));
        if let Some(first) = history
            .iter()
            .rev()
            .find(|h| {
                h.observation_version_id.is_some()
                    && h.seq.parse::<i64>().is_ok_and(|n| n > correction_boundary)
            })
            .or(corrected_holding)
            .or_else(|| {
                history
                    .iter()
                    .find(|h| h.seq.parse::<i64>().is_ok_and(|n| n > correction_boundary))
            })
        {
            let mut baseline = input_from_history(&history, &observations, Some(first))?;
            if let Some(id) = &first.correction_projection_id {
                baseline.basis = Some(HoldingBasisRef {
                    kind: HoldingBasisKind::Correction,
                    id: Some(id.clone()),
                    seq: Some(first.seq.clone()),
                });
                baseline.observation = None;
            }
            let mut changes = super::receipt::records(&self.connection, Some(&id), None)?
                .into_iter()
                .filter(|r| !reconciliations.iter().any(|s| s.receipt_id == r.id))
                .filter(|r| r.seq.parse::<i64>().ok() > first.seq.parse::<i64>().ok())
                .map(|r| HoldingChange::Receipt {
                    event_id: r.id,
                    quantity: r.quantity,
                    value: r.amount,
                    recorded_at: r.recorded_at,
                    seq: r.seq,
                })
                .collect::<Vec<_>>();
            changes.extend(
                super::reconciliation::effects(&self.connection)?
                    .into_iter()
                    .filter(|e| {
                        e.effect.material_id == id
                            && e.seq.parse::<i64>().ok() > first.seq.parse::<i64>().ok()
                    })
                    .map(|e| {
                        let receipt_id = reconciliations
                            .iter()
                            .find(|r| r.id == e.reconciliation_id)
                            .map(|r| r.receipt_id.clone())
                            .ok_or_else(|| unavailable("보정 입고 근거가 없습니다"))?;
                        Ok(HoldingChange::Reconciliation {
                            event_id: e.id,
                            reconciliation_id: e.reconciliation_id,
                            receipt_id,
                            inflow_quantity: e.effect.inflow_quantity,
                            inflow_value: e.effect.inflow_value,
                            value_delta: e.effect.value_delta,
                            recorded_at: e.recorded_at,
                            seq: e.seq,
                        })
                    })
                    .collect::<Result<Vec<_>, StoreError>>()?,
            );
            changes.extend(
                production::effects(&self.connection, None, Some(&id))?
                    .into_iter()
                    .filter(|e| e.seq.parse::<i64>().ok() > first.seq.parse::<i64>().ok())
                    .map(|e| HoldingChange::Production {
                        event_id: e.id,
                        production_id: e.production_id,
                        input_quantity: e.input_quantity,
                        input_value: e.input_value,
                        output_quantity: e.output_quantity,
                        output_value: e.output_value,
                        recorded_at: e.recorded_at,
                        seq: e.seq,
                    }),
            );
            changes.extend(
                super::waste::records(&self.connection, None, None)?
                    .into_iter()
                    .filter(|w| {
                        w.effect.material_id == id
                            && w.seq.parse::<i64>().ok() > first.seq.parse::<i64>().ok()
                    })
                    .map(|w| HoldingChange::Waste {
                        event_id: w.id,
                        quantity: w.effect.quantity,
                        loss_value: w.effect.loss_value,
                        recorded_at: w.recorded_at,
                        seq: w.seq,
                    }),
            );
            changes.sort_by_key(|c| c.seq().parse::<i64>().unwrap_or(-1));
            changes.extend(
                super::purchase_return::records(&self.connection, None, None)?
                    .into_iter()
                    .filter(|r| {
                        r.effect.material_id == id
                            && r.seq.parse::<i64>().ok() > first.seq.parse::<i64>().ok()
                    })
                    .map(|r| HoldingChange::PurchaseReturn {
                        event_id: r.id,
                        quantity: r.effect.quantity,
                        deducted_value: r.effect.deducted_value,
                        recorded_at: r.recorded_at,
                        seq: r.seq,
                    }),
            );
            changes.sort_by_key(|c| c.seq().parse::<i64>().unwrap_or(-1));
            view = project_holding(&baseline, &changes).map_err(unavailable)?;
            if first.correction_projection_id.is_some() {
                view.basis = current.basis.clone();
                view.observation = current.observation.clone();
            }
            if let Some(amounts) = &mut view.settlement_basis {
                amounts.source_id = current.source_id.clone();
            }
            if view.settlement_basis.as_ref().map(|a| a.quantity.as_str())
                != current.quantity.as_ref().map(|q| q.decimal()).as_deref()
                || view
                    .settlement_basis
                    .as_ref()
                    .and_then(|a| a.value.as_deref())
                    != current.value.as_ref().map(|v| v.decimal()).as_deref()
            {
                return Err(unavailable("확정 변동 이력과 현재 보유 기준이 다릅니다"));
            }
        }
        Ok(Some(HoldingDetail {
            material_id: id.clone(),
            material_version_id: material.id,
            unit: material.canonical_unit,
            basis: view.basis,
            observation: view.observation,
            settlement_basis: view.settlement_basis,
            last_valid_price: current.last_valid.as_ref().map(PriceBasis::from),
            confirmed_changes: view.confirmed_changes,
            price: price(
                &self.connection,
                &id,
                catalog::registered_price(&self.connection, &id)?,
            )?,
            state: catalog::state(&self.connection)?,
            correction,
        }))
    }
}

pub(super) fn validate_holding_schema(connection: &Connection) -> anyhow::Result<()> {
    let versions = versions(connection, None)?;
    let heads = heads(connection)?;
    let receipts = super::receipt::records(connection, None, None)?;
    validate_snapshot_reconciled(
        &receipts,
        &production::effects(connection, None, None)?,
        &stocktake::versions(connection)?,
        &super::waste::records(connection, None, None)?,
        &super::reconciliation::records(connection)?,
        &super::reconciliation::effects(connection)?,
        &versions,
        &heads,
        &super::correction::snapshot_rows(connection)?.projection_versions,
        &super::purchase_return::snapshot_rows(connection)?,
    )?;
    for material in versions
        .iter()
        .map(|v| &v.material_id)
        .collect::<std::collections::HashSet<_>>()
    {
        current_input(connection, material)?;
    }
    Ok(())
}

pub(super) fn migrate_observations(connection: &Connection) -> anyhow::Result<()> {
    connection.execute_batch("CREATE TABLE holding_versions_next(
        id TEXT PRIMARY KEY NOT NULL,material_id TEXT NOT NULL,material_version_id TEXT NOT NULL,previous_version_id TEXT,
        receipt_id TEXT UNIQUE REFERENCES receipts(id),baseline_receipt_id TEXT REFERENCES receipts(id),
        baseline_sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),
        quantity_micros INTEGER NOT NULL CHECK(quantity_micros>=0 AND quantity_micros<=9000000000000000),
        value_micros INTEGER NOT NULL CHECK(value_micros>=0 AND value_micros<=9000000000000000000),
        recorded_at TEXT NOT NULL,sequence INTEGER NOT NULL REFERENCES snapshot_outbox(sequence),
        production_effect_id TEXT UNIQUE,baseline_production_effect_id TEXT,
        observation_version_id TEXT UNIQUE REFERENCES stocktake_versions(id),baseline_observation_version_id TEXT REFERENCES stocktake_versions(id),waste_id TEXT UNIQUE REFERENCES wastes(id),
        UNIQUE(material_id,id),
        FOREIGN KEY(material_id,material_version_id) REFERENCES material_versions(material_id,id),
        FOREIGN KEY(material_id,previous_version_id) REFERENCES holding_versions_next(material_id,id),
        FOREIGN KEY(material_id,production_effect_id) REFERENCES production_effects(material_id,id),
        FOREIGN KEY(material_id,baseline_production_effect_id) REFERENCES production_effects(material_id,id),
        CHECK((receipt_id IS NOT NULL)+(production_effect_id IS NOT NULL)+(observation_version_id IS NOT NULL)+(waste_id IS NOT NULL)=1),
        CHECK((baseline_receipt_id IS NOT NULL)+(baseline_production_effect_id IS NOT NULL)+(baseline_observation_version_id IS NOT NULL)=1),
        CHECK(quantity_micros>0 OR value_micros=0));
        INSERT INTO holding_versions_next SELECT *,NULL,NULL,NULL FROM holding_versions;
        CREATE TABLE holding_heads_next(material_id TEXT PRIMARY KEY NOT NULL REFERENCES materials(id),current_version_id TEXT NOT NULL,last_valid_version_id TEXT,
        FOREIGN KEY(material_id,current_version_id) REFERENCES holding_versions_next(material_id,id),
        FOREIGN KEY(material_id,last_valid_version_id) REFERENCES holding_versions_next(material_id,id));
        INSERT INTO holding_heads_next SELECT * FROM holding_heads;
        CREATE TABLE sale_cost_rows_next(
        sale_id TEXT NOT NULL REFERENCES sales(id),position INTEGER NOT NULL CHECK(position>=0 AND position<100),
        recipe_line_id TEXT NOT NULL REFERENCES recipe_lines(id),source_kind TEXT NOT NULL CHECK(source_kind IN('holding_average','last_valid','preregistered','missing')),
        holding_version_id TEXT REFERENCES holding_versions_next(id),material_price_id TEXT REFERENCES material_prices(id),
        price_amount_micros INTEGER CHECK(price_amount_micros>=0 AND price_amount_micros<=9000000000000000000),
        price_quantity_micros INTEGER CHECK(price_quantity_micros>0 AND price_quantity_micros<=9000000000000000),
        missing_reason TEXT,cost_micros INTEGER CHECK(cost_micros>=0 AND cost_micros<=9000000000000000000),
        PRIMARY KEY(sale_id,position),UNIQUE(sale_id,recipe_line_id),
        CHECK((source_kind='missing' AND holding_version_id IS NULL AND material_price_id IS NULL AND price_amount_micros IS NULL AND price_quantity_micros IS NULL AND missing_reason IS NOT NULL)
        OR (source_kind='preregistered' AND holding_version_id IS NULL AND material_price_id IS NOT NULL AND price_amount_micros IS NOT NULL AND price_quantity_micros IS NOT NULL AND missing_reason IS NULL)
        OR (source_kind IN('holding_average','last_valid') AND holding_version_id IS NOT NULL AND material_price_id IS NULL AND price_amount_micros IS NOT NULL AND price_quantity_micros IS NOT NULL AND missing_reason IS NULL)));
        INSERT INTO sale_cost_rows_next SELECT * FROM sale_cost_rows;
        DROP TABLE sale_cost_rows;
        DROP TABLE holding_heads;
        DROP TABLE holding_versions;
        ALTER TABLE holding_versions_next RENAME TO holding_versions;
        ALTER TABLE holding_heads_next RENAME TO holding_heads;
        ALTER TABLE sale_cost_rows_next RENAME TO sale_cost_rows;
        CREATE INDEX holding_versions_material ON holding_versions(material_id,sequence);
        PRAGMA user_version=10;")?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn validate_snapshot_reconciled(
    receipts: &[crate::receipt::ReceiptRecord],
    effects: &[production::EffectRow],
    observations: &[stocktake::VersionRow],
    wastes: &[crate::waste::WasteRecord],
    reconciliations: &[crate::reconciliation::ReceiptReconciliation],
    reconciliation_effects: &[super::reconciliation::MaterialEffectRow],
    versions: &[HoldingVersion],
    heads: &[HoldingHead],
    corrections: &[super::correction::ProjectionVersionRow],
    purchase_returns: &[crate::purchase_return::PurchaseReturnRecord],
) -> Result<(), StoreError> {
    if !corrections.is_empty() || !purchase_returns.is_empty() {
        return validate_corrected_holdings(
            receipts,
            effects,
            observations,
            wastes,
            reconciliations,
            reconciliation_effects,
            versions,
            heads,
            corrections,
            purchase_returns,
        );
    }
    use std::collections::{BTreeMap, HashSet};
    let bad = || unavailable("보유 버전의 원천·이전·현재·마지막 기준이 다릅니다");
    let ordinary_receipts = receipts
        .iter()
        .filter(|r| !reconciliations.iter().any(|s| s.receipt_id == r.id))
        .count();
    if versions.len()
        != ordinary_receipts
            + effects.len()
            + observations.len()
            + wastes.len()
            + purchase_returns.len()
            + reconciliation_effects.len()
    {
        return Err(bad());
    }
    let mut grouped = BTreeMap::<&str, Vec<&HoldingVersion>>::new();
    let mut ids = HashSet::new();
    let mut used_receipts = HashSet::new();
    let mut used_effects = HashSet::new();
    let mut used_observations = HashSet::new();
    let mut used_wastes = HashSet::new();
    let mut used_reconciliations = HashSet::new();
    for version in versions {
        if Uuid::parse_str(&version.id)
            .map_err(unavailable)?
            .to_string()
            != version.id
        {
            return Err(bad());
        }
        if !ids.insert(&version.id) {
            return Err(bad());
        }
        grouped
            .entry(&version.material_id)
            .or_default()
            .push(version);
    }
    if grouped.len() != heads.len() {
        return Err(bad());
    }
    for (material, mut history) in grouped {
        history.sort_by_key(|v| v.seq.parse::<i64>().unwrap_or(-1));
        let mut baseline = history[0];
        let mut previous = None;
        let mut last_valid = None;
        let mut prior_seq = 0;
        let (mut q, mut v) = (0i64, 0i64);
        for h in history {
            let seq = h.seq.parse::<i64>().map_err(unavailable)?;
            let (source_material, source_version, source_seq, source_time, iq, iv, oq, ov) = match (
                &h.receipt_id,
                &h.production_effect_id,
                &h.observation_version_id,
                &h.waste_id,
                &h.reconciliation_effect_id,
            ) {
                (Some(id), None, None, None, None) => {
                    if reconciliations.iter().any(|s| &s.receipt_id == id) {
                        return Err(bad());
                    }
                    if !used_receipts.insert(id) {
                        return Err(bad());
                    }
                    let r = receipts.iter().find(|r| &r.id == id).ok_or_else(bad)?;
                    (
                        &r.material_id,
                        &r.material_version_id,
                        &r.seq,
                        &r.recorded_at,
                        0,
                        0,
                        Quantity::parse(&r.quantity).map_err(unavailable)?.micros(),
                        Money::parse(&r.amount).map_err(unavailable)?.micros(),
                    )
                }
                (None, Some(id), None, None, None) => {
                    if !used_effects.insert(id) {
                        return Err(bad());
                    }
                    let e = effects.iter().find(|e| &e.id == id).ok_or_else(bad)?;
                    (
                        &e.material_id,
                        &e.material_version_id,
                        &e.seq,
                        &e.recorded_at,
                        Quantity::parse(&e.input_quantity)
                            .map_err(unavailable)?
                            .micros(),
                        Money::parse(&e.input_value).map_err(unavailable)?.micros(),
                        Quantity::parse(&e.output_quantity)
                            .map_err(unavailable)?
                            .micros(),
                        Money::parse(&e.output_value).map_err(unavailable)?.micros(),
                    )
                }
                (None, None, Some(id), None, None) => {
                    if !used_observations.insert(id) {
                        return Err(bad());
                    }
                    let o = observations.iter().find(|o| &o.id == id).ok_or_else(bad)?;
                    if o.effect
                        .before
                        .as_ref()
                        .and_then(|b| b.source_id.as_deref())
                        != previous
                        || o.effect.after.source_id.as_deref() != Some(&h.id)
                    {
                        return Err(bad());
                    }
                    baseline = h;
                    (
                        &o.effect.material_id,
                        &o.effect.material_version_id,
                        &o.seq,
                        &o.recorded_at,
                        q,
                        v,
                        Quantity::parse(&o.effect.after.quantity)
                            .map_err(unavailable)?
                            .micros(),
                        Money::parse(o.effect.after.value.as_deref().ok_or_else(bad)?)
                            .map_err(unavailable)?
                            .micros(),
                    )
                }
                (None, None, None, Some(id), None) => {
                    if !used_wastes.insert(id) {
                        return Err(bad());
                    }
                    let w = wastes.iter().find(|w| &w.id == id).ok_or_else(bad)?;
                    if w.effect.before.source_id.as_deref() != previous
                        || w.effect.after.source_id.as_deref() != Some(&h.id)
                    {
                        return Err(bad());
                    }
                    (
                        &w.effect.material_id,
                        &w.effect.material_version_id,
                        &w.seq,
                        &w.recorded_at,
                        Quantity::parse(&w.effect.quantity)
                            .map_err(unavailable)?
                            .micros(),
                        Money::parse(&w.effect.loss_value)
                            .map_err(unavailable)?
                            .micros(),
                        0,
                        0,
                    )
                }
                (None, None, None, None, Some(id)) => {
                    if !used_reconciliations.insert(id) {
                        return Err(bad());
                    }
                    let e = reconciliation_effects
                        .iter()
                        .find(|e| &e.id == id)
                        .ok_or_else(bad)?;
                    if e.effect
                        .before
                        .as_ref()
                        .and_then(|b| b.source_id.as_deref())
                        != previous
                        || e.effect.after.source_id.as_deref() != Some(&h.id)
                    {
                        return Err(bad());
                    }
                    let delta = super::reconciliation::signed(&e.effect.value_delta)?;
                    let inflow = Money::parse(&e.effect.inflow_value)
                        .map_err(unavailable)?
                        .micros();
                    (
                        &e.effect.material_id,
                        &e.effect.material_version_id,
                        &e.seq,
                        &e.recorded_at,
                        0,
                        if delta < 0 { -delta } else { 0 },
                        Quantity::parse(&e.effect.inflow_quantity)
                            .map_err(unavailable)?
                            .micros(),
                        inflow.checked_add(delta.max(0)).ok_or_else(bad)?,
                    )
                }
                _ => return Err(bad()),
            };
            if iq > q || iv > v {
                return Err(bad());
            }
            q = q
                .checked_sub(iq)
                .and_then(|q| q.checked_add(oq))
                .ok_or_else(bad)?;
            v = v
                .checked_sub(iv)
                .and_then(|v| v.checked_add(ov))
                .ok_or_else(bad)?;
            if seq <= prior_seq
                || seq.to_string() != h.seq
                || chrono::DateTime::parse_from_rfc3339(&h.recorded_at).is_err()
                || h.previous_version_id.as_deref() != previous
                || source_material != material
                || source_version != &h.material_version_id
                || source_seq != &h.seq
                || source_time != &h.recorded_at
                || h.baseline_receipt_id != baseline.receipt_id
                || h.baseline_production_effect_id != baseline.production_effect_id
                || h.baseline_observation_version_id != baseline.observation_version_id
                || h.baseline_reconciliation_effect_id != baseline.reconciliation_effect_id
                || h.baseline_seq != baseline.seq
                || Quantity::from_micros(q).map_err(unavailable)?.decimal() != h.quantity
                || Money::from_micros(v).map_err(unavailable)?.decimal() != h.value
                || (q == 0 && v != 0)
            {
                return Err(bad());
            }
            basis(h)?;
            if q > 0 {
                last_valid = Some(h.id.as_str());
            }
            previous = Some(h.id.as_str());
            prior_seq = seq;
        }
        let head = heads
            .iter()
            .find(|h| h.material_id == material)
            .ok_or_else(bad)?;
        if head.current_version_id.as_deref() != previous
            || head.last_valid_version_id.as_deref() != last_valid
        {
            return Err(bad());
        }
    }
    if used_receipts.len() != ordinary_receipts
        || used_effects.len() != effects.len()
        || used_observations.len() != observations.len()
        || used_wastes.len() != wastes.len()
        || used_reconciliations.len() != reconciliation_effects.len()
    {
        return Err(bad());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_corrected_holdings(
    receipts: &[crate::receipt::ReceiptRecord],
    effects: &[production::EffectRow],
    observations: &[stocktake::VersionRow],
    wastes: &[crate::waste::WasteRecord],
    reconciliations: &[crate::reconciliation::ReceiptReconciliation],
    reconciliation_effects: &[super::reconciliation::MaterialEffectRow],
    versions: &[HoldingVersion],
    heads: &[HoldingHead],
    corrections: &[super::correction::ProjectionVersionRow],
    purchase_returns: &[crate::purchase_return::PurchaseReturnRecord],
) -> Result<(), StoreError> {
    use std::collections::HashSet;
    let bad = || unavailable("정정 이후 보유 원천·계산 경계·현재 값이 다릅니다");
    let mut ids = HashSet::new();
    let mut sources = HashSet::new();
    for h in versions {
        if Uuid::parse_str(&h.id).map_err(unavailable)?.to_string() != h.id || !ids.insert(&h.id) {
            return Err(bad());
        }
        let n = h.seq.parse::<i64>().map_err(unavailable)?;
        if n <= 0
            || n.to_string() != h.seq
            || chrono::DateTime::parse_from_rfc3339(&h.recorded_at).is_err()
        {
            return Err(bad());
        }
        let before = input_at(versions, observations, corrections, &h.material_id, n - 1)?;
        if h.previous_version_id != before.source_id {
            return Err(bad());
        }
        let q = Quantity::parse(&h.quantity).map_err(unavailable)?;
        let v = Money::parse(&h.value).map_err(unavailable)?;
        if q.micros() == 0 && v.micros() != 0 {
            return Err(bad());
        }
        let count = [
            h.receipt_id.is_some(),
            h.production_effect_id.is_some(),
            h.observation_version_id.is_some(),
            h.waste_id.is_some(),
            h.purchase_return_id.is_some(),
            h.reconciliation_effect_id.is_some(),
            h.correction_projection_id.is_some(),
        ]
        .into_iter()
        .filter(|x| *x)
        .count();
        if count != 1 {
            return Err(bad());
        }
        if let Some(id) = &h.correction_projection_id {
            let p = corrections.iter().find(|p| &p.id == id).ok_or_else(bad)?;
            let super::correction::Projection::Material {
                change,
                holding_version_id: Some(actual),
                price_basis_id,
                ..
            } = &p.projection
            else {
                return Err(bad());
            };
            let crate::correction::CorrectionHoldingState::Known { amounts, .. } = &change.after
            else {
                return Err(bad());
            };
            if actual != &h.id
                || change.material_id != h.material_id
                || change.material_version_id != h.material_version_id
                || p.seq != h.seq
                || p.recorded_at != h.recorded_at
                || amounts.quantity != h.quantity
                || amounts.value.as_deref() != Some(&h.value)
                || price_basis_id != &h.last_valid_correction_price_id
            {
                return Err(bad());
            }
            continue;
        }
        let mut expected_basis = before.basis.clone();
        let (owner, version, time, sequence, iq, iv, oq, ov, source) =
            if let Some(id) = &h.receipt_id {
                let r = receipts.iter().find(|r| &r.id == id).ok_or_else(bad)?;
                if reconciliations.iter().any(|s| s.receipt_id == r.id) {
                    return Err(bad());
                }
                if expected_basis.is_none() {
                    expected_basis = Some(HoldingBasisRef {
                        kind: HoldingBasisKind::Receipt,
                        id: Some(id.clone()),
                        seq: Some(h.seq.clone()),
                    });
                }
                (
                    &r.material_id,
                    &r.material_version_id,
                    &r.recorded_at,
                    &r.seq,
                    0,
                    0,
                    Quantity::parse(&r.quantity).map_err(unavailable)?.micros(),
                    Money::parse(&r.amount).map_err(unavailable)?.micros(),
                    format!("receipt:{id}"),
                )
            } else if let Some(id) = &h.production_effect_id {
                let e = effects.iter().find(|e| &e.id == id).ok_or_else(bad)?;
                if expected_basis.is_none() {
                    expected_basis = Some(HoldingBasisRef {
                        kind: HoldingBasisKind::Production,
                        id: Some(id.clone()),
                        seq: Some(h.seq.clone()),
                    });
                }
                (
                    &e.material_id,
                    &e.material_version_id,
                    &e.recorded_at,
                    &e.seq,
                    Quantity::parse(&e.input_quantity)
                        .map_err(unavailable)?
                        .micros(),
                    Money::parse(&e.input_value).map_err(unavailable)?.micros(),
                    Quantity::parse(&e.output_quantity)
                        .map_err(unavailable)?
                        .micros(),
                    Money::parse(&e.output_value).map_err(unavailable)?.micros(),
                    format!("production:{id}"),
                )
            } else if let Some(id) = &h.observation_version_id {
                let o = observations.iter().find(|o| &o.id == id).ok_or_else(bad)?;
                if o.correction_projection_id.is_some() {
                    return Err(bad());
                }
                expected_basis = Some(HoldingBasisRef {
                    kind: HoldingBasisKind::Observation,
                    id: Some(id.clone()),
                    seq: Some(h.seq.clone()),
                });
                (
                    &o.effect.material_id,
                    &o.effect.material_version_id,
                    &o.recorded_at,
                    &o.seq,
                    before.quantity.map(|q| q.micros()).unwrap_or(0),
                    before.value.map(|v| v.micros()).unwrap_or(0),
                    Quantity::parse(&o.effect.after.quantity)
                        .map_err(unavailable)?
                        .micros(),
                    Money::parse(o.effect.after.value.as_deref().ok_or_else(bad)?)
                        .map_err(unavailable)?
                        .micros(),
                    format!("observation:{id}"),
                )
            } else if let Some(id) = &h.purchase_return_id {
                let r = purchase_returns
                    .iter()
                    .find(|r| &r.id == id)
                    .ok_or_else(bad)?;
                let crate::purchase_return::PurchaseReturnProjectionCheck::Ready(expected) =
                    crate::purchase_return::project_purchase_return(
                        &before,
                        Quantity::parse(&r.effect.quantity).map_err(unavailable)?,
                    )
                    .map_err(unavailable)?
                else {
                    return Err(bad());
                };
                if expected.deducted_value != r.effect.deducted_value {
                    return Err(bad());
                }
                (
                    &r.effect.material_id,
                    &r.effect.material_version_id,
                    &r.recorded_at,
                    &r.seq,
                    Quantity::parse(&r.effect.quantity)
                        .map_err(unavailable)?
                        .micros(),
                    Money::parse(&r.effect.deducted_value)
                        .map_err(unavailable)?
                        .micros(),
                    0,
                    0,
                    format!("purchase_return:{id}"),
                )
            } else if let Some(id) = &h.waste_id {
                let w = wastes.iter().find(|w| &w.id == id).ok_or_else(bad)?;
                (
                    &w.effect.material_id,
                    &w.effect.material_version_id,
                    &w.recorded_at,
                    &w.seq,
                    Quantity::parse(&w.effect.quantity)
                        .map_err(unavailable)?
                        .micros(),
                    Money::parse(&w.effect.loss_value)
                        .map_err(unavailable)?
                        .micros(),
                    0,
                    0,
                    format!("waste:{id}"),
                )
            } else if let Some(id) = &h.reconciliation_effect_id {
                let e = reconciliation_effects
                    .iter()
                    .find(|e| &e.id == id)
                    .ok_or_else(bad)?;
                if expected_basis.is_none() {
                    expected_basis = Some(HoldingBasisRef {
                        kind: HoldingBasisKind::Reconciliation,
                        id: Some(id.clone()),
                        seq: Some(h.seq.clone()),
                    });
                }
                let added = Money::parse(&e.effect.inflow_value)
                    .map_err(unavailable)?
                    .micros()
                    .checked_add(super::reconciliation::signed(&e.effect.value_delta)?)
                    .ok_or_else(bad)?;
                (
                    &e.effect.material_id,
                    &e.effect.material_version_id,
                    &e.recorded_at,
                    &e.seq,
                    0,
                    0,
                    Quantity::parse(&e.effect.inflow_quantity)
                        .map_err(unavailable)?
                        .micros(),
                    added,
                    format!("reconciliation:{id}"),
                )
            } else {
                return Err(bad());
            };
        let beforeq = before.quantity.map(|q| q.micros()).unwrap_or(0);
        let beforev = before.value.map(|v| v.micros()).unwrap_or(0);
        let expectedq = beforeq
            .checked_sub(iq)
            .and_then(|n| n.checked_add(oq))
            .ok_or_else(bad)?;
        let expectedv = beforev
            .checked_sub(iv)
            .and_then(|n| n.checked_add(ov))
            .ok_or_else(bad)?;
        if !sources.insert(source)
            || owner != &h.material_id
            || version != &h.material_version_id
            || time != &h.recorded_at
            || sequence != &h.seq
            || expectedq != q.micros()
            || expectedv != v.micros()
            || Some(basis(h)?) != expected_basis
        {
            return Err(bad());
        }
    }
    let ordinary = receipts
        .iter()
        .filter(|r| !reconciliations.iter().any(|s| s.receipt_id == r.id))
        .count();
    if sources.len()
        != ordinary
            + effects.len()
            + observations
                .iter()
                .filter(|o| o.correction_projection_id.is_none())
                .count()
            + wastes.len()
            + purchase_returns.len()
            + reconciliation_effects.len()
    {
        return Err(bad());
    }
    let at = versions
        .iter()
        .map(|h| h.seq.parse::<i64>().unwrap_or(0))
        .chain(
            corrections
                .iter()
                .map(|p| p.seq.parse::<i64>().unwrap_or(0)),
        )
        .max()
        .unwrap_or(0);
    let mut material_ids = versions
        .iter()
        .map(|h| h.material_id.as_str())
        .collect::<HashSet<_>>();
    for p in corrections {
        if let super::correction::Projection::Material { change, .. } = &p.projection {
            material_ids.insert(&change.material_id);
        }
    }
    if material_ids.len() != heads.len() {
        return Err(bad());
    }
    for material in material_ids {
        let state = input_at(versions, observations, corrections, material, at)?;
        let head = heads
            .iter()
            .find(|h| h.material_id == material)
            .ok_or_else(bad)?;
        if head.current_version_id != state.source_id {
            return Err(bad());
        }
        if state.quantity.is_none() {
            let p=corrections.iter().filter(|p|matches!(&p.projection,super::correction::Projection::Material{change,..} if change.material_id==material)).max_by_key(|p|p.seq.parse::<i64>().unwrap_or(-1)).ok_or_else(bad)?;
            if head.unknown_correction_projection_id.as_deref() != Some(&p.id)
                || head.last_valid_version_id.is_some()
                || head.last_valid_correction_price_id.is_some()
            {
                return Err(bad());
            }
        } else {
            if head.unknown_correction_projection_id.is_some() {
                return Err(bad());
            }
            let last = state.last_valid.as_ref().map(|p| p.source_id());
            if head
                .last_valid_version_id
                .as_deref()
                .or(head.last_valid_correction_price_id.as_deref())
                != last
            {
                return Err(bad());
            }
        }
    }
    Ok(())
}
