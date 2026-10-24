use super::{
    Store, StoreError, business, catalog, holding, production, receipt, stocktake, unavailable,
};
use crate::domain::{Money, Quantity, Unit};
use crate::reconciliation::provenance::*;
use crate::reconciliation::*;
use crate::stocktake::StocktakeCalculation;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LinkRow {
    pub id: String,
    pub reconciliation_id: String,
    pub selection_position: usize,
    pub portion_position: usize,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LinkVersionRow {
    pub id: String,
    pub link_id: String,
    pub previous_version_id: Option<String>,
    pub observation_version_id: String,
    pub portion: InclusionPortionEffect,
    pub recorded_at: String,
    pub seq: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LinkHead {
    pub link_id: String,
    pub current_version_id: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReservationRow {
    pub id: String,
    pub link_version_id: String,
    pub source_key: SourceKey,
    pub source_version_id: String,
    pub reserved_source_share: SourceShare,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MaterialEffectRow {
    pub id: String,
    pub reconciliation_id: String,
    pub effect: ReconciliationMaterialEffect,
    pub recorded_at: String,
    pub seq: String,
}

pub(super) struct SnapshotRows {
    pub records: Vec<ReceiptReconciliation>,
    pub links: Vec<LinkRow>,
    pub versions: Vec<LinkVersionRow>,
    pub heads: Vec<LinkHead>,
    pub reservations: Vec<ReservationRow>,
    pub effects: Vec<MaterialEffectRow>,
}

pub(super) struct Validation<'a> {
    pub records: &'a [ReceiptReconciliation],
    pub links: &'a [LinkRow],
    pub versions: &'a [LinkVersionRow],
    pub heads: &'a [LinkHead],
    pub reservations: &'a [ReservationRow],
    pub effects: &'a [MaterialEffectRow],
    pub receipts: &'a [crate::receipt::ReceiptRecord],
    pub holdings: &'a [holding::HoldingVersion],
    pub observations: &'a [stocktake::VersionRow],
    pub observation_rows: &'a [stocktake::ObservationRow],
    pub observation_heads: &'a [stocktake::Head],
    pub productions: &'a [production::RecordRow],
    pub production_inputs: &'a [production::InputRow],
    pub production_outputs: &'a [production::OutputRow],
    pub wastes: &'a [crate::waste::WasteRecord],
    pub material_versions: &'a [crate::catalog::MaterialVersion],
    pub orders: &'a [crate::purchase_order::OrderVersion],
    pub business_records: &'a [business::Record],
    pub revisions: &'a [crate::business::BusinessRevision],
    pub confirmations: &'a [business::ConfirmationRow],
    pub mutations: &'a [(&'a str, &'a str, &'a str)],
    pub sequence: i64,
}

fn json_rows<T: serde::de::DeserializeOwned>(
    connection: &Connection,
    sql: &str,
) -> Result<Vec<T>, StoreError> {
    let rows = connection
        .prepare(sql)
        .map_err(unavailable)?
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(unavailable)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(unavailable)?;
    rows.into_iter()
        .map(|s| serde_json::from_str(&s).map_err(unavailable))
        .collect()
}

pub(super) fn records(connection: &Connection) -> Result<Vec<ReceiptReconciliation>, StoreError> {
    validate_physical_columns(connection)?;
    json_rows(
        connection,
        "SELECT result_json FROM receipt_reconciliations ORDER BY sequence,id",
    )
}

pub(super) fn effects(connection: &Connection) -> Result<Vec<MaterialEffectRow>, StoreError> {
    validate_physical_columns(connection)?;
    json_rows(
        connection,
        "SELECT row_json FROM reconciliation_material_effects ORDER BY sequence,id",
    )
}

pub(super) fn snapshot_rows(connection: &Connection) -> Result<SnapshotRows, StoreError> {
    Ok(SnapshotRows {
        records: records(connection)?,
        links: json_rows(
            connection,
            "SELECT row_json FROM inclusion_links ORDER BY reconciliation_id,selection_position,portion_position",
        )?,
        versions: json_rows(
            connection,
            "SELECT row_json FROM inclusion_link_versions ORDER BY sequence,id",
        )?,
        heads: connection
            .prepare("SELECT link_id,current_version_id FROM inclusion_link_heads ORDER BY link_id")
            .map_err(unavailable)?
            .query_map([], |r| {
                Ok(LinkHead {
                    link_id: r.get(0)?,
                    current_version_id: r.get(1)?,
                })
            })
            .map_err(unavailable)?
            .collect::<rusqlite::Result<_>>()
            .map_err(unavailable)?,
        reservations: json_rows(
            connection,
            "SELECT row_json FROM inclusion_reservations ORDER BY id",
        )?,
        effects: effects(connection)?,
    })
}

struct History<'a> {
    receipts: &'a [crate::receipt::ReceiptRecord],
    reconciliations: &'a [ReceiptReconciliation],
    holdings: &'a [holding::HoldingVersion],
    observations: &'a [stocktake::VersionRow],
    observation_rows: &'a [stocktake::ObservationRow],
    productions: &'a [production::RecordRow],
    inputs: &'a [production::InputRow],
    outputs: &'a [production::OutputRow],
    wastes: &'a [crate::waste::WasteRecord],
    materials: &'a [crate::catalog::MaterialVersion],
}

struct Loaded {
    receipts: Vec<crate::receipt::ReceiptRecord>,
    reconciliations: Vec<ReceiptReconciliation>,
    holdings: Vec<holding::HoldingVersion>,
    stocktake: stocktake::SnapshotRows,
    production: production::SnapshotRows,
    wastes: Vec<crate::waste::WasteRecord>,
    materials: Vec<crate::catalog::MaterialVersion>,
}
impl Loaded {
    fn read(connection: &Connection) -> Result<Self, StoreError> {
        Ok(Self {
            receipts: receipt::records(connection, None, None)?,
            reconciliations: records(connection)?,
            holdings: holding::versions(connection, None)?,
            stocktake: stocktake::snapshot_rows(connection)?,
            production: production::snapshot_rows(connection)?,
            wastes: super::waste::records(connection, None, None)?,
            materials: catalog::versions(connection, None)?,
        })
    }
    fn history(&self) -> History<'_> {
        History {
            receipts: &self.receipts,
            reconciliations: &self.reconciliations,
            holdings: &self.holdings,
            observations: &self.stocktake.versions,
            observation_rows: &self.stocktake.observations,
            productions: &self.production.records,
            inputs: &self.production.inputs,
            outputs: &self.production.outputs,
            wastes: &self.wastes,
            materials: &self.materials,
        }
    }
}

