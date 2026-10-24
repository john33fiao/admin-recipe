//! Exact, non-physical source shares used only to prevent double inclusion.
//! Business quantities and money retain their existing fixed-point contracts.
use super::{
    InclusionSourceSelection, SourceKey, SourceKind, SourceShare, ValueMaterial, ValueObservation,
    ValueReplayEvent,
};
use crate::domain::{Money, Quantity};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};
use std::collections::{BTreeMap, BTreeSet};

type Ratio = BigRational;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenanceError {
    InvalidReference(String),
    InvalidHistory(String),
    InsufficientShare(String),
}
impl std::fmt::Display for ProvenanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidReference(s) | Self::InvalidHistory(s) | Self::InsufficientShare(s) => {
                f.write_str(s)
            }
        }
    }
}
impl std::error::Error for ProvenanceError {}
type Result<T> = std::result::Result<T, ProvenanceError>;
fn history(message: &str) -> ProvenanceError {
    ProvenanceError::InvalidHistory(message.into())
}
fn unavailable() -> ProvenanceError {
    ProvenanceError::InsufficientShare(
        "선택한 관측의 미연결 수량 또는 원래 연결 한도를 초과합니다".into(),
    )
}
fn integer(value: i64) -> Ratio {
    Ratio::from_integer(BigInt::from(value))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenancePortion {
    pub source_key: SourceKey,
    pub source_version_id: String,
    pub included_share: SourceShare,
    pub reserved_source_share: SourceShare,
}
#[derive(Debug, Clone)]
pub struct ReservedInclusion {
    pub id: String,
    pub observation_version_id: String,
    pub material_id: String,
    pub quantity: Quantity,
    pub portions: Vec<ProvenancePortion>,
}
#[derive(Debug, Clone)]
pub struct InclusionRequest {
    pub position: usize,
    pub observation_version_id: String,
    pub material_id: String,
    pub source: InclusionSourceSelection,
    pub quantity: Quantity,
}
#[derive(Debug, Clone)]
pub struct ResolvedInclusion {
    pub position: usize,
    pub portions: Vec<ProvenancePortion>,
}
#[derive(Debug, Clone)]
pub struct ProvenanceSource {
    pub source_key: SourceKey,
    pub source_version_id: String,
    pub material_id: String,
    pub unit: String,
    pub quantity: Quantity,
    pub reserved_share: SourceShare,
    pub remaining_share: SourceShare,
    pub available_share: SourceShare,
    pub available_quantity: Quantity,
}
#[derive(Debug, Clone)]
pub struct ProvenanceObservation {
    pub observation_version_id: String,
    pub material_id: String,
    pub available_share: SourceShare,
    pub available_quantity: Quantity,
    pub sources: Vec<ProvenanceSource>,
}

/// Exact public shares are expressed in canonical units, not micro-units.
fn share(micros: &Ratio, unit: &str) -> SourceShare {
    let units = micros / integer(1_000_000);
    SourceShare {
        numerator: units.numer().to_string(),
        denominator: units.denom().to_string(),
        unit: unit.into(),
    }
}
fn parse_share(value: &SourceShare, unit: &str) -> Result<Ratio> {
    if value.unit != unit
        || value.numerator.is_empty()
        || value.denominator.is_empty()
        || !value.numerator.bytes().all(|b| b.is_ascii_digit())
        || !value.denominator.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(history("포함 몫의 단위 또는 분수가 올바르지 않습니다"));
    }
    let n: BigInt = value
        .numerator
        .parse()
        .map_err(|_| history("잘못된 포함 분자"))?;
    let d: BigInt = value
        .denominator
        .parse()
        .map_err(|_| history("잘못된 포함 분모"))?;
    if d <= BigInt::zero() {
        return Err(history("포함 몫의 분모는 양수여야 합니다"));
    }
    let ratio = Ratio::new(n, d);
    if ratio.numer().to_string() != value.numerator
        || ratio.denom().to_string() != value.denominator
    {
        return Err(history("포함 몫은 정규화된 기약분수여야 합니다"));
    }
    Ok(ratio * integer(1_000_000))
}

