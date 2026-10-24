use super::{Store, StoreError, catalog, production, unavailable};
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
    pub id: String,
    pub material_id: String,
    pub material_version_id: String,
    pub previous_version_id: Option<String>,
    pub receipt_id: Option<String>,
    pub baseline_receipt_id: Option<String>,
    pub production_effect_id: Option<String>,
    pub baseline_production_effect_id: Option<String>,
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
    pub current_version_id: String,
    pub last_valid_version_id: String,
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
    let rows=connection.prepare("SELECT id,material_id,material_version_id,previous_version_id,receipt_id,baseline_receipt_id,baseline_sequence,quantity_micros,value_micros,recorded_at,sequence,production_effect_id,baseline_production_effect_id FROM holding_versions WHERE (?1 IS NULL OR material_id=?1) AND (?2 IS NULL OR id=?2) ORDER BY sequence,id").map_err(unavailable)?
        .query_map(params![material,id], |r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,Option<String>>(3)?,r.get::<_,Option<String>>(4)?,r.get::<_,Option<String>>(5)?,r.get::<_,i64>(6)?,r.get::<_,i64>(7)?,r.get::<_,i64>(8)?,r.get::<_,String>(9)?,r.get::<_,i64>(10)?,r.get::<_,Option<String>>(11)?,r.get::<_,Option<String>>(12)?)))
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
            )| {
                Ok(HoldingVersion {
                    id,
                    material_id,
                    material_version_id,
                    previous_version_id,
                    receipt_id,
                    baseline_receipt_id,
                    production_effect_id,
                    baseline_production_effect_id,
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
    connection.prepare("SELECT material_id,current_version_id,last_valid_version_id FROM holding_heads ORDER BY material_id").map_err(unavailable)?
        .query_map([],|r|Ok(HoldingHead{material_id:r.get(0)?,current_version_id:r.get(1)?,last_valid_version_id:r.get(2)?})).map_err(unavailable)?
        .collect::<rusqlite::Result<_>>().map_err(unavailable)
}

pub(super) fn current_input(
    connection: &Connection,
    material_id: &str,
) -> Result<HoldingInput, StoreError> {
    let row=connection.query_row("SELECT current_version_id,last_valid_version_id FROM holding_heads WHERE material_id=?1",[material_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?))).optional().map_err(unavailable)?;
    let Some((id, last)) = row else {
        return Ok(HoldingInput::default());
    };
    let version = read_versions(connection, Some(material_id), Some(&id))?
        .pop()
        .ok_or_else(|| unavailable("현재 보유 버전이 없습니다"))?;
    let last = read_versions(connection, Some(material_id), Some(&last))?
        .pop()
        .ok_or_else(|| unavailable("마지막 유효 보유 버전이 없습니다"))?;
    let input = HoldingInput {
        basis: Some(basis(&version)?),
        observation: None,
        quantity: Some(Quantity::parse(&version.quantity).map_err(unavailable)?),
        value: Some(Money::parse(&version.value).map_err(unavailable)?),
        source_id: Some(id),
        last_valid: Some(
            UnitPrice::new(
                Money::parse(&last.value).map_err(unavailable)?,
                Quantity::parse(&last.quantity).map_err(unavailable)?,
                last.id,
            )
            .map_err(unavailable)?,
        ),
    };
    validate_holding_input(&input).map_err(unavailable)?;
    Ok(input)
}

pub(super) fn basis(version: &HoldingVersion) -> Result<HoldingBasisRef, StoreError> {
    let (kind, id) = match (
        &version.baseline_receipt_id,
        &version.baseline_production_effect_id,
    ) {
        (Some(id), None) => (HoldingBasisKind::Receipt, id),
        (None, Some(id)) => (HoldingBasisKind::Production, id),
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
        before,
        after,
        seq,
        time,
    } = input;
    let id = Uuid::new_v4().to_string();
    let (baseline_receipt, baseline_production, boundary) = match before.basis.as_ref() {
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
                HoldingBasisKind::Receipt => (Some(source), None, boundary),
                HoldingBasisKind::Production => (None, Some(source), boundary),
                HoldingBasisKind::Observation => {
                    return Err(unavailable("실제 관측 저장은 아직 지원하지 않습니다"));
                }
            }
        }
        None => (receipt_id, production_effect_id, seq),
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
        id.as_str()
    } else if before.quantity.as_ref().is_some_and(|q| q.micros() > 0) {
        before
            .source_id
            .as_deref()
            .ok_or_else(|| unavailable("현재 양수 보유의 근거가 없습니다"))?
    } else {
        before
            .last_valid
            .as_ref()
            .map(UnitPrice::source_id)
            .ok_or_else(|| unavailable("0재고의 마지막 유효 단가가 없습니다"))?
    };
    connection.execute("INSERT INTO holding_versions(id,material_id,material_version_id,previous_version_id,receipt_id,baseline_receipt_id,baseline_sequence,quantity_micros,value_micros,recorded_at,sequence,production_effect_id,baseline_production_effect_id) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",params![id,material_id,material_version_id,before.source_id,receipt_id,baseline_receipt,boundary,q.micros(),v.micros(),time,seq,production_effect_id,baseline_production]).map_err(unavailable)?;
    connection.execute("INSERT INTO holding_heads(material_id,current_version_id,last_valid_version_id) VALUES(?1,?2,?3) ON CONFLICT(material_id) DO UPDATE SET current_version_id=excluded.current_version_id,last_valid_version_id=excluded.last_valid_version_id",params![material_id,id,last]).map_err(unavailable)?;
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
        if let Some(first) = versions(&self.connection, Some(&id))?.first() {
            let baseline = HoldingInput {
                basis: Some(basis(first)?),
                quantity: Some(Quantity::parse(&first.quantity).map_err(unavailable)?),
                value: Some(Money::parse(&first.value).map_err(unavailable)?),
                source_id: Some(first.id.clone()),
                observation: None,
                last_valid: None,
            };
            let mut changes = super::receipt::records(&self.connection, Some(&id), None)?
                .into_iter()
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
            changes.sort_by_key(|c| c.seq().parse::<i64>().unwrap_or(-1));
            view = project_holding(&baseline, &changes).map_err(unavailable)?;
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
        }))
    }
}

