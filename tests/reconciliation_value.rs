use sunchang_hansang::{
    domain::{Money, Quantity, Unit},
    production::AllocationPercent,
    reconciliation::*,
};

fn id(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}
fn q(s: &str) -> Quantity {
    Quantity::parse(s).unwrap()
}
fn v(s: &str) -> Money {
    Money::parse(s).unwrap()
}
fn material(n: u64) -> ValueMaterial {
    ValueMaterial {
        material_id: id(n),
        material_version_id: id(n + 100),
        unit: Unit::parse("g").unwrap(),
    }
}
fn event(n: i64) -> ValueEventRef {
    ValueEventRef {
        id: id(n as u64 + 1000),
        seq: n,
    }
}
fn initial(n: i64, quantity: &str, value: &str) -> ValueReplayEvent {
    ValueReplayEvent::Observation {
        event: event(n),
        observation_id: id(n as u64 + 2000),
        material: material(1),
        measured: q(quantity),
        calculation: ValueObservation::Initial { value: v(value) },
    }
}
fn observed(
    n: i64,
    measured: &str,
    consumption: &str,
    excess: &str,
    excess_value: &str,
) -> ValueReplayEvent {
    ValueReplayEvent::Observation {
        event: event(n),
        observation_id: id(n as u64 + 2000),
        material: material(1),
        measured: q(measured),
        calculation: ValueObservation::Settlement {
            consumption: q(consumption),
            excess: q(excess),
            excess_value: v(excess_value),
        },
    }
}
fn replacement(anchor: i64, booked: &str, actual: &str) -> ValueReplacement {
    ValueReplacement {
        observation_version_id: event(anchor).id,
        material_id: id(1),
        source_key: SourceKey {
            kind: SourceKind::ObservationInitial,
            id: id(2001),
            position: 0,
        },
        selection_position: 0,
        portion_position: 0,
        booked_value: v(booked),
        actual_included_value: v(actual),
    }
}
fn head(n: u64, quantity: &str, value: &str) -> ValueActualHead {
    ValueActualHead {
        material: material(n),
        quantity: q(quantity),
        value: v(value),
        holding_version_id: id(n + 3000),
    }
}
fn history(events: Vec<ValueReplayEvent>, quantity: &str, value: &str) -> ValueReplayInput {
    ValueReplayInput {
        at_revision: 100,
        events,
        effective: vec![],
        actual_heads: vec![head(1, quantity, value)],
    }
}

#[test]
fn receipt_money_is_split_in_two_stages_and_zero_inclusion_is_explicit() {
    let split = split_receipt_amount(q("6"), v("9000"), &[q("1"), q("2")]).unwrap();
    assert_eq!(split.included_amount.decimal(), "4500");
    assert_eq!(split.unreflected_quantity.decimal(), "3");
    assert_eq!(
        split
            .selection_amounts
            .iter()
            .map(|m| m.decimal())
            .collect::<Vec<_>>(),
        ["1500", "3000"]
    );
    let empty = split_receipt_amount(q("2"), v("0"), &[]).unwrap();
    assert_eq!(empty.unreflected_quantity.decimal(), "2");
    assert_eq!(empty.included_quantity.decimal(), "0");
    assert!(split_receipt_amount(q("0"), v("0"), &[]).is_err());
    assert!(split_receipt_amount(q("1"), v("1"), &[q("2")]).is_err());
    let tiny = split_receipt_amount(q("3"), v("0.000001"), &[q("1"), q("1")]).unwrap();
    assert_eq!(tiny.included_amount.decimal(), "0.000001");
    assert_eq!(tiny.selection_amounts[0].decimal(), "0.000001");
    assert_eq!(tiny.selection_amounts[1].decimal(), "0");
}