/// Deterministic largest-remainder allocation over exact shares. Input order
/// is the original source order, never a floating-point sort key.
pub fn allocate_share_amount(total: Money, shares: &[SourceShare]) -> Result<Vec<Money>> {
    if shares.is_empty() {
        return if total.micros() == 0 {
            Ok(vec![])
        } else {
            Err(history("금액 배분 근거가 없습니다"))
        };
    }
    let unit = &shares[0].unit;
    let weights: Vec<_> = shares
        .iter()
        .map(|s| parse_share(s, unit))
        .collect::<Result<_>>()?;
    let sum: Ratio = weights.iter().cloned().sum();
    if sum.is_zero() {
        return if total.micros() == 0 {
            Ok(vec![
                Money::from_micros(0)
                    .map_err(|_| history("금액 범위"))?;
                shares.len()
            ])
        } else {
            Err(history("금액 배분 가중치가 없습니다"))
        };
    }
    let mut values = Vec::with_capacity(weights.len());
    let mut remainders = Vec::with_capacity(weights.len());
    let mut assigned = 0_i64;
    for (position, weight) in weights.into_iter().enumerate() {
        let exact = integer(total.micros()) * weight / &sum;
        let floor = exact
            .to_integer()
            .to_i64()
            .ok_or_else(|| history("배분 금액 범위"))?;
        assigned = assigned
            .checked_add(floor)
            .ok_or_else(|| history("배분 합계 범위"))?;
        remainders.push((position, exact - integer(floor)));
        values.push(floor);
    }
    let rest = usize::try_from(total.micros() - assigned).map_err(|_| history("배분 잔여 범위"))?;
    if rest > values.len() {
        return Err(history("배분 잔여 불일치"));
    }
    remainders.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (position, _) in remainders.into_iter().take(rest) {
        values[position] += 1;
    }
    values
        .into_iter()
        .map(|v| Money::from_micros(v).map_err(|_| history("배분 금액 범위")))
        .collect()
}

#[derive(Clone, Debug)]
struct Source {
    key: SourceKey,
    version_id: String,
    material_id: String,
    unit: String,
    quantity: i64,
    reserved: Ratio,
    probe_reserved: Ratio,
}
#[derive(Clone, Debug, Default)]
struct Layer {
    local: Ratio,
    origin: Ratio,
    free_local: Ratio,
    free_origin: Ratio,
    // A probe removes coefficient * x from free shares without changing the
    // unchanged replay. All subsequent quantity operations are affine in x.
    probe_local: Ratio,
    probe_origin: Ratio,
}
impl Layer {
    fn scaled(&self, ratio: &Ratio) -> Self {
        Self {
            local: &self.local * ratio,
            origin: &self.origin * ratio,
            free_local: &self.free_local * ratio,
            free_origin: &self.free_origin * ratio,
            probe_local: &self.probe_local * ratio,
            probe_origin: &self.probe_origin * ratio,
        }
    }
    fn add(&mut self, other: Self) {
        self.local += other.local;
        self.origin += other.origin;
        self.free_local += other.free_local;
        self.free_origin += other.free_origin;
        self.probe_local += other.probe_local;
        self.probe_origin += other.probe_origin;
    }
}
#[derive(Clone, Debug)]
struct Stock {
    unit: String,
    quantity: i64,
    layers: BTreeMap<usize, Layer>,
}
impl Stock {
    fn new(unit: &str) -> Self {
        Self {
            unit: unit.into(),
            quantity: 0,
            layers: BTreeMap::new(),
        }
    }
    fn validate(&self) -> Result<()> {
        let mut total = Ratio::zero();
        for layer in self.layers.values() {
            if layer.local < Ratio::zero()
                || layer.origin < Ratio::zero()
                || layer.free_local < Ratio::zero()
                || layer.free_origin < Ratio::zero()
                || layer.free_local > layer.local
                || layer.free_origin > layer.origin
                || (layer.free_local.is_zero() != layer.free_origin.is_zero())
                || (layer.local.is_zero() != layer.origin.is_zero())
            {
                return Err(history("포함 출처의 수량 지분이 일치하지 않습니다"));
            }
            total += &layer.local;
        }
        if total != integer(self.quantity) {
            return Err(history("보유 수량과 출처 지분 합계가 다릅니다"));
        }
        Ok(())
    }
    fn take(&mut self, quantity: i64) -> Result<BTreeMap<usize, Layer>> {
        if quantity < 0 || quantity > self.quantity {
            return Err(history("출처 재생의 차감 수량이 보유량을 넘습니다"));
        }
        if quantity == 0 {
            return Ok(BTreeMap::new());
        }
        let ratio = integer(quantity) / integer(self.quantity);
        let rest = Ratio::one() - &ratio;
        let taken = self
            .layers
            .iter()
            .map(|(key, l)| (*key, l.scaled(&ratio)))
            .collect();
        for layer in self.layers.values_mut() {
            *layer = layer.scaled(&rest);
        }
        self.layers.retain(|_, l| !l.local.is_zero());
        self.quantity -= quantity;
        Ok(taken)
    }
}
#[derive(Clone)]
struct ObservationView {
    id: String,
    material_id: String,
    stock: Stock,
}
struct Replay {
    sources: Vec<Source>,
    observations: Vec<ObservationView>,
    probe_cap: Option<Ratio>,
}

