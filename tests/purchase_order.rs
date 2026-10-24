use serde_json::{Value, json};
use sunchang_hansang::{
    domain::{Quantity, Unit},
    purchase_order::{
        DraftOrderActionV1, OrderAction, OrderForReceipt, OrderReferences, OrderStatus,
        ReceiptReferences, ReceiptResult, close_order, normalize_order_draft, parse_expected_date,
    },
};
use uuid::Uuid;

fn references() -> OrderReferences {
    OrderReferences {
        order_id: Uuid::new_v4(),
        material_id: Uuid::new_v4(),
        initial_order_version_id: Uuid::new_v4(),
        current_order_version_id: Uuid::new_v4(),
    }
}

fn receipt_references(order: &OrderReferences) -> ReceiptReferences {
    ReceiptReferences {
        receipt_id: Uuid::new_v4(),
        confirmation_id: Uuid::new_v4(),
        order_id: order.order_id,
        expected_order_version_id: order.current_order_version_id,
        material_id: order.material_id,
        material_version_id: Uuid::new_v4(),
    }
}

fn order(
    refs: OrderReferences,
    state: OrderStatus,
    initial: &str,
    current: &str,
    unit: &str,
) -> OrderForReceipt {
    OrderForReceipt::new(
        refs,
        state,
        Quantity::parse(initial).unwrap(),
        Quantity::parse(current).unwrap(),
        Unit::parse(unit).unwrap(),
    )
    .unwrap()
}

fn receipt(refs: ReceiptReferences, quantity: &str, unit: &str) -> ReceiptResult {
    ReceiptResult::new(
        refs,
        Quantity::parse(quantity).unwrap(),
        Unit::parse(unit).unwrap(),
    )
    .unwrap()
}

fn place() -> Value {
    json!({"action":"place","material_id":null,"material_version_id":null,
        "quantity":"1.","unit":"kg","supplier":"","expected_date":"2026-",
        "estimated_amount":"","note":"가상\r\n주문\r메모"})
}

#[test]
fn order_draft_preserves_incomplete_values_but_normalizes_references_and_note() {
    let mut input = place();
    input["material_id"] = json!("AAAAAAAA-AAAA-4AAA-8AAA-AAAAAAAAAAAA");
    let mut data: DraftOrderActionV1 = serde_json::from_value(input).unwrap();
    assert_eq!(data.action(), OrderAction::Place);
    normalize_order_draft(&mut data).unwrap();
    let normalized = serde_json::to_value(data).unwrap();
    assert_eq!(
        normalized["material_id"],
        "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa"
    );
    assert_eq!(normalized["material_version_id"], Value::Null);
    assert_eq!(normalized["quantity"], "1.");
    assert_eq!(normalized["expected_date"], "2026-");
    assert_eq!(normalized["estimated_amount"], "");
    assert_eq!(normalized["note"], "가상\n주문\n메모");
    for action in ["revise", "cancel"] {
        let mut value = json!({"action":action,"order_id":Uuid::new_v4().to_string(),
            "expected_order_version_id":Uuid::new_v4().to_string(),"note":""});
        if action == "revise" {
            let object = value.as_object_mut().unwrap();
            for (key, field) in [
                ("material_version_id", Value::Null),
                ("quantity", json!("")),
                ("unit", json!("g")),
                ("supplier", json!("")),
                ("expected_date", json!("")),
                ("estimated_amount", json!("0")),
            ] {
                object.insert(key.into(), field);
            }
        }
        let mut draft: DraftOrderActionV1 = serde_json::from_value(value).unwrap();
        normalize_order_draft(&mut draft).unwrap();
        assert_eq!(serde_json::to_value(draft).unwrap()["note"], "");
    }
}

#[test]
fn order_actions_are_closed_and_structural_limits_do_not_validate_business_numbers() {
    for (field, value) in [
        ("action", json!("receive")),
        ("quantity", json!(1)),
        ("inventory_delta", json!("1")),
        ("order_id", json!(Uuid::new_v4().to_string())),
    ] {
        let mut invalid = place();
        invalid[field] = value;
        assert!(
            serde_json::from_value::<DraftOrderActionV1>(invalid).is_err(),
            "{field}"
        );
    }
    for (field, value) in [
        ("material_id", json!("invalid")),
        ("supplier", json!("x".repeat(201))),
        ("quantity", json!("1\t")),
        ("note", json!("bad\0memo")),
    ] {
        let mut invalid = place();
        invalid[field] = value;
        let mut draft: DraftOrderActionV1 = serde_json::from_value(invalid).unwrap();
        assert!(normalize_order_draft(&mut draft).is_err(), "{field}");
    }
}

