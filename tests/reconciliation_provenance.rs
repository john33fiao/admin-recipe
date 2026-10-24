use num_bigint::BigInt;
use sunchang_hansang::{
    domain::{Money, Quantity, Unit},
    production::AllocationPercent,
    reconciliation::{
        InclusionSourceSelection, SourceKey, SourceKind, SourceShare, ValueEventRef, ValueMaterial,
        ValueObservation, ValueProductionInput, ValueProductionOutput, ValueReplayEvent,
        provenance::{
            InclusionRequest, ProvenanceError, ReservedInclusion, allocate_share_amount,
            inclusion_availability, resolve_inclusions,
        },
    },
};

fn q(s: &str) -> Quantity {
    Quantity::parse(s).unwrap()
}
fn money(s: &str) -> Money {
    Money::parse(s).unwrap()
}
fn material(id: &str, unit: &str) -> ValueMaterial {
    ValueMaterial {
        material_id: id.into(),
        material_version_id: format!("{id}-v1"),
        unit: Unit::parse(unit).unwrap(),
    }
}
fn event(id: &str, seq: i64) -> ValueEventRef {
    ValueEventRef { id: id.into(), seq }
}
fn initial(id: &str, seq: i64, m: &ValueMaterial, qty: &str) -> ValueReplayEvent {
    ValueReplayEvent::Observation {
        event: event(id, seq),
        observation_id: format!("{id}-stable"),
        material: m.clone(),
        measured: q(qty),
        calculation: ValueObservation::Initial { value: money("0") },
    }
}
fn observe(
    id: &str,
    seq: i64,
    m: &ValueMaterial,
    qty: &str,
    consumption: &str,
) -> ValueReplayEvent {
    ValueReplayEvent::Observation {
        event: event(id, seq),
        observation_id: format!("{id}-stable"),
        material: m.clone(),
        measured: q(qty),
        calculation: ValueObservation::Settlement {
            consumption: q(consumption),
            excess: q("0"),
            excess_value: money("0"),
        },
    }
}
fn request(observation: &str, m: &ValueMaterial, qty: &str) -> InclusionRequest {
    InclusionRequest {
        position: 0,
        observation_version_id: observation.into(),
        material_id: m.material_id.clone(),
        source: InclusionSourceSelection::AllAvailable {},
        quantity: q(qty),
    }
}
fn fixed(
    events: &[ValueReplayEvent],
    existing: &[ReservedInclusion],
    request: InclusionRequest,
    id: &str,
) -> ReservedInclusion {
    let resolved = resolve_inclusions(events, existing, std::slice::from_ref(&request))
        .unwrap()
        .remove(0);
    ReservedInclusion {
        id: id.into(),
        observation_version_id: request.observation_version_id,
        material_id: request.material_id,
        quantity: request.quantity,
        portions: resolved.portions,
    }
}
fn source_for(initial: &str) -> SourceKey {
    SourceKey {
        kind: SourceKind::ObservationInitial,
        id: format!("{initial}-stable"),
        position: 0,
    }
}

#[test]
fn purchase_return_reduces_live_shares_without_refunding_reserved_origin() {
    let m = material("return-material", "g");
    let events = vec![
        initial("o1", 1, &m, "10"),
        ValueReplayEvent::PurchaseReturn {
            event: event("return", 2),
            material: m.clone(),
            quantity: q("4"),
        },
        observe("o2", 3, &m, "6", "0"),
    ];
    let reserved = fixed(&events, &[], request("o1", &m, "5"), "reserved");
    let choices =
        inclusion_availability(&events, std::slice::from_ref(&reserved), &m.material_id).unwrap();
    assert_eq!(choices[1].available_quantity.decimal(), "3");
    assert_eq!(choices[1].sources[0].remaining_share.numerator, "5");
    assert!(resolve_inclusions(&events, &[reserved], &[request("o2", &m, "3.000001")]).is_err());
}
fn produce(
    id: &str,
    seq: i64,
    inputs: Vec<(ValueMaterial, &str)>,
    outputs: Vec<(ValueMaterial, &str, &str)>,
) -> ValueReplayEvent {
    ValueReplayEvent::Production {
        event: event(id, seq),
        inputs: inputs
            .into_iter()
            .enumerate()
            .map(|(position, (material, qty))| ValueProductionInput {
                position,
                material,
                quantity: q(qty),
            })
            .collect(),
        outputs: outputs
            .into_iter()
            .enumerate()
            .map(|(position, (material, qty, pct))| ValueProductionOutput {
                position,
                material,
                quantity: q(qty),
                allocation: AllocationPercent::parse(pct).unwrap(),
            })
            .collect(),
    }
}

