use super::*;
use crate::{
    holding::HoldingAmounts,
    reconciliation::{ValueEventRef, ValueMaterial, ValueObservation},
    stocktake::StocktakeValuation,
};

fn id(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}
fn observation(quantity: &str, value: &str) -> CorrectionReplayObservation {
    CorrectionReplayObservation {
        id: id(10),
        expected_version_id: id(11),
        stocktake_id: id(12),
        logical_seq: 1,
        reference: CorrectionPlanRef::Existing {
            entity: CorrectionRefKind::Observation,
            id: id(10),
            version_id: id(11),
        },
        effect: StocktakeRowEffect {
            position: 0,
            material_id: id(1),
            material_version_id: id(2),
            material_name: "가상 곡물".into(),
            unit: "g".into(),
            input_quantity: quantity.into(),
            input_unit: "g".into(),
            measured_quantity: quantity.into(),
            before: None,
            after: HoldingAmounts {
                quantity: quantity.into(),
                value: Some(value.into()),
                source_id: None,
            },
            basis_before: None,
            observation_before: None,
            calculation: StocktakeCalculation::Initial {
                value: value.into(),
                valuation: StocktakeValuation::ManualAmount {
                    amount: value.into(),
                },
            },
        },
    }
}
fn event_ref(n: u64, sequence: i64, kind: CorrectionSubjectKind) -> CorrectionEventRef {
    CorrectionEventRef {
        kind,
        original_id: id(n),
        original_confirmation_id: id(n + 100),
        confirmation_id: id(n + 100),
        version_id: id(n),
        business_record_id: id(n + 200),
        logical_seq: sequence.to_string(),
        logical_position: 0,
        original_recorded_at: "2026-01-01T00:00:00Z".into(),
        recorded_at: "2026-01-01T00:00:00Z".into(),
        seq: sequence.to_string(),
    }
}
fn receipt(
    n: u64,
    sequence: i64,
    received: &str,
    amount: &str,
    included: &str,
) -> CorrectionHistoryEvent {
    let data = DraftReceiptV1 {
        source: ReceiptSourceDraft::Direct {
            material_id: Some(id(1)),
            material_version_id: Some(id(2)),
        },
        quantity: received.into(),
        unit: "g".into(),
        amount: amount.into(),
        receipt_date: "2026-01-01".into(),
        note: "가상 입고".into(),
        stock_effect: ReceiptStockEffectDraft::Reconciled {
            selections: vec![crate::reconciliation::ReceiptInclusionSelection {
                observation_version_id: Some(id(11)),
                source: Some(InclusionSourceSelection::AllAvailable {}),
                included_quantity: included.into(),
            }],
        },
    };
    let input = CorrectionReplacementDraft::Receipt { data: data.clone() };
    CorrectionHistoryEvent {
        reference: event_ref(n, sequence, CorrectionSubjectKind::Receipt),
        original_input: input.clone(),
        effective_input: Some(input),
        effective_value: Some(CorrectionEventValue::Receipt {
            input: data,
            reconciliation: None,
        }),
        observations: vec![],
    }
}

