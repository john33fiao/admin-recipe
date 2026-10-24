use serde_json::{Value, json};
use std::collections::HashSet;
use sunchang_hansang::{
    domain::{Money, Quantity, UnitPrice},
    recipe::{
        CreateRecipe, RecipeCostInput, RecipeCostLineInput, RecipeDataInput,
        RecipeRatioUnavailable, cost_recipe, next_copy_name, normalize_recipe_data,
        parse_basis_count,
    },
};

fn money(value: &str) -> Money {
    Money::parse(value).unwrap()
}
fn quantity(value: &str) -> Quantity {
    Quantity::parse(value).unwrap()
}
fn priced(q: &str, amount: &str, price_q: &str) -> RecipeCostLineInput {
    RecipeCostLineInput {
        numerator: quantity(q),
        price: Some(
            UnitPrice::new(money(amount), quantity(price_q), "fictional-price".into()).unwrap(),
        ),
    }
}
fn input(rows: Vec<RecipeCostLineInput>) -> RecipeCostInput {
    RecipeCostInput {
        basis_count: 1,
        rows,
        selling_price: Some(money("1000")),
    }
}
fn data() -> Value {
    json!({"basis_count":"1","reference_output":null,"selling_price":null,
        "cooking_description":"","process":"","reference_time":"",
        "lines":[{"material_id":"abcdefab-1234-4000-8000-123456789abc",
            "material_version_id":"abcdefab-1234-4000-8000-123456789abd",
            "quantity":"0","unit":"g","note":"적당량"}]})
}

#[test]
fn batch_fraction_is_preserved_and_cost_rounds_each_direct_row_only_once() {
    let mut batch = input(vec![priced("100", "10", "1")]);
    batch.basis_count = 3;
    let result = cost_recipe(&batch).unwrap();
    assert_eq!(result.row_costs, vec![Some(money("333.333333"))]);
    assert_eq!(result.total_cost, Some(money("333.333333")));
    assert_eq!(result.known_subtotal, money("333.333333"));
    let ratio = result.cost_ratio.unwrap();
    assert_eq!(ratio.numerator, "333333333");
    assert_eq!(ratio.denominator, "1000000000");
    assert_eq!(batch.rows[0].numerator, quantity("100"));
    assert_eq!(batch.basis_count, 3);

    let result = cost_recipe(&input(vec![
        priced("1", "0.000001", "2"),
        priced("1", "0.000001", "2"),
    ]))
    .unwrap();
    assert_eq!(result.row_costs, vec![Some(money("0.000001")); 2]);
    assert_eq!(result.total_cost, Some(money("0.000002")));
}

#[test]
fn exact_holding_ratios_do_not_use_display_unit_price_or_multiply_row_denominators() {
    let mut batch = input(vec![priced("100", "50000", "3000"), priced("1", "10", "7")]);
    batch.basis_count = 3;
    let result = cost_recipe(&batch).unwrap();
    assert_eq!(
        result.row_costs,
        vec![Some(money("555.555556")), Some(money("0.47619"))]
    );
    assert_eq!(result.total_cost, Some(money("556.031746")));
}

#[test]
fn zero_quantity_zero_price_and_unknown_positive_cost_remain_distinct() {
    let zero_missing = RecipeCostLineInput {
        numerator: quantity("0"),
        price: None,
    };
    let result = cost_recipe(&input(vec![zero_missing.clone(), priced("2", "0", "1")])).unwrap();
    assert_eq!(result.row_costs, vec![Some(money("0")); 2]);
    assert_eq!(result.total_cost, Some(money("0")));
    assert_eq!(result.cost_ratio.unwrap().numerator, "0");
    assert!(result.ratio_unavailable_reason.is_none());

    let positive_missing = RecipeCostLineInput {
        numerator: quantity("1"),
        price: None,
    };
    let result = cost_recipe(&input(vec![
        zero_missing,
        priced("2", "3", "1"),
        positive_missing,
    ]))
    .unwrap();
    assert_eq!(
        result.row_costs,
        vec![Some(money("0")), Some(money("6")), None]
    );
    assert_eq!(result.known_subtotal, money("6"));
    assert!(result.total_cost.is_none());
    assert!(result.cost_ratio.is_none());
    assert_eq!(
        result.ratio_unavailable_reason,
        Some(RecipeRatioUnavailable::MissingCost)
    );
}

