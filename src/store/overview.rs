use super::{
    Store, StoreError, catalog,
    effective_read::{EffectiveObservation, EffectiveReadView, sequence},
    purchase_order,
};
use crate::{
    business::{BusinessPayload, BusinessStatus},
    catalog::{CatalogState, MaterialVersion},
    overview::*,
    purchase_order::{DraftOrderActionV1, OrderStatus, OrderSummary, OrderVersion},
    receipt::ReceiptSourceDraft,
    statistics::StatisticsError,
    stocktake::StocktakeCalculation,
};
use std::collections::HashMap;

mod operations;

fn unavailable() -> OverviewError {
    OverviewError::Unavailable("운영 개요 자료를 읽을 수 없습니다".into())
}
impl From<StoreError> for OverviewError {
    fn from(_: StoreError) -> Self {
        unavailable()
    }
}
impl From<StatisticsError> for OverviewError {
    fn from(_: StatisticsError) -> Self {
        unavailable()
    }
}
fn number(value: &str) -> Result<i64, OverviewError> {
    sequence(value).map_err(Into::into)
}

struct OverviewData {
    state: CatalogState,
    materials: Vec<OverviewMaterialRef>,
    latest_observations: HashMap<String, OverviewObservation>,
    changes: HashMap<String, Vec<OverviewMaterialChange>>,
    order_counts: OverviewOrderCounts,
    open_orders: Vec<OrderSummary>,
    received_orders: Vec<OverviewReceivedOrder>,
    receipts: Vec<OverviewReceipt>,
    sales: Vec<OverviewSale>,
    observations: Vec<OverviewObservation>,
    unallocated: Vec<OverviewUnallocated>,
    unconfirmed: Vec<OverviewUnconfirmed>,
    drafts: Vec<OverviewDraft>,
}