struct Fixture {
    history: CorrectionHistory,
    events: Vec<CorrectionHistoryEvent>,
    topology: Vec<ValueReplayEvent>,
    observations: Vec<CorrectionReplayObservation>,
    changed: Vec<String>,
    changed_observations: Vec<String>,
    fields: Vec<(String, String)>,
}
impl Fixture {
    fn new(quantity: &str, value: &str) -> Self {
        let o = observation(quantity, value);
        let material = MaterialVersion {
            id: id(2),
            material_id: id(1),
            name: "가상 곡물".into(),
            unit: "g".into(),
            canonical_unit: "g".into(),
            dimension: "mass".into(),
            kind: "ingredient".into(),
            description: "".into(),
            archived: false,
            recorded_at: "2026-01-01T00:00:00Z".into(),
            seq: Some("0".into()),
            source: "test".into(),
        };
        let stocktake_input = CorrectionReplacementDraft::Stocktake {
            data: DraftStocktakeCorrectionV1 { rows: vec![] },
        };
        let stocktake = CorrectionHistoryEvent {
            reference: event_ref(12, 1, CorrectionSubjectKind::Stocktake),
            original_input: stocktake_input.clone(),
            effective_input: Some(stocktake_input),
            effective_value: None,
            observations: vec![StocktakeObservationRecord {
                id: id(10),
                version_id: id(11),
                previous_version_id: None,
                stocktake_id: id(12),
                previous_observation_version_id: None,
                consumption_version_id: None,
                effect: o.effect.clone(),
                recorded_at: "2026-01-01T00:00:00Z".into(),
                seq: "1".into(),
            }],
        };
        Self {
            history: CorrectionHistory {
                as_of_revision: 10,
                today: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                events: vec![stocktake.clone()],
                materials: vec![material],
                prices: vec![],
                orders: vec![],
                closures: vec![],
                sales: vec![],
                sale_versions: vec![],
                existing_inclusions: vec![],
                actual_materials: vec![],
                actual_allocations: vec![],
            },
            events: vec![stocktake],
            observations: vec![o],
            topology: vec![ValueReplayEvent::Observation {
                event: ValueEventRef { id: id(11), seq: 1 },
                observation_id: id(10),
                material: ValueMaterial {
                    material_id: id(1),
                    material_version_id: id(2),
                    unit: Unit::parse("g").unwrap(),
                },
                measured: q(quantity).unwrap(),
                calculation: ValueObservation::Initial {
                    value: money(value).unwrap(),
                },
            }],
            changed: vec![],
            changed_observations: vec![],
            fields: vec![],
        }
    }
    fn add_receipt(&mut self, n: u64, sequence: i64, received: &str, amount: &str, included: &str) {
        let event = receipt(n, sequence, received, amount, included);
        self.history.events.push(event.clone());
        self.events.push(event);
        self.changed.push(id(n));
        self.fields.push((id(n), "action.replacement.data".into()));
    }
    fn run(&self) -> CorrectionInclusionOutput {
        let mut result = resolve_correction_inclusions(&CorrectionInclusionInput {
            history: &self.history,
            events: &self.events,
            topology: &self.topology,
            observations: &self.observations,
            changed_receipt_ids: &self.changed,
            changed_observation_ids: &self.changed_observations,
            receipt_fields: &self.fields,
        })
        .unwrap();
        if !result.issues.is_empty() {
            return result;
        }
        let mut ordered = std::mem::take(&mut result.receipts);
        ordered.sort_by_key(|(id, _)| {
            self.events
                .iter()
                .find(|e| &e.reference.original_id == id)
                .unwrap()
                .reference
                .logical_seq
                .parse::<i64>()
                .unwrap()
        });
        for (id, effect) in ordered {
            let refreshed = refresh_correction_receipt_values(
                &self.history,
                &id,
                &effect,
                &self.observations,
                &result.receipts,
                &self.changed,
            )
            .unwrap();
            result.receipts.push((id, refreshed));
        }
        result.replacements.clear();
        let mut position = 0;
        for (_, effect) in &result.receipts {
            for selection in &effect.selections {
                for portion in &selection.portions {
                    result.replacements.push(ValueReplacement {
                        observation_version_id: selection.observation_version_id.clone(),
                        material_id: effect.material_id.clone(),
                        source_key: portion.source_key.clone(),
                        selection_position: position,
                        portion_position: portion.position,
                        booked_value: money(&portion.booked_value).unwrap(),
                        actual_included_value: money(&portion.actual_included_value).unwrap(),
                    });
                }
                position += 1;
            }
        }
        result.requires_value_refresh.clear();
        result
    }
    fn freeze(&mut self) {
        let output = self.run();
        assert!(output.issues.is_empty());
        for (receipt_id, effect) in output.receipts {
            self.history
                .existing_inclusions
                .push(CorrectionEffectiveInclusion {
                    receipt_id: receipt_id.clone(),
                    effective_confirmation_id: id(120),
                    reconciliation: ReceiptReconciliation {
                        id: id(40),
                        receipt_id,
                        confirmation_id: id(120),
                        business_record_id: id(220),
                        effect,
                        selection_results: vec![],
                        material_results: vec![],
                        recorded_at: "2026-01-01T00:00:00Z".into(),
                        seq: "2".into(),
                    },
                });
        }
        self.changed.clear();
        self.fields.clear();
    }
    fn revise_observation(&mut self, quantity: &str, value: &str) {
        self.observations[0] = observation(quantity, value);
        self.observations[0].reference = CorrectionPlanRef::Proposed {
            entity: CorrectionRefKind::Observation,
            id: id(10),
            expected_version_id: Some(id(11)),
        };
        self.changed_observations = vec![id(10)];
        if let ValueReplayEvent::Observation {
            measured,
            calculation,
            ..
        } = &mut self.topology[0]
        {
            *measured = q(quantity).unwrap();
            *calculation = ValueObservation::Initial {
                value: money(value).unwrap(),
            };
        }
    }
}