#[derive(Clone)]
struct ProbeRequest {
    observation_version_id: String,
    material_id: String,
    source_key: SourceKey,
}
struct ProbeState {
    request: ProbeRequest,
    cap: Option<Ratio>,
}

fn restrict(probe: &mut Option<ProbeState>, remaining: Ratio, coefficient: &Ratio) -> Result<()> {
    let Some(cap) = probe.as_mut().and_then(|state| state.cap.as_mut()) else {
        return Ok(());
    };
    if remaining < Ratio::zero() || coefficient < &Ratio::zero() {
        return Err(history("포함 상한 계산의 잔량 또는 계수가 음수입니다"));
    }
    if !coefficient.is_zero() {
        *cap = cap.clone().min(remaining / coefficient);
    }
    Ok(())
}

fn restrict_all(
    probe: &mut Option<ProbeState>,
    stocks: &BTreeMap<String, Stock>,
    sources: &[Source],
) -> Result<()> {
    for stock in stocks.values() {
        for layer in stock.layers.values() {
            restrict(probe, layer.free_local.clone(), &layer.probe_local)?;
            restrict(probe, layer.free_origin.clone(), &layer.probe_origin)?;
        }
    }
    for source in sources {
        restrict(
            probe,
            integer(source.quantity) - &source.reserved,
            &source.probe_reserved,
        )?;
    }
    Ok(())
}

fn material_stock<'a>(
    stocks: &'a mut BTreeMap<String, Stock>,
    material: &ValueMaterial,
) -> Result<&'a mut Stock> {
    let value = stocks
        .entry(material.material_id.clone())
        .or_insert_with(|| Stock::new(material.unit.canonical()));
    if value.unit != material.unit.canonical() {
        return Err(history("품목의 기준 단위가 변경되었습니다"));
    }
    Ok(value)
}
fn inflow(
    stocks: &mut BTreeMap<String, Stock>,
    sources: &mut Vec<Source>,
    material: &ValueMaterial,
    quantity: i64,
    kind: SourceKind,
    id: &str,
    version_id: &str,
) -> Result<()> {
    if quantity < 0 {
        return Err(history("음수 출처 수량"));
    }
    let stock = material_stock(stocks, material)?;
    if quantity == 0 {
        return Ok(());
    }
    let key = SourceKey {
        kind,
        id: id.into(),
        position: 0,
    };
    if sources.iter().any(|s| s.key == key) {
        return Err(history("중복 포함 출처"));
    }
    let source_index = sources.len();
    sources.push(Source {
        key,
        version_id: version_id.into(),
        material_id: material.material_id.clone(),
        unit: material.unit.canonical().into(),
        quantity,
        reserved: Ratio::zero(),
        probe_reserved: Ratio::zero(),
    });
    let q = integer(quantity);
    stock.layers.insert(
        source_index,
        Layer {
            local: q.clone(),
            origin: q.clone(),
            free_local: q.clone(),
            free_origin: q,
            ..Layer::default()
        },
    );
    stock.quantity = stock
        .quantity
        .checked_add(quantity)
        .ok_or_else(|| history("출처 보유 수량 범위"))?;
    Quantity::from_micros(stock.quantity).map_err(|_| history("출처 보유 수량 범위"))?;
    Ok(())
}

