use serde_json::{Value, json};
use sunchang_hansang::business::{
    BusinessEffect, BusinessPayload, BusinessResult, CreateBusinessDraft, DraftPriceV1,
    normalize_business_payload,
};

fn price(payload: &BusinessPayload) -> &DraftPriceV1 {
    let BusinessPayload::CatalogPrice { data, .. } = payload else {
        panic!("expected price draft");
    };
    data
}

fn price_mut(payload: &mut BusinessPayload) -> &mut DraftPriceV1 {
    let BusinessPayload::CatalogPrice { data, .. } = payload else {
        panic!("expected price draft");
    };
    data
}

fn input() -> Value {
    json!({
        "meta": {"request_id":"00000000-0000-4000-8000-000000000001",
            "epoch":"00000000-0000-4000-8000-000000000002","expected_revision":"0"},
        "payload": {"kind":"catalog_price","version":1,"data": {
            "material_id":null,"material_version_id":null,"quantity":"1.",
            "unit":"kg","amount":"0","note":"가상 입력\r\n다음 줄"}},
        "corrects_confirmation_id":null
    })
}

#[test]
fn business_wire_rejects_unknown_fields_versions_and_numeric_input() {
    let valid = input();
    assert!(serde_json::from_value::<CreateBusinessDraft>(valid.clone()).is_ok());
    for pointer in ["", "/meta", "/payload", "/payload/data"] {
        let mut bad = valid.clone();
        bad.pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("inventory_delta".into(), json!(100));
        assert!(
            serde_json::from_value::<CreateBusinessDraft>(bad).is_err(),
            "{pointer}"
        );
    }
    for (pointer, value) in [
        ("/payload/kind", json!("sale")),
        ("/payload/version", json!(2)),
        ("/payload/version", json!("1")),
        ("/payload/data/amount", json!(0)),
    ] {
        let mut bad = valid.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(serde_json::from_value::<CreateBusinessDraft>(bad).is_err());
    }
}

#[test]
fn incomplete_draft_keeps_raw_numbers_but_checks_structure() {
    let wire: CreateBusinessDraft = serde_json::from_value(input()).unwrap();
    let mut draft = normalize_business_payload(wire.payload).unwrap();
    assert_eq!(price(&draft).quantity, "1.");
    assert_eq!(price(&draft).amount, "0");
    assert_eq!(price(&draft).note, "가상 입력\n다음 줄");
    price_mut(&mut draft).quantity.clear();
    price_mut(&mut draft).amount = "입력 중".into();
    assert!(normalize_business_payload(draft.clone()).is_ok());
    price_mut(&mut draft).quantity = "1".repeat(81);
    assert!(normalize_business_payload(draft.clone()).is_err());
    price_mut(&mut draft).quantity.clear();
    price_mut(&mut draft).note = "숨김\t문자".into();
    assert!(normalize_business_payload(draft.clone()).is_err());
    price_mut(&mut draft).note.clear();
    price_mut(&mut draft).material_id = Some("not-an-id".into());
    assert!(normalize_business_payload(draft).is_err());
}

#[test]
fn draft_reference_ids_are_canonical_without_inventing_missing_values() {
    let wire: CreateBusinessDraft = serde_json::from_value(input()).unwrap();
    let mut draft = wire.payload;
    price_mut(&mut draft).material_id = Some("AABBCCDD-0000-4000-8000-000000000001".into());
    let normalized = normalize_business_payload(draft).unwrap();
    assert_eq!(
        price(&normalized).material_id.as_deref(),
        Some("aabbccdd-0000-4000-8000-000000000001")
    );
    assert!(price(&normalized).material_version_id.is_none());
    let meta: sunchang_hansang::catalog::MutationMeta = wire.meta.into();
    assert_eq!(meta.expected_revision, "0");
}

#[test]
fn order_actions_keep_incomplete_text_and_reject_mixed_or_unknown_fields() {
    for data in [
        json!({"action":"place","material_id":null,"material_version_id":null,
            "quantity":"1.","unit":"kg","supplier":"","expected_date":"2026-",
            "estimated_amount":"","note":"가상 메모\r\n다음 줄"}),
        json!({"action":"revise","order_id":"00000000-0000-4000-8000-000000000003",
            "expected_order_version_id":"00000000-0000-4000-8000-000000000004",
            "material_version_id":null,"quantity":"","unit":"g","supplier":"",
            "expected_date":"","estimated_amount":"0","note":""}),
        json!({"action":"cancel","order_id":"00000000-0000-4000-8000-000000000003",
            "expected_order_version_id":"00000000-0000-4000-8000-000000000004","note":""}),
    ] {
        let mut value = input();
        value["payload"] = json!({"kind":"purchase_order","version":1,"data":data});
        let decoded: CreateBusinessDraft = serde_json::from_value(value.clone()).unwrap();
        assert!(normalize_business_payload(decoded.payload).is_ok());
        for (field, extra) in [("inventory_delta", json!(10)), ("price_id", json!("x"))] {
            let mut bad = value.clone();
            bad["payload"]["data"][field] = extra;
            assert!(serde_json::from_value::<CreateBusinessDraft>(bad).is_err());
        }
        let mut bad = value.clone();
        bad["payload"]["data"]["quantity"] = json!(1);
        assert!(serde_json::from_value::<CreateBusinessDraft>(bad).is_err());
        let mut bad = value.clone();
        bad["payload"]["data"]["action"] = json!("receive");
        assert!(serde_json::from_value::<CreateBusinessDraft>(bad).is_err());
        value["payload"]["version"] = json!(2);
        assert!(serde_json::from_value::<CreateBusinessDraft>(value).is_err());
    }
}