impl OverviewData {
    fn load(store: &Store) -> Result<Self, OverviewError> {
        let view = EffectiveReadView::load(&store.connection, (store.today)())?;
        let selected = view.observations()?;
        let mut observations = selected
            .iter()
            .filter(|o| o.effect.is_some())
            .map(observation)
            .collect::<Result<Vec<_>, _>>()?;
        observations.sort_by(|a, b| {
            number(&b.logical_seq)
                .unwrap_or(-1)
                .cmp(&number(&a.logical_seq).unwrap_or(-1))
                .then(a.position.cmp(&b.position))
                .then(a.observation_id.cmp(&b.observation_id))
        });
        let mut latest_observations = HashMap::new();
        for row in &observations {
            latest_observations
                .entry(row.material_id.clone())
                .or_insert_with(|| row.clone());
        }
        let unallocated = observations
            .iter()
            .filter(|o| o.consumption.as_ref().is_some_and(|c| c.reason.is_some()))
            .map(|o| OverviewUnallocated {
                observation: o.clone(),
            })
            .collect();
        let mut latest: HashMap<&str, (&MaterialVersion, i64)> = HashMap::new();
        for material in &view.history.materials {
            let seq = number(material.seq.as_deref().unwrap_or("0"))?;
            if latest
                .get(material.material_id.as_str())
                .is_none_or(|(_, old)| seq > *old)
            {
                latest.insert(&material.material_id, (material, seq));
            }
        }
        let mut materials = latest
            .values()
            .map(|(m, _)| OverviewMaterialRef {
                id: m.material_id.clone(),
                version_id: m.id.clone(),
                name: m.name.clone(),
                unit: m.canonical_unit.clone(),
                archived: m.archived,
            })
            .collect::<Vec<_>>();
        materials.sort_by(|a, b| {
            a.archived
                .cmp(&b.archived)
                .then(a.name.cmp(&b.name))
                .then(a.id.cmp(&b.id))
        });

        let (changes, receipts, sales, unconfirmed) =
            operations::collect(&view, &selected, &latest_observations)?;
        let (order_counts, open_orders, received_orders) = orders(store, &view)?;
        let drafts = drafts(&view)?;
        Ok(Self {
            state: view.state,
            materials,
            latest_observations,
            changes,
            order_counts,
            open_orders,
            received_orders,
            receipts,
            sales,
            observations,
            unallocated,
            unconfirmed,
            drafts,
        })
    }
    fn boundary(&self, id: &str) -> OverviewChangeBoundary {
        self.latest_observations
            .get(id)
            .map_or(OverviewChangeBoundary::NotObserved {}, |o| {
                OverviewChangeBoundary::Observation {
                    observation_id: o.observation_id.clone(),
                    observation_version_id: o.observation_version_id.clone(),
                    stocktake_id: o.origin.id.clone(),
                    logical_seq: o.logical_seq.clone(),
                    observed_at: o.observed_at.clone(),
                }
            })
    }
    fn material(&self, m: &OverviewMaterialRef) -> OverviewMaterial {
        OverviewMaterial {
            material: m.clone(),
            last_observation: self.latest_observations.get(&m.id).cloned(),
            changes_boundary: self.boundary(&m.id),
            changes_after_observation: overview_page(
                self.changes.get(&m.id).map_or(&[], Vec::as_slice),
                0,
                OverviewSection::MaterialChanges,
            ),
        }
    }
    fn materials_page(&self, offset: usize) -> OverviewPage<OverviewMaterial> {
        let page = overview_page(&self.materials, offset, OverviewSection::Materials);
        OverviewPage {
            total_count: page.total_count,
            returned_count: page.returned_count,
            offset: page.offset,
            page_size: page.page_size,
            next_offset: page.next_offset,
            rows: page.rows.iter().map(|m| self.material(m)).collect(),
        }
    }
    fn summary(&self) -> OperationsOverview {
        OperationsOverview {
            state: self.state.clone(),
            materials: self.materials_page(0),
            order_counts: self.order_counts.clone(),
            open_orders: overview_page(&self.open_orders, 0, OverviewSection::OpenOrders),
            received_orders: overview_page(
                &self.received_orders,
                0,
                OverviewSection::ReceivedOrders,
            ),
            receipts: overview_page(&self.receipts, 0, OverviewSection::Receipts),
            sales: overview_page(&self.sales, 0, OverviewSection::Sales),
            observations: overview_page(&self.observations, 0, OverviewSection::Observations),
            unallocated: overview_page(&self.unallocated, 0, OverviewSection::Unallocated),
            unconfirmed: overview_page(&self.unconfirmed, 0, OverviewSection::Unconfirmed),
            drafts: overview_page(&self.drafts, 0, OverviewSection::Drafts),
        }
    }
    fn list(&self, scope: &OverviewScope, offset: usize) -> Result<OverviewList, OverviewError> {
        Ok(match scope.section {
            OverviewSection::Materials => OverviewList::Materials {
                page: self.materials_page(offset),
            },
            OverviewSection::MaterialChanges => {
                let id = scope.material_id.as_deref().ok_or_else(unavailable)?;
                let material =
                    self.materials.iter().find(|m| m.id == id).ok_or_else(|| {
                        OverviewError::Validation("조회할 품목이 없습니다".into())
                    })?;
                OverviewList::MaterialChanges {
                    material: material.clone(),
                    boundary: Box::new(self.boundary(id)),
                    page: overview_page(
                        self.changes.get(id).map_or(&[], Vec::as_slice),
                        offset,
                        scope.section,
                    ),
                }
            }
            OverviewSection::OpenOrders => OverviewList::OpenOrders {
                page: overview_page(&self.open_orders, offset, scope.section),
            },
            OverviewSection::ReceivedOrders => OverviewList::ReceivedOrders {
                page: overview_page(&self.received_orders, offset, scope.section),
            },
            OverviewSection::Receipts => OverviewList::Receipts {
                page: overview_page(&self.receipts, offset, scope.section),
            },
            OverviewSection::Sales => OverviewList::Sales {
                page: overview_page(&self.sales, offset, scope.section),
            },
            OverviewSection::Observations => OverviewList::Observations {
                page: overview_page(&self.observations, offset, scope.section),
            },
            OverviewSection::Unallocated => OverviewList::Unallocated {
                page: overview_page(&self.unallocated, offset, scope.section),
            },
            OverviewSection::Unconfirmed => OverviewList::Unconfirmed {
                page: overview_page(&self.unconfirmed, offset, scope.section),
            },
            OverviewSection::Drafts => OverviewList::Drafts {
                page: overview_page(&self.drafts, offset, scope.section),
            },
        })
    }
}

