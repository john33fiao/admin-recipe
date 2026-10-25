use serde_json::{Value, json};
use sunchang_hansang::overview::{
    OverviewChangeBoundary, OverviewError, OverviewListQuery, OverviewListResponse,
    OverviewSection, overview_page,
};

const EPOCH: &str = "00000000-0000-4000-8000-00000000000a";
const MATERIAL: &str = "00000000-0000-4000-8000-00000000000b";
const SECTIONS: [&str; 10] = [
    "materials",
    "material_changes",
    "open_orders",
    "received_orders",
    "receipts",
    "sales",
    "observations",
    "unallocated",
    "unconfirmed",
    "drafts",
];
fn input(section: &str) -> Value {
    let mut input = json!({"section":section,"offset":"0","epoch":EPOCH,"revision":"0"});
    if section == "material_changes" {
        input["material_id"] = json!(MATERIAL);
    }
    input
}
fn parse(input: Value) -> OverviewListQuery {
    serde_json::from_value(input).unwrap()
}

#[test]
fn all_ten_sections_have_exact_scope_and_only_material_changes_accepts_a_material() {
    for section in SECTIONS {
        let query = parse(input(section));
        assert_eq!(query.validate().unwrap(), 0);
        let scope = serde_json::to_value(query.scope()).unwrap();
        assert_eq!(scope["section"], section);
        assert_eq!(
            scope["material_id"],
            if section == "material_changes" {
                json!(MATERIAL)
            } else {
                Value::Null
            }
        );
        assert_eq!(
            query.section.page_size(),
            if section == "materials" { 25 } else { 20 }
        );
        let mut wrong = input(section);
        if section == "material_changes" {
            wrong.as_object_mut().unwrap().remove("material_id");
        } else {
            wrong["material_id"] = json!(MATERIAL);
        }
        assert!(matches!(
            parse(wrong).validate(),
            Err(OverviewError::Validation(_))
        ));
    }
    for id in [
        "",
        "bad",
        "00000000-0000-4000-8000-00000000000B",
        "0000000000004000800000000000000b",
    ] {
        let mut query = input("material_changes");
        query["material_id"] = json!(id);
        assert!(matches!(
            parse(query).validate(),
            Err(OverviewError::Validation(_))
        ));
    }
}

#[test]
fn numeric_and_uuid_query_inputs_are_canonical_and_checked_at_their_own_bounds() {
    let mut maximum = input("materials");
    maximum["offset"] = json!(usize::MAX.to_string());
    maximum["revision"] = json!(i64::MAX.to_string());
    assert_eq!(parse(maximum).validate().unwrap(), usize::MAX);
    for key in ["offset", "revision"] {
        for bad in [
            "",
            "-1",
            "+1",
            "01",
            " 0",
            "0 ",
            "1.0",
            "1e2",
            "１",
            "184467440737095516160",
        ] {
            let mut v = input("materials");
            v[key] = json!(bad);
            assert!(
                matches!(parse(v).validate(), Err(OverviewError::Validation(_))),
                "{key}={bad}"
            );
        }
    }
    let mut overflow = input("materials");
    overflow["revision"] = json!((i128::from(i64::MAX) + 1).to_string());
    assert!(parse(overflow).validate().is_err());
    for bad in [
        "",
        "not-a-uuid",
        "00000000-0000-4000-8000-00000000000A",
        "0000000000004000800000000000000a",
        "urn:uuid:00000000-0000-4000-8000-00000000000a",
        "{00000000-0000-4000-8000-00000000000a}",
    ] {
        let mut v = input("materials");
        v["epoch"] = json!(bad);
        assert!(parse(v).validate().is_err(), "epoch={bad}");
    }
}

#[test]
fn query_deserialization_rejects_missing_unknown_duplicate_and_numeric_json_fields() {
    for key in ["section", "offset", "epoch", "revision"] {
        let mut v = input("materials");
        v.as_object_mut().unwrap().remove(key);
        assert!(
            serde_json::from_value::<OverviewListQuery>(v).is_err(),
            "missing {key}"
        );
    }
    for key in ["limit", "sort", "from", "until", "unrecognized"] {
        let mut v = input("materials");
        v[key] = json!("1");
        assert!(
            serde_json::from_value::<OverviewListQuery>(v).is_err(),
            "unknown {key}"
        );
    }
    for key in ["offset", "revision"] {
        let mut v = input("materials");
        v[key] = json!(0);
        assert!(serde_json::from_value::<OverviewListQuery>(v).is_err());
    }
    let mut v = input("materials");
    v["section"] = json!("holding");
    assert!(serde_json::from_value::<OverviewListQuery>(v).is_err());
    let valid = serde_json::to_string(&input("materials")).unwrap();
    for (key, value) in [
        ("section", "materials"),
        ("offset", "0"),
        ("epoch", EPOCH),
        ("revision", "0"),
    ] {
        let duplicate = format!("{},\"{key}\":\"{value}\"}}", &valid[..valid.len() - 1]);
        assert!(
            serde_json::from_str::<OverviewListQuery>(&duplicate).is_err(),
            "duplicate {key}"
        );
    }
}