#[test]
fn missing_or_zero_selling_price_is_not_divided_and_empty_recipe_is_zero() {
    let mut recipe = input(vec![]);
    recipe.selling_price = None;
    let missing = cost_recipe(&recipe).unwrap();
    assert_eq!(missing.total_cost, Some(money("0")));
    assert_eq!(
        missing.ratio_unavailable_reason,
        Some(RecipeRatioUnavailable::MissingSellingPrice)
    );
    recipe.selling_price = Some(money("0"));
    let zero = cost_recipe(&recipe).unwrap();
    assert!(zero.cost_ratio.is_none());
    assert_eq!(
        zero.ratio_unavailable_reason,
        Some(RecipeRatioUnavailable::ZeroSellingPrice)
    );
}

#[test]
fn cost_bounds_are_checked_with_wide_intermediates_and_invalid_basis_is_rejected() {
    let mut largest = input(vec![priced("9000000000", "9000000000000", "9000000000")]);
    largest.basis_count = 1_000_000_000;
    assert_eq!(
        cost_recipe(&largest).unwrap().total_cost,
        Some(money("9000"))
    );
    assert!(
        cost_recipe(&input(vec![priced(
            "9000000000",
            "9000000000000",
            "0.000001"
        )]))
        .is_err()
    );
    assert!(
        cost_recipe(&input(vec![
            priced("1", "9000000000000", "1"),
            priced("1", "0.000001", "1")
        ]))
        .is_err()
    );
    for basis in [0, 1_000_000_001] {
        largest.basis_count = basis;
        assert!(cost_recipe(&largest).is_err());
    }
}

#[test]
fn basis_count_accepts_only_bounded_integer_strings() {
    assert_eq!(parse_basis_count(" 0003 ").unwrap(), 3);
    assert_eq!(parse_basis_count("1000000000").unwrap(), 1_000_000_000);
    for value in [
        "",
        "0",
        "-1",
        "+1",
        "1.0",
        "1e2",
        "1,000",
        "1000000001",
        "１２",
    ] {
        assert!(parse_basis_count(value).is_err(), "{value}");
    }
}

#[test]
fn recipe_inputs_keep_explicit_zero_notes_and_normalize_only_valid_text_and_references() {
    let mut raw = data();
    raw["lines"][0]["material_id"] = json!("ABCDEFAB-1234-4000-8000-123456789ABC");
    raw["lines"][0]["quantity"] = json!(" 000.000000 ");
    raw["lines"][0]["note"] = json!("적당량\r\n나중에 확인");
    raw["process"] = json!("첫 공정\r둘째 공정");
    raw["selling_price"] = json!("0");
    let mut recipe: RecipeDataInput = serde_json::from_value(raw).unwrap();
    normalize_recipe_data(&mut recipe).unwrap();
    assert_eq!(
        recipe.lines[0].material_id,
        "abcdefab-1234-4000-8000-123456789abc"
    );
    assert_eq!(recipe.lines[0].quantity, "000.000000");
    assert_eq!(recipe.lines[0].note, "적당량\n나중에 확인");
    assert_eq!(recipe.process, "첫 공정\n둘째 공정");
    assert_eq!(recipe.selling_price.as_deref(), Some("0"));
}

#[test]
fn reference_output_requires_one_basis_and_positive_quantity_but_does_not_rewrite_lines() {
    let mut raw = data();
    raw["reference_output"] = json!({"quantity":"1.25","unit":"kg"});
    let mut recipe: RecipeDataInput = serde_json::from_value(raw.clone()).unwrap();
    normalize_recipe_data(&mut recipe).unwrap();
    assert_eq!(recipe.lines[0].quantity, "0");
    assert_eq!(recipe.reference_output.unwrap().quantity, "1.25");
    for (field, value) in [("quantity", "0"), ("quantity", "1.0000001"), ("unit", "")] {
        let mut invalid = raw.clone();
        invalid["reference_output"][field] = json!(value);
        let mut invalid: RecipeDataInput = serde_json::from_value(invalid).unwrap();
        assert!(normalize_recipe_data(&mut invalid).is_err());
    }
    raw["basis_count"] = json!("2");
    assert!(normalize_recipe_data(&mut serde_json::from_value(raw).unwrap()).is_err());
}