#[test]
fn an_old_reservation_propagates_through_a_later_normal_observation() {
    let m = material("a", "g");
    let events = vec![initial("o1", 1, &m, "10"), observe("o2", 2, &m, "5", "5")];
    let reserved = fixed(&events, &[], request("o1", &m, "5"), "l1");
    assert!(
        resolve_inclusions(
            &events,
            std::slice::from_ref(&reserved),
            &[request("o2", &m, "3")]
        )
        .is_err()
    );
    let second = resolve_inclusions(
        &events,
        std::slice::from_ref(&reserved),
        &[request("o2", &m, "2.5")],
    )
    .unwrap();
    assert_eq!(second[0].portions[0].included_share.numerator, "5");
    assert_eq!(second[0].portions[0].included_share.denominator, "2");
    let choices = inclusion_availability(&events, &[reserved], "a").unwrap();
    assert_eq!(choices[1].available_quantity.decimal(), "2.5");
    assert_eq!(choices[1].sources[0].remaining_share.numerator, "5");
}

#[test]
fn a_new_older_reservation_cannot_intrude_into_a_fixed_later_one() {
    let m = material("a", "g");
    let events = vec![initial("o1", 1, &m, "10"), observe("o2", 2, &m, "5", "5")];
    let reserved = fixed(&events, &[], request("o2", &m, "5"), "later");
    assert!(matches!(
        resolve_inclusions(
            &events,
            std::slice::from_ref(&reserved),
            &[request("o1", &m, "0.000001")]
        ),
        Err(ProvenanceError::InsufficientShare(_))
    ));
    assert_eq!(
        inclusion_availability(&events, &[reserved], "a").unwrap()[0]
            .available_quantity
            .decimal(),
        "0"
    );
}

#[test]
fn consumed_stock_still_accepts_a_historical_inclusion_but_never_refunds_its_quota() {
    let m = material("a", "g");
    let events = vec![initial("o1", 1, &m, "10"), observe("o2", 2, &m, "0", "10")];
    let reserved = fixed(&events, &[], request("o1", &m, "10"), "full");
    let choices = inclusion_availability(&events, std::slice::from_ref(&reserved), "a").unwrap();
    assert_eq!(choices[0].available_quantity.decimal(), "0");
    assert_eq!(choices[0].sources[0].reserved_share.numerator, "10");
    assert_eq!(choices[0].sources[0].remaining_share.numerator, "0");
    assert!(resolve_inclusions(&events, &[reserved], &[request("o1", &m, "1")]).is_err());
}

#[test]
fn multiple_proposed_anchors_are_checked_together_in_original_order() {
    let m = material("a", "g");
    let events = vec![initial("o1", 1, &m, "10"), observe("o2", 2, &m, "5", "5")];
    let old = request("o1", &m, "5");
    let mut new = request("o2", &m, "3");
    new.position = 1;
    assert!(resolve_inclusions(&events, &[], &[new.clone(), old.clone()]).is_err());
    assert!(resolve_inclusions(&events, &[], &[old.clone(), new.clone()]).is_err());
    new.quantity = q("2.5");
    assert_eq!(
        resolve_inclusions(&events, &[], &[new, old]).unwrap().len(),
        2
    );
}