fn seq(s: &str) -> Result<i64, StoreError> {
    let value = s.parse::<i64>().map_err(unavailable)?;
    if value <= 0 || value.to_string() != s {
        return Err(unavailable("보정 순번이 올바르지 않습니다"));
    }
    Ok(value)
}
fn q(s: &str) -> Result<Quantity, StoreError> {
    Quantity::parse(s).map_err(unavailable)
}
fn money(s: &str) -> Result<Money, StoreError> {
    Money::parse(s).map_err(unavailable)
}
pub(super) fn signed(s: &str) -> Result<i64, StoreError> {
    let negative = s.starts_with('-');
    let value = money(s.strip_prefix('-').unwrap_or(s))?.micros();
    let value = if negative { -value } else { value };
    if ValueDelta::from_micros(value)
        .map_err(unavailable)?
        .decimal()
        != s
    {
        return Err(unavailable("보정 금액 표기가 올바르지 않습니다"));
    }
    Ok(value)
}
fn material(id: &str, version: &str, unit: &str) -> Result<ValueMaterial, StoreError> {
    Ok(ValueMaterial {
        material_id: id.into(),
        material_version_id: version.into(),
        unit: Unit::parse(unit).map_err(unavailable)?,
    })
}
impl History<'_> {
    fn current_material(
        &self,
        id: &str,
        at: i64,
    ) -> Result<&crate::catalog::MaterialVersion, StoreError> {
        self.materials
            .iter()
            .filter(|m| {
                m.material_id == id
                    && m.seq
                        .as_ref()
                        .is_none_or(|s| s.parse::<i64>().is_ok_and(|n| n <= at))
            })
            .max_by_key(|m| {
                m.seq
                    .as_ref()
                    .and_then(|s| s.parse::<i64>().ok())
                    .unwrap_or(0)
            })
            .ok_or_else(|| unavailable("보정 품목 버전이 없습니다"))
    }
    fn input(&self, id: &str, at: i64) -> Result<crate::holding::HoldingInput, StoreError> {
        let head = self
            .holdings
            .iter()
            .filter(|h| h.material_id == id && h.seq.parse::<i64>().is_ok_and(|n| n <= at))
            .max_by_key(|h| h.seq.parse::<i64>().unwrap_or(0));
        holding::input_from_history(self.holdings, self.observations, head)
    }
    fn events(&self, at: i64) -> Result<Vec<ValueReplayEvent>, StoreError> {
        let mut events = vec![];
        for r in self.receipts {
            let n = seq(&r.seq)?;
            if n > at {
                continue;
            }
            let rec = self.reconciliations.iter().find(|s| s.receipt_id == r.id);
            let (quantity, amount) = match rec {
                Some(s) => (
                    q(&s.effect.unreflected_quantity)?,
                    money(&s.effect.unreflected_amount)?,
                ),
                None => (q(&r.quantity)?, money(&r.amount)?),
            };
            if quantity.micros() > 0 {
                events.push(ValueReplayEvent::Inflow {
                    event: ValueEventRef {
                        id: r.id.clone(),
                        seq: n,
                    },
                    material: material(&r.material_id, &r.material_version_id, &r.unit)?,
                    quantity,
                    amount,
                });
            }
        }
        for o in self.observations {
            let n = seq(&o.seq)?;
            if n > at {
                continue;
            }
            let calculation = match &o.effect.calculation {
                StocktakeCalculation::Initial { value, .. } => ValueObservation::Initial {
                    value: money(value)?,
                },
                StocktakeCalculation::Settlement {
                    consumption_quantity,
                    excess_quantity,
                    excess_value,
                    ..
                } => ValueObservation::Settlement {
                    consumption: q(consumption_quantity)?,
                    excess: q(excess_quantity)?,
                    excess_value: money(excess_value)?,
                },
            };
            events.push(ValueReplayEvent::Observation {
                event: ValueEventRef {
                    id: o.id.clone(),
                    seq: n,
                },
                observation_id: o.observation_id.clone(),
                material: material(
                    &o.effect.material_id,
                    &o.effect.material_version_id,
                    &o.effect.unit,
                )?,
                measured: q(&o.effect.measured_quantity)?,
                calculation,
            });
        }
        for p in self.productions {
            let n = seq(&p.seq)?;
            if n > at {
                continue;
            }
            let inputs = self
                .inputs
                .iter()
                .filter(|r| r.production_id == p.id)
                .map(|r| {
                    Ok(ValueProductionInput {
                        position: r.row.position.parse().map_err(unavailable)?,
                        material: material(
                            &r.row.material_id,
                            &r.row.material_version_id,
                            &r.row.unit,
                        )?,
                        quantity: q(&r.row.quantity)?,
                    })
                })
                .collect::<Result<Vec<_>, StoreError>>()?;
            let outputs = self
                .outputs
                .iter()
                .filter(|r| r.production_id == p.id)
                .map(|r| {
                    Ok(ValueProductionOutput {
                        position: r.row.position.parse().map_err(unavailable)?,
                        material: material(
                            &r.row.material_id,
                            &r.row.material_version_id,
                            &r.row.unit,
                        )?,
                        quantity: q(&r.row.quantity)?,
                        allocation: crate::production::AllocationPercent::parse(
                            &r.row.allocation_percent,
                        )
                        .map_err(unavailable)?,
                    })
                })
                .collect::<Result<Vec<_>, StoreError>>()?;
            events.push(ValueReplayEvent::Production {
                event: ValueEventRef {
                    id: p.id.clone(),
                    seq: n,
                },
                inputs,
                outputs,
            });
        }
        for w in self.wastes {
            let n = seq(&w.seq)?;
            if n <= at {
                events.push(ValueReplayEvent::Waste {
                    event: ValueEventRef {
                        id: w.id.clone(),
                        seq: n,
                    },
                    material: material(
                        &w.effect.material_id,
                        &w.effect.material_version_id,
                        &w.effect.unit,
                    )?,
                    quantity: q(&w.effect.quantity)?,
                });
            }
        }
        let position = |event: &ValueReplayEvent| match event {
            ValueReplayEvent::Observation { observation_id, .. } => self
                .observation_rows
                .iter()
                .find(|o| &o.id == observation_id)
                .map(|o| o.position)
                .unwrap_or(0),
            _ => 0,
        };
        events.sort_by(|a, b| {
            a.event()
                .seq
                .cmp(&b.event().seq)
                .then(position(a).cmp(&position(b)))
                .then(a.event().id.cmp(&b.event().id))
        });
        Ok(events)
    }
    fn reserved(&self, at: i64) -> Result<Vec<ReservedInclusion>, StoreError> {
        let mut out = vec![];
        for r in self
            .reconciliations
            .iter()
            .filter(|r| r.seq.parse::<i64>().is_ok_and(|n| n <= at))
        {
            for s in &r.effect.selections {
                out.push(ReservedInclusion {
                    id: format!("{}:{}", r.id, s.position),
                    observation_version_id: s.observation_version_id.clone(),
                    material_id: r.effect.material_id.clone(),
                    quantity: q(&s.included_quantity)?,
                    portions: s
                        .portions
                        .iter()
                        .map(|p| ProvenancePortion {
                            source_key: p.source_key.clone(),
                            source_version_id: p.source_version_id.clone(),
                            included_share: p.included_share.clone(),
                            reserved_source_share: p.reserved_source_share.clone(),
                        })
                        .collect(),
                });
            }
        }
        Ok(out)
    }
    fn value_input(
        &self,
        at: i64,
        events: Vec<ValueReplayEvent>,
    ) -> Result<ValueReplayInput, StoreError> {
        let mut effective = vec![];
        for r in self
            .reconciliations
            .iter()
            .filter(|r| r.seq.parse::<i64>().is_ok_and(|n| n <= at))
        {
            for s in &r.effect.selections {
                for p in &s.portions {
                    let link = r
                        .selection_results
                        .iter()
                        .find(|l| {
                            l.selection_position == s.position && l.portion_position == p.position
                        })
                        .ok_or_else(|| unavailable("포함 연결 결과가 없습니다"))?;
                    effective.push(EffectiveValueReplacement {
                        link_id: link.link_id.clone(),
                        link_version_id: link.link_version_id.clone(),
                        value: ValueReplacement {
                            observation_version_id: s.observation_version_id.clone(),
                            material_id: r.effect.material_id.clone(),
                            source_key: p.source_key.clone(),
                            selection_position: s.position,
                            portion_position: p.position,
                            booked_value: money(&p.booked_value)?,
                            actual_included_value: money(&p.actual_included_value)?,
                        },
                    });
                }
            }
        }
        let mut heads = BTreeMap::new();
        for h in self
            .holdings
            .iter()
            .filter(|h| h.seq.parse::<i64>().is_ok_and(|n| n <= at))
        {
            let entry = heads.entry(&h.material_id).or_insert(h);
            if seq(&h.seq)? > seq(&entry.seq)? {
                *entry = h;
            }
        }
        let actual_heads = heads
            .into_values()
            .map(|h| {
                let m = self
                    .materials
                    .iter()
                    .find(|m| m.id == h.material_version_id)
                    .ok_or_else(|| unavailable("보유 품목 단위가 없습니다"))?;
                Ok(ValueActualHead {
                    material: material(&h.material_id, &h.material_version_id, &m.canonical_unit)?,
                    quantity: q(&h.quantity)?,
                    value: money(&h.value)?,
                    holding_version_id: h.id.clone(),
                })
            })
            .collect::<Result<_, StoreError>>()?;
        Ok(ValueReplayInput {
            at_revision: at,
            events,
            effective,
            actual_heads,
        })
    }
}