/// Group only commensurable canonical units. Each unit group has equal,
/// dimensionless responsibility; inside it actual quantities determine shares.
fn grouped_weights(
    items: &BTreeMap<String, (ValueMaterial, i64)>,
) -> Result<BTreeMap<String, Ratio>> {
    let mut groups = BTreeMap::<String, BigInt>::new();
    for (material, q) in items.values() {
        if *q <= 0 {
            return Err(history("생산 출처의 실제 수량은 양수여야 합니다"));
        }
        *groups.entry(material.unit.canonical().into()).or_default() += BigInt::from(*q);
    }
    if groups.is_empty() {
        return Err(history("생산 출처가 없습니다"));
    }
    let count = BigInt::from(groups.len());
    Ok(items
        .iter()
        .map(|(id, (m, q))| {
            (
                id.clone(),
                Ratio::new(BigInt::from(*q), &groups[m.unit.canonical()] * &count),
            )
        })
        .collect())
}
fn aggregate(
    items: impl Iterator<Item = (ValueMaterial, i64)>,
) -> Result<BTreeMap<String, (ValueMaterial, i64)>> {
    let mut result = BTreeMap::<String, (ValueMaterial, i64)>::new();
    for (material, quantity) in items {
        if quantity < 0 {
            return Err(history("음수 생산 수량"));
        }
        if quantity == 0 {
            continue;
        }
        let entry = result
            .entry(material.material_id.clone())
            .or_insert((material.clone(), 0));
        if entry.0.unit.canonical() != material.unit.canonical() {
            return Err(history("생산 품목 단위 불일치"));
        }
        entry.1 = entry
            .1
            .checked_add(quantity)
            .ok_or_else(|| history("생산 수량 합계 범위"))?;
    }
    Ok(result)
}

fn production(
    stocks: &mut BTreeMap<String, Stock>,
    inputs: &BTreeMap<String, (ValueMaterial, i64)>,
    outputs: &BTreeMap<String, (ValueMaterial, i64)>,
) -> Result<()> {
    let input_weights = grouped_weights(inputs)?;
    let output_weights = grouped_weights(outputs)?;
    let mut pool = BTreeMap::<usize, Layer>::new();
    // Take every input from its before-state, then append every output.
    for (id, (material, q)) in inputs {
        let removed = material_stock(stocks, material)?.take(*q)?;
        for (source, layer) in removed {
            let p = pool.entry(source).or_default();
            p.local += &layer.local / integer(*q) * &input_weights[id];
            p.free_local += &layer.free_local / integer(*q) * &input_weights[id];
            p.probe_local += &layer.probe_local / integer(*q) * &input_weights[id];
            p.origin += layer.origin;
            p.free_origin += layer.free_origin;
            p.probe_origin += layer.probe_origin;
        }
    }
    for (id, (material, q)) in outputs {
        let stock = material_stock(stocks, material)?;
        for (source, layer) in &pool {
            stock.layers.entry(*source).or_default().add(Layer {
                local: &layer.local * integer(*q),
                free_local: &layer.free_local * integer(*q),
                origin: &layer.origin * &output_weights[id],
                free_origin: &layer.free_origin * &output_weights[id],
                probe_local: &layer.probe_local * integer(*q),
                probe_origin: &layer.probe_origin * &output_weights[id],
            });
        }
        stock.quantity = stock
            .quantity
            .checked_add(*q)
            .ok_or_else(|| history("생산 결과 수량 범위"))?;
        Quantity::from_micros(stock.quantity).map_err(|_| history("생산 결과 수량 범위"))?;
    }
    Ok(())
}