fn observation(o: &EffectiveObservation) -> Result<OverviewObservation, OverviewError> {
    let effect = o.effect.as_ref().ok_or_else(unavailable)?;
    let consumption = match &effect.calculation {
        StocktakeCalculation::Initial { .. } => None,
        StocktakeCalculation::Settlement {
            excess_quantity,
            excess_value,
            ..
        } => {
            let a = o.allocation.as_ref().ok_or_else(unavailable)?;
            Some(OverviewConsumption {
                consumption_version_id: o.consumption_version_id.clone().ok_or_else(unavailable)?,
                quantity: a.total_quantity.clone(),
                value: a.total_value.clone(),
                excess_quantity: excess_quantity.clone(),
                excess_value: excess_value.clone(),
                unallocated_quantity: a.unallocated_quantity.clone(),
                unallocated_value: a.unallocated_value.clone(),
                reason: a.reason,
                candidate_count: a.candidates.len(),
            })
        }
    };
    Ok(OverviewObservation {
        origin: o.origin.clone(),
        observation_id: o.observation_id.clone(),
        observation_version_id: o
            .effective_observation_version_id
            .clone()
            .ok_or_else(unavailable)?,
        position: o.position,
        material_id: effect.material_id.clone(),
        material_version_id: effect.material_version_id.clone(),
        material_name: effect.material_name.clone(),
        unit: effect.unit.clone(),
        quantity: effect.measured_quantity.clone(),
        observed_at: o.origin.recorded_at.clone(),
        logical_seq: o.origin.seq.clone(),
        mode: if consumption.is_some() {
            OverviewObservationMode::Settlement
        } else {
            OverviewObservationMode::Initial
        },
        consumption,
        changes: o.changes.clone(),
    })
}

type OrderLists = (
    OverviewOrderCounts,
    Vec<OrderSummary>,
    Vec<OverviewReceivedOrder>,
);
fn orders(store: &Store, view: &EffectiveReadView) -> Result<OrderLists, OverviewError> {
    let mut heads = purchase_order::read_versions(&store.connection, None, None, true)?;
    heads.sort_by(|a, b| {
        number(&b.seq)
            .unwrap_or(-1)
            .cmp(&number(&a.seq).unwrap_or(-1))
            .then(a.order_id.cmp(&b.order_id))
    });
    let first: HashMap<_, _> = view
        .history
        .orders
        .iter()
        .filter(|v| v.previous_version_id.is_none())
        .map(|v| (v.order_id.as_str(), v.recorded_at.as_str()))
        .collect();
    let mut counts = OverviewOrderCounts::default();
    let mut open = vec![];
    let mut received = vec![];
    for head in heads {
        let created_at = first
            .get(head.order_id.as_str())
            .ok_or_else(unavailable)?
            .to_string();
        let status = head.status;
        let summary = order_summary(head, created_at);
        match status {
            OrderStatus::Open => {
                counts.open += 1;
                open.push(summary);
            }
            OrderStatus::Cancelled => counts.cancelled += 1,
            OrderStatus::Received => {
                counts.received += 1;
                let mut matches = view
                    .history
                    .closures
                    .iter()
                    .filter(|c| c.order_id.to_string() == summary.id);
                let closure = matches.next().ok_or_else(unavailable)?.clone();
                if matches.next().is_some() {
                    return Err(unavailable());
                }
                received.push(OverviewReceivedOrder {
                    order: summary,
                    closure,
                });
            }
        }
    }
    Ok((counts, open, received))
}
fn order_summary(v: OrderVersion, created_at: String) -> OrderSummary {
    OrderSummary {
        id: v.order_id,
        head_id: v.id,
        status: v.status,
        material_id: v.material_id,
        material_version_id: v.material_version_id,
        material_name: v.material_name,
        quantity: v.quantity,
        unit: v.unit,
        input_quantity: v.input_quantity,
        input_unit: v.input_unit,
        supplier: v.supplier,
        expected_date: v.expected_date,
        estimated_amount: v.estimated_amount,
        created_at,
        updated_at: v.recorded_at,
    }
}