#[test]
fn normal_observation_replacement_changes_value_without_inventing_consumption() {
    let input = history(
        vec![initial(1, "10", "10000"), observed(2, "10", "0", "0", "0")],
        "10",
        "10000",
    );
    let result = replay_receipt_values(&input, &[replacement(2, "3000", "6000")]).unwrap();
    assert_eq!(result.materials[0].after_value.decimal(), "13000");
    assert_eq!(result.materials[0].quantity.decimal(), "10");
    assert_eq!(result.separate_adjustment.decimal(), "0");
}

#[test]
fn later_consumption_splits_positive_and_negative_differences() {
    let input = history(
        vec![initial(1, "10", "10000"), observed(2, "6", "4", "0", "0")],
        "6",
        "6000",
    );
    for (actual, expected_value, expected_separate) in
        [("6000", "7800", "1200"), ("0", "4200", "-1200")]
    {
        let result = replay_receipt_values(&input, &[replacement(1, "3000", actual)]).unwrap();
        assert_eq!(result.materials[0].after_value.decimal(), expected_value);
        assert_eq!(result.separate_adjustment.decimal(), expected_separate);
    }
}

#[test]
fn prior_replacement_is_replayed_once_and_later_observation_bv_stays_frozen() {
    let mut input = history(
        vec![initial(1, "10", "10"), observed(2, "5", "5", "0", "0")],
        "5",
        "7.5",
    );
    input.effective.push(EffectiveValueReplacement {
        link_id: id(7001),
        link_version_id: id(7002),
        value: replacement(1, "5", "10"),
    });
    let result = replay_receipt_values(&input, &[replacement(2, "2.5", "5")]).unwrap();
    assert_eq!(result.materials[0].before_value.decimal(), "7.5");
    assert_eq!(result.materials[0].after_value.decimal(), "10");
    assert_eq!(result.separate_adjustment.decimal(), "0");
    input.events.push(observed(3, "5", "0", "0", "0"));
    let later = replay_receipt_values(&input, &[replacement(3, "3.75", "5")]).unwrap();
    assert_eq!(later.materials[0].after_value.decimal(), "8.75");
}

#[test]
fn prior_unreflected_amount_is_only_a_normal_inflow_at_confirmation_time() {
    let mut input = history(
        vec![
            initial(1, "10", "10"),
            observed(2, "5", "5", "0", "0"),
            ValueReplayEvent::Inflow {
                event: event(3),
                material: material(1),
                quantity: q("2"),
                amount: v("4"),
            },
        ],
        "7",
        "11.5",
    );
    input.effective.push(EffectiveValueReplacement {
        link_id: id(7001),
        link_version_id: id(7002),
        value: replacement(1, "5", "10"),
    });
    let result = replay_receipt_values(&input, &[replacement(2, "2.5", "5")]).unwrap();
    assert_eq!(result.materials[0].after_value.decimal(), "14");
    assert_eq!(result.materials[0].quantity.decimal(), "7");
}

#[test]
fn already_confirmed_excess_value_is_not_repriced() {
    let input = history(
        vec![initial(1, "10", "10"), observed(2, "15", "0", "5", "5")],
        "15",
        "15",
    );
    let result = replay_receipt_values(&input, &[replacement(1, "5", "10")]).unwrap();
    assert_eq!(result.materials[0].after_value.decimal(), "20");
    assert_eq!(result.value_difference.decimal(), "5");
    assert_eq!(result.separate_adjustment.decimal(), "0");
}

#[test]
fn partial_waste_retains_value_and_full_waste_closes_the_difference() {
    for (waste, left, base, expected, separate) in [
        ("2", "8", "10000", "13000", "0"),
        ("10", "0", "0", "0", "3000"),
    ] {
        let input = history(
            vec![
                initial(1, "10", "10000"),
                ValueReplayEvent::Waste {
                    event: event(2),
                    material: material(1),
                    quantity: q(waste),
                },
            ],
            left,
            base,
        );
        let result = replay_receipt_values(&input, &[replacement(1, "3000", "6000")]).unwrap();
        assert_eq!(result.materials[0].after_value.decimal(), expected);
        assert_eq!(result.separate_adjustment.decimal(), separate);
    }
}

