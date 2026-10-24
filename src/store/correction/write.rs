//! A single transaction materializes the reviewed plan; original rows and the
//! symbolic plan remain immutable. UUIDs are recorded separately from that plan.
use super::*;
use crate::domain::{Money, Quantity};
use crate::store::stocktake;
use rusqlite::params;
use uuid::Uuid;

fn id() -> String {
    Uuid::new_v4().to_string()
}
fn json<T: Serialize>(v: &T) -> Result<String, StoreError> {
    serde_json::to_string(v).map_err(unavailable)
}
fn seq(v: &str) -> Result<i64, StoreError> {
    v.parse().map_err(unavailable)
}
fn kind(k: CorrectionSubjectKind) -> &'static str {
    match k {
        CorrectionSubjectKind::Receipt => "receipt",
        CorrectionSubjectKind::Production => "production",
        CorrectionSubjectKind::SaleCount => "sale_count",
        CorrectionSubjectKind::Waste => "waste",
        CorrectionSubjectKind::Stocktake => "stocktake",
    }
}
fn entity(k: CorrectionSubjectKind) -> CorrectionRefKind {
    match k {
        CorrectionSubjectKind::Receipt => CorrectionRefKind::Receipt,
        CorrectionSubjectKind::Production => CorrectionRefKind::Production,
        CorrectionSubjectKind::SaleCount => CorrectionRefKind::Sale,
        CorrectionSubjectKind::Waste => CorrectionRefKind::Waste,
        CorrectionSubjectKind::Stocktake => CorrectionRefKind::Stocktake,
    }
}

pub(super) fn resolve(
    reference: &CorrectionPlanRef,
    refs: &[CorrectionResolvedRef],
) -> Result<String, StoreError> {
    if let Some(r) = refs.iter().find(|r| &r.reference == reference) {
        return Ok(r.version_id.clone());
    }
    match reference {
        CorrectionPlanRef::Existing { version_id, .. } => Ok(version_id.clone()),
        CorrectionPlanRef::Proposed { .. } => {
            Err(unavailable("미리보기 참조의 실제 버전이 없습니다"))
        }
    }
}
fn map_ref(
    refs: &mut Vec<CorrectionResolvedRef>,
    reference: CorrectionPlanRef,
    version_id: String,
) {
    if !refs.iter().any(|r| r.reference == reference) {
        refs.push(CorrectionResolvedRef {
            reference,
            version_id,
        });
    }
}
pub(super) fn resolve_value<T: Serialize + DeserializeOwned>(
    value: &T,
    refs: &[CorrectionResolvedRef],
) -> Result<T, StoreError> {
    let mut value = serde_json::to_value(value).map_err(unavailable)?;
    fn walk(v: &mut serde_json::Value, refs: &[CorrectionResolvedRef]) {
        match v {
            serde_json::Value::Array(values) => {
                for x in values {
                    walk(x, refs)
                }
            }
            serde_json::Value::Object(object) => {
                let holding_ref = object.contains_key("seq")
                    && (object.contains_key("quantity")
                        || object
                            .get("kind")
                            .and_then(|k| k.as_str())
                            .is_some_and(|k| {
                                matches!(
                                    k,
                                    "receipt"
                                        | "production"
                                        | "observation"
                                        | "reconciliation"
                                        | "correction"
                                )
                            }));
                if let Ok(r) = serde_json::from_value::<CorrectionPlanRef>(
                    serde_json::Value::Object(object.clone()),
                ) && let Some(actual) = refs.iter().find(|x| x.reference == r)
                {
                    let (entity, id) = match r {
                        CorrectionPlanRef::Existing { entity, id, .. }
                        | CorrectionPlanRef::Proposed { entity, id, .. } => (entity, id),
                    };
                    *v = serde_json::to_value(CorrectionPlanRef::Existing {
                        entity,
                        id,
                        version_id: actual.version_id.clone(),
                    })
                    .expect("plan ref serialization");
                    return;
                }
                for (key, x) in object.iter_mut() {
                    if (matches!(
                        key.as_str(),
                        "observation_version_id"
                            | "source_version_id"
                            | "sale_version_id"
                            | "expected_version_id"
                    ) || (holding_ref && key == "id"))
                        && let Some(old) = x.as_str()
                        && let Some(actual) = refs.iter().find(|r| match &r.reference {
                            CorrectionPlanRef::Proposed {
                                id,
                                expected_version_id,
                                ..
                            } => id == old || expected_version_id.as_deref() == Some(old),
                            _ => false,
                        })
                    {
                        *x = serde_json::Value::String(actual.version_id.clone());
                    } else {
                        walk(x, refs)
                    }
                }
            }
            _ => {}
        }
    }
    walk(&mut value, refs);
    serde_json::from_value(value).map_err(unavailable)
}

