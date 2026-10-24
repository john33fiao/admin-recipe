use sunchang_hansang::{
    domain::{Money, Quantity},
    holding::{HoldingBasisKind, HoldingBasisRef, HoldingInput},
    waste::*,
};
fn id(n: u128) -> String {
    uuid::Uuid::from_u128(n).to_string()
}
fn context(q: &str, v: Option<&str>) -> WasteMaterialContext {
    WasteMaterialContext {
        material_id: id(1),
        material_version_id: id(2),
        material_name: "가상 폐기".into(),
        unit: "g".into(),
        holding: HoldingInput {
            basis: Some(HoldingBasisRef {
                kind: HoldingBasisKind::Receipt,
                id: Some(id(3)),
                seq: Some("1".into()),
            }),
            quantity: Some(Quantity::parse(q).unwrap()),
            value: v.map(|v| Money::parse(v).unwrap()),
            source_id: Some(id(4)),
            ..Default::default()
        },
    }
}
fn data(q: &str) -> DraftWasteV1 {
    DraftWasteV1 {
        material_id: Some(id(1)),
        material_version_id: Some(id(2)),
        quantity: q.into(),
        unit: "g".into(),
        reason: WasteReason::Discard,
        note: String::new(),
    }
}
fn ready(data: &DraftWasteV1, ctx: &WasteMaterialContext) -> WasteEffect {
    match calculate_waste(data, ctx).unwrap() {
        WasteCheck::Ready(result) => *result,
        other => panic!("{other:?}"),
    }
}
#[test]
fn partial_waste_keeps_the_entire_value_in_the_remaining_quantity() {
    let ctx = context("10000", Some("10000"));
    let result = ready(&data("1000"), &ctx);
    assert_eq!(result.after.quantity, "9000");
    assert_eq!(result.after.value.as_deref(), Some("10000"));
    assert_eq!(result.loss_value, "0");
    assert_eq!(result.before.source_id.as_deref(), Some(id(4).as_str()));
    assert!(result.after.source_id.is_none());
    assert!(ctx.holding.observation.is_none());
}
#[test]
fn full_waste_ends_the_exact_value_including_fractional_remainder() {
    let result = ready(&data("3"), &context("3", Some("0.000001")));
    assert_eq!(result.after.quantity, "0");
    assert_eq!(result.after.value.as_deref(), Some("0"));
    assert_eq!(result.loss_value, "0.000001");
}
#[test]
fn staff_meal_and_zero_value_use_the_same_explicit_waste_rule() {
    let mut input = data("1");
    input.reason = WasteReason::StaffMeal;
    let result = ready(&input, &context("2", Some("0")));
    assert_eq!(result.reason, WasteReason::StaffMeal);
    assert_eq!(result.after.value.as_deref(), Some("0"));
    assert_eq!(result.loss_value, "0");
}
#[test]
fn unknown_and_excess_inputs_remain_reconciliation_conflicts() {
    for ctx in [context("1", Some("10")), context("3", None)] {
        assert!(matches!(
            calculate_waste(&data("2"), &ctx).unwrap(),
            WasteCheck::NeedsReconciliation(_)
        ));
    }
    let mut ctx = context("1", Some("10"));
    ctx.holding = HoldingInput::default();
    assert!(matches!(
        calculate_waste(&data("1"), &ctx).unwrap(),
        WasteCheck::NeedsReconciliation(_)
    ));
}
#[test]
fn zero_invalid_numbers_dimensions_and_wrong_material_versions_are_rejected() {
    for q in ["0", "-1", "abc", "0.0000001"] {
        assert!(calculate_waste(&data(q), &context("2", Some("10"))).is_err());
    }
    let mut input = data("1");
    input.unit = "L".into();
    assert!(calculate_waste(&input, &context("2", Some("10"))).is_err());
    input.unit = "g".into();
    input.material_version_id = Some(id(8));
    assert!(calculate_waste(&input, &context("2", Some("10"))).is_err());
}
#[test]
fn canonical_unit_conversion_happens_exactly_once() {
    let mut input = data("1");
    input.unit = "kg".into();
    let result = ready(&input, &context("2000", Some("10")));
    assert_eq!(result.quantity, "1000");
    assert_eq!(result.after.quantity, "1000");
    assert_eq!(result.input_quantity, "1");
    assert_eq!(result.input_unit, "kg");
}
#[test]
fn normalizer_preserves_incomplete_input_but_rejects_unknown_structure() {
    let mut input = data("");
    input.note = "가상\r\n메모".into();
    normalize_waste_draft(&mut input).unwrap();
    assert_eq!(input.quantity, "");
    assert_eq!(input.note, "가상\n메모");
    let mut value = serde_json::to_value(&input).unwrap();
    value["quantity"] = serde_json::json!(0);
    assert!(serde_json::from_value::<DraftWasteV1>(value).is_err());
}