pub(super) struct ReceiptFacts {
    pub material: crate::catalog::MaterialVersion,
    pub quantity: Quantity,
    pub amount: Money,
    pub input_quantity: String,
    pub input_unit: String,
    pub receipt_date: String,
    pub order_before: Option<Box<crate::purchase_order::OrderVersion>>,
    pub order_after: Option<crate::receipt::OrderReceiptPreview>,
}
fn validation(e: impl std::fmt::Display) -> StoreError {
    StoreError::Validation(e.to_string())
}

fn exact_booked_value(
    observation: &stocktake::VersionRow,
    source: &InclusionSourceSelection,
    quantity: Quantity,
) -> Result<num_rational::BigRational, StoreError> {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    if quantity.micros() == 0 {
        return Ok(BigRational::from_integer(BigInt::from(0)));
    }
    let (basis_q, basis_v) = match (source, &observation.effect.calculation) {
        (
            InclusionSourceSelection::Source { source_key },
            StocktakeCalculation::Settlement {
                excess_quantity,
                excess_value,
                ..
            },
        ) if source_key.kind == SourceKind::ObservationExcess
            && source_key.id == observation.observation_id =>
        {
            (q(excess_quantity)?, money(excess_value)?)
        }
        _ => (
            q(&observation.effect.measured_quantity)?,
            money(
                observation
                    .effect
                    .after
                    .value
                    .as_deref()
                    .ok_or_else(|| unavailable("관측 가치가 없습니다"))?,
            )?,
        ),
    };
    if basis_q.micros() == 0 {
        return Err(validation("포함 가치의 기준 수량이 0입니다"));
    }
    Ok(BigRational::new(
        BigInt::from(basis_v.micros()) * BigInt::from(quantity.micros()),
        BigInt::from(basis_q.micros()),
    ))
}

fn booked_values(
    history: &History<'_>,
    at: i64,
    requests: &[InclusionRequest],
    observed: &[&stocktake::VersionRow],
) -> Result<Vec<Money>, StoreError> {
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::ToPrimitive;
    let mut results = vec![Money::from_micros(0).map_err(validation)?; requests.len()];
    let mut groups = BTreeMap::<&str, Vec<usize>>::new();
    for r in requests {
        groups
            .entry(&r.observation_version_id)
            .or_default()
            .push(r.position);
    }
    for (id, positions) in groups {
        let observation = observed[positions[0]];
        let mut exact = BigRational::from_integer(BigInt::from(0));
        let mut prior_booked = 0i64;
        for prior in history
            .reconciliations
            .iter()
            .filter(|r| r.seq.parse::<i64>().is_ok_and(|n| n <= at))
            .flat_map(|r| &r.effect.selections)
            .filter(|s| s.observation_version_id == id)
        {
            exact += exact_booked_value(observation, &prior.source, q(&prior.included_quantity)?)?;
            prior_booked = prior_booked
                .checked_add(money(&prior.booked_value)?.micros())
                .ok_or_else(|| unavailable("이미 연결한 가치 합계 범위를 넘습니다"))?;
        }
        let mut weights = vec![];
        for position in &positions {
            let r = &requests[*position];
            let part = exact_booked_value(observation, &r.source, r.quantity)?;
            exact += &part;
            let units = part / BigInt::from(1_000_000);
            weights.push(SourceShare {
                numerator: units.numer().to_string(),
                denominator: units.denom().to_string(),
                unit: "원".into(),
            });
        }
        // All terms are nonnegative. Round the complete frozen-value basis once,
        // then retain previously stored micro-won instead of rounding them again.
        let rounded = ((exact.numer() * 2u32 + exact.denom()) / (exact.denom() * 2u32))
            .to_i64()
            .ok_or_else(|| validation("포함 가치 합계 범위를 넘습니다"))?;
        let observed_value = money(
            observation
                .effect
                .after
                .value
                .as_deref()
                .ok_or_else(|| unavailable("관측 가치가 없습니다"))?,
        )?
        .micros();
        if rounded > observed_value {
            return Err(validation("포함 가치 합계가 관측 가치를 넘습니다"));
        }
        let amount = Money::from_micros(
            rounded
                .checked_sub(prior_booked)
                .ok_or_else(|| unavailable("이미 연결한 가치 잔여가 일치하지 않습니다"))?,
        )
        .map_err(validation)?;
        let allocated = allocate_share_amount(amount, &weights).map_err(validation)?;
        for (position, amount) in positions.into_iter().zip(allocated) {
            results[position] = amount;
        }
    }
    Ok(results)
}

