use serde_json::json;
use sunchang_hansang::statistics::{
    DecimalTotal, StatisticsError, StatisticsQuery, StatisticsWindow, enforce_report_limit,
};

#[test]
fn storage_errors_do_not_expose_internal_details() {
    let error: StatisticsError =
        sunchang_hansang::store::StoreError::Unavailable("가상 내부 파일 경로와 SQL 진단".into())
            .into();
    assert!(matches!(error, StatisticsError::Unavailable(_)));
    assert_eq!(error.to_string(), "통계 자료를 읽을 수 없습니다");
}

#[test]
fn range_is_offset_aware_and_half_open() {
    let window = StatisticsWindow::parse(&StatisticsQuery {
        from: "2026-01-01T09:00:00+09:00".into(),
        until: "2026-01-02T09:00:00+09:00".into(),
    })
    .unwrap();
    assert_eq!(window.range().from, "2026-01-01T00:00:00Z");
    assert_eq!(window.range().until, "2026-01-02T00:00:00Z");
    assert!(window.contains("2026-01-01T00:00:00Z").unwrap());
    assert!(window.contains("2026-01-01T23:59:59.999999Z").unwrap());
    assert!(!window.contains("2026-01-02T00:00:00Z").unwrap());
    assert!(!window.contains("2025-12-31T23:59:59Z").unwrap());
    assert!(matches!(
        window.contains("bad-time"),
        Err(StatisticsError::Unavailable(_))
    ));
}

#[test]
fn invalid_or_ambiguous_ranges_are_rejected() {
    for (from, until) in [
        ("2026-01-01T00:00:00", "2026-01-02T00:00:00Z"),
        ("2026-02-30T00:00:00Z", "2026-03-02T00:00:00Z"),
        ("2026-01-01T00:00:00Z", "2026-01-01T00:00:00Z"),
        ("2026-01-02T00:00:00Z", "2026-01-01T00:00:00Z"),
        ("2026-01-01T00:00:00.0000000001Z", "2026-01-02T00:00:00Z"),
        ("2026-01-01T00:00:00.1234567890Z", "2026-01-02T00:00:00Z"),
    ] {
        assert!(matches!(
            StatisticsWindow::parse(&StatisticsQuery {
                from: from.into(),
                until: until.into()
            }),
            Err(StatisticsError::Validation(_))
        ));
    }
    assert!(
        serde_json::from_value::<StatisticsQuery>(json!({"from":"a","until":"b","extra":true}))
            .is_err()
    );
}

#[test]
fn utc_conversion_does_not_impose_twenty_four_hour_days() {
    let window = StatisticsWindow::parse(&StatisticsQuery {
        from: "2026-03-08T00:00:00-05:00".into(),
        until: "2026-03-09T00:00:00-04:00".into(),
    })
    .unwrap();
    assert_eq!(window.range().from, "2026-03-08T05:00:00Z");
    assert_eq!(window.range().until, "2026-03-09T04:00:00Z");
}

#[test]
fn nanosecond_boundaries_are_preserved_in_comparison_and_echo() {
    let window = StatisticsWindow::parse(&StatisticsQuery {
        from: "2026-01-01T09:00:00.000000001+09:00".into(),
        until: "2026-01-01T09:00:00.000000002+09:00".into(),
    })
    .unwrap();
    assert_eq!(window.range().from, "2026-01-01T00:00:00.000000001Z");
    assert!(window.contains("2026-01-01T00:00:00.000000001Z").unwrap());
    assert!(!window.contains("2026-01-01T00:00:00Z").unwrap());
    assert!(!window.contains("2026-01-01T00:00:00.000000002Z").unwrap());
}

#[test]
fn wide_decimal_totals_preserve_micro_units_and_exceed_single_money_limits() {
    let mut total = DecimalTotal::default();
    total.add("9000000000000").unwrap();
    total.add("9000000000000").unwrap();
    total.add("0.000001").unwrap();
    assert_eq!(total.decimal(), "18000000000000.000001");
    assert_eq!(DecimalTotal::default().decimal(), "0");
    assert!(total.add("0.0000001").is_err());
}

#[test]
fn report_limit_counts_nested_rows_and_never_silently_truncates() {
    let exact = json!({"rows":vec![json!({"value":"0"}); 10_000]});
    enforce_report_limit(&exact).unwrap();
    let over = json!({"rows":vec![json!({"children":[{"value":"0"}]} ); 5_001]});
    assert!(matches!(
        enforce_report_limit(&over),
        Err(StatisticsError::ReportTooLarge)
    ));
}
