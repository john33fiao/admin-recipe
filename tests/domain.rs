use sunchang_hansang::domain::{
    HoldingBasis, Money, Quantity, Unit, UnitPrice, resolve_price, round_ratio,
};

fn q(value: &str) -> Quantity {
    Quantity::parse(value).unwrap()
}
fn m(value: &str) -> Money {
    Money::parse(value).unwrap()
}
fn price(amount: &str, quantity: &str, source: &str) -> UnitPrice {
    UnitPrice::new(m(amount), q(quantity), source.into()).unwrap()
}

#[test]
fn decimal_boundaries_and_json_are_exact() {
    assert_eq!(q("0.000001").micros(), 1);
    assert_eq!(q(" 00012.340000 ").decimal(), "12.34");
    assert_eq!(q("9000000000").micros(), 9_000_000_000_000_000);
    assert_eq!(m("9000000000000").micros(), 9_000_000_000_000_000_000);
    for bad in [
        "",
        "-1",
        "-0",
        "+1",
        "1e3",
        "NaN",
        "1,000",
        "1.0000001",
        ".1",
        "1.",
        "１２",
        "1 2",
    ] {
        assert!(Quantity::parse(bad).is_err(), "{bad}");
    }
    assert!(Quantity::parse("9000000000.000001").is_err());
    assert!(Money::parse("9000000000000.000001").is_err());
    assert!(Quantity::from_micros(-1).is_err());
    assert!(Money::from_micros(i64::MAX).is_err());
    let encoded = serde_json::to_string(&m("1234567890123.123456")).unwrap();
    assert_eq!(encoded, "\"1234567890123.123456\"");
    assert_eq!(
        serde_json::from_str::<Money>(&encoded).unwrap().decimal(),
        "1234567890123.123456"
    );
    assert!(serde_json::from_str::<Money>("0").is_err());
    assert!(serde_json::from_str::<Quantity>("null").is_err());
}

#[test]
fn unit_dimensions_and_canonical_bounds_do_not_cross() {
    let kg = Unit::parse("kg").unwrap();
    let grams = Unit::parse("g").unwrap();
    assert_eq!(kg.to_canonical(q("1.25")).unwrap(), q("1250"));
    assert_eq!(kg.canonical(), "g");
    assert_eq!(kg.dimension(), "mass");
    assert!(kg.same_dimension(&grams));
    assert!(!grams.same_dimension(&Unit::parse("mL").unwrap()));
    assert_eq!(
        Unit::parse("L").unwrap().to_canonical(q("0.001")).unwrap(),
        q("1")
    );
    assert!(
        !Unit::parse("장")
            .unwrap()
            .same_dimension(&Unit::parse("개").unwrap())
    );
    assert!(
        Unit::parse("묶음")
            .unwrap()
            .same_dimension(&Unit::parse("묶음").unwrap())
    );
    assert_eq!(Unit::parse("KG").unwrap().dimension(), "custom");
    assert!(kg.to_canonical(q("9000000000")).is_err());
    assert!(Unit::parse("\n").is_err());
}

#[test]
fn proportional_rounding_uses_checked_signed_integers() {
    assert_eq!(round_ratio(5, 2).unwrap(), 3);
    assert_eq!(round_ratio(-5, 2).unwrap(), -3);
    assert_eq!(round_ratio(4, 3).unwrap(), 1);
    assert_eq!(round_ratio(-4, 3).unwrap(), -1);
    assert_eq!(round_ratio(i128::MAX, i128::MAX).unwrap(), 1);
    assert!(round_ratio(1, 0).is_err());
    assert!(round_ratio(1, -1).is_err());
    assert!(round_ratio(i128::MIN, 1).is_err());
    let cost = round_ratio(
        i128::from(m("40000").micros()) * i128::from(q("1000").micros()),
        i128::from(q("3000").micros()),
    )
    .unwrap();
    assert_eq!(cost, i128::from(m("13333.333333").micros()));
    assert_eq!(
        m("40000").micros() - i64::try_from(cost).unwrap(),
        m("26666.666667").micros()
    );
}

#[test]
fn price_priority_preserves_zero_missing_and_exact_sources() {
    let last = price("1000", "100", "last");
    let preregistered = price("0", "100", "pre");
    let held = resolve_price(
        HoldingBasis::Known {
            quantity: q("100"),
            value: m("0"),
            source_id: "receipt".into(),
        },
        Some(last.clone()),
        Some(preregistered.clone()),
    );
    assert_eq!(held.source, "holding_average");
    assert_eq!(held.amount.as_deref(), Some("0"));
    assert_eq!(held.quantity.as_deref(), Some("100"));
    assert_eq!(held.source_id.as_deref(), Some("receipt"));
    let empty = resolve_price(
        HoldingBasis::Known {
            quantity: q("0"),
            value: m("0"),
            source_id: "take".into(),
        },
        Some(last),
        Some(preregistered.clone()),
    );
    assert_eq!(empty.source, "last_valid");
    assert_eq!(empty.source_id.as_deref(), Some("last"));
    let pre = resolve_price(
        HoldingBasis::NotEstablished,
        None,
        Some(preregistered.clone()),
    );
    assert_eq!(pre.source, "preregistered");
    assert_eq!(pre.amount.as_deref(), Some("0"));
    let unknown = resolve_price(
        HoldingBasis::ValueUnknown { quantity: q("1") },
        None,
        Some(preregistered.clone()),
    );
    assert_eq!(unknown.source, "missing");
    assert!(unknown.amount.is_none());
    assert!(unknown.reason.is_some());
    let empty_without_history = HoldingBasis::Known {
        quantity: q("0"),
        value: m("0"),
        source_id: "empty".into(),
    };
    assert_eq!(
        resolve_price(empty_without_history.clone(), None, Some(preregistered)).source,
        "preregistered"
    );
    assert_eq!(
        resolve_price(empty_without_history, None, None).source,
        "missing"
    );
    let inconsistent = resolve_price(
        HoldingBasis::Known {
            quantity: q("0"),
            value: m("1"),
            source_id: "invalid".into(),
        },
        Some(price("10", "1", "last")),
        Some(price("5", "1", "pre")),
    );
    assert_eq!(inconsistent.source, "missing");
    assert!(inconsistent.amount.is_none());
    assert_eq!(
        resolve_price(HoldingBasis::NotEstablished, None, None).source,
        "missing"
    );
    assert!(UnitPrice::new(m("10"), q("0"), "invalid".into()).is_err());
}