fn calculate(
    history: &History<'_>,
    at: i64,
    facts: ReceiptFacts,
    selections: &[ReceiptInclusionSelection],
) -> Result<ReconciledReceiptEffect, StoreError> {
    let events = history.events(at)?;
    let reserved = history.reserved(at)?;
    let mut requests = vec![];
    let mut observed = vec![];
    for (position, s) in selections.iter().enumerate() {
        let id = s
            .observation_version_id
            .as_deref()
            .ok_or_else(|| validation("포함 근거의 실사를 선택하세요"))?;
        let o = history
            .observations
            .iter()
            .find(|o| o.id == id && o.seq.parse::<i64>().is_ok_and(|n| n <= at))
            .ok_or_else(|| validation("포함 관측이 없습니다"))?;
        let current = history
            .observations
            .iter()
            .filter(|v| {
                v.observation_id == o.observation_id && v.seq.parse::<i64>().is_ok_and(|n| n <= at)
            })
            .max_by_key(|v| v.seq.parse::<i64>().unwrap_or(0));
        if current.is_none_or(|v| v.id != o.id)
            || o.effect.material_id != facts.material.material_id
        {
            return Err(validation("선택 관측의 현재 버전과 입고 품목을 확인하세요"));
        }
        let quantity = Quantity::parse(&s.included_quantity).map_err(validation)?;
        requests.push(InclusionRequest {
            position,
            observation_version_id: id.into(),
            material_id: facts.material.material_id.clone(),
            source: s
                .source
                .clone()
                .ok_or_else(|| validation("포함 원천을 선택하세요"))?,
            quantity,
        });
        observed.push(o);
    }
    let split = split_receipt_amount(
        facts.quantity,
        facts.amount,
        &requests.iter().map(|r| r.quantity).collect::<Vec<_>>(),
    )
    .map_err(validation)?;
    let resolved = resolve_inclusions(&events, &reserved, &requests).map_err(validation)?;
    let booked_by_row = booked_values(history, at, &requests, &observed)?;
    let mut selected = vec![];
    let mut proposed = vec![];
    let mut booked_total = 0i64;
    for r in &requests {
        let o = observed[r.position];
        let booked = booked_by_row[r.position];
        booked_total = booked_total
            .checked_add(booked.micros())
            .ok_or_else(|| validation("포함 가치 합계 범위를 넘습니다"))?;
        let result = resolved
            .iter()
            .find(|v| v.position == r.position)
            .ok_or_else(|| unavailable("포함 출처 계산 결과가 없습니다"))?;
        let weights = result
            .portions
            .iter()
            .map(|p| p.included_share.clone())
            .collect::<Vec<_>>();
        let booked_parts = allocate_share_amount(booked, &weights).map_err(validation)?;
        let actual_parts = allocate_share_amount(split.selection_amounts[r.position], &weights)
            .map_err(validation)?;
        let portions = result
            .portions
            .iter()
            .enumerate()
            .map(|(position, p)| {
                proposed.push(ValueReplacement {
                    observation_version_id: o.id.clone(),
                    material_id: facts.material.material_id.clone(),
                    source_key: p.source_key.clone(),
                    selection_position: r.position,
                    portion_position: position,
                    booked_value: booked_parts[position],
                    actual_included_value: actual_parts[position],
                });
                InclusionPortionEffect {
                    position,
                    source_key: p.source_key.clone(),
                    source_version_id: p.source_version_id.clone(),
                    included_share: p.included_share.clone(),
                    reserved_source_share: p.reserved_source_share.clone(),
                    booked_value: booked_parts[position].decimal(),
                    actual_included_value: actual_parts[position].decimal(),
                }
            })
            .collect();
        let original = history
            .observation_rows
            .iter()
            .find(|v| v.id == o.observation_id)
            .ok_or_else(|| unavailable("관측 원본이 없습니다"))?;
        selected.push(ReceiptInclusionEffect {
            position: r.position,
            stocktake_id: original.stocktake_id.clone(),
            observation_id: o.observation_id.clone(),
            observation_version_id: o.id.clone(),
            source: r.source.clone(),
            included_quantity: r.quantity.decimal(),
            unit: o.effect.unit.clone(),
            booked_value: booked.decimal(),
            actual_included_value: split.selection_amounts[r.position].decimal(),
            value_difference: ValueDelta::from_micros(
                split.selection_amounts[r.position].micros() - booked.micros(),
            )
            .map_err(validation)?
            .decimal(),
            portions,
        });
    }
    let replay = replay_receipt_values(&history.value_input(at, events)?, &proposed).map_err(
        |e| match e {
            ValueReplayError::InvalidHistory(_) => unavailable(e),
            _ => validation(e),
        },
    )?;
    let mut deltas = replay
        .materials
        .into_iter()
        .map(|d| (d.material_id.clone(), d))
        .collect::<BTreeMap<_, _>>();
    let mut ids = deltas
        .iter()
        .filter(|(_, v)| v.delta.micros() != 0)
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    if !ids.contains(&facts.material.material_id) {
        ids.push(facts.material.material_id.clone());
    }
    ids.sort();
    let mut materials = vec![];
    for id in ids {
        let m = history.current_material(&id, at)?;
        let before = history.input(&id, at)?;
        let delta = deltas.remove(&id).map(|v| v.delta.micros()).unwrap_or(0);
        let (inflow_q, inflow_v) = if id == facts.material.material_id {
            (
                split.unreflected_quantity.micros(),
                split.unreflected_amount.micros(),
            )
        } else {
            (0, 0)
        };
        let after_q = Quantity::from_micros(
            before
                .quantity
                .map(|q| q.micros())
                .unwrap_or(0)
                .checked_add(inflow_q)
                .ok_or_else(|| validation("보유 수량 범위를 넘습니다"))?,
        )
        .map_err(validation)?;
        let after_v = Money::from_micros(
            i64::try_from(
                i128::from(before.value.map(|v| v.micros()).unwrap_or(0))
                    + i128::from(delta)
                    + i128::from(inflow_v),
            )
            .map_err(validation)?,
        )
        .map_err(validation)?;
        if after_q.micros() == 0 && after_v.micros() != 0 {
            return Err(validation("재고 0에 가치를 남길 수 없습니다"));
        }
        materials.push(ReconciliationMaterialEffect {
            material_id: id,
            material_version_id: m.id.clone(),
            material_name: m.name.clone(),
            unit: m.canonical_unit.clone(),
            before: before.quantity.map(|q| crate::holding::HoldingAmounts {
                quantity: q.decimal(),
                value: before.value.map(|v| v.decimal()),
                source_id: before.source_id.clone(),
            }),
            after: crate::holding::HoldingAmounts {
                quantity: after_q.decimal(),
                value: Some(after_v.decimal()),
                source_id: None,
            },
            basis_before: before.basis,
            observation: before.observation,
            value_delta: ValueDelta::from_micros(delta)
                .map_err(validation)?
                .decimal(),
            inflow_quantity: Quantity::from_micros(inflow_q)
                .map_err(validation)?
                .decimal(),
            inflow_value: Money::from_micros(inflow_v).map_err(validation)?.decimal(),
        });
    }
    Ok(ReconciledReceiptEffect {
        material_id: facts.material.material_id,
        material_version_id: facts.material.id,
        material_name: facts.material.name,
        quantity: facts.quantity.decimal(),
        amount: facts.amount.decimal(),
        unit: facts.material.canonical_unit,
        input_quantity: facts.input_quantity,
        input_unit: facts.input_unit,
        receipt_date: facts.receipt_date,
        included_quantity: split.included_quantity.decimal(),
        unreflected_quantity: split.unreflected_quantity.decimal(),
        included_amount: split.included_amount.decimal(),
        unreflected_amount: split.unreflected_amount.decimal(),
        booked_value: Money::from_micros(booked_total)
            .map_err(validation)?
            .decimal(),
        value_difference: replay.value_difference.decimal(),
        separate_adjustment: replay.separate_adjustment.decimal(),
        selections: selected,
        materials,
        trace: replay.trace,
        order_before: facts.order_before,
        order_after: facts.order_after,
    })
}