#[test]
fn submicro_sources_can_be_selected_as_one_observation_pool() {
    let a = material("a", "g");
    let b = material("b", "g");
    let c = material("c", "개");
    let d = material("d", "개");
    let events = vec![
        initial("oa", 1, &a, "0.000001"),
        initial("ob", 2, &b, "0.000001"),
        produce(
            "p",
            3,
            vec![(a, "0.000001"), (b, "0.000001")],
            vec![(c.clone(), "0.000001", "100"), (d.clone(), "0.000001", "0")],
        ),
        observe("oc", 4, &c, "0.000001", "0"),
        observe("od", 5, &d, "0.000001", "0"),
    ];
    let choices = inclusion_availability(&events, &[], "d").unwrap();
    assert_eq!(choices[0].available_quantity.decimal(), "0.000001");
    assert!(
        choices[0]
            .sources
            .iter()
            .all(|s| s.available_quantity.micros() == 0)
    );
    let reserved = fixed(
        &events,
        &[],
        request("od", &d, "0.000001"),
        "zero_cost_output",
    );
    assert_eq!(reserved.portions.len(), 2);
    assert!(reserved.portions.iter().all(|p| p.included_share.numerator=="1" && p.included_share.denominator=="2000000"));
    assert!(resolve_inclusions(&events, &[reserved], &[request("oc", &c, "0.000001")]).is_ok());
}

#[test]
fn production_uses_commensurable_quantities_and_does_not_weight_sources_by_cost() {
    let a = material("a", "g");
    let b = material("b", "g");
    let c = material("c", "g");
    let mut events = vec![
        initial("oa", 1, &a, "1000"),
        initial("ob", 2, &b, "0.000001"),
    ];
    let reserved = fixed(&events, &[], request("oa", &a, "1000"), "all_a");
    events.push(produce(
        "p",
        3,
        vec![(a, "1000"), (b, "0.000001")],
        vec![(c.clone(), "1000", "100")],
    ));
    events.push(observe("oc", 4, &c, "1000", "0"));
    let choices = inclusion_availability(&events, &[reserved], "c").unwrap();
    assert_eq!(choices[0].available_quantity.decimal(), "0"); // less than one micro, not 500g
}

#[test]
fn different_units_and_zero_percent_outputs_preserve_original_source_quota() {
    let a = material("a", "g");
    let b = material("b", "mL");
    let c = material("c", "개");
    let d = material("d", "개");
    let events = vec![
        initial("oa", 1, &a, "4"),
        initial("ob", 2, &b, "8"),
        produce(
            "p",
            3,
            vec![(a.clone(), "4"), (b, "8")],
            vec![(c.clone(), "4", "100"), (d.clone(), "12", "0")],
        ),
        observe("oc", 4, &c, "4", "0"),
        observe("od", 5, &d, "12", "0"),
    ];
    let mut r1 = request("oc", &c, "2");
    r1.source = InclusionSourceSelection::Source {
        source_key: source_for("oa"),
    };
    let first = fixed(&events, &[], r1, "first");
    assert_eq!(first.portions[0].reserved_source_share.numerator, "1");
    assert_eq!(first.portions[0].reserved_source_share.unit, "g");
    let mut r2 = request("od", &d, "6");
    r2.source = InclusionSourceSelection::Source {
        source_key: source_for("oa"),
    };
    let second = fixed(&events, std::slice::from_ref(&first), r2, "second");
    assert_eq!(second.portions[0].reserved_source_share.numerator, "3");
    assert!(
        resolve_inclusions(&events, &[first, second], &[request("oa", &a, "0.000001")]).is_err()
    );
}

#[test]
fn splitting_input_and_output_rows_does_not_create_additional_source_rights() {
    let a = material("a", "g");
    let b = material("b", "개");
    let baseline = vec![
        initial("oa", 1, &a, "10"),
        produce(
            "p",
            2,
            vec![(a.clone(), "10")],
            vec![(b.clone(), "4", "100")],
        ),
        observe("ob", 3, &b, "4", "0"),
    ];
    let split = vec![
        initial("oa", 1, &a, "10"),
        produce(
            "p",
            2,
            vec![(a.clone(), "7"), (a, "3")],
            vec![(b.clone(), "1", "30"), (b.clone(), "3", "70")],
        ),
        observe("ob", 3, &b, "4", "0"),
    ];
    let expected = resolve_inclusions(&baseline, &[], &[request("ob", &b, "2")]).unwrap();
    let actual = resolve_inclusions(&split, &[], &[request("ob", &b, "2")]).unwrap();
    assert_eq!(expected[0].portions, actual[0].portions);
}