pub(super) fn validate_holding_schema(connection: &Connection) -> anyhow::Result<()> {
    let versions = versions(connection, None)?;
    let heads = heads(connection)?;
    let receipts = super::receipt::records(connection, None, None)?;
    validate_snapshot(
        &receipts,
        &production::effects(connection, None, None)?,
        &versions,
        &heads,
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

pub(super) fn validate_snapshot(
    receipts: &[crate::receipt::ReceiptRecord],
    effects: &[production::EffectRow],
    versions: &[HoldingVersion],
    heads: &[HoldingHead],
) -> Result<(), StoreError> {
    use std::collections::{BTreeMap, HashSet};
    let bad = || unavailable("보유 버전의 원천·이전·현재·마지막 기준이 다릅니다");
    if versions.len() != receipts.len() + effects.len() {
        return Err(bad());
    }
    let mut grouped = BTreeMap::<&str, Vec<&HoldingVersion>>::new();
    let mut ids = HashSet::new();
    let mut used_receipts = HashSet::new();
    let mut used_effects = HashSet::new();
    for version in versions {
        Uuid::parse_str(&version.id).map_err(unavailable)?;
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
        let first = history[0];
        let mut previous = None;
        let mut last_valid = None;
        let mut prior_seq = 0;
        let (mut q, mut v) = (0i64, 0i64);
        for h in history {
            let seq = h.seq.parse::<i64>().map_err(unavailable)?;
            let (source_material, source_version, source_seq, source_time, iq, iv, oq, ov) =
                match (&h.receipt_id, &h.production_effect_id) {
                    (Some(id), None) => {
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
                    (None, Some(id)) => {
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
                || h.previous_version_id.as_deref() != previous
                || source_material != material
                || source_version != &h.material_version_id
                || source_seq != &h.seq
                || source_time != &h.recorded_at
                || h.baseline_receipt_id != first.receipt_id
                || h.baseline_production_effect_id != first.production_effect_id
                || h.baseline_seq != first.seq
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
        if Some(head.current_version_id.as_str()) != previous
            || Some(head.last_valid_version_id.as_str()) != last_valid
        {
            return Err(bad());
        }
    }
    if used_receipts.len() != receipts.len() || used_effects.len() != effects.len() {
        return Err(bad());
    }
    Ok(())
}
