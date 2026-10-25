use super::*;
use crate::{
    correction::{CorrectionEventValue, CorrectionPlanRef},
    statistics::{StatisticsConsumptionReason, StatisticsConsumptionStatus},
};

type OperationLists = (
    HashMap<String, Vec<OverviewMaterialChange>>,
    Vec<OverviewReceipt>,
    Vec<OverviewSale>,
    Vec<OverviewUnconfirmed>,
);

pub(super) fn collect(
    view: &EffectiveReadView,
    observations: &[EffectiveObservation],
    latest: &HashMap<String, OverviewObservation>,
) -> Result<OperationLists, OverviewError> {
    let mut changes: HashMap<String, Vec<OverviewMaterialChange>> = HashMap::new();
    let mut receipts = vec![];
    let mut sales = vec![];
    let mut unconfirmed = vec![];
    let versions: HashMap<_, _> = view
        .history
        .materials
        .iter()
        .map(|m| (m.id.as_str(), m))
        .collect();
    for event in &view.history.events {
        let Some(value) = &event.effective_value else {
            continue;
        };
        let origin = view.origin(event)?;
        let sequence = number(&event.reference.logical_seq)?;
        let references = view.changes(event, None)?;
        let mut add = |material: OverviewEffectMaterial,
                       effect: OverviewMaterialEffect|
         -> Result<(), OverviewError> {
            if latest
                .get(&material.id)
                .map(|o| number(&o.logical_seq))
                .transpose()?
                .is_some_and(|n| sequence <= n)
            {
                return Ok(());
            }
            changes
                .entry(material.id.clone())
                .or_default()
                .push(OverviewMaterialChange {
                    origin: origin.clone(),
                    effective_ref: event.reference.clone(),
                    changes: references.clone(),
                    material,
                    effect,
                });
            Ok(())
        };
        match value {
            CorrectionEventValue::Receipt { reconciliation, .. } => {
                let basis = view.receipt_basis(event)?.ok_or_else(unavailable)?;
                let effective = basis.effective.ok_or_else(unavailable)?;
                let reconciliation_summary =
                    reconciliation.as_ref().map(|r| OverviewReconciliation {
                        included_quantity: r.included_quantity.clone(),
                        included_amount: r.included_amount.clone(),
                        unreflected_quantity: r.unreflected_quantity.clone(),
                        unreflected_amount: r.unreflected_amount.clone(),
                        value_difference: r.value_difference.clone(),
                        separate_adjustment: r.separate_adjustment.clone(),
                        affected_material_count: r.materials.len(),
                        selection_count: r.selections.len(),
                    });
                receipts.push(OverviewReceipt {
                    origin: origin.clone(),
                    effective_ref: event.reference.clone(),
                    changes: references.clone(),
                    original: basis.original,
                    effective: effective.clone(),
                    reconciliation: reconciliation_summary,
                });
                if let Some(r) = reconciliation {
                    let mut source_seen = false;
                    for m in &r.materials {
                        let source = m.material_id == effective.material_id;
                        source_seen |= source;
                        if !source && m.inflow_quantity == "0" && m.value_delta == "0" {
                            continue;
                        }
                        add(
                            OverviewEffectMaterial {
                                id: m.material_id.clone(),
                                version_id: m.material_version_id.clone(),
                                name: m.material_name.clone(),
                                unit: m.unit.clone(),
                            },
                            OverviewMaterialEffect::ReconciledReceipt {
                                receipt: Box::new(effective.clone()),
                                included_quantity: r.included_quantity.clone(),
                                included_amount: r.included_amount.clone(),
                                inflow_quantity: m.inflow_quantity.clone(),
                                inflow_value: m.inflow_value.clone(),
                                value_delta: m.value_delta.clone(),
                                separate_adjustment: r.separate_adjustment.clone(),
                            },
                        )?;
                    }
                    // A fully included purchase may only have a separate value adjustment.
                    if !source_seen {
                        add(
                            receipt_material(&effective),
                            OverviewMaterialEffect::ReconciledReceipt {
                                receipt: Box::new(effective.clone()),
                                included_quantity: r.included_quantity.clone(),
                                included_amount: r.included_amount.clone(),
                                inflow_quantity: "0".into(),
                                inflow_value: "0".into(),
                                value_delta: "0".into(),
                                separate_adjustment: r.separate_adjustment.clone(),
                            },
                        )?;
                    }
                } else {
                    add(
                        receipt_material(&effective),
                        OverviewMaterialEffect::Receipt {
                            quantity: effective.quantity.clone(),
                            amount: effective.amount.clone(),
                            receipt_date: effective.receipt_date.clone(),
                        },
                    )?;
                }
            }
            CorrectionEventValue::Production { result, .. } => {
                for m in &result.materials {
                    let material = versions
                        .get(m.material_version_id.as_str())
                        .filter(|v| v.material_id == m.material_id)
                        .ok_or_else(unavailable)?;
                    add(
                        OverviewEffectMaterial {
                            id: m.material_id.clone(),
                            version_id: m.material_version_id.clone(),
                            name: material.name.clone(),
                            unit: m.unit.clone(),
                        },
                        OverviewMaterialEffect::Production {
                            input_quantity: m.input_quantity.clone(),
                            input_value: m.input_value.clone(),
                            output_quantity: m.output_quantity.clone(),
                            output_value: m.output_value.clone(),
                        },
                    )?;
                }
            }
            CorrectionEventValue::Waste { result: r, .. } => {
                add(
                    OverviewEffectMaterial {
                        id: r.material_id.clone(),
                        version_id: r.material_version_id.clone(),
                        name: r.material_name.clone(),
                        unit: r.unit.clone(),
                    },
                    OverviewMaterialEffect::Waste {
                        quantity: r.quantity.clone(),
                        loss_value: r.loss_value.clone(),
                    },
                )?;
            }
            CorrectionEventValue::PurchaseReturn { result: r, .. } => {
                let source = match &r.source.reference {
                    CorrectionPlanRef::Existing { id, .. }
                    | CorrectionPlanRef::Proposed { id, .. } => id,
                };
                add(
                    OverviewEffectMaterial {
                        id: r.material_id.clone(),
                        version_id: r.material_version_id.clone(),
                        name: r.material_name.clone(),
                        unit: r.unit.clone(),
                    },
                    OverviewMaterialEffect::PurchaseReturn {
                        quantity: r.quantity.clone(),
                        deducted_value: r.deducted_value.clone(),
                        source_receipt_id: source.clone(),
                    },
                )?;
            }
            CorrectionEventValue::SaleCount { result, .. } => {
                let original = view
                    .history
                    .sales
                    .iter()
                    .find(|s| s.id == origin.id)
                    .ok_or_else(unavailable)?;
                let links = super::super::effective_read::sale_consumption(original, observations)?;
                let mut counts = OverviewConsumptionCounts::default();
                for link in links {
                    match link.status {
                        StatisticsConsumptionStatus::Allocated => counts.allocated += 1,
                        StatisticsConsumptionStatus::Unallocated => counts.unallocated += 1,
                        StatisticsConsumptionStatus::Unconfirmed => {
                            counts.unconfirmed += 1;
                            let direct = original
                                .recipe_cost
                                .rows
                                .iter()
                                .find(|r| r.material_id == link.material_id)
                                .ok_or_else(unavailable)?;
                            let initial = match (
                                &link.stocktake_id,
                                &link.observation_id,
                                &link.observation_version_id,
                                &link.observation_recorded_at,
                            ) {
                                (
                                    Some(stocktake_id),
                                    Some(observation_id),
                                    Some(observation_version_id),
                                    Some(observed_at),
                                ) => Some(OverviewInitialObservation {
                                    stocktake_id: stocktake_id.clone(),
                                    observation_id: observation_id.clone(),
                                    observation_version_id: observation_version_id.clone(),
                                    observed_at: observed_at.clone(),
                                }),
                                (None, None, None, None) => None,
                                _ => return Err(unavailable()),
                            };
                            unconfirmed.push(OverviewUnconfirmed {
                                sale: OverviewUnconfirmedSale {
                                    origin: origin.clone(),
                                    effective_ref: event.reference.clone(),
                                    material_id: original.material_id.clone(),
                                    material_name: original.material_name.clone(),
                                    count: result.count.clone(),
                                },
                                material: OverviewEffectMaterial {
                                    id: direct.material_id.clone(),
                                    version_id: direct.material_version_id.clone(),
                                    name: direct.material_name.clone(),
                                    unit: direct.unit.clone(),
                                },
                                reason: if link.reason
                                    == Some(StatisticsConsumptionReason::InitialBasisMissing)
                                {
                                    OverviewUnconfirmedReason::InitialBasisMissing
                                } else {
                                    OverviewUnconfirmedReason::NoEffectiveAllocation
                                },
                                initial_observation: initial,
                            });
                        }
                    }
                }
                sales.push(OverviewSale {
                    origin,
                    effective_ref: event.reference.clone(),
                    changes: references,
                    material_id: original.material_id.clone(),
                    material_version_id: original.material_version_id.clone(),
                    material_name: original.material_name.clone(),
                    recipe_version_id: original.recipe_version_id.clone(),
                    count: result.count.clone(),
                    amount: result.amount.clone(),
                    consumption_counts: counts,
                });
            }
            CorrectionEventValue::Stocktake { .. } => {}
        }
    }
    for rows in changes.values_mut() {
        rows.sort_by(|a, b| {
            number(&b.origin.seq)
                .unwrap_or(-1)
                .cmp(&number(&a.origin.seq).unwrap_or(-1))
                .then(kind(&a.effect).cmp(kind(&b.effect)))
                .then(a.origin.id.cmp(&b.origin.id))
        });
    }
    receipts.sort_by(|a, b| {
        number(&b.origin.seq)
            .unwrap_or(-1)
            .cmp(&number(&a.origin.seq).unwrap_or(-1))
            .then(a.origin.id.cmp(&b.origin.id))
    });
    sales.sort_by(|a, b| {
        number(&b.origin.seq)
            .unwrap_or(-1)
            .cmp(&number(&a.origin.seq).unwrap_or(-1))
            .then(a.origin.id.cmp(&b.origin.id))
    });
    unconfirmed.sort_by(|a, b| {
        number(&b.sale.origin.seq)
            .unwrap_or(-1)
            .cmp(&number(&a.sale.origin.seq).unwrap_or(-1))
            .then(a.sale.origin.id.cmp(&b.sale.origin.id))
            .then(a.material.id.cmp(&b.material.id))
    });
    Ok((changes, receipts, sales, unconfirmed))
}
fn receipt_material(r: &crate::statistics::StatisticsReceiptBasis) -> OverviewEffectMaterial {
    OverviewEffectMaterial {
        id: r.material_id.clone(),
        version_id: r.material_version_id.clone(),
        name: r.material_name.clone(),
        unit: r.unit.clone(),
    }
}
fn kind(effect: &OverviewMaterialEffect) -> &'static str {
    match effect {
        OverviewMaterialEffect::Receipt { .. } => "receipt",
        OverviewMaterialEffect::ReconciledReceipt { .. } => "reconciled_receipt",
        OverviewMaterialEffect::Production { .. } => "production",
        OverviewMaterialEffect::PurchaseReturn { .. } => "purchase_return",
        OverviewMaterialEffect::Waste { .. } => "waste",
    }
}