#[test]
fn repeated_small_real_events_preserve_shares_beyond_i128_without_rounding() {
    let m = material("a", "g");
    let mut events = vec![initial("o1", 1, &m, "1000")];
    for step in 0..13 {
        events.push(observe(
            &format!("o{}", step + 2),
            2 + step * 2,
            &m,
            "999",
            "1",
        ));
        events.push(ValueReplayEvent::Inflow {
            event: event(&format!("r{step}"), 3 + step * 2),
            material: m.clone(),
            quantity: q("1"),
            amount: money("0"),
        });
    }
    events.push(observe("last", 28, &m, "1000", "0"));
    let portions = resolve_inclusions(&events, &[], &[request("last", &m, "1000")])
        .unwrap()
        .remove(0)
        .portions;
    let original = portions
        .iter()
        .find(|p| p.source_key == source_for("o1"))
        .unwrap();
    assert!(
        original
            .included_share
            .numerator
            .parse::<BigInt>()
            .unwrap()
            .bits()
            > 127
    );
    assert_eq!(original.included_share, original.reserved_source_share);
}

#[test]
fn malformed_fixed_shares_are_history_errors_and_do_not_become_available_inventory() {
    let m = material("a", "g");
    let events = vec![initial("o1", 1, &m, "10")];
    let mut reserved = fixed(&events, &[], request("o1", &m, "3"), "fixed");
    reserved.portions[0].reserved_source_share.numerator = "2".into();
    assert!(matches!(
        resolve_inclusions(&events, &[reserved], &[]),
        Err(ProvenanceError::InvalidHistory(_))
    ));
}

#[test]
fn source_and_pool_money_allocation_keeps_minimum_units_and_rejects_noncanonical_fractions() {
    let shares = vec![
        SourceShare {
            numerator: "1".into(),
            denominator: "3".into(),
            unit: "g".into()
        };
        3
    ];
    let values = allocate_share_amount(money("1.000001"), &shares).unwrap();
    assert_eq!(
        values.iter().map(|v| v.micros()).collect::<Vec<_>>(),
        vec![333334, 333334, 333333]
    );
    let mut bad = shares;
    bad[0].numerator = "01".into();
    assert!(allocate_share_amount(money("1"), &bad).is_err());
    bad[0].numerator = "1".into();
    bad[0].denominator = "0".into();
    assert!(allocate_share_amount(money("1"), &bad).is_err());
}

#[test]
fn zero_and_missing_observations_are_distinct_and_waste_does_not_create_a_source() {
    let m = material("a", "g");
    let events = vec![initial("o1", 1, &m, "0")];
    assert!(
        resolve_inclusions(&events, &[], &[request("o1", &m, "0")]).unwrap()[0]
            .portions
            .is_empty()
    );
    assert!(matches!(
        resolve_inclusions(&events, &[], &[request("missing", &m, "0")]),
        Err(ProvenanceError::InvalidReference(_))
    ));
    let events = vec![
        initial("o1", 1, &m, "10"),
        ValueReplayEvent::Waste {
            event: event("w", 2),
            material: m.clone(),
            quantity: q("4"),
        },
        observe("o2", 3, &m, "6", "0"),
    ];
    let choices = inclusion_availability(&events, &[], "a").unwrap();
    assert_eq!(choices[1].sources.len(), 1);
    assert_eq!(choices[1].available_quantity.decimal(), "6");
}