#[test]
fn fixed_pages_preserve_total_and_cover_all_rows_without_duplicates_or_overflow() {
    let rows: Vec<usize> = (0..53).collect();
    for (section, size) in [
        (OverviewSection::Materials, 25),
        (OverviewSection::Receipts, 20),
    ] {
        let mut offset = 0;
        let mut seen = Vec::new();
        loop {
            let page = overview_page(&rows, offset, section);
            assert_eq!(page.total_count, 53);
            assert_eq!(page.page_size, size);
            assert_eq!(page.offset, offset);
            assert_eq!(page.returned_count, page.rows.len());
            assert!(page.rows.len() <= size);
            seen.extend(page.rows);
            if let Some(next) = page.next_offset {
                assert_eq!(next, offset + size);
                offset = next;
            } else {
                break;
            }
        }
        assert_eq!(seen, rows);
        for offset in [53, 54, usize::MAX - 1, usize::MAX] {
            let page = overview_page(&rows, offset, section);
            assert_eq!(page.total_count, 53);
            assert_eq!(page.offset, offset);
            assert_eq!(page.returned_count, 0);
            assert!(page.rows.is_empty());
            assert_eq!(page.next_offset, None);
        }
    }
    let empty = overview_page::<usize>(&[], usize::MAX, OverviewSection::Materials);
    assert_eq!(empty.total_count, 0);
    assert_eq!(empty.returned_count, 0);
    assert!(empty.rows.is_empty());
    assert_eq!(empty.next_offset, None);
}

#[test]
fn exact_page_boundary_has_no_phantom_continuation_and_wire_counts_are_numbers() {
    for section in [OverviewSection::Materials, OverviewSection::Drafts] {
        let size = section.page_size();
        let rows: Vec<String> = (0..size).map(|i| i.to_string()).collect();
        let first = overview_page(&rows, 0, section);
        assert_eq!(first.next_offset, None);
        let last = overview_page(&rows, size - 1, section);
        assert_eq!(last.rows, vec![(size - 1).to_string()]);
        assert_eq!(last.next_offset, None);
        let wire = serde_json::to_value(&first).unwrap();
        for key in ["total_count", "returned_count", "offset", "page_size"] {
            assert!(wire[key].is_u64());
        }
        assert!(wire["rows"][0].is_string());
        assert_eq!(wire.as_object().unwrap().len(), 6);
    }
}

#[test]
fn response_tags_are_closed_and_not_observed_never_serializes_a_fake_zero() {
    let missing = OverviewChangeBoundary::NotObserved {};
    assert_eq!(
        serde_json::to_value(&missing).unwrap(),
        json!({"kind":"not_observed"})
    );
    assert!(
        serde_json::from_value::<OverviewChangeBoundary>(
            json!({"kind":"not_observed","quantity":"0"})
        )
        .is_err()
    );
    let mut response = json!({
        "state":{"epoch":EPOCH,"revision":"0"},
        "scope":{"section":"material_changes","material_id":MATERIAL},
        "list":{"section":"material_changes","material":{"id":MATERIAL,"version_id":EPOCH,"name":"가상 품목","unit":"g","archived":false},"boundary":{"kind":"not_observed"},
          "page":{"total_count":0,"returned_count":0,"offset":0,"page_size":20,"next_offset":null,"rows":[]}}
    });
    let decoded: OverviewListResponse = serde_json::from_value(response.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), response);
    response["list"]["holding_quantity"] = json!("0");
    assert!(serde_json::from_value::<OverviewListResponse>(response.clone()).is_err());
    response["list"]
        .as_object_mut()
        .unwrap()
        .remove("holding_quantity");
    response["list"]["section"] = json!("predicted_stock");
    assert!(serde_json::from_value::<OverviewListResponse>(response).is_err());
}