fn reserve_portions(
    stock: &mut Stock,
    sources: &mut [Source],
    portions: &[ProvenancePortion],
    quantity: i64,
    probe: &mut Option<ProbeState>,
) -> Result<()> {
    let mut total = Ratio::zero();
    let mut seen = BTreeSet::new();
    for part in portions {
        if !seen.insert(part.source_key.clone()) {
            return Err(history("한 포함 선택의 원천이 중복됩니다"));
        }
        let index = sources
            .iter()
            .position(|s| s.key == part.source_key)
            .ok_or_else(|| history("포함 출처를 찾을 수 없습니다"))?;
        let source = &mut sources[index];
        if source.version_id != part.source_version_id {
            return Err(history("포함 출처 버전이 다릅니다"));
        }
        let local = parse_share(&part.included_share, &stock.unit)?;
        let origin = parse_share(&part.reserved_source_share, &source.unit)?;
        if local <= Ratio::zero() || origin <= Ratio::zero() {
            return Err(history("영구 포함 몫은 양수여야 합니다"));
        }
        let layer = stock.layers.get_mut(&index).ok_or_else(unavailable)?;
        if local > layer.free_local
            || origin > layer.free_origin
            || &source.reserved + &origin > integer(source.quantity)
        {
            return Err(unavailable());
        }
        // Prior link shares are immutable. A later request never silently
        // reallocates an earlier reservation to another source or path.
        if &layer.free_origin * &local != &origin * &layer.free_local {
            return Err(unavailable());
        }
        restrict(probe, &layer.free_local - &local, &layer.probe_local)?;
        restrict(probe, &layer.free_origin - &origin, &layer.probe_origin)?;
        restrict(
            probe,
            integer(source.quantity) - &source.reserved - &origin,
            &source.probe_reserved,
        )?;
        if &layer.probe_origin * &local != &origin * &layer.probe_local
            && let Some(cap) = probe.as_mut().and_then(|state| state.cap.as_mut())
        {
            // The existing portion is fixed, including its origin/local ratio.
            // No positive probe may silently move it to another path.
            *cap = Ratio::zero();
        }
        layer.free_local -= &local;
        layer.free_origin -= &origin;
        source.reserved += origin;
        total += local;
    }
    if total != integer(quantity) {
        return Err(history("포함 수량과 원천별 몫 합계가 다릅니다"));
    }
    Ok(())
}
fn replay(
    events: &[ValueReplayEvent],
    existing: &[ReservedInclusion],
    probe_request: Option<&ProbeRequest>,
) -> Result<Replay> {
    let mut stocks = BTreeMap::<String, Stock>::new();
    let mut sources = Vec::new();
    let mut observations = Vec::new();
    let mut probe = probe_request
        .cloned()
        .map(|request| ProbeState { request, cap: None });
    let mut seen_events = BTreeSet::new();
    let mut seen_existing = BTreeSet::new();
    if existing
        .iter()
        .any(|r| r.id.is_empty() || !seen_existing.insert(r.id.clone()))
    {
        return Err(history("중복 포함 연결"));
    }
    seen_existing.clear();
    let mut last_seq = 0;
    for event in events {
        let event_ref = match event {
            ValueReplayEvent::Inflow { event, .. }
            | ValueReplayEvent::PurchaseReturn { event, .. }
            | ValueReplayEvent::Observation { event, .. }
            | ValueReplayEvent::Production { event, .. }
            | ValueReplayEvent::Waste { event, .. } => event,
        };
        if event_ref.seq <= 0
            || event_ref.seq < last_seq
            || !seen_events.insert(event_ref.id.clone())
        {
            return Err(history("출처 원사건의 순서 또는 ID가 올바르지 않습니다"));
        }
        last_seq = event_ref.seq;
        match event {
            ValueReplayEvent::Inflow {
                event,
                material,
                quantity,
                ..
            } => {
                inflow(
                    &mut stocks,
                    &mut sources,
                    material,
                    quantity.micros(),
                    SourceKind::ReceiptInflow,
                    &event.id,
                    &event.id,
                )?;
            }
            ValueReplayEvent::Observation {
                event,
                observation_id,
                material,
                measured,
                calculation,
            } => {
                match calculation {
                    ValueObservation::Initial { .. } => {
                        if stocks.contains_key(&material.material_id) {
                            return Err(history(
                                "기존 보유 기준을 초기 관측으로 재설정할 수 없습니다",
                            ));
                        }
                        inflow(
                            &mut stocks,
                            &mut sources,
                            material,
                            measured.micros(),
                            SourceKind::ObservationInitial,
                            observation_id,
                            &event.id,
                        )?;
                    }
                    ValueObservation::Settlement {
                        consumption,
                        excess,
                        ..
                    } => {
                        let stock = stocks
                            .get_mut(&material.material_id)
                            .ok_or_else(|| history("정산 관측의 이전 출처가 없습니다"))?;
                        stock.take(consumption.micros())?;
                        inflow(
                            &mut stocks,
                            &mut sources,
                            material,
                            excess.micros(),
                            SourceKind::ObservationExcess,
                            observation_id,
                            &event.id,
                        )?;
                        if stocks[&material.material_id].quantity != measured.micros() {
                            return Err(history("확정 실측과 출처 재생 수량이 다릅니다"));
                        }
                    }
                }
                let stock = stocks
                    .get_mut(&material.material_id)
                    .ok_or_else(|| history("관측 출처가 없습니다"))?;
                for reservation in existing
                    .iter()
                    .filter(|r| r.observation_version_id == event.id)
                {
                    if reservation.material_id != material.material_id {
                        return Err(history("연결 관측 품목이 다릅니다"));
                    }
                    reserve_portions(
                        stock,
                        &mut sources,
                        &reservation.portions,
                        reservation.quantity.micros(),
                        &mut probe,
                    )?;
                    seen_existing.insert(reservation.id.clone());
                }
                if let Some(state) = &mut probe
                    && state.request.observation_version_id == event.id
                {
                    if state.request.material_id != material.material_id || state.cap.is_some() {
                        return Err(history("포함 상한 탐색의 관측이 다릅니다"));
                    }
                    let index = sources
                        .iter()
                        .position(|s| s.key == state.request.source_key)
                        .ok_or_else(|| {
                            ProvenanceError::InvalidReference("포함 상한의 원천이 없습니다".into())
                        })?;
                    let layer = stock.layers.get_mut(&index).ok_or_else(|| {
                        ProvenanceError::InvalidReference(
                            "선택 관측에 해당 원천 몫이 없습니다".into(),
                        )
                    })?;
                    state.cap = Some(layer.free_local.clone());
                    if !layer.free_local.is_zero() {
                        layer.probe_local = Ratio::one();
                        layer.probe_origin = &layer.free_origin / &layer.free_local;
                        sources[index].probe_reserved = layer.probe_origin.clone();
                    }
                }
                observations.push(ObservationView {
                    id: event.id.clone(),
                    material_id: material.material_id.clone(),
                    stock: stock.clone(),
                });
            }
            ValueReplayEvent::Production {
                inputs, outputs, ..
            } => {
                let inputs = aggregate(
                    inputs
                        .iter()
                        .map(|r| (r.material.clone(), r.quantity.micros())),
                )?;
                let outputs = aggregate(
                    outputs
                        .iter()
                        .map(|r| (r.material.clone(), r.quantity.micros())),
                )?;
                production(&mut stocks, &inputs, &outputs)?;
            }
            ValueReplayEvent::PurchaseReturn {
                material, quantity, ..
            } => {
                stocks
                    .get_mut(&material.material_id)
                    .ok_or_else(|| history("반품 출처가 없습니다"))?
                    .take(quantity.micros())?;
            }
            ValueReplayEvent::Waste {
                material, quantity, ..
            } => {
                stocks
                    .get_mut(&material.material_id)
                    .ok_or_else(|| history("폐기 출처가 없습니다"))?
                    .take(quantity.micros())?;
            }
        }
        for stock in stocks.values() {
            stock.validate()?;
        }
        restrict_all(&mut probe, &stocks, &sources)?;
    }
    if seen_existing.len() != existing.len() {
        return Err(history("기존 포함 연결의 관측이 없습니다"));
    }
    if probe.as_ref().is_some_and(|state| state.cap.is_none()) {
        return Err(ProvenanceError::InvalidReference(
            "선택 관측의 유효 버전을 찾을 수 없습니다".into(),
        ));
    }
    Ok(Replay {
        sources,
        observations,
        probe_cap: probe.and_then(|state| state.cap),
    })
}

