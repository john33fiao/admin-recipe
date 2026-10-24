use serde_json::{Value, json};
use sunchang_hansang::{
    domain::{Money, ResolvedPrice, allocate_largest_remainder},
    sale::{
        DraftSaleV1, SaleConsumptionSource, SaleCount, SaleInterval, SalePriceSnapshot,
        SalePriceSource, SaleRecipeCost, calculate_sale_amount, eligible_sales,
        normalize_sale_draft, sum_sale_amounts,
    },
};

const MATERIAL: &str = "00000000-0000-4000-8000-000000000001";
const OTHER_MATERIAL: &str = "00000000-0000-4000-8000-000000000002";
const RECIPE: &str = "00000000-0000-4000-8000-000000000003";
const PRICE: &str = "00000000-0000-4000-8000-000000000004";

fn count(value: &str) -> SaleCount {
    SaleCount::parse(value).unwrap()
}

fn money(value: &str) -> Money {
    Money::parse(value).unwrap()
}

fn source(id: u64, seq: i64, amount: &str, refs: &[&str]) -> SaleConsumptionSource {
    SaleConsumptionSource {
        sale_id: format!("10000000-0000-4000-8000-{id:012}"),
        sale_version_id: format!("20000000-0000-4000-8000-{id:012}"),
        recipe_version_id: RECIPE.into(),
        original_seq: seq,
        count: count(amount),
        direct_material_ids: refs.iter().map(|id| (*id).to_owned()).collect(),
    }
}

fn interval() -> SaleInterval {
    SaleInterval {
        material_id: MATERIAL.into(),
        after_seq: 10,
        before_seq: 30,
    }
}

fn cost_json() -> Value {
    json!({
        "recipe_version_id": RECIPE, "basis_count":"3",
        "rows":[{"line_id":PRICE,"material_id":MATERIAL,"material_version_id":OTHER_MATERIAL,
            "material_name":"가상 당시 재료", "numerator_quantity":"0", "basis_count":"3", "unit":"g",
            "price":{"source":"missing","source_id":null,"amount":null,"quantity":null,"reason":"등록 가격 없음"},
            "cost":"0"}],
        "missing_price_line_ids":[PRICE],"known_subtotal":"0","total_cost":"0","selling_price":null,
        "cost_ratio":null,"ratio_unavailable_reason":"missing_selling_price"
    })
}

#[test]
fn sale_count_accepts_zero_and_upper_bound_without_decimal_coercion() {
    for (raw, expected) in [("0", 0), (" 0003 ", 3), ("1000000000", 1_000_000_000)] {
        assert_eq!(count(raw).value(), expected);
        assert_eq!(count(raw).decimal(), expected.to_string());
    }
    for raw in [
        "",
        " ",
        "-1",
        "+1",
        "1.0",
        "1e3",
        "１",
        "1000000001",
        "999999999999999999999",
    ] {
        assert!(SaleCount::parse(raw).is_err(), "{raw:?}");
    }
}

#[test]
fn selling_price_absence_remains_absent_even_for_zero_sales() {
    assert_eq!(calculate_sale_amount(count("0"), None).unwrap(), None);
    assert_eq!(calculate_sale_amount(count("3"), None).unwrap(), None);
    for (quantity, price) in [("0", "1200"), ("3", "0")] {
        assert_eq!(
            calculate_sale_amount(count(quantity), Some(money(price)))
                .unwrap()
                .unwrap()
                .decimal(),
            "0"
        );
    }
    assert_eq!(
        calculate_sale_amount(count("3"), Some(money("0.123456")))
            .unwrap()
            .unwrap()
            .decimal(),
        "0.370368"
    );
}

#[test]
fn each_sale_keeps_the_money_bound_with_checked_integer_multiplication() {
    assert_eq!(
        calculate_sale_amount(count("1"), Some(money("9000000000000")))
            .unwrap()
            .unwrap()
            .decimal(),
        "9000000000000"
    );
    assert!(calculate_sale_amount(count("2"), Some(money("9000000000000"))).is_err());
    assert_eq!(
        calculate_sale_amount(count("1000000000"), Some(money("9000")))
            .unwrap()
            .unwrap()
            .decimal(),
        "9000000000000"
    );
    assert!(calculate_sale_amount(count("1000000000"), Some(money("9000.000001"))).is_err());
}