#[test]
fn strict_envelopes_and_nested_shapes_reject_extra_fields_and_numeric_values() {
    let good = json!({"meta":{"request_id":"abcdefab-1234-4000-8000-123456789abc",
        "epoch":"abcdefab-1234-4000-8000-123456789abd","expected_revision":"0"},
        "owner":{"mode":"new","name":"가상 메뉴","unit":"개","kind":"menu","description":""},"data":data()});
    serde_json::from_value::<CreateRecipe>(good.clone()).unwrap();
    for pointer in [
        "/data/basis_count",
        "/data/lines/0/quantity",
        "/data/selling_price",
        "/meta/expected_revision",
    ] {
        let mut invalid = good.clone();
        *invalid.pointer_mut(pointer).unwrap() = json!(1);
        assert!(
            serde_json::from_value::<CreateRecipe>(invalid).is_err(),
            "{pointer}"
        );
    }
    for pointer in ["", "/meta", "/owner", "/data", "/data/lines/0"] {
        let mut invalid = good.clone();
        invalid
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("extra".into(), json!(true));
        assert!(
            serde_json::from_value::<CreateRecipe>(invalid).is_err(),
            "{pointer}"
        );
    }
    let mut invalid = good;
    invalid["data"]["reference_output"] = json!({"quantity":"1","unit":"g","extra":true});
    assert!(serde_json::from_value::<CreateRecipe>(invalid).is_err());
}

#[test]
fn recipe_limits_reject_excess_rows_text_bad_numbers_and_control_characters() {
    let original = data();
    let mut cases = Vec::new();
    for value in ["", "-1", "1e3", "0.0000001", "9000000001"] {
        let mut invalid = original.clone();
        invalid["lines"][0]["quantity"] = json!(value);
        cases.push(invalid);
    }
    for (field, text) in [
        ("cooking_description", "가".repeat(2001)),
        ("process", "가".repeat(4001)),
        ("reference_time", "가".repeat(201)),
        ("reference_time", "1\t2".into()),
    ] {
        let mut invalid = original.clone();
        invalid[field] = json!(text);
        cases.push(invalid);
    }
    let mut many = original.clone();
    many["lines"] = json!(vec![original["lines"][0].clone(); 101]);
    cases.push(many);
    let mut note = original;
    note["lines"][0]["note"] = json!("가".repeat(201));
    cases.push(note);
    for raw in cases {
        let mut invalid: RecipeDataInput = serde_json::from_value(raw).unwrap();
        assert!(normalize_recipe_data(&mut invalid).is_err());
    }
}

#[test]
fn independent_copy_names_skip_collisions_without_changing_the_original() {
    let mut occupied = HashSet::from([
        "가상 메뉴".into(),
        "가상 메뉴 (2)".into(),
        "가상 메뉴 (3)".into(),
    ]);
    assert_eq!(
        next_copy_name("가상 메뉴", &occupied).unwrap(),
        "가상 메뉴 (4)"
    );
    assert_eq!(occupied.len(), 3);
    assert_eq!(
        next_copy_name("가상 메뉴 (2)", &occupied).unwrap(),
        "가상 메뉴 (2) (2)"
    );
    for n in 4..=9 {
        occupied.insert(format!("가상 메뉴 ({n})"));
    }
    assert_eq!(
        next_copy_name("가상 메뉴", &occupied).unwrap(),
        "가상 메뉴 (10)"
    );
    assert!(next_copy_name(" ", &occupied).is_err());
    assert!(next_copy_name("줄\n바꿈", &occupied).is_err());
}

#[test]
fn copy_name_capacity_uses_unicode_scalars_and_recomputes_space_for_number_digits() {
    for character in ["가", "😀"] {
        let original = character.repeat(100);
        let mut occupied = HashSet::new();
        for n in 2..=9 {
            occupied.insert(format!("{} ({n})", character.repeat(96)));
        }
        let actual = next_copy_name(&original, &occupied).unwrap();
        assert_eq!(actual, format!("{} (10)", character.repeat(95)));
        assert_eq!(actual.chars().count(), 100);
        assert_eq!(original.chars().count(), 100);
    }
}
