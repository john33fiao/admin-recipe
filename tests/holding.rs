use serde_json::json;
use sunchang_hansang::{
    domain::{Money, Quantity, UnitPrice},
    holding::{
        HoldingBasisKind, HoldingBasisRef, HoldingChange, HoldingChangeKind, HoldingInput,
        HoldingObservation, project_holding, project_receipt, resolve_holding_price,
        validate_holding_input,
    },
};
use uuid::Uuid;

fn id(n: u128) -> String {
    Uuid::from_u128(n).to_string()
}
fn q(value: &str) -> Quantity {
    Quantity::parse(value).unwrap()
}
fn money(value: &str) -> Money {
    Money::parse(value).unwrap()
}
fn price(value: &str, quantity: &str, source: u128) -> UnitPrice {
    UnitPrice::new(money(value), q(quantity), id(source)).unwrap()
}
fn known(quantity: &str, value: Option<&str>) -> HoldingInput {
    HoldingInput {
        basis: Some(HoldingBasisRef {
            kind: HoldingBasisKind::Receipt,
            id: Some(id(1)),
            seq: Some("10".into()),
        }),
        observation: None,
        quantity: Some(q(quantity)),
        value: value.map(money),
        source_id: Some(id(2)),
        last_valid: None,
    }
}
fn change(event: u128, seq: &str, quantity: &str, value: &str) -> HoldingChange {
    HoldingChange {
        kind: HoldingChangeKind::Receipt,
        event_id: id(event),
        quantity: quantity.into(),
        value: value.into(),
        recorded_at: "2026-01-01T12:00:00.000Z".into(),
        seq: seq.into(),
    }
}
fn observed(quantity: &str, value: &str) -> HoldingInput {
    HoldingInput {
        basis: Some(HoldingBasisRef {
            kind: HoldingBasisKind::Observation,
            id: Some(id(10)),
            seq: Some("10".into()),
        }),
        observation: Some(HoldingObservation {
            id: id(10),
            quantity: quantity.into(),
            recorded_at: "2026-01-01T10:00:00.000Z".into(),
            seq: "10".into(),
        }),
        quantity: Some(q(quantity)),
        value: Some(money(value)),
        source_id: Some(id(11)),
        last_valid: None,
    }
}

#[test]
fn first_receipt_establishes_a_basis_and_weighted_value_without_fabricating_an_observation() {
    let missing = HoldingInput::default();
    let first = project_receipt(&missing, q("1000"), money("10000")).unwrap();
    assert!(first.before.is_none());
    assert!(first.basis_before.is_none());
    assert!(first.observation.is_none());
    assert_eq!(first.after.quantity, "1000");
    assert_eq!(first.after.value.as_deref(), Some("10000"));
    assert!(first.after.source_id.is_none());
    assert!(missing.quantity.is_none());

    let current = known("1000", Some("10000"));
    let second = project_receipt(&current, q("2000"), money("40000")).unwrap();
    assert_eq!(
        second.before.unwrap().source_id.as_deref(),
        Some(id(2).as_str())
    );
    assert_eq!(second.after.quantity, "3000");
    assert_eq!(second.after.value.as_deref(), Some("50000"));
    let after = known("3000", Some("50000"));
    let resolved = resolve_holding_price(&after, Some(price("999", "1", 90))).unwrap();
    assert_eq!(resolved.source, "holding_average");
    assert_eq!(resolved.amount.as_deref(), Some("50000"));
    assert_eq!(resolved.quantity.as_deref(), Some("3000"));
    assert_eq!(resolved.source_id.as_deref(), Some(id(2).as_str()));
}

#[test]
fn missing_zero_and_unknown_value_have_distinct_price_and_receipt_results() {
    let missing = HoldingInput::default();
    assert_eq!(
        resolve_holding_price(&missing, None).unwrap().source,
        "missing"
    );
    assert_eq!(
        resolve_holding_price(&missing, Some(price("0", "100", 90)))
            .unwrap()
            .source,
        "preregistered"
    );
    let free = project_receipt(&missing, q("1"), money("0")).unwrap();
    assert_eq!(free.after.value.as_deref(), Some("0"));
    let current_free = known("1", Some("0"));
    let resolved = resolve_holding_price(&current_free, None).unwrap();
    assert_eq!(resolved.source, "holding_average");
    assert_eq!(resolved.amount.as_deref(), Some("0"));
    let mut empty = known("0", Some("0"));
    empty.last_valid = Some(price("1000", "100", 80));
    let resolved = resolve_holding_price(&empty, Some(price("5", "1", 90))).unwrap();
    assert_eq!(resolved.source, "last_valid");
    assert_eq!(resolved.amount.as_deref(), Some("1000"));
    assert_eq!(resolved.source_id.as_deref(), Some(id(80).as_str()));
    let unknown = known("10", None);
    assert_eq!(
        resolve_holding_price(&unknown, Some(price("0", "1", 90)))
            .unwrap()
            .source,
        "missing"
    );
    assert!(project_receipt(&unknown, q("2"), money("200")).is_err());
    assert_eq!(unknown.value, None);
}