#[test]
fn an_older_pool_uses_exact_available_sources_without_intruding_into_a_later_fixed_source() {
    let a = material("a", "g");
    let b = material("b", "g");
    let c = material("c", "g");
    let x = material("x", "g");
    let events = vec![
        initial("oa", 1, &a, "0.000002"),
        initial("ob", 2, &b, "0.000001"),
        initial("oc", 3, &c, "0.000001"),
        produce(
            "p",
            4,
            vec![(a, "0.000002"), (b, "0.000001"), (c, "0.000001")],
            vec![(x.clone(), "0.000002", "100")],
        ),
        observe("o1", 5, &x, "0.000002", "0"),
        observe("o2", 6, &x, "0.000002", "0"),
    ];
    let mut later = request("o2", &x, "0.000001");
    later.source = InclusionSourceSelection::Source {
        source_key: source_for("oa"),
    };
    let reserved = fixed(&events, &[], later, "later-a");

    // A is already fixed at O2. B/C each have half a micro at O1, so only
    // their exact combined pool can represent this valid one-micro input.
    let resolved = resolve_inclusions(
        &events,
        std::slice::from_ref(&reserved),
        &[request("o1", &x, "0.000001")],
    );
    assert!(
        resolved.is_ok(),
        "the untouched B/C pool must remain selectable: {resolved:?}"
    );
    let portions = &resolved.as_ref().unwrap()[0].portions;
    assert_eq!(portions.len(), 2);
    assert!(
        portions
            .iter()
            .all(|part| part.source_key != source_for("oa"))
    );
    assert!(
        portions
            .iter()
            .all(|part| part.included_share.numerator == "1"
                && part.included_share.denominator == "2000000")
    );

    let available = inclusion_availability(&events, &[reserved], "x").unwrap();
    assert_eq!(available[0].available_quantity.decimal(), "0.000001");
    assert!(
        available[0]
            .sources
            .iter()
            .all(|source| source.available_quantity.micros() == 0)
    );
}

#[test]
fn a_partial_later_reservation_sets_an_exact_positive_earlier_cap() {
    let m = material("a", "g");
    let events = vec![initial("o1", 1, &m, "10"), observe("o2", 2, &m, "5", "5")];
    let later = fixed(&events, &[], request("o2", &m, "2.5"), "later-half");

    // Half of every earlier share survives into O2. Its fixed 2.5 requires
    // the other half of O1 to remain free, even though root quota left is 7.5.
    let earlier = fixed(
        &events,
        std::slice::from_ref(&later),
        request("o1", &m, "5"),
        "earlier-half",
    );
    assert_eq!(earlier.portions[0].reserved_source_share.numerator, "5");
    assert_eq!(earlier.portions[0].reserved_source_share.denominator, "1");
    assert!(matches!(
        resolve_inclusions(
            &events,
            std::slice::from_ref(&later),
            &[request("o1", &m, "5.000001")]
        ),
        Err(ProvenanceError::InsufficientShare(_))
    ));
    let choices = inclusion_availability(&events, &[later], "a").unwrap();
    assert_eq!(choices[0].available_quantity.decimal(), "5");
    assert_eq!(choices[0].sources[0].available_quantity.decimal(), "5");
}

#[test]
fn a_reserved_branch_and_a_free_branch_merge_with_an_exact_future_cap() {
    let a = material("a", "g");
    let b = material("b", "mL");
    let c = material("c", "g");
    let d = material("d", "개");
    let events = vec![
        initial("oa", 1, &a, "10"),
        produce(
            "split",
            2,
            vec![(a, "10")],
            vec![(b.clone(), "1", "100"), (c.clone(), "9", "0")],
        ),
        observe("ob", 3, &b, "1", "0"),
        observe("oc", 4, &c, "9", "0"),
        produce(
            "merge",
            5,
            vec![(b.clone(), "1"), (c.clone(), "9")],
            vec![(d.clone(), "10", "100")],
        ),
        observe("od", 6, &d, "10", "0"),
    ];
    let first = fixed(&events, &[], request("ob", &b, "0.5"), "branch");
    assert_eq!(first.portions[0].reserved_source_share.numerator, "5");
    assert_eq!(first.portions[0].reserved_source_share.denominator, "2");
    let later = fixed(
        &events,
        std::slice::from_ref(&first),
        request("od", &d, "3"),
        "merged",
    );
    let existing = vec![first, later];
    // A 1g C probe carries 5/9g of original A and 5/9 pieces after merging.
    // Existing root reservations total 5.5g, leaving C at most 8.1g.
    let allowed = fixed(&events, &existing, request("oc", &c, "8.1"), "free-branch");
    assert_eq!(allowed.portions[0].included_share.numerator, "81");
    assert_eq!(allowed.portions[0].included_share.denominator, "10");
    assert_eq!(allowed.portions[0].reserved_source_share.numerator, "9");
    assert_eq!(allowed.portions[0].reserved_source_share.denominator, "2");
    assert_eq!(
        inclusion_availability(&events, &existing, "c").unwrap()[0]
            .available_quantity
            .decimal(),
        "8.1"
    );
    assert!(matches!(
        resolve_inclusions(&events, &existing, &[request("oc", &c, "8.100001")]),
        Err(ProvenanceError::InsufficientShare(_))
    ));
}