fn drafts(view: &EffectiveReadView) -> Result<Vec<OverviewDraft>, OverviewError> {
    let heads: HashMap<_, _> = view
        .business
        .heads
        .iter()
        .map(|h| (h.record_id.as_str(), h.current_revision_id.as_str()))
        .collect();
    let revisions: HashMap<_, _> = view
        .business
        .revisions
        .iter()
        .map(|r| (r.id.as_str(), r))
        .collect();
    let materials: HashMap<_, _> = view
        .history
        .materials
        .iter()
        .map(|m| (m.id.as_str(), m))
        .collect();
    let mut rows = vec![];
    for record in &view.business.records {
        let head = revisions
            .get(heads.get(record.id.as_str()).ok_or_else(unavailable)?)
            .ok_or_else(unavailable)?;
        if head.status != BusinessStatus::Draft {
            continue;
        }
        rows.push((
            number(&head.seq)?,
            OverviewDraft {
                id: record.id.clone(),
                kind: record.kind,
                head_id: head.id.clone(),
                created_at: record.created_at.clone(),
                updated_at: head.recorded_at.clone(),
                corrects_confirmation_id: record.corrects_confirmation_id.clone(),
                material_name: draft_name(&head.payload, view, &materials),
                validation: OverviewDraftValidation::NotEvaluated,
            },
        ));
    }
    rows.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.id.cmp(&b.1.id)));
    Ok(rows.into_iter().map(|(_, r)| r).collect())
}
fn draft_name(
    payload: &BusinessPayload,
    view: &EffectiveReadView,
    materials: &HashMap<&str, &MaterialVersion>,
) -> Option<String> {
    let pair = match payload {
        BusinessPayload::CatalogPrice { data, .. } => (
            data.material_id.as_deref(),
            data.material_version_id.as_deref(),
        ),
        BusinessPayload::Sale { data, .. } => (
            data.material_id.as_deref(),
            data.material_version_id.as_deref(),
        ),
        BusinessPayload::Waste { data, .. } => (
            data.material_id.as_deref(),
            data.material_version_id.as_deref(),
        ),
        BusinessPayload::Stocktake { data, .. } => data
            .rows
            .iter()
            .find_map(|r| Some((r.material_id.as_deref()?, r.material_version_id.as_deref()?)))
            .map_or((None, None), |(a, b)| (Some(a), Some(b))),
        BusinessPayload::Production { data, .. } => data
            .outputs
            .iter()
            .find_map(|r| Some((r.material_id.as_deref()?, r.material_version_id.as_deref()?)))
            .or_else(|| {
                data.inputs.iter().find_map(|r| {
                    Some((r.material_id.as_deref()?, r.material_version_id.as_deref()?))
                })
            })
            .map_or((None, None), |(a, b)| (Some(a), Some(b))),
        BusinessPayload::Receipt { data, .. } => match &data.source {
            ReceiptSourceDraft::Direct {
                material_id,
                material_version_id,
            } => (material_id.as_deref(), material_version_id.as_deref()),
            ReceiptSourceDraft::Order {
                order_id,
                material_version_id,
                ..
            } => (
                order_id
                    .as_ref()
                    .and_then(|id| view.history.orders.iter().find(|o| &o.order_id == id))
                    .map(|o| o.material_id.as_str()),
                material_version_id.as_deref(),
            ),
        },
        BusinessPayload::PurchaseOrder { data, .. } => match data {
            DraftOrderActionV1::Place {
                material_id,
                material_version_id,
                ..
            } => (material_id.as_deref(), material_version_id.as_deref()),
            DraftOrderActionV1::Revise {
                order_id,
                material_version_id,
                ..
            } => (
                view.history
                    .orders
                    .iter()
                    .find(|o| &o.order_id == order_id)
                    .map(|o| o.material_id.as_str()),
                material_version_id.as_deref(),
            ),
            DraftOrderActionV1::Cancel { .. } => (None, None),
        },
        BusinessPayload::PurchaseReturn { .. } | BusinessPayload::Correction { .. } => (None, None),
    };
    let (Some(id), Some(version)) = pair else {
        return None;
    };
    materials
        .get(version)
        .filter(|m| m.material_id == id)
        .map(|m| m.name.clone())
}

impl Store {
    pub fn operations_overview(&self) -> Result<OperationsOverview, OverviewError> {
        Ok(OverviewData::load(self)?.summary())
    }
    pub fn overview_list(
        &self,
        query: OverviewListQuery,
    ) -> Result<OverviewListResponse, OverviewError> {
        let offset = query.validate()?;
        let current = catalog::state(&self.connection)?;
        if current.epoch != query.epoch || current.revision != query.revision {
            return Err(OverviewError::StaleState(
                "기록이 변경되었습니다. 개요를 새로 읽고 첫 페이지부터 확인하세요".into(),
            ));
        }
        let data = OverviewData::load(self)?;
        if data.state != current {
            return Err(unavailable());
        }
        let scope = query.scope();
        let list = data.list(&scope, offset)?;
        Ok(OverviewListResponse {
            state: data.state,
            scope,
            list,
        })
    }
}