fn source_capacity(
    events: &[ValueReplayEvent],
    existing: &[ReservedInclusion],
    view: &ObservationView,
    source: &Source,
) -> Result<Ratio> {
    let probe = ProbeRequest {
        observation_version_id: view.id.clone(),
        material_id: view.material_id.clone(),
        source_key: source.key.clone(),
    };
    replay(events, existing, Some(&probe))?
        .probe_cap
        .ok_or_else(|| history("포함 가능한 원천 상한이 없습니다"))
}

fn capacities(
    events: &[ValueReplayEvent],
    existing: &[ReservedInclusion],
    base: &Replay,
    view: &ObservationView,
    selector: &InclusionSourceSelection,
) -> Result<Vec<(usize, Ratio)>> {
    let indices: Vec<_> = match selector {
        InclusionSourceSelection::AllAvailable {} => view.stock.layers.keys().copied().collect(),
        InclusionSourceSelection::Source { source_key } => {
            let index = base
                .sources
                .iter()
                .position(|s| &s.key == source_key)
                .filter(|i| view.stock.layers.contains_key(i))
                .ok_or_else(|| {
                    ProvenanceError::InvalidReference(
                        "선택 관측에서 해당 출처를 찾을 수 없습니다".into(),
                    )
                })?;
            vec![index]
        }
    };
    indices
        .into_iter()
        .map(|index| {
            let cap = if view.stock.layers[&index].free_local.is_zero() {
                Ratio::zero()
            } else {
                source_capacity(events, existing, view, &base.sources[index])?
            };
            Ok((index, cap))
        })
        .collect()
}