#[test]
fn price_command_canonical_bytes_stay_compatible_with_original_requests() {
    let mut wire: CreateBusinessDraft = serde_json::from_value(input()).unwrap();
    wire.payload = normalize_business_payload(wire.payload).unwrap();
    assert_eq!(
        serde_json::to_string(&wire).unwrap(),
        r#"{"meta":{"request_id":"00000000-0000-4000-8000-000000000001","epoch":"00000000-0000-4000-8000-000000000002","expected_revision":"0"},"payload":{"kind":"catalog_price","version":1,"data":{"material_id":null,"material_version_id":null,"quantity":"1.","unit":"kg","amount":"0","note":"가상 입력\n다음 줄"}},"corrects_confirmation_id":null}"#
    );
}

#[test]
fn original_price_result_and_preview_wire_bytes_survive_the_kind_extension() {
    let result = r#"{"kind":"catalog_price","price_id":"00000000-0000-4000-8000-000000000001","material_id":"00000000-0000-4000-8000-000000000002","material_version_id":"00000000-0000-4000-8000-000000000003"}"#;
    let parsed: BusinessResult = serde_json::from_str(result).unwrap();
    assert_eq!(serde_json::to_string(&parsed).unwrap(), result);
    let effect = r#"{"kind":"catalog_price","material_id":"00000000-0000-4000-8000-000000000002","material_version_id":"00000000-0000-4000-8000-000000000003","quantity":"1000","amount":"0","unit":"g","corrects_confirmation_id":null,"previous_price":null,"corrected_price":null}"#;
    let parsed: BusinessEffect = serde_json::from_str(effect).unwrap();
    assert_eq!(serde_json::to_string(&parsed).unwrap(), effect);
}

#[test]
fn receipt_nested_sources_and_stock_effects_preserve_unfinished_inputs_and_reject_mixed_fields() {
    for source in [
        json!({"kind":"direct","material_id":null,"material_version_id":null}),
        json!({"kind":"order","order_id":null,"expected_order_version_id":null,"material_version_id":null}),
    ] {
        for stock_effect in [
            json!({"kind":"unreflected"}),
            json!({"kind":"needs_reconciliation","basis_id":null,"included_quantity":"1."}),
        ] {
            let mut value = input();
            value["payload"] = json!({"kind":"receipt","version":1,"data":{
                "source":source,"stock_effect":stock_effect,"quantity":"1.","unit":"kg",
                "amount":"","receipt_date":"2026-","note":"가상 입고\r\n확인 필요"
            }});
            let decoded: CreateBusinessDraft = serde_json::from_value(value.clone()).unwrap();
            let normalized = normalize_business_payload(decoded.payload).unwrap();
            let output = serde_json::to_value(normalized).unwrap();
            assert_eq!(output["data"]["quantity"], "1.");
            assert_eq!(output["data"]["amount"], "");
            assert_eq!(output["data"]["note"], "가상 입고\n확인 필요");
            for key in ["source", "stock_effect"] {
                let mut bad = value.clone();
                bad["payload"]["data"][key]["extra"] = json!(true);
                assert!(serde_json::from_value::<CreateBusinessDraft>(bad).is_err());
            }
            let mut bad = value.clone();
            bad["payload"]["data"]["amount"] = json!(0);
            assert!(serde_json::from_value::<CreateBusinessDraft>(bad).is_err());
            let mut bad = value.clone();
            bad["payload"]["version"] = json!(2);
            assert!(serde_json::from_value::<CreateBusinessDraft>(bad).is_err());
            let mut bad = value.clone();
            bad["payload"]["data"]["source"]["kind"] = json!("automatic");
            assert!(serde_json::from_value::<CreateBusinessDraft>(bad).is_err());
        }
    }
}

#[test]
fn production_drafts_preserve_incomplete_rows_and_close_every_nested_wire_boundary() {
    let mut value = input();
    value["payload"] = json!({"kind":"production","version":1,"data":{
        "inputs":[{"material_id":null,"material_version_id":null,"quantity":"1.","unit":"kg"}],
        "outputs":[{"material_id":null,"material_version_id":null,"quantity":"","unit":"",
            "allocation_percent":"100."}],
        "note":"가상 생산\r\n확인 필요"
    }});
    let decoded: CreateBusinessDraft = serde_json::from_value(value.clone()).unwrap();
    let normalized = normalize_business_payload(decoded.payload).unwrap();
    assert_eq!(
        normalized.kind(),
        sunchang_hansang::business::BusinessKind::Production
    );
    let normalized = serde_json::to_value(normalized).unwrap();
    assert_eq!(normalized["data"]["inputs"][0]["quantity"], "1.");
    assert_eq!(normalized["data"]["outputs"][0]["quantity"], "");
    assert_eq!(
        normalized["data"]["outputs"][0]["allocation_percent"],
        "100."
    );
    assert_eq!(normalized["data"]["note"], "가상 생산\n확인 필요");
    for path in [
        "/payload",
        "/payload/data",
        "/payload/data/inputs/0",
        "/payload/data/outputs/0",
    ] {
        let mut wrong = value.clone();
        wrong
            .pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("extra".into(), json!(true));
        assert!(serde_json::from_value::<CreateBusinessDraft>(wrong).is_err());
    }
    for (path, replacement) in [
        ("/payload/version", json!(2)),
        ("/payload/data/inputs/0/quantity", json!(1)),
        ("/payload/data/outputs/0/allocation_percent", json!(100)),
    ] {
        let mut wrong = value.clone();
        *wrong.pointer_mut(path).unwrap() = replacement;
        assert!(serde_json::from_value::<CreateBusinessDraft>(wrong).is_err());
    }
}