#[test]
fn multistage_production_back_to_an_original_material_preserves_only_original_roots() {
    let a = material("a", "g");
    let b = material("b", "mL");
    let c = material("c", "개");
    let d = material("d", "개");
    let e = material("e", "개");
    let events = vec![
        initial("oa", 1, &a, "10"),
        initial("ob", 2, &b, "10"),
        produce(
            "first",
            3,
            vec![(a.clone(), "10"), (b.clone(), "10")],
            vec![(c.clone(), "4", "100"), (d.clone(), "6", "0")],
        ),
        produce(
            "second",
            4,
            vec![(c.clone(), "2"), (d.clone(), "3")],
            vec![(a.clone(), "5", "0"), (e.clone(), "5", "100")],
        ),
        observe("oa2", 5, &a, "5", "0"),
        observe("oc", 6, &c, "2", "0"),
        observe("od", 7, &d, "3", "0"),
        observe("oe", 8, &e, "5", "0"),
    ];
    let first = fixed(&events, &[], request("oa2", &a, "5"), "back-to-a");
    assert_eq!(first.portions.len(), 2);
    for (part, source, unit) in [
        (&first.portions[0], source_for("oa"), "g"),
        (&first.portions[1], source_for("ob"), "mL"),
    ] {
        assert_eq!(part.source_key, source);
        assert_eq!(part.included_share.numerator, "5");
        assert_eq!(part.included_share.denominator, "2");
        assert_eq!(part.included_share.unit, "g");
        assert_eq!(part.reserved_source_share.numerator, "5");
        assert_eq!(part.reserved_source_share.denominator, "2");
        assert_eq!(part.reserved_source_share.unit, unit);
    }
    let mut existing = vec![first];
    for (id, observation, material, quantity, origin_numerator, origin_denominator) in [
        ("rest-c", "oc", &c, "2", "2", "1"),
        ("rest-d", "od", &d, "3", "3", "1"),
        ("rest-e", "oe", &e, "5", "5", "2"),
    ] {
        let reservation = fixed(
            &events,
            &existing,
            request(observation, material, quantity),
            id,
        );
        assert_eq!(reservation.portions.len(), 2);
        for part in &reservation.portions {
            assert!(part.source_key == source_for("oa") || part.source_key == source_for("ob"));
            assert_eq!(part.reserved_source_share.numerator, origin_numerator);
            assert_eq!(part.reserved_source_share.denominator, origin_denominator);
        }
        existing.push(reservation);
    }
    // 2.5 + 2 + 3 + 2.5 = 10 in each root's own unit. Neither production
    // creates a fresh source merely because its output reuses material A.
    for material in [&a, &b] {
        let choices = inclusion_availability(&events, &existing, &material.material_id).unwrap();
        assert_eq!(choices[0].sources.len(), 1);
        assert_eq!(choices[0].sources[0].reserved_share.numerator, "10");
        assert_eq!(choices[0].sources[0].remaining_share.numerator, "0");
    }
    assert!(matches!(
        resolve_inclusions(&events, &existing, &[request("oa", &a, "0.000001")]),
        Err(ProvenanceError::InsufficientShare(_))
    ));
}