#[test]
fn list_revenue_can_exceed_one_sale_bound_and_distinguishes_null_from_zero() {
    let maximum = money("9000000000000");
    assert_eq!(
        sum_sale_amounts(&[Some(maximum), Some(maximum), Some(money("0")), None]).unwrap(),
        ("18000000000000".into(), 1)
    );
    assert_eq!(
        sum_sale_amounts(&[Some(maximum), Some(money("0")), None]).unwrap(),
        ("9000000000000".into(), 1)
    );
    assert_eq!(
        sum_sale_amounts(&[Some(money("0.000001")), Some(money("0.1"))]).unwrap(),
        ("0.100001".into(), 0)
    );
    assert_eq!(sum_sale_amounts(&[None, None]).unwrap(), ("0".into(), 2));
    assert_eq!(sum_sale_amounts(&[]).unwrap(), ("0".into(), 0));
}

#[test]
fn draft_normalization_preserves_incomplete_and_zero_counts() {
    let mut draft = DraftSaleV1 {
        material_id: Some("AAAAAAAA-0000-4000-8000-000000000001".into()),
        material_version_id: None,
        recipe_version_id: None,
        count: "".into(),
        note: "가상 첫줄\r\n다음줄\r끝".into(),
    };
    normalize_sale_draft(&mut draft).unwrap();
    assert_eq!(
        draft.material_id.as_deref(),
        Some("aaaaaaaa-0000-4000-8000-000000000001")
    );
    assert_eq!(draft.note, "가상 첫줄\n다음줄\n끝");
    assert_eq!(draft.count, "");
    draft.count = "0".into();
    normalize_sale_draft(&mut draft).unwrap();
    assert_eq!(draft.count, "0");
}

#[test]
fn draft_structure_bounds_do_not_turn_invalid_text_into_a_sale() {
    let base = DraftSaleV1 {
        material_id: None,
        material_version_id: None,
        recipe_version_id: None,
        count: "-1.2".into(),
        note: String::new(),
    };
    normalize_sale_draft(&mut base.clone()).unwrap();
    let mut invalid = base.clone();
    invalid.count = "1".repeat(81);
    assert!(normalize_sale_draft(&mut invalid).is_err());
    let mut invalid = base.clone();
    invalid.note = "가".repeat(2001);
    assert!(normalize_sale_draft(&mut invalid).is_err());
    let mut invalid = base.clone();
    invalid.note = "bad\0text".into();
    assert!(normalize_sale_draft(&mut invalid).is_err());
    let mut invalid = base;
    invalid.recipe_version_id = Some("bad".into());
    assert!(normalize_sale_draft(&mut invalid).is_err());
    let raw = json!({"material_id":null,"material_version_id":null,"recipe_version_id":null,"count":"0","note":""});
    let mut numeric = raw.clone();
    numeric["count"] = json!(0);
    assert!(serde_json::from_value::<DraftSaleV1>(numeric).is_err());
    let mut unknown = raw;
    unknown["inventory_delta"] = json!(0);
    assert!(serde_json::from_value::<DraftSaleV1>(unknown).is_err());
}

#[test]
fn price_snapshot_preserves_zero_and_validates_source_shape() {
    let resolved = ResolvedPrice {
        source: "holding_average".into(),
        source_id: Some(PRICE.into()),
        amount: Some("0".into()),
        quantity: Some("3".into()),
        reason: None,
    };
    let price = SalePriceSnapshot::from_resolved(resolved).unwrap();
    assert_eq!(price.source, SalePriceSource::HoldingAverage);
    assert_eq!(price.amount.as_deref(), Some("0"));
    price.validate().unwrap();
    let mut invalid = price.clone();
    invalid.quantity = Some("0".into());
    assert!(invalid.validate().is_err());
    let mut invalid = price.clone();
    invalid.source_id = Some("missing-id".into());
    assert!(invalid.validate().is_err());
    let mut invalid = price;
    invalid.reason = Some("not a missing price".into());
    assert!(invalid.validate().is_err());
    let missing = SalePriceSnapshot::from_resolved(ResolvedPrice {
        source: "missing".into(),
        source_id: None,
        amount: None,
        quantity: None,
        reason: Some("가치 미확인".into()),
    })
    .unwrap();
    let mut invalid = missing;
    invalid.amount = Some("0".into());
    assert!(invalid.validate().is_err());
    assert!(
        SalePriceSnapshot::from_resolved(ResolvedPrice {
            source: "future_price".into(),
            source_id: None,
            amount: None,
            quantity: None,
            reason: None
        })
        .is_err()
    );
}

