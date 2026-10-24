use super::{Store, StoreError, catalog, unavailable};
use crate::{
    domain::{Money, Quantity, ResolvedPrice, UnitPrice},
    holding::*,
};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HoldingVersion {
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
    let rows=connection.prepare("SELECT id,material_id,material_version_id,previous_version_id,receipt_id,baseline_receipt_id,baseline_sequence,quantity_micros,value_micros,recorded_at,sequence FROM holding_versions WHERE (?1 IS NULL OR material_id=?1) ORDER BY sequence,id").map_err(unavailable)?
        .query_map([material], |r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,Option<String>>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,i64>(6)?,r.get::<_,i64>(7)?,r.get::<_,i64>(8)?,r.get::<_,String>(9)?,r.get::<_,i64>(10)?)))
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
            )| {
                Ok(HoldingVersion {
                    id,
                    material_id,
                    material_version_id,
                    previous_version_id,
                    receipt_id,
                    baseline_receipt_id,
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
    let row=connection.query_row("SELECT v.id,v.baseline_receipt_id,v.baseline_sequence,v.quantity_micros,v.value_micros,l.id,l.quantity_micros,l.value_micros FROM holding_heads h JOIN holding_versions v ON v.id=h.current_version_id AND v.material_id=h.material_id JOIN holding_versions l ON l.id=h.last_valid_version_id AND l.material_id=h.material_id WHERE h.material_id=?1",[material_id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,r.get::<_,i64>(3)?,r.get::<_,i64>(4)?,r.get::<_,String>(5)?,r.get::<_,i64>(6)?,r.get::<_,i64>(7)?))).optional().map_err(unavailable)?;
    let Some((id, baseline, seq, q, v, last, lq, lv)) = row else {
        return Ok(HoldingInput::default());
    };
    let input = HoldingInput {
        basis: Some(HoldingBasisRef {
            kind: HoldingBasisKind::Receipt,
            id: Some(baseline),
            seq: Some(seq.to_string()),
        }),
        observation: None,
        quantity: Some(Quantity::from_micros(q).map_err(unavailable)?),
        value: Some(Money::from_micros(v).map_err(unavailable)?),
        source_id: Some(id),
        last_valid: Some(
            UnitPrice::new(
                Money::from_micros(lv).map_err(unavailable)?,
                Quantity::from_micros(lq).map_err(unavailable)?,
                last,
            )
            .map_err(unavailable)?,
        ),
    };
    validate_holding_input(&input).map_err(unavailable)?;
    Ok(input)
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
    let id = Uuid::new_v4().to_string();
    let baseline = before
        .basis
        .as_ref()
        .and_then(|b| b.id.as_deref())
        .unwrap_or(receipt_id);
    let boundary = before
        .basis
        .as_ref()
        .and_then(|b| b.seq.as_deref())
        .map(str::parse::<i64>)
        .transpose()
        .map_err(unavailable)?
        .unwrap_or(seq);
    connection.execute("INSERT INTO holding_versions(id,material_id,material_version_id,previous_version_id,receipt_id,baseline_receipt_id,baseline_sequence,quantity_micros,value_micros,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![id,material_id,material_version_id,before.source_id,receipt_id,baseline,boundary,Quantity::parse(&projected.after.quantity).map_err(unavailable)?.micros(),Money::parse(projected.after.value.as_deref().ok_or_else(||unavailable("보유 가치가 없습니다"))?).map_err(unavailable)?.micros(),time,seq]).map_err(unavailable)?;
    connection.execute("INSERT INTO holding_heads(material_id,current_version_id,last_valid_version_id) VALUES(?1,?2,?2) ON CONFLICT(material_id) DO UPDATE SET current_version_id=excluded.current_version_id,last_valid_version_id=excluded.last_valid_version_id",params![material_id,id]).map_err(unavailable)?;
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
                basis: Some(HoldingBasisRef {
                    kind: HoldingBasisKind::Receipt,
                    id: Some(first.receipt_id.clone()),
                    seq: Some(first.seq.clone()),
                }),
                quantity: Some(Quantity::parse(&first.quantity).map_err(unavailable)?),
                value: Some(Money::parse(&first.value).map_err(unavailable)?),
                source_id: Some(first.id.clone()),
                observation: None,
                last_valid: None,
            };
            let changes = super::receipt::records(&self.connection, Some(&id), None)?
                .into_iter()
                .filter(|r| r.seq.parse::<i64>().ok() > first.seq.parse::<i64>().ok())
                .map(|r| HoldingChange {
                    kind: HoldingChangeKind::Receipt,
                    event_id: r.id,
                    quantity: r.quantity,
                    value: r.amount,
                    recorded_at: r.recorded_at,
                    seq: r.seq,
                })
                .collect::<Vec<_>>();
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
                return Err(unavailable("입고 이력과 현재 보유 기준이 다릅니다"));
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
    if versions.len() != receipts.len() {
        anyhow::bail!("입고와 보유 버전 수가 다릅니다")
    }
    let materials = versions
        .iter()
        .map(|v| &v.material_id)
        .collect::<std::collections::HashSet<_>>();
    if materials.len() != heads.len() {
        anyhow::bail!("현재 보유 기준이 없습니다")
    }
    for material in materials {
        let history = versions
            .iter()
            .filter(|v| &v.material_id == material)
            .collect::<Vec<_>>();
        let first = history[0];
        let mut q = 0i64;
        let mut value = 0i64;
        let mut previous: Option<&str> = None;
        for version in &history {
            let receipt = receipts
                .iter()
                .find(|r| r.id == version.receipt_id)
                .ok_or_else(|| anyhow::anyhow!("보유 원입고가 없습니다"))?;
            q = q
                .checked_add(Quantity::parse(&receipt.quantity)?.micros())
                .ok_or_else(|| anyhow::anyhow!("보유 수량 범위 초과"))?;
            value = value
                .checked_add(Money::parse(&receipt.amount)?.micros())
                .ok_or_else(|| anyhow::anyhow!("보유 가치 범위 초과"))?;
            if version.previous_version_id.as_deref() != previous
                || version.baseline_receipt_id != first.receipt_id
                || version.baseline_seq != first.seq
                || version.material_id != receipt.material_id
                || version.material_version_id != receipt.material_version_id
                || version.seq != receipt.seq
                || version.recorded_at != receipt.recorded_at
                || Quantity::parse(&version.quantity)?.micros() != q
                || Money::parse(&version.value)?.micros() != value
            {
                anyhow::bail!("보유 버전과 원입고·이전 기준이 다릅니다")
            }
            previous = Some(&version.id);
        }
        let head = heads
            .iter()
            .find(|h| &h.material_id == material)
            .ok_or_else(|| anyhow::anyhow!("보유 head가 없습니다"))?;
        if Some(head.current_version_id.as_str()) != previous
            || head.last_valid_version_id != head.current_version_id
        {
            anyhow::bail!("현재·마지막 유효 보유 버전이 다릅니다")
        }
        current_input(connection, material)?;
    }
    Ok(())
}