fn production(n: i64, output_percentages: &[&str]) -> ValueReplayEvent {
    ValueReplayEvent::Production {
        event: event(n),
        inputs: vec![ValueProductionInput {
            position: 0,
            material: material(1),
            quantity: q("2"),
        }],
        outputs: output_percentages
            .iter()
            .enumerate()
            .map(|(p, percent)| ValueProductionOutput {
                position: p,
                material: material(2 + p as u64),
                quantity: q("2"),
                allocation: AllocationPercent::parse(percent).unwrap(),
            })
            .collect(),
    }
}

#[test]
fn production_moves_difference_and_keeps_zero_percent_outputs() {
    let mut input = history(
        vec![initial(1, "10", "10000"), production(2, &["100", "0"])],
        "8",
        "8000",
    );
    input
        .actual_heads
        .extend([head(2, "2", "2000"), head(3, "2", "0")]);
    let result = replay_receipt_values(&input, &[replacement(1, "3000", "6000")]).unwrap();
    assert_eq!(
        result
            .materials
            .iter()
            .map(|m| m.after_value.decimal())
            .collect::<Vec<_>>(),
        ["10400", "2600", "0"]
    );
    assert_eq!(result.separate_adjustment.decimal(), "0");
}

#[test]
fn grouped_production_rounding_and_multiple_outputs_conserve_one_micro() {
    let p = ValueReplayEvent::Production {
        event: event(2),
        inputs: vec![
            ValueProductionInput {
                position: 0,
                material: material(1),
                quantity: q("0.5"),
            },
            ValueProductionInput {
                position: 1,
                material: material(1),
                quantity: q("0.5"),
            },
        ],
        outputs: vec![
            ValueProductionOutput {
                position: 0,
                material: material(2),
                quantity: q("1"),
                allocation: AllocationPercent::parse("70").unwrap(),
            },
            ValueProductionOutput {
                position: 1,
                material: material(3),
                quantity: q("1"),
                allocation: AllocationPercent::parse("30").unwrap(),
            },
        ],
    };
    let mut input = history(vec![initial(1, "1", "0.000001"), p], "0", "0");
    input
        .actual_heads
        .extend([head(2, "1", "0.000001"), head(3, "1", "0")]);
    let result = replay_receipt_values(&input, &[replacement(1, "0.000001", "0.000002")]).unwrap();
    assert_eq!(
        result
            .materials
            .iter()
            .map(|m| m.after_value.decimal())
            .collect::<Vec<_>>(),
        ["0", "0.000001", "0.000001"]
    );
    assert_eq!(result.separate_adjustment.decimal(), "0");
}

#[test]
fn base_mismatch_missing_anchor_duplicate_links_and_negative_values_are_errors() {
    let mut input = history(vec![initial(1, "10", "10")], "10", "11");
    assert!(matches!(
        replay_receipt_values(&input, &[]),
        Err(ValueReplayError::InvalidHistory(_))
    ));
    input.actual_heads[0].value = v("10");
    assert!(replay_receipt_values(&input, &[replacement(99, "1", "2")]).is_err());
    assert!(matches!(
        replay_receipt_values(&input, &[replacement(1, "11", "0")]),
        Err(ValueReplayError::NegativeValue(_))
    ));
    let old = EffectiveValueReplacement {
        link_id: id(7001),
        link_version_id: id(7002),
        value: replacement(1, "0", "0"),
    };
    input.effective = vec![old.clone(), old];
    assert!(replay_receipt_values(&input, &[]).is_err());
}

