use serde_json::{Value, json};
use sunchang_hansang::business::{CreateBusinessDraft, normalize_business_payload};

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
    assert_eq!(draft.data.quantity, "1.");
    assert_eq!(draft.data.amount, "0");
    assert_eq!(draft.data.note, "가상 입력\n다음 줄");
    draft.data.quantity.clear();
    draft.data.amount = "입력 중".into();
    assert!(normalize_business_payload(draft.clone()).is_ok());
    draft.data.quantity = "1".repeat(81);
    assert!(normalize_business_payload(draft.clone()).is_err());
    draft.data.quantity.clear();
    draft.data.note = "숨김\t문자".into();
    assert!(normalize_business_payload(draft.clone()).is_err());
    draft.data.note.clear();
    draft.data.material_id = Some("not-an-id".into());
    assert!(normalize_business_payload(draft).is_err());
}

#[test]
fn draft_reference_ids_are_canonical_without_inventing_missing_values() {
    let wire: CreateBusinessDraft = serde_json::from_value(input()).unwrap();
    let mut draft = wire.payload;
    draft.data.material_id = Some("AABBCCDD-0000-4000-8000-000000000001".into());
    let normalized = normalize_business_payload(draft).unwrap();
    assert_eq!(
        normalized.data.material_id.as_deref(),
        Some("aabbccdd-0000-4000-8000-000000000001")
    );
    assert!(normalized.data.material_version_id.is_none());
    let meta: sunchang_hansang::catalog::MutationMeta = wire.meta.into();
    assert_eq!(meta.expected_revision, "0");
}