fn resolve_one(
    events: &[ValueReplayEvent],
    existing: &[ReservedInclusion],
    base: &Replay,
    request: &InclusionRequest,
) -> Result<Vec<ProvenancePortion>> {
    let view = base
        .observations
        .iter()
        .find(|view| view.id == request.observation_version_id)
        .ok_or_else(|| {
            ProvenanceError::InvalidReference("선택 관측의 유효 버전을 찾을 수 없습니다".into())
        })?;
    if view.material_id != request.material_id {
        return Err(ProvenanceError::InvalidReference(
            "입고와 선택 관측 품목이 다릅니다".into(),
        ));
    }
    let caps = capacities(events, existing, base, view, &request.source)?;
    let available: Ratio = caps.iter().map(|(_, cap)| cap.clone()).sum();
    let desired = integer(request.quantity.micros());
    if desired > available {
        return Err(unavailable());
    }
    if desired.is_zero() {
        return Ok(vec![]);
    }
    let mut result = Vec::new();
    for (index, cap) in caps {
        if cap.is_zero() {
            continue;
        }
        let layer = &view.stock.layers[&index];
        let source = &base.sources[index];
        let local = &desired * cap / &available;
        let origin = &local * &layer.free_origin / &layer.free_local;
        result.push(ProvenancePortion {
            source_key: source.key.clone(),
            source_version_id: source.version_id.clone(),
            included_share: share(&local, &view.stock.unit),
            reserved_source_share: share(&origin, &source.unit),
        });
    }
    Ok(result)
}

