use sunchang_hansang::{
    correction::{CorrectionPlanRef, CorrectionRefKind},
    domain::{Money, Quantity, UnitPrice},
    holding::{HoldingBasisKind, HoldingBasisRef, HoldingInput, HoldingObservation},
    purchase_return::*,
    receipt::{ReceiptRecord, ReceiptSource},
};

fn id(n: u128) -> String {
    uuid::Uuid::from_u128(n).to_string()
}
fn context(q: &str, value: Option<&str>) -> PurchaseReturnContext {
    PurchaseReturnContext {
        original: ReceiptRecord {
            id: id(1),
            confirmation_id: id(2),
            business_record_id: id(3),
            material_id: id(4),
            material_version_id: id(5),
            material_name: "가상 반품 품목".into(),
            quantity: "3000".into(),
            unit: "g".into(),
            input_quantity: "3".into(),
            input_unit: "kg".into(),
            amount: "30000".into(),
            receipt_date: "2026-10-01".into(),
            note: String::new(),
            recorded_at: "2026-10-01T00:00:00Z".into(),
            seq: "2".into(),
            source: ReceiptSource::Direct {},
        },
        source: PurchaseReturnSourceState {
            reference: CorrectionPlanRef::Existing {
                entity: CorrectionRefKind::Receipt,
                id: id(1),
                version_id: id(1),
            },
            material_id: id(4),
            material_version_id: id(5),
            material_name: "가상 반품 품목".into(),
            unit: "g".into(),
            quantity: "3000".into(),
            amount: "30000".into(),
            returned_quantity: "0".into(),
            remaining_quantity: "3000".into(),
            logical_seq: "2".into(),
            seq: "2".into(),
        },
        holding: HoldingInput {
            basis: Some(HoldingBasisRef {
                kind: HoldingBasisKind::Receipt,
                id: Some(id(1)),
                seq: Some("2".into()),
            }),
            quantity: Some(Quantity::parse(q).unwrap()),
            value: value.map(|v| Money::parse(v).unwrap()),
            source_id: Some(id(6)),
            last_valid: Some(
                UnitPrice::new(
                    Money::parse("30000").unwrap(),
                    Quantity::parse("3000").unwrap(),
                    id(6),
                )
                .unwrap(),
            ),
            observation: None,
        },
    }
}
fn draft(q: &str) -> DraftPurchaseReturnV1 {
    DraftPurchaseReturnV1 {
        receipt_id: Some(id(1)),
        expected_receipt_version_id: Some(id(1)),
        quantity: q.into(),
        unit: "g".into(),
        note: String::new(),
    }
}
fn ready(data: &DraftPurchaseReturnV1, context: &PurchaseReturnContext) -> PurchaseReturnEffect {
    match calculate_purchase_return(data, context).unwrap() {
        PurchaseReturnCheck::Ready(effect) => *effect,
        other => panic!("{other:?}"),
    }
}
#[test]
fn partial_return_deducts_current_average_and_preserves_purchase_reference() {
    let result = ready(&draft("1000"), &context("2000", Some("20000")));
    assert_eq!(result.deducted_value, "10000");
    assert_eq!(result.after.quantity, "1000");
    assert_eq!(result.after.value.as_deref(), Some("10000"));
    assert_eq!(result.source.original_amount, "30000");
    assert_eq!(result.source.original_unit, "g");
    assert_eq!(result.before.source_id, Some(id(6)));
    assert!(result.after.source_id.is_none());
}
#[test]
fn mixed_average_and_full_return_take_the_exact_final_remainder() {
    let first = ready(&draft("1000"), &context("3000", Some("40000")));
    assert_eq!(first.deducted_value, "13333.333333");
    assert_eq!(first.after.value.as_deref(), Some("26666.666667"));
    let mut next = context("2000", Some("26666.666667"));
    next.source.returned_quantity = "1000".into();
    next.source.remaining_quantity = "2000".into();
    let last = ready(&draft("2000"), &next);
    assert_eq!(last.deducted_value, "26666.666667");
    assert_eq!(last.after.quantity, "0");
    assert_eq!(last.after.value.as_deref(), Some("0"));
}
#[test]
fn source_receipt_limit_is_separate_from_sufficient_current_stock() {
    let mut ctx = context("9000", Some("90000"));
    ctx.source.returned_quantity = "2500".into();
    ctx.source.remaining_quantity = "500".into();
    assert!(calculate_purchase_return(&draft("1000"), &ctx).is_err());
    assert_eq!(ready(&draft("500"), &ctx).source.remaining_quantity, "500");
}
#[test]
fn insufficient_unknown_and_unestablished_holdings_require_reconciliation() {
    let mut unknown = context("2000", Some("10"));
    unknown.holding = HoldingInput::default();
    for ctx in [context("500", Some("10")), context("2000", None), unknown] {
        assert!(matches!(
            calculate_purchase_return(&draft("1000"), &ctx).unwrap(),
            PurchaseReturnCheck::NeedsReconciliation(_)
        ));
    }
}
#[test]
fn zero_value_is_valid_and_does_not_use_old_last_valid_price() {
    let result = ready(&draft("1000"), &context("2000", Some("0")));
    assert_eq!(result.deducted_value, "0");
    assert_eq!(result.after.value.as_deref(), Some("0"));
}
#[test]
fn source_uses_actual_receipt_r_independently_of_current_inflow() {
    let mut ctx = context("3000", Some("60000"));
    ctx.holding.basis = Some(HoldingBasisRef {
        kind: HoldingBasisKind::Observation,
        id: Some(id(7)),
        seq: Some("1".into()),
    });
    ctx.holding.observation = Some(HoldingObservation {
        id: id(7),
        quantity: "3000".into(),
        recorded_at: "2026-10-01T00:00:00Z".into(),
        seq: "1".into(),
    });
    let result = ready(&draft("3000"), &ctx);
    assert_eq!(result.source.quantity, "3000");
    assert_eq!(result.after.quantity, "0");
    assert_eq!(result.observation, ctx.holding.observation);
}
#[test]
fn changed_source_keeps_the_original_purchase_unit_separate() {
    let mut ctx = context("3", Some("30"));
    ctx.source.unit = "개".into();
    ctx.source.quantity = "3".into();
    ctx.source.remaining_quantity = "3".into();
    let mut input = draft("1");
    input.unit = "개".into();
    let result = ready(&input, &ctx);
    assert_eq!(result.unit, "개");
    assert_eq!(result.source.original_unit, "g");
    assert_eq!(result.source.original_quantity, "3000");
}
#[test]
fn canonical_conversion_is_once_and_input_text_is_preserved() {
    let mut input = draft("1");
    input.unit = "kg".into();
    let result = ready(&input, &context("2000", Some("20")));
    assert_eq!(result.quantity, "1000");
    assert_eq!(result.input_quantity, "1");
    assert_eq!(result.input_unit, "kg");
}
#[test]
fn invalid_quantity_dimensions_sources_and_context_limits_are_rejected() {
    for q in ["0", "-1", "abc", "0.0000001"] {
        assert!(calculate_purchase_return(&draft(q), &context("2000", Some("20"))).is_err());
    }
    let mut input = draft("1");
    input.unit = "L".into();
    assert!(calculate_purchase_return(&input, &context("2000", Some("20"))).is_err());
    input.unit = "g".into();
    input.receipt_id = Some(id(8));
    assert!(calculate_purchase_return(&input, &context("2000", Some("20"))).is_err());
    let mut ctx = context("2000", Some("20"));
    ctx.source.remaining_quantity = "3001".into();
    assert!(calculate_purchase_return(&draft("1"), &ctx).is_err());
}
#[test]
fn projection_has_no_receipt_cas_and_preserves_observation_and_input() {
    let ctx = context("3", Some("0.000001"));
    let before = ctx.holding.clone();
    let PurchaseReturnProjectionCheck::Ready(result) =
        project_purchase_return(&before, Quantity::parse("3").unwrap()).unwrap()
    else {
        panic!()
    };
    assert_eq!(result.deducted_value, "0.000001");
    assert_eq!(result.after.quantity, "0");
    assert_eq!(before.quantity, Some(Quantity::parse("3").unwrap()));
    assert_eq!(before.last_valid.unwrap().amount().decimal(), "30000");
}
#[test]
fn incomplete_draft_is_preserved_but_unknown_fields_and_number_values_are_rejected() {
    let mut input = draft("");
    input.receipt_id = None;
    input.expected_receipt_version_id = None;
    input.note = "가상\r\n메모".into();
    normalize_purchase_return_draft(&mut input).unwrap();
    assert_eq!(input.note, "가상\n메모");
    assert_eq!(input.quantity, "");
    let mut json = serde_json::to_value(&input).unwrap();
    json["quantity"] = serde_json::json!(1);
    assert!(serde_json::from_value::<DraftPurchaseReturnV1>(json).is_err());
    let mut json = serde_json::to_value(&input).unwrap();
    json["refund_amount"] = serde_json::json!("100");
    assert!(serde_json::from_value::<DraftPurchaseReturnV1>(json).is_err());
}