#[test]
fn explicit_link_recalculation_uses_observation_value_before_its_own_old_difference() {
    let mut fixture = Fixture::new("10", "10");
    fixture.add_receipt(20, 2, "3", "6", "3");
    fixture.freeze();
    assert_eq!(
        fixture.history.existing_inclusions[0]
            .reconciliation
            .effect
            .value_difference,
        "3"
    );
    fixture.revise_observation("10", "20");
    fixture.changed.push(id(20));
    let result = fixture.run();
    assert!(result.issues.is_empty());
    assert_eq!(result.receipts[0].1.booked_value, "6");
    assert_eq!(result.receipts[0].1.included_amount, "6");
    assert_eq!(result.receipts[0].1.value_difference, "0");
    assert_eq!(result.replacements.len(), 1);
}

#[test]
fn unchanged_link_keeps_frozen_amounts_when_upstream_valuation_changes() {
    let mut fixture = Fixture::new("10", "10");
    fixture.add_receipt(20, 2, "3", "6", "3");
    fixture.freeze();
    fixture.revise_observation("10", "20");
    let result = fixture.run();
    assert!(result.issues.is_empty());
    assert_eq!(result.receipts[0].1.booked_value, "3");
    assert_eq!(result.receipts[0].1.value_difference, "3");
    assert_eq!(result.replacements[0].booked_value.decimal(), "3");
}

#[test]
fn source_shrink_requires_explicit_related_receipt_review_and_shows_remaining_capacity() {
    let mut fixture = Fixture::new("10", "10");
    fixture.add_receipt(20, 2, "3", "6", "3");
    fixture.freeze();
    fixture.revise_observation("2", "2");
    let result = fixture.run();
    assert!(!result.issues.is_empty());
    assert!(result.replacements.is_empty());
    let CorrectionReviewRequest::ReceiptInclusions {
        receipt_id,
        candidates,
        ..
    } = &result.review_requests[0]
    else {
        panic!()
    };
    assert_eq!(receipt_id, &id(20));
    assert_eq!(candidates[0].available_quantity, "2");
}

#[test]
fn explicit_reduced_link_uses_new_source_share_and_only_unincluded_actual_inflow() {
    let mut fixture = Fixture::new("10", "10");
    fixture.add_receipt(20, 2, "3", "6", "3");
    fixture.freeze();
    fixture.revise_observation("2", "2");
    fixture.events[1] = receipt(20, 2, "3", "6", "2");
    fixture.changed.push(id(20));
    fixture.topology.push(ValueReplayEvent::Inflow {
        event: ValueEventRef { id: id(20), seq: 2 },
        material: ValueMaterial {
            material_id: id(1),
            material_version_id: id(2),
            unit: Unit::parse("g").unwrap(),
        },
        quantity: q("1").unwrap(),
        amount: money("2").unwrap(),
    });
    let result = fixture.run();
    assert!(result.issues.is_empty());
    let effect = &result.receipts[0].1;
    assert_eq!(effect.included_quantity, "2");
    assert_eq!(effect.unreflected_quantity, "1");
    assert_eq!(effect.included_amount, "4");
    assert_eq!(effect.unreflected_amount, "2");
    assert_eq!(effect.booked_value, "2");
}

#[test]
fn void_receipt_removes_effective_reservation_without_erasing_original_record() {
    let mut fixture = Fixture::new("10", "10");
    fixture.add_receipt(20, 2, "3", "6", "3");
    fixture.freeze();
    fixture.events[1].effective_input = None;
    fixture.events[1].effective_value = None;
    fixture.changed.push(id(20));
    let result = fixture.run();
    assert!(result.issues.is_empty());
    assert!(result.replacements.is_empty());
    assert!(result.receipts.is_empty());
    assert_eq!(
        fixture.history.existing_inclusions[0]
            .reconciliation
            .effect
            .booked_value,
        "3"
    );
}

