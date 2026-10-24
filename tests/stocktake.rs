use sunchang_hansang::{
    domain::{Money, Quantity, UnitPrice},
    holding::{HoldingBasisKind, HoldingBasisRef, HoldingInput, resolve_holding_price},
    sale::{SaleConsumptionSource, SaleCount},
    stocktake::*,
};

fn id(n: u128) -> String {
    uuid::Uuid::from_u128(n).to_string()
}
fn q(text: &str) -> Quantity {
    Quantity::parse(text).unwrap()
}
fn money(text: &str) -> Money {
    Money::parse(text).unwrap()
}
fn known(quantity: &str, value: &str) -> HoldingInput {
    HoldingInput {
        basis: Some(HoldingBasisRef {
            kind: HoldingBasisKind::Receipt,
            id: Some(id(10)),
            seq: Some("10".into()),
        }),
        quantity: Some(q(quantity)),
        value: Some(money(value)),
        source_id: Some(id(11)),
        ..Default::default()
    }
}
fn context(holding: HoldingInput) -> StocktakeMaterialContext {
    let price = resolve_holding_price(&holding, None).unwrap();
    StocktakeMaterialContext {
        material_id: id(1),
        material_version_id: id(2),
        material_name: "가상 재료".into(),
        unit: "g".into(),
        holding,
        price,
        sales: vec![],
    }
}
fn row(quantity: &str) -> DraftStocktakeRowV1 {
    DraftStocktakeRowV1 {
        material_id: Some(id(1)),
        material_version_id: Some(id(2)),
        quantity: quantity.into(),
        unit: "g".into(),
        initial_value: None,
        excess: None,
    }
}
fn draft(rows: Vec<DraftStocktakeRowV1>) -> DraftStocktakeV1 {
    DraftStocktakeV1 {
        rows,
        note: String::new(),
    }
}
fn ready(data: DraftStocktakeV1, context: StocktakeMaterialContext) -> StocktakePreview {
    match calculate_stocktake(&data, &[context], 20, 19).unwrap() {
        StocktakeCheck::Ready(result) => *result,
        other => panic!("{other:?}"),
    }
}
fn issue(data: DraftStocktakeV1, context: StocktakeMaterialContext) -> Vec<StocktakeIssue> {
    match calculate_stocktake(&data, &[context], 20, 19).unwrap() {
        StocktakeCheck::NeedsConfirmation(issues) => issues,
        other => panic!("{other:?}"),
    }
}
fn sale(n: u128, count: &str) -> SaleConsumptionSource {
    SaleConsumptionSource {
        sale_id: id(n),
        sale_version_id: id(n + 100),
        recipe_version_id: id(n + 200),
        original_seq: 12,
        count: SaleCount::parse(count).unwrap(),
        direct_material_ids: vec![id(1), id(1)],
    }
}

#[test]
fn blank_is_unchecked_but_zero_consumes_the_entire_known_value() {
    let result = ready(
        draft(vec![row(" "), row("0")]),
        context(known("3", "0.000001")),
    );
    assert_eq!(result.unchecked_positions, vec![0]);
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.rows[0].position, 1);
    assert_eq!(result.rows[0].after.quantity, "0");
    assert_eq!(result.rows[0].after.value.as_deref(), Some("0"));
    assert!(result.rows[0].after.source_id.is_none());
    match &result.rows[0].calculation {
        StocktakeCalculation::Settlement {
            consumption_quantity,
            consumption_value,
            ..
        } => {
            assert_eq!(consumption_quantity, "3");
            assert_eq!(consumption_value, "0.000001");
        }
        _ => panic!(),
    }
}

#[test]
fn unknown_first_zero_is_initial_and_does_not_settle_earlier_sales() {
    let mut row = row("0");
    row.initial_value = Some(StocktakeValueDraft::ManualAmount { amount: "0".into() });
    let mut context = context(HoldingInput::default());
    context.sales = vec![sale(21, "3")];
    let result = ready(draft(vec![row]), context);
    assert!(result.rows[0].before.is_none());
    assert!(result.rows[0].basis_before.is_none());
    assert!(
        matches!(&result.rows[0].calculation,StocktakeCalculation::Initial{value,..} if value=="0")
    );
}