pub(super) fn inspect(
    connection: &Connection,
    facts: ReceiptFacts,
    selections: &[ReceiptInclusionSelection],
) -> Result<
    (
        Vec<crate::business::BusinessIssue>,
        Option<crate::business::BusinessEffect>,
    ),
    StoreError,
> {
    let loaded = Loaded::read(connection)?;
    let at = catalog::state(connection)?
        .revision
        .parse()
        .map_err(unavailable)?;
    match calculate(&loaded.history(), at, facts, selections) {
        Ok(receipt) => Ok((
            vec![],
            Some(crate::business::BusinessEffect::ReconciledReceipt {
                receipt: Box::new(receipt),
            }),
        )),
        Err(StoreError::Validation(message)) => Ok((
            vec![crate::business::BusinessIssue {
                field: "stock_effect.selections".into(),
                code: "needs_reconciliation".into(),
                message,
            }],
            None,
        )),
        Err(e) => Err(e),
    }
}

impl Store {
    pub fn receipt_inclusion_candidates(
        &self,
        material_id: &str,
    ) -> Result<ReceiptInclusionCandidates, StoreError> {
        let id = catalog::uuid(material_id, "품목 ID")?;
        let loaded = Loaded::read(&self.connection)?;
        let history = loaded.history();
        let state = catalog::state(&self.connection)?;
        let at = state.revision.parse().map_err(unavailable)?;
        let views = inclusion_availability(&history.events(at)?, &history.reserved(at)?, &id)
            .map_err(unavailable)?;
        let mut items = vec![];
        for view in views {
            let o = history
                .observations
                .iter()
                .find(|o| o.id == view.observation_version_id)
                .ok_or_else(|| unavailable("포함 관측이 없습니다"))?;
            if !loaded
                .stocktake
                .heads
                .iter()
                .any(|h| h.current_version_id == o.id)
            {
                continue;
            }
            let original = history
                .observation_rows
                .iter()
                .find(|v| v.id == o.observation_id)
                .ok_or_else(|| unavailable("관측 원본이 없습니다"))?;
            let sources = view
                .sources
                .into_iter()
                .map(|s| {
                    Ok(ReceiptInclusionSource {
                        source_key: s.source_key,
                        source_version_id: s.source_version_id,
                        source_material_name: history
                            .current_material(&s.material_id, at)?
                            .name
                            .clone(),
                        source_material_id: s.material_id,
                        source_unit: s.unit,
                        source_quantity: s.quantity.decimal(),
                        reserved_source_share: s.reserved_share,
                        remaining_source_share: s.remaining_share,
                        available_share: s.available_share,
                        available_quantity: s.available_quantity.decimal(),
                    })
                })
                .collect::<Result<_, StoreError>>()?;
            items.push(ReceiptInclusionObservation {
                stocktake_id: original.stocktake_id.clone(),
                observation_id: o.observation_id.clone(),
                observation_version_id: o.id.clone(),
                material_id: o.effect.material_id.clone(),
                material_version_id: o.effect.material_version_id.clone(),
                material_name: o.effect.material_name.clone(),
                unit: o.effect.unit.clone(),
                quantity: o.effect.measured_quantity.clone(),
                value: o
                    .effect
                    .after
                    .value
                    .clone()
                    .ok_or_else(|| unavailable("관측 가치가 없습니다"))?,
                available_share: view.available_share,
                available_quantity: view.available_quantity.decimal(),
                recorded_at: o.recorded_at.clone(),
                seq: o.seq.clone(),
                sources,
            });
        }
        Ok(ReceiptInclusionCandidates { items, state })
    }
}