#[test]
fn replaced_receipts_carry_exact_remainders_in_their_original_logical_order() {
    let mut fixture = Fixture::new("3", "0.000001");
    fixture.add_receipt(20, 2, "1", "0", "1");
    fixture.add_receipt(21, 3, "1", "0", "1");
    let result = fixture.run();
    assert!(result.issues.is_empty());
    assert_eq!(result.receipts[0].1.booked_value, "0");
    assert_eq!(result.receipts[1].1.booked_value, "0.000001");
    assert_eq!(
        result
            .replacements
            .iter()
            .map(|r| r.booked_value.micros())
            .sum::<i64>(),
        1
    );
    assert_ne!(
        result.replacements[0].selection_position,
        result.replacements[1].selection_position
    );
}

#[test]
fn explicit_zero_inclusion_has_no_reservation_and_blank_is_an_error() {
    let mut fixture = Fixture::new("10", "10");
    fixture.add_receipt(20, 2, "1", "2", "0");
    fixture.topology.push(ValueReplayEvent::Inflow {
        event: ValueEventRef { id: id(20), seq: 2 },
        material: ValueMaterial {
            material_id: id(1),
            material_version_id: id(2),
            unit: Unit::parse("g").unwrap(),
        },
        quantity: q("1").unwrap(),
        amount: money("2").unwrap(),
    });
    let result = fixture.run();
    assert!(result.issues.is_empty());
    assert!(result.replacements.is_empty());
    assert_eq!(result.receipts[0].1.unreflected_quantity, "1");
    fixture.events[1] = receipt(20, 2, "1", "2", "");
    let result = fixture.run();
    assert!(
        result
            .issues
            .iter()
            .any(|i| i.field.ends_with("included_quantity"))
    );
}

#[test]
fn a_receipt_cannot_use_its_own_future_observation_as_inclusion_evidence() {
    let mut fixture = Fixture::new("10", "10");
    fixture.add_receipt(20, 1, "1", "2", "1");
    let result = fixture.run();
    assert!(!result.issues.is_empty());
    assert!(result.replacements.is_empty());
    let CorrectionReviewRequest::ReceiptInclusions { candidates, .. } = &result.review_requests[0]
    else {
        panic!()
    };
    assert!(candidates.is_empty());
}

#[test]
fn provenance_pass_marks_edited_values_pending_and_refresh_uses_then_known_value() {
    let mut fixture = Fixture::new("10", "10");
    fixture.add_receipt(20, 2, "3", "6", "3");
    let provisional = resolve_correction_inclusions(&CorrectionInclusionInput {
        history: &fixture.history,
        events: &fixture.events,
        topology: &fixture.topology,
        observations: &fixture.observations,
        changed_receipt_ids: &fixture.changed,
        changed_observation_ids: &fixture.changed_observations,
        receipt_fields: &fixture.fields,
    })
    .unwrap();
    assert_eq!(provisional.requires_value_refresh, vec![id(20)]);
    assert_eq!(provisional.receipts[0].1.booked_value, "0");
    // The known-value caller has now reached this observation with an earlier
    // confirmed value correction. The new receipt's own D is still absent.
    let known = vec![observation("10", "13")];
    let value = refresh_correction_receipt_values(
        &fixture.history,
        &id(20),
        &provisional.receipts[0].1,
        &known,
        &[],
        &fixture.changed,
    )
    .unwrap();
    assert_eq!(value.booked_value, "3.9");
    assert_eq!(value.value_difference, "2.1");
    assert_eq!(
        value.selections[0].portions[0].reserved_source_share,
        provisional.receipts[0].1.selections[0].portions[0].reserved_source_share
    );
}

#[test]
fn refresh_rejects_its_own_old_effect_as_a_prior_value_basis() {
    let mut fixture = Fixture::new("10", "10");
    fixture.add_receipt(20, 2, "3", "6", "3");
    let output = fixture.run();
    assert!(
        refresh_correction_receipt_values(
            &fixture.history,
            &id(20),
            &output.receipts[0].1,
            &fixture.observations,
            &output.receipts,
            &fixture.changed
        )
        .is_err()
    );
}