#[test]
fn first_positive_stock_requires_value_and_can_explicitly_accept_preregistered_price() {
    let mut ctx = context(HoldingInput::default());
    ctx.price = resolve_holding_price(
        &ctx.holding,
        Some(UnitPrice::new(money("15"), q("2"), id(31)).unwrap()),
    )
    .unwrap();
    assert!(!issue(draft(vec![row("4")]), ctx.clone()).is_empty());
    let mut input = row("4");
    input.initial_value = Some(StocktakeValueDraft::ExistingPrice {});
    let result = ready(draft(vec![input]), ctx);
    assert_eq!(result.rows[0].after.value.as_deref(), Some("30"));
    assert!(matches!(
        &result.rows[0].calculation,
        StocktakeCalculation::Initial {
            valuation: StocktakeValuation::ExistingPrice {
                source: StocktakePriceSource::Preregistered,
                ..
            },
            ..
        }
    ));
}

#[test]
fn confirmed_first_zero_cannot_keep_a_positive_value() {
    let mut input = row("0");
    input.initial_value = Some(StocktakeValueDraft::ManualAmount { amount: "1".into() });
    assert!(!issue(draft(vec![input]), context(HoldingInput::default())).is_empty());
}

#[test]
fn known_receipt_basis_allocates_by_count_and_removes_value_once() {
    let mut ctx = context(known("1000", "100"));
    ctx.sales = vec![sale(22, "1"), sale(21, "3")];
    let result = ready(draft(vec![row("600")]), ctx);
    let effect = &result.rows[0];
    assert_eq!(effect.after.value.as_deref(), Some("60"));
    assert!(effect.observation_before.is_none());
    match &effect.calculation {
        StocktakeCalculation::Settlement {
            consumption_quantity,
            consumption_value,
            allocation,
            ..
        } => {
            assert_eq!(
                (consumption_quantity.as_str(), consumption_value.as_str()),
                ("400", "40")
            );
            assert_eq!(allocation.candidates.len(), 2);
            assert_eq!(allocation.candidates[0].sale_id, id(21));
            assert_eq!(allocation.candidates[0].quantity.as_deref(), Some("300"));
            assert_eq!(allocation.candidates[1].value.as_deref(), Some("10"));
            assert_eq!(allocation.unallocated_quantity, "0");
        }
        _ => panic!(),
    }
}

#[test]
fn zero_weights_keep_candidate_relations_and_null_allocations() {
    let mut ctx = context(known("5", "10"));
    ctx.sales = vec![sale(21, "0")];
    let result = ready(draft(vec![row("3")]), ctx);
    match &result.rows[0].calculation {
        StocktakeCalculation::Settlement { allocation, .. } => {
            assert_eq!(allocation.reason, Some(NoAllocationReason::ZeroWeight));
            assert_eq!(allocation.candidates.len(), 1);
            assert!(allocation.candidates[0].quantity.is_none());
            assert!(allocation.candidates[0].value.is_none());
            assert_eq!(allocation.unallocated_quantity, "2");
            assert_eq!(allocation.unallocated_value, "4");
        }
        _ => panic!(),
    }
}

#[test]
fn zero_weight_inside_a_positive_sum_is_real_zero_and_remainders_are_independent() {
    let mut ctx = context(known("0.000002", "0.000001"));
    ctx.sales = vec![sale(21, "1"), sale(22, "1"), sale(23, "0")];
    let result = ready(draft(vec![row("0")]), ctx);
    match &result.rows[0].calculation {
        StocktakeCalculation::Settlement { allocation, .. } => {
            assert_eq!(
                allocation.candidates[0].quantity.as_deref(),
                Some("0.000001")
            );
            assert_eq!(allocation.candidates[0].value.as_deref(), Some("0.000001"));
            assert_eq!(allocation.candidates[1].value.as_deref(), Some("0"));
            assert_eq!(allocation.candidates[2].quantity.as_deref(), Some("0"));
        }
        _ => panic!(),
    }
}

#[test]
fn excess_requires_review_and_reason_and_uses_the_current_zero_price() {
    let ctx = context(known("1", "0"));
    assert!(!issue(draft(vec![row("2")]), ctx.clone()).is_empty());
    let mut input = row("2");
    input.excess = Some(StocktakeExcessDraft {
        reviewed: true,
        reason: "가상 누락 확인".into(),
        value: Some(StocktakeValueDraft::ExistingPrice {}),
    });
    let result = ready(draft(vec![input.clone()]), ctx.clone());
    assert_eq!(result.rows[0].after.value.as_deref(), Some("0"));
    input.excess.as_mut().unwrap().value = Some(StocktakeValueDraft::ManualAmount {
        amount: "100".into(),
    });
    assert!(!issue(draft(vec![input]), ctx).is_empty());
}

