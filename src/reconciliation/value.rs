use super::{ReconciliationTraceKind, ReconciliationValueTrace, SourceKey};
use crate::{
    domain::{DomainError, Money, Quantity, Unit, allocate_largest_remainder, round_ratio},
    production::AllocationPercent,
};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiptAmountSplit {
    pub included_quantity: Quantity,
    pub unreflected_quantity: Quantity,
    pub included_amount: Money,
    pub unreflected_amount: Money,
    pub selection_amounts: Vec<Money>,
}

pub fn split_receipt_amount(
    received: Quantity,
    amount: Money,
    included: &[Quantity],
) -> Result<ReceiptAmountSplit, DomainError> {
    if received.micros() == 0 || included.len() > 20 {
        return Err(DomainError(
            "실입고는 양수이며 포함 선택은 20행 이내여야 합니다".into(),
        ));
    }
    let total = included.iter().try_fold(0i64, |sum, q| {
        sum.checked_add(q.micros())
            .filter(|sum| *sum <= received.micros())
            .ok_or_else(|| DomainError("포함 수량 합계가 실입고 수량을 넘습니다".into()))
    })?;
    let unreflected = received.micros() - total;
    let amounts = allocate_largest_remainder(amount.micros(), &[total, unreflected])?;
    let selections = if total == 0 {
        vec![0; included.len()]
    } else {
        allocate_largest_remainder(
            amounts[0],
            &included.iter().map(|q| q.micros()).collect::<Vec<_>>(),
        )?
    };
    Ok(ReceiptAmountSplit {
        included_quantity: Quantity::from_micros(total)?,
        unreflected_quantity: Quantity::from_micros(unreflected)?,
        included_amount: Money::from_micros(amounts[0])?,
        unreflected_amount: Money::from_micros(amounts[1])?,
        selection_amounts: selections
            .into_iter()
            .map(Money::from_micros)
            .collect::<Result<_, _>>()?,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueDelta(i64);
impl ValueDelta {
    pub fn from_micros(value: i64) -> Result<Self, DomainError> {
        if value.unsigned_abs() > 9_000_000_000_000_000_000 {
            return Err(DomainError("보정 금액이 허용 범위를 넘습니다".into()));
        }
        Ok(Self(value))
    }
    pub fn micros(self) -> i64 {
        self.0
    }
    pub fn decimal(self) -> String {
        let magnitude = Money::from_micros(self.0.abs())
            .expect("validated signed money")
            .decimal();
        if self.0 < 0 {
            format!("-{magnitude}")
        } else {
            magnitude
        }
    }
    fn checked(value: i128) -> Result<Self, ValueReplayError> {
        Self::from_micros(i64::try_from(value).map_err(|_| range())?).map_err(Into::into)
    }
}

#[derive(Debug, Clone)]
pub struct ValueMaterial {
    pub material_id: String,
    pub material_version_id: String,
    pub unit: Unit,
}
#[derive(Debug, Clone)]
pub struct ValueEventRef {
    pub id: String,
    pub seq: i64,
}
#[derive(Debug, Clone)]
pub struct ValueProductionInput {
    pub position: usize,
    pub material: ValueMaterial,
    pub quantity: Quantity,
}
#[derive(Debug, Clone)]
pub struct ValueProductionOutput {
    pub position: usize,
    pub material: ValueMaterial,
    pub quantity: Quantity,
    pub allocation: AllocationPercent,
}
#[derive(Debug, Clone)]
pub enum ValueObservation {
    Initial {
        value: Money,
    },
    Settlement {
        consumption: Quantity,
        excess: Quantity,
        excess_value: Money,
    },
}
#[derive(Debug, Clone)]
pub enum ValueReplayEvent {
    Inflow {
        event: ValueEventRef,
        material: ValueMaterial,
        quantity: Quantity,
        amount: Money,
    },
    Observation {
        event: ValueEventRef,
        observation_id: String,
        material: ValueMaterial,
        measured: Quantity,
        calculation: ValueObservation,
    },
    Production {
        event: ValueEventRef,
        inputs: Vec<ValueProductionInput>,
        outputs: Vec<ValueProductionOutput>,
    },
    Waste {
        event: ValueEventRef,
        material: ValueMaterial,
        quantity: Quantity,
    },
}
impl ValueReplayEvent {
    pub fn event(&self) -> &ValueEventRef {
        match self {
            Self::Inflow { event, .. }
            | Self::Observation { event, .. }
            | Self::Production { event, .. }
            | Self::Waste { event, .. } => event,
        }
    }
}
#[derive(Debug, Clone)]
pub struct ValueReplacement {
    pub observation_version_id: String,
    pub material_id: String,
    pub source_key: SourceKey,
    pub selection_position: usize,
    pub portion_position: usize,
    pub booked_value: Money,
    pub actual_included_value: Money,
}
#[derive(Debug, Clone)]
pub struct EffectiveValueReplacement {
    pub link_id: String,
    pub link_version_id: String,
    pub value: ValueReplacement,
}
#[derive(Debug, Clone)]
pub struct ValueActualHead {
    pub material: ValueMaterial,
    pub quantity: Quantity,
    pub value: Money,
    pub holding_version_id: String,
}
#[derive(Debug, Clone)]
pub struct ValueReplayInput {
    pub at_revision: i64,
    pub events: Vec<ValueReplayEvent>,
    pub effective: Vec<EffectiveValueReplacement>,
    pub actual_heads: Vec<ValueActualHead>,
}
#[derive(Debug, Clone)]
pub struct ValueMaterialDelta {
    pub material_id: String,
    pub quantity: Quantity,
    pub before_value: Money,
    pub after_value: Money,
    pub delta: ValueDelta,
}
#[derive(Debug, Clone)]
pub struct ValueReplayDelta {
    pub materials: Vec<ValueMaterialDelta>,
    pub value_difference: ValueDelta,
    pub separate_adjustment: ValueDelta,
    pub trace: Vec<ReconciliationValueTrace>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueReplayError {
    InvalidReference(String),
    InvalidHistory(String),
    OutOfRange(String),
    NegativeValue(String),
}
impl fmt::Display for ValueReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidReference(s)
            | Self::InvalidHistory(s)
            | Self::OutOfRange(s)
            | Self::NegativeValue(s) => f.write_str(s),
        }
    }
}
impl std::error::Error for ValueReplayError {}
impl From<DomainError> for ValueReplayError {
    fn from(error: DomainError) -> Self {
        Self::OutOfRange(error.0)
    }
}
fn range() -> ValueReplayError {
    ValueReplayError::OutOfRange("가치 재생 계산이 허용 범위를 넘습니다".into())
}
fn invalid() -> ValueReplayError {
    ValueReplayError::InvalidHistory("가치 재생의 확정 수량·순서·원천이 일치하지 않습니다".into())
}
fn reference() -> ValueReplayError {
    ValueReplayError::InvalidReference("가치 보정의 관측·품목·연결 참조가 올바르지 않습니다".into())
}
fn identifier(value: &str) -> Result<(), ValueReplayError> {
    if uuid::Uuid::parse_str(value)
        .map_err(|_| reference())?
        .to_string()
        != value
    {
        return Err(reference());
    }
    Ok(())
}
fn sum(left: i128, right: i128) -> Result<i128, ValueReplayError> {
    left.checked_add(right).ok_or_else(range)
}
fn money(value: i128) -> Result<i64, ValueReplayError> {
    if value < 0 {
        return Err(ValueReplayError::NegativeValue(
            "보정 후 가치가 음수가 됩니다. 포함 근거와 이전 보정을 확인하세요".into(),
        ));
    }
    Ok(Money::from_micros(i64::try_from(value).map_err(|_| range())?)?.micros())
}
fn quantity(value: i128) -> Result<i64, ValueReplayError> {
    Ok(Quantity::from_micros(i64::try_from(value).map_err(|_| range())?)?.micros())
}
fn validate_material(material: &ValueMaterial) -> Result<(), ValueReplayError> {
    identifier(&material.material_id)?;
    identifier(&material.material_version_id)?;
    if material
        .unit
        .to_canonical(Quantity::from_micros(1)?)?
        .micros()
        != 1
    {
        return Err(reference());
    }
    Ok(())
}

#[derive(Clone, Default)]
struct Amounts {
    q: i64,
    v: i64,
}
#[derive(Clone)]
struct LedgerState {
    amounts: Amounts,
    unit: String,
}
struct Step {
    kind: ReconciliationTraceKind,
    event: ValueEventRef,
    material_id: String,
    before: Amounts,
    after: Amounts,
    closed: i64,
}
#[derive(Default)]
struct Engine {
    current: BTreeMap<String, LedgerState>,
    steps: Vec<Step>,
    closed: i128,
}
impl Engine {
    fn before(&self, material: &ValueMaterial) -> Result<Amounts, ValueReplayError> {
        validate_material(material)?;
        match self.current.get(&material.material_id) {
            Some(state) if state.unit != material.unit.canonical() => Err(reference()),
            Some(state) => Ok(state.amounts.clone()),
            None => Ok(Amounts::default()),
        }
    }
    fn write(
        &mut self,
        kind: ReconciliationTraceKind,
        event: &ValueEventRef,
        material: &ValueMaterial,
        before: Amounts,
        next: (i128, i128),
        closed: i64,
    ) -> Result<(), ValueReplayError> {
        let after = Amounts {
            q: quantity(next.0)?,
            v: money(next.1)?,
        };
        if after.q == 0 && after.v != 0 {
            return Err(invalid());
        }
        self.closed = sum(self.closed, closed as i128)?;
        self.current.insert(
            material.material_id.clone(),
            LedgerState {
                amounts: after.clone(),
                unit: material.unit.canonical().into(),
            },
        );
        self.steps.push(Step {
            kind,
            event: event.clone(),
            material_id: material.material_id.clone(),
            before,
            after,
            closed,
        });
        Ok(())
    }
}

fn take_value(before: &Amounts, q: i64) -> Result<i64, ValueReplayError> {
    if q < 0 || q > before.q {
        return Err(invalid());
    }
    if q == 0 {
        return Ok(0);
    }
    if q == before.q {
        return Ok(before.v);
    }
    money(round_ratio(before.v as i128 * q as i128, before.q as i128)?)
}

fn run(
    events: &[&ValueReplayEvent],
    replacements: &[&ValueReplacement],
) -> Result<Engine, ValueReplayError> {
    let mut adjustments = HashMap::<&str, i128>::new();
    for replacement in replacements {
        let current = adjustments
            .entry(&replacement.observation_version_id)
            .or_default();
        *current = sum(
            *current,
            replacement.actual_included_value.micros() as i128
                - replacement.booked_value.micros() as i128,
        )?;
    }
    let mut engine = Engine::default();
    for source in events {
        match source {
            ValueReplayEvent::Inflow {
                event,
                material,
                quantity: q,
                amount,
            } => {
                if q.micros() == 0 {
                    return Err(invalid());
                }
                let before = engine.before(material)?;
                engine.write(
                    ReconciliationTraceKind::Inflow,
                    event,
                    material,
                    before.clone(),
                    (
                        before.q as i128 + q.micros() as i128,
                        before.v as i128 + amount.micros() as i128,
                    ),
                    0,
                )?;
            }
            ValueReplayEvent::Observation {
                event,
                observation_id,
                material,
                measured,
                calculation,
            } => {
                identifier(observation_id)?;
                let before = engine.before(material)?;
                let (next, closed) = match calculation {
                    ValueObservation::Initial { value } => {
                        if engine.current.contains_key(&material.material_id) {
                            return Err(invalid());
                        }
                        (value.micros() as i128, 0)
                    }
                    ValueObservation::Settlement {
                        consumption,
                        excess,
                        excess_value,
                    } => {
                        if !engine.current.contains_key(&material.material_id)
                            || (consumption.micros() > 0 && excess.micros() > 0)
                            || (excess.micros() == 0 && excess_value.micros() != 0)
                            || before.q as i128 - consumption.micros() as i128
                                + excess.micros() as i128
                                != measured.micros() as i128
                        {
                            return Err(invalid());
                        }
                        let closed = take_value(&before, consumption.micros())?;
                        (
                            before.v as i128 - closed as i128 + excess_value.micros() as i128,
                            closed,
                        )
                    }
                };
                let next = sum(
                    next,
                    adjustments.get(event.id.as_str()).copied().unwrap_or(0),
                )?;
                engine.write(
                    ReconciliationTraceKind::Observation,
                    event,
                    material,
                    before,
                    (measured.micros() as i128, next),
                    closed,
                )?;
            }
            ValueReplayEvent::Waste {
                event,
                material,
                quantity: q,
            } => {
                let before = engine.before(material)?;
                if q.micros() == 0 || q.micros() > before.q {
                    return Err(invalid());
                }
                let closed = if q.micros() == before.q { before.v } else { 0 };
                engine.write(
                    ReconciliationTraceKind::Waste,
                    event,
                    material,
                    before.clone(),
                    (
                        before.q as i128 - q.micros() as i128,
                        before.v as i128 - closed as i128,
                    ),
                    closed,
                )?;
            }
            ValueReplayEvent::Production {
                event,
                inputs,
                outputs,
            } => production(&mut engine, event, inputs, outputs)?,
        }
    }
    Ok(engine)
}

#[derive(Default)]
struct ProductionChange {
    input: i64,
    input_value: i64,
    output: i64,
    output_value: i64,
}
fn production(
    engine: &mut Engine,
    event: &ValueEventRef,
    inputs: &[ValueProductionInput],
    outputs: &[ValueProductionOutput],
) -> Result<(), ValueReplayError> {
    if inputs.is_empty() || outputs.is_empty() || inputs.len() > 100 || outputs.len() > 100 {
        return Err(invalid());
    }
    let mut materials = BTreeMap::<String, &ValueMaterial>::new();
    let mut changes = BTreeMap::<String, ProductionChange>::new();
    for (position, row) in inputs.iter().enumerate() {
        validate_material(&row.material)?;
        if position != row.position || row.quantity.micros() == 0 {
            return Err(invalid());
        }
        if let Some(prior) = materials.insert(row.material.material_id.clone(), &row.material)
            && (prior.material_version_id != row.material.material_version_id
                || prior.unit != row.material.unit)
        {
            return Err(reference());
        }
        let change = changes.entry(row.material.material_id.clone()).or_default();
        change.input = quantity(change.input as i128 + row.quantity.micros() as i128)?;
    }
    let mut total = 0i64;
    for (id, change) in &mut changes {
        change.input_value = take_value(&engine.before(materials[id])?, change.input)?;
        total = money(total as i128 + change.input_value as i128)?;
    }
    let weights = outputs
        .iter()
        .map(|row| row.allocation.micros())
        .collect::<Vec<_>>();
    if weights.iter().map(|v| *v as i128).sum::<i128>() != 100_000_000 {
        return Err(invalid());
    }
    let allocated = allocate_largest_remainder(total, &weights)?;
    for (position, row) in outputs.iter().enumerate() {
        validate_material(&row.material)?;
        if position != row.position || row.quantity.micros() == 0 {
            return Err(invalid());
        }
        if let Some(prior) = materials.insert(row.material.material_id.clone(), &row.material)
            && (prior.material_version_id != row.material.material_version_id
                || prior.unit != row.material.unit)
        {
            return Err(reference());
        }
        let change = changes.entry(row.material.material_id.clone()).or_default();
        change.output = quantity(change.output as i128 + row.quantity.micros() as i128)?;
        change.output_value = money(change.output_value as i128 + allocated[position] as i128)?;
    }
    for (id, change) in changes {
        let material = materials[&id];
        let before = engine.before(material)?;
        engine.write(
            ReconciliationTraceKind::Production,
            event,
            material,
            before.clone(),
            (
                before.q as i128 - change.input as i128 + change.output as i128,
                before.v as i128 - change.input_value as i128 + change.output_value as i128,
            ),
            0,
        )?;
    }
    Ok(())
}

fn validate_replacement(
    value: &ValueReplacement,
    anchors: &HashMap<&str, &str>,
) -> Result<(), ValueReplayError> {
    for id in [
        &value.observation_version_id,
        &value.material_id,
        &value.source_key.id,
    ] {
        identifier(id)?;
    }
    if value.source_key.position != 0
        || anchors.get(value.observation_version_id.as_str()).copied()
            != Some(value.material_id.as_str())
    {
        return Err(reference());
    }
    Ok(())
}

/// Existing replacements are applied at their observation anchors. Their
/// materialized current deltas must never also appear in `events`.
pub fn replay_receipt_values(
    history: &ValueReplayInput,
    proposed: &[ValueReplacement],
) -> Result<ValueReplayDelta, ValueReplayError> {
    if history.at_revision < 0 {
        return Err(invalid());
    }
    let mut events = history.events.iter().collect::<Vec<_>>();
    events.sort_by(|a, b| {
        a.event()
            .seq
            .cmp(&b.event().seq)
            .then_with(|| a.event().id.cmp(&b.event().id))
    });
    let mut event_ids = HashSet::new();
    let mut anchors = HashMap::new();
    for source in &events {
        let event = source.event();
        identifier(&event.id)?;
        if event.seq <= 0 || event.seq > history.at_revision || !event_ids.insert(&event.id) {
            return Err(invalid());
        }
        if let ValueReplayEvent::Observation { material, .. } = source {
            anchors.insert(event.id.as_str(), material.material_id.as_str());
        }
    }
    let mut links = HashSet::new();
    let mut versions = HashSet::new();
    for old in &history.effective {
        identifier(&old.link_id)?;
        identifier(&old.link_version_id)?;
        if !links.insert(&old.link_id) || !versions.insert(&old.link_version_id) {
            return Err(reference());
        }
        validate_replacement(&old.value, &anchors)?;
    }
    let mut positions = HashSet::new();
    for value in proposed {
        validate_replacement(value, &anchors)?;
        if !positions.insert((value.selection_position, value.portion_position)) {
            return Err(reference());
        }
    }
    let old = history
        .effective
        .iter()
        .map(|link| &link.value)
        .collect::<Vec<_>>();
    let base = run(&events, &old).map_err(|e| {
        ValueReplayError::InvalidHistory(format!("기존 유효 원장을 재생할 수 없습니다: {e}"))
    })?;
    if base.current.len() != history.actual_heads.len() {
        return Err(invalid());
    }
    let mut head_ids = HashSet::new();
    for head in &history.actual_heads {
        validate_material(&head.material)?;
        identifier(&head.holding_version_id)?;
        if !head_ids.insert(&head.material.material_id) {
            return Err(invalid());
        }
        let actual = base
            .current
            .get(&head.material.material_id)
            .ok_or_else(invalid)?;
        if actual.unit != head.material.unit.canonical()
            || actual.amounts.q != head.quantity.micros()
            || actual.amounts.v != head.value.micros()
        {
            return Err(ValueReplayError::InvalidHistory(
                "기존 유효 가치 재생이 현재 보유 수량/가치와 다릅니다".into(),
            ));
        }
    }
    let mut all = old;
    all.extend(proposed);
    let next = run(&events, &all)?;
    let mut difference = 0i128;
    for replacement in proposed {
        difference = sum(
            difference,
            replacement.actual_included_value.micros() as i128
                - replacement.booked_value.micros() as i128,
        )?;
    }
    let mut current_delta = 0i128;
    let mut materials = Vec::with_capacity(base.current.len());
    for (id, before) in &base.current {
        let after = next.current.get(id).ok_or_else(invalid)?;
        if before.amounts.q != after.amounts.q {
            return Err(invalid());
        }
        let delta = after.amounts.v as i128 - before.amounts.v as i128;
        current_delta = sum(current_delta, delta)?;
        materials.push(ValueMaterialDelta {
            material_id: id.clone(),
            quantity: Quantity::from_micros(before.amounts.q)?,
            before_value: Money::from_micros(before.amounts.v)?,
            after_value: Money::from_micros(after.amounts.v)?,
            delta: ValueDelta::checked(delta)?,
        });
    }
    let separate = difference.checked_sub(current_delta).ok_or_else(range)?;
    if next.closed.checked_sub(base.closed).ok_or_else(range)? != separate
        || base.steps.len() != next.steps.len()
    {
        return Err(invalid());
    }
    let selected = proposed
        .iter()
        .map(|r| r.observation_version_id.as_str())
        .collect::<HashSet<_>>();
    let mut trace = Vec::new();
    for (left, right) in base.steps.iter().zip(&next.steps) {
        if left.event.id != right.event.id
            || left.material_id != right.material_id
            || left.before.q != right.before.q
            || left.after.q != right.after.q
        {
            return Err(invalid());
        }
        if left.before.v == right.before.v
            && left.after.v == right.after.v
            && left.closed == right.closed
            && !selected.contains(left.event.id.as_str())
        {
            continue;
        }
        trace.push(ReconciliationValueTrace {
            kind: left.kind,
            event_id: left.event.id.clone(),
            seq: left.event.seq.to_string(),
            material_id: left.material_id.clone(),
            quantity_before: Quantity::from_micros(left.before.q)?.decimal(),
            quantity_after: Quantity::from_micros(left.after.q)?.decimal(),
            base_value_before: Money::from_micros(left.before.v)?.decimal(),
            base_value_after: Money::from_micros(left.after.v)?.decimal(),
            proposed_value_before: Money::from_micros(right.before.v)?.decimal(),
            proposed_value_after: Money::from_micros(right.after.v)?.decimal(),
            base_closed_value: Money::from_micros(left.closed)?.decimal(),
            proposed_closed_value: Money::from_micros(right.closed)?.decimal(),
        });
    }
    Ok(ValueReplayDelta {
        materials,
        value_difference: ValueDelta::checked(difference)?,
        separate_adjustment: ValueDelta::checked(separate)?,
        trace,
    })
}