pub(in crate::store) fn apply_correction(
    connection: &Connection,
    data: &DraftCorrectionV1,
    target: &str,
    event: &business::ConfirmationRow,
    sequence: i64,
    time: &str,
    today: NaiveDate,
) -> Result<(), StoreError> {
    check_target(connection, Some(target), data)?;
    let check = build_correction_plan(&history::load(connection, today)?, target, data)
        .map_err(unavailable)?;
    if !check.issues.is_empty() {
        return Err(StoreError::Validation(
            check
                .issues
                .iter()
                .map(|i| i.message.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        ));
    }
    // Several derived observations can refer to each other in this same
    // atomic plan. All references must exist at commit, irrespective of UUID
    // order in the plan's presentation.
    connection
        .execute_batch("PRAGMA defer_foreign_keys=ON")
        .map_err(unavailable)?;
    let plan = check.plan;
    let old = snapshot_rows(connection)?;
    let correction_id = id();
    let mut refs = vec![];
    let mut events = vec![];
    for change in &plan.events {
        let version_id = id();
        let sale_version_id =
            matches!(change.after, Some(CorrectionEventValue::SaleCount { .. })).then(id);
        map_ref(
            &mut refs,
            CorrectionPlanRef::Proposed {
                entity: entity(change.reference.kind),
                id: change.reference.original_id.clone(),
                expected_version_id: Some(change.reference.version_id.clone()),
            },
            sale_version_id.as_ref().unwrap_or(&version_id).clone(),
        );
        if let Some(actual) = &sale_version_id {
            let previous: String = connection
                .query_row(
                    "SELECT current_version_id FROM sale_heads WHERE sale_id=?1",
                    [&change.reference.original_id],
                    |r| r.get(0),
                )
                .map_err(unavailable)?;
            map_ref(
                &mut refs,
                CorrectionPlanRef::Proposed {
                    entity: CorrectionRefKind::Sale,
                    id: change.reference.original_id.clone(),
                    expected_version_id: Some(previous),
                },
                actual.clone(),
            );
        }
        let previous = old
            .event_versions
            .iter()
            .find(|v| v.id == change.reference.version_id);
        let mut reference = change.reference.clone();
        reference.version_id = version_id.clone();
        reference.confirmation_id = event.id.clone();
        reference.business_record_id = event.record_id.clone();
        reference.seq = sequence.to_string();
        reference.recorded_at = time.into();
        events.push(EventVersionRow {
            id: version_id,
            sale_version_id,
            correction_id: correction_id.clone(),
            previous_version_id: previous.map(|v| v.id.clone()),
            previous_original_version_id: previous
                .is_none()
                .then(|| change.reference.version_id.clone()),
            reference,
            after: change.after.clone(),
        });
    }
    let mut projections = vec![];
    let mut push = |projection: Projection| {
        let previous = old
            .projection_heads
            .iter()
            .find(|h| h.kind.text() == projection.kind() && h.subject_id == projection.subject_id())
            .map(|h| h.current_version_id.clone());
        projections.push(ProjectionVersionRow {
            id: id(),
            correction_id: correction_id.clone(),
            previous_version_id: previous,
            recorded_at: time.into(),
            seq: sequence.to_string(),
            projection,
        });
    };
    for change in &plan.observations {
        let actual = change.after.as_ref().map(|_| id());
        if let Some(actual) = &actual {
            map_ref(&mut refs, change.reference.clone(), actual.clone());
        }
        push(Projection::Observation {
            change: change.clone(),
            observation_version_id: actual,
            price_basis_id: change.valuation_basis.as_ref().map(|_| id()),
        });
    }
    for change in &plan.materials {
        let known = matches!(change.after, CorrectionHoldingState::Known { .. });
        let holding_version_id = known.then(id);
        let price_basis_id = match &change.after {
            CorrectionHoldingState::Known {
                last_valid_price: Some(_),
                ..
            } => Some(id()),
            _ => None,
        };
        if let Some(actual) = &holding_version_id {
            map_ref(
                &mut refs,
                CorrectionPlanRef::Proposed {
                    entity: CorrectionRefKind::Material,
                    id: change.material_id.clone(),
                    expected_version_id: match &change.before {
                        CorrectionHoldingState::Known { amounts, .. } => amounts.source_id.clone(),
                        _ => None,
                    },
                },
                actual.clone(),
            );
        }
        push(Projection::Material {
            change: change.clone(),
            holding_version_id,
            price_basis_id,
        });
    }
    for change in &plan.allocations {
        push(Projection::Allocation {
            change: change.clone(),
            consumption_version_id: change.after.as_ref().map(|_| id()),
        });
    }
    let original_links = super::super::reconciliation::snapshot_rows(connection)?;
    for change in &plan.inclusions {
        let previous = old
            .projection_versions
            .iter()
            .filter(|p| {
                p.projection.kind() == "inclusion" && p.projection.subject_id() == change.receipt_id
            })
            .max_by_key(|p| p.seq.parse::<i64>().unwrap_or(-1));
        let previous_links = match previous.map(|p| &p.projection) {
            Some(Projection::Inclusion { links, .. }) => links.clone(),
            _ => {
                let original = original_links
                    .records
                    .iter()
                    .find(|r| r.receipt_id == change.receipt_id);
                original_links
                    .links
                    .iter()
                    .filter(|l| original.is_some_and(|r| r.id == l.reconciliation_id))
                    .filter_map(|l| {
                        let v = original_links
                            .versions
                            .iter()
                            .filter(|v| v.link_id == l.id)
                            .max_by_key(|v| v.seq.parse::<i64>().unwrap_or(-1))?;
                        Some(InclusionVersionRow {
                            id: v.id.clone(),
                            link_id: l.id.clone(),
                            previous_version_id: v.previous_version_id.clone(),
                            reservation_id: original_links
                                .reservations
                                .iter()
                                .find(|r| r.link_version_id == v.id)?
                                .id
                                .clone(),
                            observation_version_id: v.observation_version_id.clone(),
                            selection_position: l.selection_position,
                            portion_position: l.portion_position,
                            portion: v.portion.clone(),
                        })
                    })
                    .collect()
            }
        };
        let mut links = vec![];
        if let Some(after) = &change.after {
            let effective: crate::reconciliation::ReconciledReceiptEffect =
                resolve_value(after, &refs)?;
            for selection in &effective.selections {
                for portion in &selection.portions {
                    let previous = previous_links.iter().find(|l| {
                        l.selection_position == selection.position
                            && l.portion.source_key == portion.source_key
                    });
                    links.push(InclusionVersionRow {
                        id: id(),
                        link_id: previous.map(|l| l.link_id.clone()).unwrap_or_else(id),
                        previous_version_id: previous.map(|l| l.id.clone()),
                        reservation_id: id(),
                        observation_version_id: selection.observation_version_id.clone(),
                        selection_position: selection.position,
                        portion_position: portion.position,
                        portion: portion.clone(),
                    });
                }
            }
        }
        push(Projection::Inclusion {
            change: change.clone(),
            active_link_version_ids: links.iter().map(|l| l.id.clone()).collect(),
            links,
        });
    }
    for change in &plan.order_closures {
        push(Projection::OrderClosure {
            change: change.clone(),
            order_version_id: Some(id()),
        });
    }
    let record = CorrectionRecord {
        id: correction_id.clone(),
        original_id: plan.target.original_id.clone(),
        target_confirmation_id: target.into(),
        previous_version_id: old
            .records
            .iter()
            .filter(|r| r.original_id == plan.target.original_id)
            .max_by_key(|r| r.seq.parse::<i64>().unwrap_or(-1))
            .map(|r| r.id.clone()),
        confirmation_id: event.id.clone(),
        business_record_id: event.record_id.clone(),
        reason: data.reason.clone(),
        input: data.clone(),
        plan,
        resolved_refs: refs,
        recorded_at: time.into(),
        seq: sequence.to_string(),
    };
    connection.execute("INSERT INTO correction_records(id,original_id,target_confirmation_id,previous_version_id,confirmation_id,business_record_id,recorded_at,sequence,record_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![record.id,record.original_id,target,record.previous_version_id,event.id,event.record_id,time,sequence,json(&record)?]).map_err(unavailable)?;
    for row in &events {
        connection.execute("INSERT INTO correction_event_versions(id,correction_id,kind,original_id,original_confirmation_id,previous_version_id,previous_original_version_id,logical_sequence,logical_position,recorded_at,sequence,row_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",params![row.id,record.id,kind(row.reference.kind),row.reference.original_id,row.reference.original_confirmation_id,row.previous_version_id,row.previous_original_version_id,seq(&row.reference.logical_seq)?,i64::try_from(row.reference.logical_position).map_err(unavailable)?,time,sequence,json(row)?]).map_err(unavailable)?;
        connection.execute("INSERT INTO correction_event_heads(kind,original_id,current_version_id) VALUES(?1,?2,?3) ON CONFLICT(kind,original_id) DO UPDATE SET current_version_id=excluded.current_version_id",params![kind(row.reference.kind),row.reference.original_id,row.id]).map_err(unavailable)?;
        if let Some(CorrectionEventValue::SaleCount { input, result }) = &row.after {
            let previous: String = connection
                .query_row(
                    "SELECT current_version_id FROM sale_heads WHERE sale_id=?1",
                    [&row.reference.original_id],
                    |r| r.get(0),
                )
                .map_err(unavailable)?;
            let actual = row
                .sale_version_id
                .as_ref()
                .ok_or_else(|| unavailable("판매 실제 버전이 없습니다"))?;
            connection.execute("INSERT INTO sale_versions(id,sale_id,previous_version_id,confirmation_id,count,amount_micros,note) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![actual,row.reference.original_id,previous,event.id,input.count.parse::<i64>().map_err(unavailable)?,result.amount.as_deref().map(Money::parse).transpose().map_err(unavailable)?.map(|m|m.micros()),data.reason]).map_err(unavailable)?;
            connection
                .execute(
                    "UPDATE sale_heads SET current_version_id=?1 WHERE sale_id=?2",
                    params![actual, row.reference.original_id],
                )
                .map_err(unavailable)?;
        }
    }
    // Projection parents exist before their derived FK children.
    for p in &projections {
        connection.execute("INSERT INTO correction_projection_versions(id,correction_id,kind,subject_id,previous_version_id,recorded_at,sequence,row_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![p.id,record.id,p.projection.kind(),p.projection.subject_id(),p.previous_version_id,time,sequence,json(p)?]).map_err(unavailable)?;
        connection.execute("INSERT INTO correction_projection_heads(kind,subject_id,current_version_id) VALUES(?1,?2,?3) ON CONFLICT(kind,subject_id) DO UPDATE SET current_version_id=excluded.current_version_id",params![p.projection.kind(),p.projection.subject_id(),p.id]).map_err(unavailable)?;
    }
    for p in &projections {
        if let Projection::Observation {
            change,
            observation_version_id: Some(actual),
            price_basis_id,
        } = &p.projection
        {
            write_observation(
                connection,
                &record,
                p,
                change,
                actual,
                price_basis_id.as_deref(),
            )?;
        }
    }
    for p in &projections {
        match &p.projection {
            Projection::Material {
                change,
                holding_version_id,
                price_basis_id,
            } => write_material(
                connection,
                &record,
                p,
                change,
                holding_version_id.as_deref(),
                price_basis_id.as_deref(),
            )?,
            Projection::Allocation {
                change,
                consumption_version_id,
            } => write_allocation(
                connection,
                &record,
                p,
                change,
                consumption_version_id.as_deref(),
            )?,
            Projection::OrderClosure {
                change,
                order_version_id: Some(actual),
            } => write_order(connection, &record, p, change, actual)?,
            _ => {}
        }
    }
    Ok(())
}

fn write_observation(
    c: &Connection,
    r: &CorrectionRecord,
    p: &ProjectionVersionRow,
    change: &CorrectionObservationChange,
    actual: &str,
    price_basis_id: Option<&str>,
) -> Result<(), StoreError> {
    let mut effect = resolve_value(
        change
            .after
            .as_ref()
            .ok_or_else(|| unavailable("관측 정정 결과 없음"))?,
        &r.resolved_refs,
    )?;
    effect.after.source_id = None;
    if let (Some(actual), Some(price)) = (price_basis_id, &change.valuation_basis) {
        write_price(
            c,
            r,
            &effect.material_id,
            &effect.material_version_id,
            actual,
            price,
        )?;
        let valuation = match &mut effect.calculation {
            crate::stocktake::StocktakeCalculation::Initial { valuation, .. } => Some(valuation),
            crate::stocktake::StocktakeCalculation::Settlement { valuation, .. } => {
                valuation.as_mut()
            }
        };
        if let Some(crate::stocktake::StocktakeValuation::ExistingPrice { source_id, .. }) =
            valuation
        {
            *source_id = actual.into();
        } else {
            return Err(unavailable("관측 단가 근거의 계산 종류가 다릅니다"));
        }
    }
    let previous: Option<String> = c
        .query_row(
            "SELECT current_version_id FROM stocktake_heads WHERE observation_id=?1",
            [ref_id(&change.reference)],
            |row| row.get(0),
        )
        .optional()
        .map_err(unavailable)?;
    let observed_at: String = c
        .query_row(
            "SELECT b.recorded_at FROM stocktake_batches b WHERE b.id=?1",
            [&change.stocktake_id],
            |row| row.get(0),
        )
        .map_err(unavailable)?;
    let prevobs = effect.observation_before.as_ref().map(|o| o.id.clone());
    c.execute("INSERT INTO stocktake_versions(id,observation_id,previous_version_id,previous_observation_version_id,before_holding_version_id,effect_json,recorded_at,sequence,correction_projection_id,logical_sequence,observed_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![actual,ref_id(&change.reference),previous,prevobs,effect.before.as_ref().and_then(|h|h.source_id.as_deref()),json(&effect)?,r.recorded_at,seq(&r.seq)?,p.id,seq(&change.logical_seq)?,observed_at]).map_err(unavailable)?;
    c.execute("INSERT INTO stocktake_heads(observation_id,current_version_id) VALUES(?1,?2) ON CONFLICT(observation_id) DO UPDATE SET current_version_id=excluded.current_version_id",params![ref_id(&change.reference),actual]).map_err(unavailable)?;
    Ok(())
}

fn write_price(
    c: &Connection,
    r: &CorrectionRecord,
    material: &str,
    version: &str,
    id: &str,
    price: &CorrectionPriceBasis,
) -> Result<(), StoreError> {
    let basis = CorrectionPriceBasisRow {
        id: id.into(),
        correction_id: r.id.clone(),
        material_id: material.into(),
        material_version_id: version.into(),
        amount: price.amount.clone(),
        quantity: price.quantity.clone(),
        source: price.source.clone(),
        logical_seq: price.logical_seq.clone(),
        recorded_at: r.recorded_at.clone(),
        seq: r.seq.clone(),
    };
    c.execute("INSERT INTO correction_price_bases(id,correction_id,material_id,material_version_id,amount_micros,quantity_micros,logical_sequence,recorded_at,sequence,row_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![id,r.id,material,version,Money::parse(&price.amount).map_err(unavailable)?.micros(),Quantity::parse(&price.quantity).map_err(unavailable)?.micros(),seq(&price.logical_seq)?,r.recorded_at,seq(&r.seq)?,json(&basis)?]).map_err(unavailable)?;
    Ok(())
}

fn write_material(
    c: &Connection,
    r: &CorrectionRecord,
    p: &ProjectionVersionRow,
    change: &CorrectionMaterialChange,
    actual: Option<&str>,
    price_id: Option<&str>,
) -> Result<(), StoreError> {
    let previous: Option<String> = c
        .query_row(
            "SELECT current_version_id FROM holding_heads WHERE material_id=?1",
            [&change.material_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(unavailable)?
        .flatten();
    let CorrectionHoldingState::Known {
        amounts,
        last_valid_price,
        observation_boundary,
        initial_boundary,
        ..
    } = &change.after
    else {
        c.execute("INSERT INTO holding_heads(material_id,current_version_id,last_valid_version_id,unknown_correction_projection_id,last_valid_correction_price_id) VALUES(?1,NULL,NULL,?2,NULL) ON CONFLICT(material_id) DO UPDATE SET current_version_id=NULL,last_valid_version_id=NULL,unknown_correction_projection_id=excluded.unknown_correction_projection_id,last_valid_correction_price_id=NULL",params![change.material_id,p.id]).map_err(unavailable)?;
        return Ok(());
    };
    let actual = actual.ok_or_else(|| unavailable("정정 보유 버전 없음"))?;
    let quantity = Quantity::parse(&amounts.quantity).map_err(unavailable)?;
    let value = Money::parse(
        amounts
            .value
            .as_deref()
            .ok_or_else(|| unavailable("정정 보유 가치 없음"))?,
    )
    .map_err(unavailable)?;
    if let (Some(id), Some(price)) = (price_id, last_valid_price) {
        let basis = CorrectionPriceBasisRow {
            id: id.into(),
            correction_id: r.id.clone(),
            material_id: change.material_id.clone(),
            material_version_id: change.material_version_id.clone(),
            amount: price.amount.clone(),
            quantity: price.quantity.clone(),
            source: price.source.clone(),
            logical_seq: price.logical_seq.clone(),
            recorded_at: r.recorded_at.clone(),
            seq: r.seq.clone(),
        };
        c.execute("INSERT INTO correction_price_bases(id,correction_id,material_id,material_version_id,amount_micros,quantity_micros,logical_sequence,recorded_at,sequence,row_json) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![id,r.id,change.material_id,change.material_version_id,Money::parse(&price.amount).map_err(unavailable)?.micros(),Quantity::parse(&price.quantity).map_err(unavailable)?.micros(),seq(&price.logical_seq)?,r.recorded_at,seq(&r.seq)?,json(&basis)?]).map_err(unavailable)?;
    }
    let observation = observation_boundary
        .as_ref()
        .map(|o| resolve(&o.reference, &r.resolved_refs))
        .transpose()?;
    let logical = observation_boundary
        .as_ref()
        .map(|o| o.logical_seq.as_str())
        .or_else(|| initial_boundary.as_ref().map(|i| i.logical_seq.as_str()))
        .ok_or_else(|| unavailable("정정 계산 시작 기준이 없습니다"))?;
    c.execute("INSERT INTO holding_versions(id,material_id,material_version_id,previous_version_id,baseline_sequence,quantity_micros,value_micros,recorded_at,sequence,baseline_observation_version_id,correction_projection_id,baseline_correction_projection_id,last_valid_correction_price_id) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",params![actual,change.material_id,change.material_version_id,previous,seq(logical)?,quantity.micros(),value.micros(),r.recorded_at,seq(&r.seq)?,observation,p.id,observation.is_none().then_some(&p.id),price_id]).map_err(unavailable)?;
    c.execute("INSERT INTO holding_heads(material_id,current_version_id,last_valid_version_id,unknown_correction_projection_id,last_valid_correction_price_id) VALUES(?1,?2,?3,NULL,?4) ON CONFLICT(material_id) DO UPDATE SET current_version_id=excluded.current_version_id,last_valid_version_id=excluded.last_valid_version_id,unknown_correction_projection_id=NULL,last_valid_correction_price_id=excluded.last_valid_correction_price_id",params![change.material_id,actual,(quantity.micros()>0).then_some(actual),(quantity.micros()==0).then_some(price_id).flatten()]).map_err(unavailable)?;
    Ok(())
}

fn write_allocation(
    c: &Connection,
    r: &CorrectionRecord,
    p: &ProjectionVersionRow,
    change: &CorrectionAllocationChange,
    actual: Option<&str>,
) -> Result<(), StoreError> {
    let observation_id = ref_id(&change.observation);
    let Some(after) = &change.after else {
        c.execute(
            "DELETE FROM consumption_heads WHERE observation_id=?1",
            [observation_id],
        )
        .map_err(unavailable)?;
        return Ok(());
    };
    let actual = actual.ok_or_else(|| unavailable("정정 배분 버전 없음"))?;
    let a: crate::stocktake::ConsumptionAllocation = resolve_value(after, &r.resolved_refs)?;
    let previous: Option<String> = c
        .query_row(
            "SELECT current_version_id FROM consumption_heads WHERE observation_id=?1",
            [observation_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(unavailable)?;
    let cv = stocktake::ConsumptionVersion {
        id: actual.into(),
        observation_version_id: resolve(&change.observation, &r.resolved_refs)?,
        previous_version_id: previous,
        basis: a.basis.clone(),
        end_seq: a.end_seq.clone(),
        as_of_revision: a.as_of_revision.clone(),
        total_quantity: a.total_quantity.clone(),
        total_value: a.total_value.clone(),
        unallocated_quantity: a.unallocated_quantity.clone(),
        unallocated_value: a.unallocated_value.clone(),
        reason: a.reason,
        correction_projection_id: Some(p.id.clone()),
        seq: Some(r.seq.clone()),
    };
    c.execute("INSERT INTO consumption_versions(id,observation_version_id,previous_version_id,allocation_json,correction_projection_id,sequence) VALUES(?1,?2,?3,?4,?5,?6)",params![cv.id,cv.observation_version_id,cv.previous_version_id,json(&cv)?,p.id,seq(&r.seq)?]).map_err(unavailable)?;
    c.execute("INSERT INTO consumption_heads(observation_id,current_version_id) VALUES(?1,?2) ON CONFLICT(observation_id) DO UPDATE SET current_version_id=excluded.current_version_id",params![observation_id,actual]).map_err(unavailable)?;
    for row in a.candidates {
        c.execute("INSERT INTO consumption_rows(consumption_version_id,position,sale_id,sale_version_id,recipe_version_id,row_json) VALUES(?1,?2,?3,?4,?5,?6)",params![actual,i64::try_from(row.position).map_err(unavailable)?,row.sale_id,row.sale_version_id,row.recipe_version_id,json(&row)?]).map_err(unavailable)?;
    }
    Ok(())
}

fn write_order(
    c: &Connection,
    r: &CorrectionRecord,
    p: &ProjectionVersionRow,
    change: &CorrectionOrderClosureChange,
    actual: &str,
) -> Result<(), StoreError> {
    let prior = super::super::purchase_order::read_versions(
        c,
        Some(&change.order_id),
        Some(&change.expected_order_version_id),
        false,
    )?
    .pop()
    .ok_or_else(|| unavailable("정정 주문 버전 없음"))?;
    let status = if change.after.is_some() {
        "received"
    } else {
        "open"
    };
    c.execute("INSERT INTO purchase_order_versions(id,order_id,previous_version_id,confirmation_id,action,status,material_id,material_version_id,quantity_micros,unit,input_quantity,input_unit,supplier,expected_date,estimated_amount_micros,note,recorded_at,sequence,correction_projection_id) VALUES(?1,?2,?3,?4,'correct',?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",params![actual,prior.order_id,prior.id,r.confirmation_id,status,prior.material_id,prior.material_version_id,Quantity::parse(&prior.quantity).map_err(unavailable)?.micros(),prior.unit,prior.input_quantity,prior.input_unit,prior.supplier,prior.expected_date,prior.estimated_amount.as_deref().map(Money::parse).transpose().map_err(unavailable)?.map(|m|m.micros()),prior.note,r.recorded_at,seq(&r.seq)?,p.id]).map_err(unavailable)?;
    c.execute(
        "UPDATE purchase_order_heads SET current_version_id=?1 WHERE order_id=?2",
        params![actual, change.order_id],
    )
    .map_err(unavailable)?;
    Ok(())
}
