use chrono::NaiveDate;
use serde_json::{Value, json};
use sunchang_hansang::receipt::{
    DraftReceiptV1, ReceiptSource, ReceiptSourceDraft, ReceiptStockEffectDraft,
    normalize_receipt_draft, parse_receipt_date,
};

fn raw() -> Value {
    json!({"source":{"kind":"direct","material_id":null,"material_version_id":null},
        "quantity":"","unit":"kg","amount":"","receipt_date":"","note":"",
        "stock_effect":{"kind":"unreflected"}})
}

fn day(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

#[test]
fn receipt_draft_keeps_incomplete_values_and_normalizes_only_structure() {
    let mut input = raw();
    input["source"]["material_id"] = json!("ABCDEFAB-1234-4000-8000-123456789ABC");
    input["quantity"] = json!("unfinished");
    input["amount"] = json!("1e3");
    input["receipt_date"] = json!("2026-02-");
    input["note"] = json!("첫 줄\r\n둘째 줄\r셋째 줄");
    let mut draft: DraftReceiptV1 = serde_json::from_value(input).unwrap();
    normalize_receipt_draft(&mut draft).unwrap();
    assert_eq!(draft.quantity, "unfinished");
    assert_eq!(draft.amount, "1e3");
    assert_eq!(draft.receipt_date, "2026-02-");
    assert_eq!(draft.note, "첫 줄\n둘째 줄\n셋째 줄");
    match draft.source {
        ReceiptSourceDraft::Direct {
            material_id,
            material_version_id,
        } => {
            assert_eq!(
                material_id.as_deref(),
                Some("abcdefab-1234-4000-8000-123456789abc")
            );
            assert!(material_version_id.is_none());
        }
        _ => panic!("expected direct source"),
    }
}

#[test]
fn order_and_reconciliation_references_can_remain_incomplete() {
    let mut input = raw();
    input["source"] = json!({"kind":"order","order_id":null,
        "expected_order_version_id":null,"material_version_id":null});
    input["stock_effect"] = json!({"kind":"needs_reconciliation",
        "basis_id":"ABCDEFAB-1234-4000-8000-123456789ABC","included_quantity":"not measured"});
    let mut draft: DraftReceiptV1 = serde_json::from_value(input).unwrap();
    normalize_receipt_draft(&mut draft).unwrap();
    match draft.stock_effect {
        ReceiptStockEffectDraft::NeedsReconciliation {
            basis_id,
            included_quantity,
        } => {
            assert_eq!(
                basis_id.as_deref(),
                Some("abcdefab-1234-4000-8000-123456789abc")
            );
            assert_eq!(included_quantity, "not measured");
        }
        _ => panic!("expected reconciliation draft"),
    }
    assert!(matches!(
        draft.source,
        ReceiptSourceDraft::Order {
            order_id: None,
            expected_order_version_id: None,
            material_version_id: None
        }
    ));
}

#[test]
fn receipt_nested_shapes_reject_unknown_fields_and_numeric_inputs() {
    let original = raw();
    let mut cases = Vec::new();
    let mut numeric = original.clone();
    numeric["quantity"] = json!(1);
    cases.push(numeric);
    let mut numeric_amount = original.clone();
    numeric_amount["amount"] = json!(0);
    cases.push(numeric_amount);
    let mut mixed_source = original.clone();
    mixed_source["source"]["order_id"] = json!(null);
    cases.push(mixed_source);
    let mut unknown_source = original.clone();
    unknown_source["source"]["kind"] = json!("opening_balance");
    cases.push(unknown_source);
    let mut mixed_effect = original.clone();
    mixed_effect["stock_effect"]["included_quantity"] = json!("1");
    cases.push(mixed_effect);
    let mut numeric_included = original.clone();
    numeric_included["stock_effect"] =
        json!({"kind":"needs_reconciliation","basis_id":null,"included_quantity":1});
    cases.push(numeric_included);
    let mut extra = original;
    extra["confirmed"] = json!(true);
    cases.push(extra);
    for input in cases {
        assert!(
            serde_json::from_value::<DraftReceiptV1>(input.clone()).is_err(),
            "{input}"
        );
    }
}

#[test]
fn draft_limits_uuid_and_single_line_controls_are_enforced() {
    for (field, invalid) in [
        ("quantity", "1".repeat(81)),
        ("amount", "0".repeat(81)),
        ("unit", "가".repeat(21)),
        ("receipt_date", "2".repeat(33)),
        ("note", "가".repeat(2001)),
        ("quantity", "1\n2".into()),
        ("note", "메모\t내용".into()),
    ] {
        let mut input = raw();
        input[field] = json!(invalid);
        let mut draft = serde_json::from_value(input).unwrap();
        assert!(normalize_receipt_draft(&mut draft).is_err(), "{field}");
    }
    let mut input = raw();
    input["source"]["material_id"] = json!("not-a-uuid");
    assert!(normalize_receipt_draft(&mut serde_json::from_value(input).unwrap()).is_err());
    let mut input = raw();
    input["stock_effect"] =
        json!({"kind":"needs_reconciliation","basis_id":null,"included_quantity":"1".repeat(81)});
    assert!(normalize_receipt_draft(&mut serde_json::from_value(input).unwrap()).is_err());
    let mut input = raw();
    input["amount"] = json!("0");
    input["note"] = json!("가".repeat(2000));
    normalize_receipt_draft(&mut serde_json::from_value(input).unwrap()).unwrap();
}

#[test]
fn receipt_date_requires_a_real_non_future_calendar_day() {
    let today = day(2026, 10, 24);
    for (text, expected) in [
        ("2026-10-24", today),
        (" 2000-02-29 ", day(2000, 2, 29)),
        ("0001-01-01", day(1, 1, 1)),
    ] {
        assert_eq!(parse_receipt_date(text, today).unwrap(), expected);
    }
    for text in [
        "",
        " ",
        "2026-10-25",
        "1900-02-29",
        "2026-02-29",
        "2026-04-31",
        "2026-1-01",
        "2026-01-1",
        "2026/01/01",
        "0000-01-01",
        "10000-01-01",
        "２０２６-01-01",
        "2026-10-24T00:00:00Z",
    ] {
        assert!(parse_receipt_date(text, today).is_err(), "{text}");
    }
    assert_eq!(
        parse_receipt_date("9999-12-31", day(9999, 12, 31)).unwrap(),
        day(9999, 12, 31)
    );
}

#[test]
fn injected_local_day_is_authoritative_without_reading_the_wall_clock() {
    let utc_day = day(2026, 10, 23);
    let local_day = day(2026, 10, 24);
    assert!(parse_receipt_date("2026-10-24", utc_day).is_err());
    assert_eq!(
        parse_receipt_date("2026-10-24", local_day).unwrap(),
        local_day
    );
    assert!(parse_receipt_date("2026-10-25", local_day).is_err());
}

#[test]
fn confirmed_direct_source_preserves_wire_shape_and_rejects_unknown_snapshot_fields() {
    let source = r#"{"kind":"direct"}"#;
    let decoded: ReceiptSource = serde_json::from_str(source).unwrap();
    assert_eq!(serde_json::to_string(&decoded).unwrap(), source);
    assert!(
        serde_json::from_value::<ReceiptSource>(json!({"kind":"direct","order_id":null})).is_err()
    );
    assert!(
        serde_json::from_value::<ReceiptSource>(json!({"kind":"direct","extra":true})).is_err()
    );
    let stock = r#"{"kind":"unreflected"}"#;
    let decoded: ReceiptStockEffectDraft = serde_json::from_str(stock).unwrap();
    assert_eq!(serde_json::to_string(&decoded).unwrap(), stock);
    assert!(
        serde_json::from_value::<ReceiptStockEffectDraft>(
            json!({"kind":"unreflected","extra":true})
        )
        .is_err()
    );
}