#[test]
fn zero_basis_without_price_requires_an_explicit_excess_amount_including_zero() {
    let ctx = context(known("0", "0"));
    let mut input = row("3");
    input.excess = Some(StocktakeExcessDraft {
        reviewed: true,
        reason: "가상 초과 확인".into(),
        value: None,
    });
    assert!(!issue(draft(vec![input.clone()]), ctx.clone()).is_empty());
    input.excess.as_mut().unwrap().value =
        Some(StocktakeValueDraft::ManualAmount { amount: "0".into() });
    assert_eq!(
        ready(draft(vec![input]), ctx).rows[0]
            .after
            .value
            .as_deref(),
        Some("0")
    );
}

#[test]
fn unknown_value_never_falls_back_and_duplicate_or_invalid_rows_are_not_skipped() {
    let mut holding = known("3", "10");
    holding.value = None;
    let ctx = context(holding);
    assert!(
        issue(draft(vec![row("2")]), ctx)
            .iter()
            .any(|i| i.code == "needs_reconciliation")
    );
    assert!(!issue(draft(vec![row("1"), row("2")]), context(known("3", "10"))).is_empty());
    assert!(!issue(draft(vec![row("abc")]), context(known("3", "10"))).is_empty());
    assert!(!issue(draft(vec![row(" ")]), context(known("3", "10"))).is_empty());
}

#[test]
fn intervals_and_units_are_exact_and_cannot_reuse_old_or_future_sales() {
    let mut ctx = context(known("2000", "30"));
    let mut old = sale(21, "1");
    old.original_seq = 10;
    let mut future = sale(22, "1");
    future.original_seq = 20;
    ctx.sales = vec![old, future, sale(23, "1")];
    let mut input = row("1");
    input.unit = "kg".into();
    let result = ready(draft(vec![input]), ctx.clone());
    assert_eq!(result.rows[0].measured_quantity, "1000");
    match &result.rows[0].calculation {
        StocktakeCalculation::Settlement { allocation, .. } => {
            assert_eq!(allocation.candidates.len(), 1)
        }
        _ => panic!(),
    }
    assert!(calculate_stocktake(&draft(vec![row("1")]), &[ctx], 19, 19).is_err());
    let mut input = row("1");
    input.unit = "L".into();
    assert!(!issue(draft(vec![input]), context(known("3", "10"))).is_empty());
}

#[test]
fn no_consumption_is_a_real_settlement_and_stale_confirmation_branch_is_rejected() {
    let result = ready(draft(vec![row("0")]), context(known("0", "0")));
    assert!(
        matches!(&result.rows[0].calculation,StocktakeCalculation::Settlement{consumption_quantity,consumption_value,..} if consumption_quantity=="0" && consumption_value=="0")
    );
    let mut input = row("2");
    input.initial_value = Some(StocktakeValueDraft::ManualAmount { amount: "0".into() });
    assert!(!issue(draft(vec![input]), context(known("2", "0"))).is_empty());
}

#[test]
fn draft_shape_is_strict_and_normalization_preserves_semantic_incompleteness() {
    let mut data = draft(vec![row("abc")]);
    data.note = "첫줄\r\n다음줄".into();
    normalize_stocktake_draft(&mut data).unwrap();
    assert_eq!(data.rows[0].quantity, "abc");
    assert_eq!(data.note, "첫줄\n다음줄");
    let value = serde_json::json!({"kind":"existing_price","extra":1});
    assert!(serde_json::from_value::<StocktakeValueDraft>(value).is_err());
    let mut data = draft(vec![row(""); 101]);
    assert!(normalize_stocktake_draft(&mut data).is_err());
}

#[test]
fn excess_uses_last_valid_price_and_overflow_is_an_issue_without_partial_results() {
    let mut holding = known("0", "0");
    holding.last_valid = Some(UnitPrice::new(money("15"), q("3"), id(80)).unwrap());
    let mut input = row("2");
    input.excess = Some(StocktakeExcessDraft {
        reviewed: true,
        reason: "가상 확인".into(),
        value: Some(StocktakeValueDraft::ExistingPrice {}),
    });
    let result = ready(draft(vec![input.clone()]), context(holding));
    assert_eq!(result.rows[0].after.value.as_deref(), Some("10"));
    input.quantity = "3".into();
    assert!(!issue(draft(vec![input]), context(known("1", "9000000000000"))).is_empty());
}