#[test]
fn explicit_empty_history_and_signed_money_bounds_are_checked() {
    let input = ValueReplayInput {
        at_revision: 0,
        events: vec![],
        effective: vec![],
        actual_heads: vec![],
    };
    assert_eq!(
        replay_receipt_values(&input, &[])
            .unwrap()
            .value_difference
            .decimal(),
        "0"
    );
    assert_eq!(ValueDelta::from_micros(-1).unwrap().decimal(), "-0.000001");
    assert!(ValueDelta::from_micros(i64::MIN).is_err());
    let input = history(vec![initial(1, "1", "9000000000000")], "1", "9000000000000");
    assert!(replay_receipt_values(&input, &[replacement(1, "0", "0.000001")]).is_err());
}

#[test]
fn new_nested_selector_is_strict_and_incomplete_drafts_are_preserved() {
    let valid = serde_json::json!({"observation_version_id":null,"source":{"kind":"all_available"},"included_quantity":"abc"});
    let mut rows: Vec<ReceiptInclusionSelection> = vec![serde_json::from_value(valid).unwrap()];
    normalize_receipt_inclusions(&mut rows).unwrap();
    assert_eq!(rows[0].included_quantity, "abc");
    assert!(
        serde_json::from_value::<InclusionSourceSelection>(
            serde_json::json!({"kind":"all_available","extra":1})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<ReceiptInclusionSelection>(
            serde_json::json!({"observation_version_id":null,"source":null,"included_quantity":1})
        )
        .is_err()
    );
    let row = rows[0].clone();
    assert!(normalize_receipt_inclusions(&mut vec![row; 21]).is_err());
}

#[test]
fn multi_stage_production_and_final_waste_follow_only_the_remaining_value() {
    let mut liquid = material(3);
    liquid.unit = Unit::parse("mL").unwrap();
    let second = ValueReplayEvent::Production {
        event: event(3),
        inputs: vec![ValueProductionInput {
            position: 0,
            material: material(2),
            quantity: q("1"),
        }],
        outputs: vec![ValueProductionOutput {
            position: 0,
            material: liquid.clone(),
            quantity: q("3"),
            allocation: AllocationPercent::parse("100").unwrap(),
        }],
    };
    let mut input = history(
        vec![
            initial(1, "10", "10000"),
            production(2, &["100"]),
            second,
            ValueReplayEvent::Waste {
                event: event(4),
                material: liquid.clone(),
                quantity: q("3"),
            },
        ],
        "8",
        "8000",
    );
    input.actual_heads.push(head(2, "1", "1000"));
    input.actual_heads.push(ValueActualHead {
        material: liquid,
        quantity: q("0"),
        value: v("0"),
        holding_version_id: id(3003),
    });
    let result = replay_receipt_values(&input, &[replacement(1, "3000", "6000")]).unwrap();
    assert_eq!(
        result
            .materials
            .iter()
            .map(|m| m.after_value.decimal())
            .collect::<Vec<_>>(),
        ["10400", "1300", "0"]
    );
    assert_eq!(result.separate_adjustment.decimal(), "300");
}

#[test]
fn same_material_output_cannot_cover_input_shortage_and_same_anchor_links_are_additive() {
    let input = history(vec![initial(1, "10", "10")], "10", "10");
    let one = replacement(1, "1", "2");
    let mut two = replacement(1, "2", "4");
    two.selection_position = 1;
    let result = replay_receipt_values(&input, &[one, two]).unwrap();
    assert_eq!(result.materials[0].after_value.decimal(), "13");
    let mut invalid = history(vec![initial(1, "1", "10")], "1", "10");
    invalid.events.push(ValueReplayEvent::Production {
        event: event(2),
        inputs: vec![ValueProductionInput {
            position: 0,
            material: material(1),
            quantity: q("2"),
        }],
        outputs: vec![ValueProductionOutput {
            position: 0,
            material: material(1),
            quantity: q("3"),
            allocation: AllocationPercent::parse("100").unwrap(),
        }],
    });
    assert!(replay_receipt_values(&invalid, &[]).is_err());
}