#[test]
fn optional_expected_date_requires_exact_real_calendar_dates() {
    assert_eq!(parse_expected_date("  ").unwrap(), None);
    for valid in ["0001-01-01", "2028-02-29", "9999-12-31"] {
        assert_eq!(parse_expected_date(valid).unwrap().as_deref(), Some(valid));
    }
    assert_eq!(
        parse_expected_date(" 2026-10-24 ").unwrap().as_deref(),
        Some("2026-10-24")
    );
    for invalid in [
        "2026-02-29",
        "2026-04-31",
        "2026-1-01",
        "2026-01-1",
        "0000-01-01",
        "+2026-01-01",
        "2026-10-24T00:00:00Z",
        "2026-",
        "２０２６-01-01",
    ] {
        assert!(parse_expected_date(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn receipt_closure_uses_exact_current_order_and_preserves_initial_reference() {
    let refs = references();
    let open = order(refs.clone(), OrderStatus::Open, "10000", "12000", "g");
    let supplied = receipt(receipt_references(&refs), "11.8", "kg");
    let result = close_order(&open, &supplied).unwrap();
    let json = serde_json::to_value(&result).unwrap();
    assert_eq!(json["status"], "received");
    assert_eq!(json["initial_ordered_quantity"], "10000");
    assert_eq!(json["ordered_quantity"], "12000");
    assert_eq!(json["received_quantity"], "11800");
    assert_eq!(json["difference"], "-200");
    assert_eq!(json["unit"], "g");
    assert_eq!(
        json["initial_order_version_id"],
        refs.initial_order_version_id.to_string()
    );
    assert_eq!(
        json["ordered_version_id"],
        refs.current_order_version_id.to_string()
    );
    assert_eq!(
        json["receipt_material_version_id"],
        supplied.references().material_version_id.to_string()
    );
    assert_eq!(open.state(), OrderStatus::Open);
    assert_eq!(supplied.quantity().decimal(), "11800");
    assert_eq!(
        serde_json::to_value(close_order(&open, &supplied).unwrap()).unwrap(),
        json
    );
    let closed = order(refs, OrderStatus::Received, "10000", "12000", "g");
    assert!(close_order(&closed, &supplied).is_err());
}

#[test]
fn receipt_difference_handles_short_equal_excess_volume_custom_and_micro_units() {
    for (ordered, actual, input_unit, canonical, expected) in [
        ("10000", "9.8", "kg", "g", "-200"),
        ("10000", "10", "kg", "g", "0"),
        ("10000", "10.2", "kg", "g", "200"),
        ("1000", "0.9", "L", "mL", "-100"),
        ("4", "3", "장", "장", "-1"),
        ("0.000002", "0.000001", "g", "g", "-0.000001"),
        ("0.000001", "9000000000", "g", "g", "8999999999.999999"),
    ] {
        let refs = references();
        let open = order(refs.clone(), OrderStatus::Open, ordered, ordered, canonical);
        let actual = receipt(receipt_references(&refs), actual, input_unit);
        let closed = close_order(&open, &actual).unwrap();
        assert_eq!(closed.difference, expected);
        assert_eq!(closed.unit, canonical);
    }
}

#[test]
fn receipt_closure_rejects_wrong_refs_dimensions_and_terminal_orders() {
    let refs = references();
    let open = order(refs.clone(), OrderStatus::Open, "10000", "10000", "g");
    for index in 0..3 {
        let mut wrong = receipt_references(&refs);
        match index {
            0 => wrong.order_id = Uuid::new_v4(),
            1 => wrong.material_id = Uuid::new_v4(),
            _ => wrong.expected_order_version_id = Uuid::new_v4(),
        }
        assert!(close_order(&open, &receipt(wrong, "1", "kg")).is_err());
    }
    assert!(close_order(&open, &receipt(receipt_references(&refs), "1", "L")).is_err());
    let custom = order(refs.clone(), OrderStatus::Open, "1", "1", "장");
    assert!(close_order(&custom, &receipt(receipt_references(&refs), "1", "개")).is_err());
    let cancelled = order(refs.clone(), OrderStatus::Cancelled, "10000", "10000", "g");
    assert!(close_order(&cancelled, &receipt(receipt_references(&refs), "1", "kg")).is_err());
}

#[test]
fn checked_receipt_and_order_constructors_reject_zero_overflow_and_noncanonical_basis() {
    let refs = references();
    assert!(
        ReceiptResult::new(
            receipt_references(&refs),
            Quantity::parse("0").unwrap(),
            Unit::parse("g").unwrap()
        )
        .is_err()
    );
    assert!(
        ReceiptResult::new(
            receipt_references(&refs),
            Quantity::parse("9000000000").unwrap(),
            Unit::parse("kg").unwrap()
        )
        .is_err()
    );
    assert!(
        OrderForReceipt::new(
            refs.clone(),
            OrderStatus::Open,
            Quantity::parse("0").unwrap(),
            Quantity::parse("1").unwrap(),
            Unit::parse("g").unwrap()
        )
        .is_err()
    );
    assert!(
        OrderForReceipt::new(
            refs,
            OrderStatus::Open,
            Quantity::parse("1").unwrap(),
            Quantity::parse("1").unwrap(),
            Unit::parse("kg").unwrap()
        )
        .is_err()
    );
}