pub fn resolve_inclusions(
    events: &[ValueReplayEvent],
    existing: &[ReservedInclusion],
    proposed: &[InclusionRequest],
) -> Result<Vec<ResolvedInclusion>> {
    let base = replay(events, existing, None).map_err(|e| history(&e.to_string()))?;
    let mut positions = BTreeSet::new();
    let mut ordered = Vec::new();
    for request in proposed {
        if !positions.insert(request.position) {
            return Err(history("중복 포함 제안 위치"));
        }
        let anchor = base
            .observations
            .iter()
            .position(|view| view.id == request.observation_version_id)
            .ok_or_else(|| {
                ProvenanceError::InvalidReference("선택 관측의 유효 버전을 찾을 수 없습니다".into())
            })?;
        ordered.push((anchor, request));
    }
    ordered.sort_by_key(|(anchor, request)| (*anchor, request.position));
    let mut combined = existing.to_vec();
    let mut resolved = Vec::new();
    for (_, request) in ordered {
        let current = replay(events, &combined, None).map_err(|e| history(&e.to_string()))?;
        let portions = resolve_one(events, &combined, &current, request)?;
        // These are temporary resolver identities, never persisted event UUIDs.
        let id = (0..=combined.len())
            .map(|suffix| format!("__inclusion_probe_{}_{}", request.position, suffix))
            .find(|id| combined.iter().all(|item| &item.id != id))
            .ok_or_else(|| history("임시 포함 순번 충돌"))?;
        combined.push(ReservedInclusion {
            id,
            observation_version_id: request.observation_version_id.clone(),
            material_id: request.material_id.clone(),
            quantity: request.quantity,
            portions: portions.clone(),
        });
        replay(events, &combined, None)?;
        resolved.push(ResolvedInclusion {
            position: request.position,
            portions,
        });
    }
    replay(events, &combined, None)?;
    resolved.sort_by_key(|r| r.position);
    Ok(resolved)
}

fn floor_quantity(value: &Ratio) -> Result<Quantity> {
    if value < &Ratio::zero() {
        return Err(history("표시 가능한 포함량이 음수입니다"));
    }
    Quantity::from_micros(
        value
            .to_integer()
            .to_i64()
            .ok_or_else(|| history("표시 가능한 포함 수량 범위"))?,
    )
    .map_err(|_| history("표시 가능한 포함 수량 범위"))
}

pub fn inclusion_availability(
    events: &[ValueReplayEvent],
    existing: &[ReservedInclusion],
    material_id: &str,
) -> Result<Vec<ProvenanceObservation>> {
    let base = replay(events, existing, None).map_err(|e| history(&e.to_string()))?;
    let mut result = Vec::new();
    for view in base
        .observations
        .iter()
        .filter(|v| v.material_id == material_id)
    {
        let caps = capacities(
            events,
            existing,
            &base,
            view,
            &InclusionSourceSelection::AllAvailable {},
        )?;
        let total: Ratio = caps.iter().map(|(_, cap)| cap.clone()).sum();
        let available = floor_quantity(&total)?;
        let mut output_sources = Vec::new();
        for (index, cap) in caps {
            let source = &base.sources[index];
            let local = floor_quantity(&cap)?;
            output_sources.push(ProvenanceSource {
                source_key: source.key.clone(),
                source_version_id: source.version_id.clone(),
                material_id: source.material_id.clone(),
                unit: source.unit.clone(),
                quantity: Quantity::from_micros(source.quantity)
                    .map_err(|_| history("원천 수량 범위"))?,
                reserved_share: share(&source.reserved, &source.unit),
                remaining_share: share(
                    &(integer(source.quantity) - &source.reserved),
                    &source.unit,
                ),
                available_share: share(&integer(local.micros()), &view.stock.unit),
                available_quantity: local,
            });
        }
        result.push(ProvenanceObservation {
            observation_version_id: view.id.clone(),
            material_id: view.material_id.clone(),
            available_share: share(&integer(available.micros()), &view.stock.unit),
            available_quantity: available,
            sources: output_sources,
        });
    }
    Ok(result)
}