pub(super) fn apply(
    connection: &Connection,
    receipt_id: &str,
    confirmation_id: &str,
    effect: ReconciledReceiptEffect,
    sequence: i64,
    time: &str,
) -> Result<(), StoreError> {
    let id = Uuid::new_v4().to_string();
    let record_id: String = connection
        .query_row(
            "SELECT record_id FROM business_confirmations WHERE id=?1",
            [confirmation_id],
            |r| r.get(0),
        )
        .map_err(unavailable)?;
    connection.execute("INSERT INTO receipt_reconciliations(id,receipt_id,confirmation_id,result_json,recorded_at,sequence) VALUES(?1,?2,?3,'{}',?4,?5)",params![id,receipt_id,confirmation_id,time,sequence]).map_err(unavailable)?;
    let mut selection_results = vec![];
    for s in &effect.selections {
        for p in &s.portions {
            let link = LinkRow {
                id: Uuid::new_v4().to_string(),
                reconciliation_id: id.clone(),
                selection_position: s.position,
                portion_position: p.position,
            };
            let version = LinkVersionRow {
                id: Uuid::new_v4().to_string(),
                link_id: link.id.clone(),
                previous_version_id: None,
                observation_version_id: s.observation_version_id.clone(),
                portion: p.clone(),
                recorded_at: time.into(),
                seq: sequence.to_string(),
            };
            let reservation = ReservationRow {
                id: Uuid::new_v4().to_string(),
                link_version_id: version.id.clone(),
                source_key: p.source_key.clone(),
                source_version_id: p.source_version_id.clone(),
                reserved_source_share: p.reserved_source_share.clone(),
            };
            connection.execute("INSERT INTO inclusion_links(id,reconciliation_id,selection_position,portion_position,row_json) VALUES(?1,?2,?3,?4,?5)",params![link.id,id,s.position as i64,p.position as i64,serde_json::to_string(&link).map_err(unavailable)?]).map_err(unavailable)?;
            connection.execute("INSERT INTO inclusion_link_versions(id,link_id,previous_version_id,observation_version_id,row_json,recorded_at,sequence) VALUES(?1,?2,NULL,?3,?4,?5,?6)",params![version.id,link.id,s.observation_version_id,serde_json::to_string(&version).map_err(unavailable)?,time,sequence]).map_err(unavailable)?;
            connection
                .execute(
                    "INSERT INTO inclusion_link_heads(link_id,current_version_id) VALUES(?1,?2)",
                    params![link.id, version.id],
                )
                .map_err(unavailable)?;
            connection.execute("INSERT INTO inclusion_reservations(id,link_version_id,row_json) VALUES(?1,?2,?3)",params![reservation.id,version.id,serde_json::to_string(&reservation).map_err(unavailable)?]).map_err(unavailable)?;
            selection_results.push(ReceiptInclusionResult {
                selection_position: s.position,
                portion_position: p.position,
                link_id: link.id,
                link_version_id: version.id,
                reservation_id: reservation.id,
            });
        }
    }
    let mut material_results = vec![];
    for e in &effect.materials {
        if q(&e.inflow_quantity)?.micros() == 0 && signed(&e.value_delta)? == 0 {
            continue;
        }
        let mut row = MaterialEffectRow {
            id: Uuid::new_v4().to_string(),
            reconciliation_id: id.clone(),
            effect: e.clone(),
            recorded_at: time.into(),
            seq: sequence.to_string(),
        };
        let before = holding::current_input(connection, &e.material_id)?;
        connection.execute("INSERT INTO reconciliation_material_effects(id,reconciliation_id,material_id,material_version_id,before_holding_version_id,row_json,recorded_at,sequence) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![row.id,id,e.material_id,e.material_version_id,before.source_id,serde_json::to_string(&row).map_err(unavailable)?,time,sequence]).map_err(unavailable)?;
        let holding_id = holding::append(
            connection,
            holding::AppendHolding {
                material_id: &e.material_id,
                material_version_id: &e.material_version_id,
                receipt_id: None,
                production_effect_id: None,
                observation_version_id: None,
                waste_id: None,
                reconciliation_effect_id: Some(&row.id),
                before: &before,
                after: &e.after,
                seq: sequence,
                time,
            },
        )?;
        row.effect.after.source_id = Some(holding_id.clone());
        connection
            .execute(
                "UPDATE reconciliation_material_effects SET row_json=?1 WHERE id=?2",
                params![serde_json::to_string(&row).map_err(unavailable)?, row.id],
            )
            .map_err(unavailable)?;
        material_results.push(ReconciliationMaterialResult {
            material_id: e.material_id.clone(),
            effect_id: row.id,
            holding_version_id: holding_id,
        });
    }
    let result = ReceiptReconciliation {
        id: id.clone(),
        receipt_id: receipt_id.into(),
        confirmation_id: confirmation_id.into(),
        business_record_id: record_id,
        effect,
        selection_results,
        material_results,
        recorded_at: time.into(),
        seq: sequence.to_string(),
    };
    connection
        .execute(
            "UPDATE receipt_reconciliations SET result_json=?1 WHERE id=?2",
            params![serde_json::to_string(&result).map_err(unavailable)?, id],
        )
        .map_err(unavailable)?;
    Ok(())
}

pub(super) fn result(
    connection: &Connection,
    confirmation_id: &str,
) -> Result<Option<crate::business::BusinessResult>, StoreError> {
    use rusqlite::OptionalExtension;
    connection.query_row("SELECT r.receipt_id,r.id,p.material_id,p.material_version_id,p.order_id,c.received_version_id FROM receipt_reconciliations r JOIN receipts p ON p.id=r.receipt_id LEFT JOIN order_receipt_closures c ON c.receipt_id=p.id WHERE r.confirmation_id=?1",[confirmation_id],|r|Ok(crate::business::BusinessResult::ReconciledReceipt{receipt_id:r.get(0)?,reconciliation_id:r.get(1)?,material_id:r.get(2)?,material_version_id:r.get(3)?,order_id:r.get(4)?,order_version_id:r.get(5)?})).optional().map_err(unavailable)
}

fn equal<T: Serialize>(a: &T, b: &T) -> Result<bool, StoreError> {
    Ok(serde_json::to_value(a).map_err(unavailable)?
        == serde_json::to_value(b).map_err(unavailable)?)
}
fn identifier(id: &str) -> Result<(), StoreError> {
    if Uuid::parse_str(id).map_err(unavailable)?.to_string() != id {
        return Err(unavailable("보정 ID가 올바르지 않습니다"));
    }
    Ok(())
}
pub(super) fn validate_snapshot(s: Validation<'_>) -> Result<(), StoreError> {
    use crate::business::BusinessPayload;
    use crate::receipt::{ReceiptSource, ReceiptStockEffectDraft};
    let bad = || unavailable("입고 보정·출처 예약·가치 재생 근거가 일치하지 않습니다");
    let history = History {
        receipts: s.receipts,
        reconciliations: s.records,
        holdings: s.holdings,
        observations: s.observations,
        observation_rows: s.observation_rows,
        productions: s.productions,
        inputs: s.production_inputs,
        outputs: s.production_outputs,
        wastes: s.wastes,
        materials: s.material_versions,
    };
    let mut ids = HashSet::new();
    let mut used_links = HashSet::new();
    let mut used_versions = HashSet::new();
    let mut used_reservations = HashSet::new();
    let mut used_effects = HashSet::new();
    let mut receipt_ids = HashSet::new();
    for head in s.observation_heads {
        let latest = s
            .observations
            .iter()
            .filter(|v| v.observation_id == head.observation_id)
            .max_by_key(|v| v.seq.parse::<i64>().unwrap_or(0))
            .ok_or_else(bad)?;
        if latest.id != head.current_version_id {
            return Err(bad());
        }
    }
    for r in s.records {
        for id in [
            &r.id,
            &r.receipt_id,
            &r.confirmation_id,
            &r.business_record_id,
        ] {
            identifier(id)?;
        }
        let n = seq(&r.seq)?;
        if n > s.sequence
            || !ids.insert(&r.id)
            || !receipt_ids.insert(&r.receipt_id)
            || chrono::DateTime::parse_from_rfc3339(&r.recorded_at).is_err()
        {
            return Err(bad());
        }
        let receipt = s
            .receipts
            .iter()
            .find(|v| v.id == r.receipt_id)
            .ok_or_else(bad)?;
        let c = s
            .confirmations
            .iter()
            .find(|v| v.id == r.confirmation_id)
            .ok_or_else(bad)?;
        let b = s
            .business_records
            .iter()
            .find(|v| v.id == r.business_record_id)
            .ok_or_else(bad)?;
        let revision = s
            .revisions
            .iter()
            .find(|v| v.id == c.revision_id)
            .ok_or_else(bad)?;
        identifier(&c.request_id)?;
        if receipt.confirmation_id != r.confirmation_id
            || receipt.business_record_id != r.business_record_id
            || receipt.seq != r.seq
            || receipt.recorded_at != r.recorded_at
            || c.record_id != r.business_record_id
            || c.seq != r.seq
            || c.recorded_at != r.recorded_at
            || revision.record_id != b.id
            || b.kind != crate::business::BusinessKind::Receipt
            || !s.mutations.iter().any(|(seq, op, id)| {
                *seq == r.seq && *op == "business.confirm" && *id == c.request_id
            })
        {
            return Err(bad());
        }
        let BusinessPayload::Receipt { version: 1, data } = &revision.payload else {
            return Err(bad());
        };
        let ReceiptStockEffectDraft::Reconciled { selections } = &data.stock_effect else {
            return Err(bad());
        };
        if data.quantity.trim() != receipt.input_quantity
            || data.unit.trim() != receipt.input_unit
            || Money::parse(&data.amount).map_err(unavailable)? != money(&receipt.amount)?
            || data.receipt_date.trim() != receipt.receipt_date
            || data.note != receipt.note
        {
            return Err(bad());
        }
        // Decimal input spellings are preserved in the draft; compare values for amount.
        let m = history.current_material(&receipt.material_id, n - 1)?;
        if m.id != receipt.material_version_id {
            return Err(bad());
        }
        let mut order_before = None;
        let mut order_after = None;
        match (&receipt.source, &data.source) {
            (
                ReceiptSource::Direct {},
                crate::receipt::ReceiptSourceDraft::Direct {
                    material_id,
                    material_version_id,
                },
            ) if material_id.as_deref() == Some(&receipt.material_id)
                && material_version_id.as_deref() == Some(&receipt.material_version_id) => {}
            (
                ReceiptSource::Order {
                    order_id,
                    ordered_version_id,
                },
                crate::receipt::ReceiptSourceDraft::Order {
                    order_id: input_id,
                    expected_order_version_id,
                    material_version_id,
                },
            ) if input_id.as_deref() == Some(order_id)
                && expected_order_version_id.as_deref() == Some(ordered_version_id)
                && material_version_id.as_deref() == Some(&receipt.material_version_id) =>
            {
                let before = s
                    .orders
                    .iter()
                    .find(|o| &o.id == ordered_version_id && &o.order_id == order_id)
                    .ok_or_else(bad)?;
                let first = s
                    .orders
                    .iter()
                    .find(|o| &o.order_id == order_id && o.previous_version_id.is_none())
                    .ok_or_else(bad)?;
                let difference = ValueDelta::from_micros(
                    q(&receipt.quantity)?.micros() - q(&before.quantity)?.micros(),
                )
                .map_err(unavailable)?
                .decimal();
                order_before = Some(Box::new(before.clone()));
                order_after = Some(crate::receipt::OrderReceiptPreview {
                    status: crate::purchase_order::OrderStatus::Received,
                    order_id: order_id.clone(),
                    initial_order_version_id: first.id.clone(),
                    initial_ordered_quantity: first.quantity.clone(),
                    ordered_version_id: before.id.clone(),
                    ordered_quantity: before.quantity.clone(),
                    received_quantity: receipt.quantity.clone(),
                    difference,
                    unit: before.unit.clone(),
                });
            }
            _ => return Err(bad()),
        }
        let expected = calculate(
            &history,
            n - 1,
            ReceiptFacts {
                material: m.clone(),
                quantity: q(&receipt.quantity)?,
                amount: money(&receipt.amount)?,
                input_quantity: receipt.input_quantity.clone(),
                input_unit: receipt.input_unit.clone(),
                receipt_date: receipt.receipt_date.clone(),
                order_before,
                order_after,
            },
            selections,
        )
        .map_err(unavailable)?;
        if !equal(&expected, &r.effect)? {
            return Err(bad());
        }
        let expected_count = r
            .effect
            .selections
            .iter()
            .map(|v| v.portions.len())
            .sum::<usize>();
        if r.selection_results.len() != expected_count {
            return Err(bad());
        }
        for selection in &r.effect.selections {
            for portion in &selection.portions {
                let result = r
                    .selection_results
                    .iter()
                    .find(|l| {
                        l.selection_position == selection.position
                            && l.portion_position == portion.position
                    })
                    .ok_or_else(bad)?;
                let link = s
                    .links
                    .iter()
                    .find(|l| l.id == result.link_id)
                    .ok_or_else(bad)?;
                let version = s
                    .versions
                    .iter()
                    .find(|l| l.id == result.link_version_id)
                    .ok_or_else(bad)?;
                let head = s
                    .heads
                    .iter()
                    .find(|h| h.link_id == link.id)
                    .ok_or_else(bad)?;
                let reservation = s
                    .reservations
                    .iter()
                    .find(|l| l.id == result.reservation_id)
                    .ok_or_else(bad)?;
                for id in [&link.id, &version.id, &reservation.id] {
                    identifier(id)?;
                }
                if !used_links.insert(&link.id)
                    || !used_versions.insert(&version.id)
                    || !used_reservations.insert(&reservation.id)
                    || link.reconciliation_id != r.id
                    || link.selection_position != selection.position
                    || link.portion_position != portion.position
                    || version.link_id != link.id
                    || version.previous_version_id.is_some()
                    || version.observation_version_id != selection.observation_version_id
                    || version.seq != r.seq
                    || version.recorded_at != r.recorded_at
                    || head.current_version_id != version.id
                    || reservation.link_version_id != version.id
                    || reservation.source_key != portion.source_key
                    || reservation.source_version_id != portion.source_version_id
                    || reservation.reserved_source_share != portion.reserved_source_share
                    || !equal(&version.portion, portion)?
                {
                    return Err(bad());
                }
            }
        }
        let changed = r
            .effect
            .materials
            .iter()
            .filter(|m| {
                q(&m.inflow_quantity).is_ok_and(|q| q.micros() != 0)
                    || signed(&m.value_delta).is_ok_and(|v| v != 0)
            })
            .collect::<Vec<_>>();
        if changed.len() != r.material_results.len() {
            return Err(bad());
        }
        for e in changed {
            let result = r
                .material_results
                .iter()
                .find(|v| v.material_id == e.material_id)
                .ok_or_else(bad)?;
            let row = s
                .effects
                .iter()
                .find(|v| v.id == result.effect_id)
                .ok_or_else(bad)?;
            let h = s
                .holdings
                .iter()
                .find(|v| v.id == result.holding_version_id)
                .ok_or_else(bad)?;
            identifier(&row.id)?;
            let mut expected = e.clone();
            expected.after.source_id = Some(h.id.clone());
            if !used_effects.insert(&row.id)
                || row.reconciliation_id != r.id
                || row.seq != r.seq
                || row.recorded_at != r.recorded_at
                || h.reconciliation_effect_id.as_deref() != Some(&row.id)
                || h.material_id != e.material_id
                || h.material_version_id != e.material_version_id
                || h.seq != r.seq
                || h.recorded_at != r.recorded_at
                || h.quantity != e.after.quantity
                || Some(h.value.as_str()) != e.after.value.as_deref()
                || !equal(&expected, &row.effect)?
            {
                return Err(bad());
            }
        }
    }
    if used_links.len() != s.links.len()
        || used_versions.len() != s.versions.len()
        || used_reservations.len() != s.reservations.len()
        || used_effects.len() != s.effects.len()
        || s.heads.len() != s.links.len()
    {
        return Err(bad());
    }
    for receipt in s.receipts {
        let c = s
            .confirmations
            .iter()
            .find(|c| c.id == receipt.confirmation_id)
            .ok_or_else(bad)?;
        let revision = s
            .revisions
            .iter()
            .find(|v| v.id == c.revision_id)
            .ok_or_else(bad)?;
        let is_reconciled = matches!(&revision.payload,BusinessPayload::Receipt{data,..} if matches!(data.stock_effect,ReceiptStockEffectDraft::Reconciled{..}));
        if is_reconciled != receipt_ids.contains(&receipt.id) {
            return Err(bad());
        }
    }
    // Empty reconciliations still pass every relation/count/confirmed-tag check
    // above. Existing validators already cover ordinary history in old schemas.
    if !s.records.is_empty() {
        replay_receipt_values(
            &history.value_input(s.sequence, history.events(s.sequence)?)?,
            &[],
        )
        .map_err(unavailable)?;
        resolve_inclusions(
            &history.events(s.sequence)?,
            &history.reserved(s.sequence)?,
            &[],
        )
        .map_err(unavailable)?;
    }
    Ok(())
}

pub(super) fn validate_physical_columns(connection: &Connection) -> Result<(), StoreError> {
    // The JSON is a typed immutable projection, not an alternative authority
    // for relational keys. Detect disagreement before returning any records.
    for (table, column, fields) in [
        (
            "receipt_reconciliations",
            "result_json",
            &[
                ("id", "id"),
                ("receipt_id", "receipt_id"),
                ("confirmation_id", "confirmation_id"),
                ("recorded_at", "recorded_at"),
                ("sequence", "seq"),
            ][..],
        ),
        (
            "inclusion_links",
            "row_json",
            &[
                ("id", "id"),
                ("reconciliation_id", "reconciliation_id"),
                ("selection_position", "selection_position"),
                ("portion_position", "portion_position"),
            ][..],
        ),
        (
            "inclusion_link_versions",
            "row_json",
            &[
                ("id", "id"),
                ("link_id", "link_id"),
                ("previous_version_id", "previous_version_id"),
                ("observation_version_id", "observation_version_id"),
                ("recorded_at", "recorded_at"),
                ("sequence", "seq"),
            ][..],
        ),
        (
            "inclusion_reservations",
            "row_json",
            &[("id", "id"), ("link_version_id", "link_version_id")][..],
        ),
        (
            "reconciliation_material_effects",
            "row_json",
            &[
                ("id", "id"),
                ("reconciliation_id", "reconciliation_id"),
                ("material_id", "effect.material_id"),
                ("material_version_id", "effect.material_version_id"),
                ("before_holding_version_id", "effect.before.source_id"),
                ("recorded_at", "recorded_at"),
                ("sequence", "seq"),
            ][..],
        ),
    ] {
        let predicates = fields
            .iter()
            .map(|(sql, json)| {
                format!(
                    "CAST({sql} AS TEXT) IS NOT CAST(json_extract({column}, '$.{json}') AS TEXT)"
                )
            })
            .collect::<Vec<_>>()
            .join(" OR ");
        let count: i64 = connection
            .query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE {predicates}"),
                [],
                |r| r.get(0),
            )
            .map_err(unavailable)?;
        if count != 0 {
            return Err(unavailable("입고 보정 관계와 저장 원문이 다릅니다"));
        }
    }
    Ok(())
}