#[test]
fn frozen_cost_dto_keeps_row_names_and_rejects_nested_unknown_price_fields() {
    let raw = cost_json();
    let cost: SaleRecipeCost = serde_json::from_value(raw.clone()).unwrap();
    assert_eq!(cost.rows[0].material_name, "가상 당시 재료");
    assert_eq!(cost.rows[0].cost.as_deref(), Some("0"));
    assert_eq!(cost.rows[0].price.source, SalePriceSource::Missing);
    let mut unknown = raw.clone();
    unknown["rows"][0]["price"]["future"] = json!(true);
    assert!(serde_json::from_value::<SaleRecipeCost>(unknown).is_err());
    let mut unknown = raw.clone();
    unknown["rows"][0]["price"]["source"] = json!("future_price");
    assert!(serde_json::from_value::<SaleRecipeCost>(unknown).is_err());
    let mut numeric = raw;
    numeric["rows"][0]["numerator_quantity"] = json!(0);
    assert!(serde_json::from_value::<SaleRecipeCost>(numeric).is_err());
}

#[test]
fn consumption_candidates_use_open_original_interval_and_direct_refs_once() {
    let inputs = vec![
        source(1, 10, "1", &[MATERIAL]),
        source(2, 15, "3", &[MATERIAL, MATERIAL]),
        source(3, 20, "0", &[MATERIAL]),
        source(4, 30, "1", &[MATERIAL]),
        source(5, 25, "9", &[OTHER_MATERIAL]),
    ];
    let preserved = inputs.clone();
    let result = eligible_sales(&interval(), &inputs).unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].sale_id, inputs[1].sale_id);
    assert_eq!(result[0].direct_material_ids, vec![MATERIAL]);
    assert_eq!(result[1].count.value(), 0);
    assert_eq!(inputs, preserved);
}

#[test]
fn consumption_candidates_reject_duplicate_sales_and_invalid_references() {
    let valid = source(1, 15, "1", &[MATERIAL]);
    assert!(eligible_sales(&interval(), &[valid.clone(), valid.clone()]).is_err());
    let mut invalid = valid.clone();
    invalid.original_seq = 0;
    assert!(eligible_sales(&interval(), &[invalid]).is_err());
    let mut invalid = valid.clone();
    invalid.sale_version_id = "bad".into();
    assert!(eligible_sales(&interval(), &[invalid]).is_err());
    let mut invalid = valid;
    invalid.direct_material_ids.push("bad".into());
    assert!(eligible_sales(&interval(), &[invalid]).is_err());
    let mut invalid = interval();
    invalid.before_seq = invalid.after_seq;
    assert!(eligible_sales(&invalid, &[]).is_err());
}

#[test]
fn synthetic_correction_changes_weights_without_moving_original_interval() {
    let first = source(1, 15, "3", &[MATERIAL]);
    let second = source(2, 20, "1", &[MATERIAL]);
    let original = eligible_sales(&interval(), &[first.clone(), second.clone()]).unwrap();
    let weights: Vec<i64> = original
        .iter()
        .map(|row| i64::from(row.count.value()))
        .collect();
    assert_eq!(
        allocate_largest_remainder(400_000_000, &weights).unwrap(),
        vec![300_000_000, 100_000_000]
    );
    let mut corrected = first;
    corrected.count = count("1");
    corrected.sale_version_id = "30000000-0000-4000-8000-000000000001".into();
    let revised = eligible_sales(&interval(), &[corrected, second]).unwrap();
    assert_eq!(revised[0].original_seq, 15);
    assert_eq!(revised[0].recipe_version_id, RECIPE);
    let weights: Vec<i64> = revised
        .iter()
        .map(|row| i64::from(row.count.value()))
        .collect();
    assert_eq!(
        allocate_largest_remainder(400_000_000, &weights).unwrap(),
        vec![200_000_000, 200_000_000]
    );
    assert_eq!(
        allocate_largest_remainder(4_000_000_000, &weights).unwrap(),
        vec![2_000_000_000, 2_000_000_000]
    );
    // These are synthetic consumption inputs, not an inventory or stocktake writer.
}

#[test]
fn zero_weights_and_ties_keep_explicit_unallocated_and_deterministic_contracts() {
    let zero = eligible_sales(&interval(), &[source(1, 15, "0", &[MATERIAL])]).unwrap();
    assert_eq!(zero[0].count.value(), 0);
    assert!(allocate_largest_remainder(1, &[0]).is_err());
    let ordered = eligible_sales(
        &interval(),
        &[
            source(2, 15, "1", &[MATERIAL]),
            source(1, 15, "1", &[MATERIAL]),
        ],
    )
    .unwrap();
    assert!(ordered[0].sale_id < ordered[1].sale_id);
    assert_eq!(allocate_largest_remainder(1, &[1, 1]).unwrap(), vec![1, 0]);
}