#[test]
fn receipt_quantity_and_value_totals_are_checked_without_float_or_saturation() {
    let missing = HoldingInput::default();
    assert!(project_receipt(&missing, q("0"), money("10")).is_err());
    assert!(project_receipt(&known("9000000000", Some("0")), q("0.000001"), money("0")).is_err());
    assert!(
        project_receipt(
            &known("1", Some("9000000000000")),
            q("1"),
            money("0.000001")
        )
        .is_err()
    );
    assert!(
        project_receipt(
            &known("1", Some("9000000000000")),
            q("1"),
            money("9000000000000")
        )
        .is_err()
    );
    let result = project_receipt(
        &known("0.000001", Some("0.000001")),
        q("0.000001"),
        money("0.000001"),
    )
    .unwrap();
    assert_eq!(result.after.quantity, "0.000002");
    assert_eq!(result.after.value.as_deref(), Some("0.000002"));
}

#[test]
fn view_preserves_the_last_measurement_and_only_adds_events_after_its_sequence() {
    let baseline = observed("10000", "10000");
    let changes = [
        change(8, "8", "9000", "9000"),
        change(12, "12", "2000", "6000"),
    ];
    let view = project_holding(&baseline, &changes).unwrap();
    assert_eq!(view.observation, baseline.observation);
    assert_eq!(view.basis, baseline.basis);
    assert_eq!(view.confirmed_changes, changes[1..]);
    let amounts = view.settlement_basis.unwrap();
    assert_eq!(amounts.quantity, "12000");
    assert_eq!(amounts.value.as_deref(), Some("16000"));
    assert!(amounts.source_id.is_none());
    let preview = project_receipt(&baseline, q("2000"), money("6000")).unwrap();
    assert_eq!(preview.observation, baseline.observation);
    assert_eq!(preview.after.quantity, "12000");
    let json = serde_json::to_value(view.observation.unwrap()).unwrap();
    assert_eq!(json["quantity"], "10000");
    assert_eq!(json["seq"], "10");
    assert_eq!(json["recorded_at"], "2026-01-01T10:00:00.000Z");
    assert_eq!(serde_json::to_value(&changes[1]).unwrap()["seq"], "12");
}

#[test]
fn baseline_receipt_is_not_added_twice_and_unknown_start_can_be_established_once() {
    let changes = [
        change(1, "10", "1000", "10000"),
        change(2, "11", "2000", "40000"),
    ];
    let baseline = known("1000", Some("10000"));
    let known_view = project_holding(&baseline, &changes).unwrap();
    let first_view = project_holding(&HoldingInput::default(), &changes).unwrap();
    assert_eq!(known_view, first_view);
    assert_eq!(known_view.confirmed_changes, changes[1..]);
    assert_eq!(known_view.settlement_basis.unwrap().quantity, "3000");
    assert!(known_view.observation.is_none());
    let empty = project_holding(&HoldingInput::default(), &[]).unwrap();
    assert!(empty.basis.is_none());
    assert!(empty.settlement_basis.is_none());
}

#[test]
fn duplicate_out_of_order_or_mismatched_baseline_events_cannot_invent_holdings() {
    let baseline = known("1000", Some("10000"));
    for changes in [
        vec![change(1, "10", "999", "10000")],
        vec![change(9, "10", "1000", "10000")],
        vec![change(1, "11", "1000", "10000")],
        vec![change(2, "11", "1", "0"), change(2, "12", "1", "0")],
        vec![change(2, "12", "1", "0"), change(3, "11", "1", "0")],
        vec![change(2, "11", "0", "0")],
        vec![change(2, "01", "1", "0")],
    ] {
        assert!(project_holding(&baseline, &changes).is_err());
    }
    let observation = observed("1000", "10000");
    assert!(project_holding(&observation, &[change(10, "10", "1000", "10000")]).is_err());
    let mut accumulated = observation;
    accumulated.quantity = Some(q("2000"));
    assert!(project_holding(&accumulated, &[]).is_err());
    assert!(project_holding(&known("1", None), &[change(2, "11", "1", "0")]).is_err());
}

#[test]
fn impossible_basis_and_observation_combinations_are_rejected_and_wire_sequences_are_strings() {
    let invalid = HoldingInput {
        value: Some(money("0")),
        ..HoldingInput::default()
    };
    assert!(validate_holding_input(&invalid).is_err());
    for mut invalid in [
        known("0", Some("1")),
        known("0", None),
        known("1", Some("1")),
    ] {
        if invalid.quantity == Some(q("1")) {
            invalid.basis = None;
        }
        assert!(validate_holding_input(&invalid).is_err());
    }
    let mut invalid = observed("1", "1");
    invalid.observation.as_mut().unwrap().seq = "9".into();
    assert!(validate_holding_input(&invalid).is_err());
    invalid = known("1", Some("1"));
    invalid.observation = observed("1", "1").observation;
    assert!(validate_holding_input(&invalid).is_err());
    invalid = known("1", Some("1"));
    invalid.source_id = None;
    assert!(validate_holding_input(&invalid).is_err());
    invalid = known("1", Some("1"));
    invalid.basis.as_mut().unwrap().seq = Some("0".into());
    assert!(validate_holding_input(&invalid).is_err());

    let basis = known("1", Some("0")).basis.unwrap();
    let wire = serde_json::to_value(basis).unwrap();
    assert_eq!(wire["seq"], json!("10"));
    let mut bad = wire;
    bad["seq"] = json!(10);
    assert!(serde_json::from_value::<HoldingBasisRef>(bad).is_err());
}