pub(super) fn validate_reconciliation_schema(connection: &Connection) -> anyhow::Result<()> {
    validate_physical_columns(connection)?;
    let rows = snapshot_rows(connection)?;
    let loaded = Loaded::read(connection)?;
    let business = business::snapshot_rows(connection)?;
    let mutations = connection
        .prepare("SELECT sequence,operation,request_id FROM mutation_requests ORDER BY sequence")?
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?.to_string(),
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let sequence = connection.query_row(
        "SELECT COALESCE(MAX(sequence),0) FROM snapshot_outbox",
        [],
        |r| r.get(0),
    )?;
    validate_snapshot(Validation {
        records: &rows.records,
        links: &rows.links,
        versions: &rows.versions,
        heads: &rows.heads,
        reservations: &rows.reservations,
        effects: &rows.effects,
        receipts: &loaded.receipts,
        holdings: &loaded.holdings,
        observations: &loaded.stocktake.versions,
        observation_rows: &loaded.stocktake.observations,
        observation_heads: &loaded.stocktake.heads,
        productions: &loaded.production.records,
        production_inputs: &loaded.production.inputs,
        production_outputs: &loaded.production.outputs,
        wastes: &loaded.wastes,
        material_versions: &loaded.materials,
        orders: &super::purchase_order::read_versions(connection, None, None, false)?,
        business_records: &business.records,
        revisions: &business.revisions,
        confirmations: &business.confirmations,
        mutations: &mutations
            .iter()
            .map(|(s, o, r)| (s.as_str(), o.as_str(), r.as_str()))
            .collect::<Vec<_>>(),
        sequence,
    })?;
    Ok(())
}
