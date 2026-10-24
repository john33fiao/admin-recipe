//! The fixture is captured through the pre-return release's normal APIs.
//! Rejected examples alter only in-memory copies, never the stored evidence.
use super::{frozen12, validate_source};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/snapshot-v2-schema12.json"
    ))
    .unwrap()
}

#[test]
fn frozen_twelve_preserves_the_exact_normal_api_wire_shape() {
    let expected = fixture();
    let decoded: frozen12::Snapshot = serde_json::from_value(expected.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), expected);
}

#[test]
fn schema_twelve_normal_api_evidence_validates_after_returns_are_added() {
    let value = fixture();
    let seq = value["sequence"].as_str().unwrap().parse().unwrap();
    validate_source(&serde_json::to_string(&value).unwrap(), seq, 12).unwrap();
}

#[test]
fn frozen_twelve_rejects_future_return_business_kind_and_payload() {
    let original = fixture();
    let mut value = original.clone();
    value["state"]["business_records"][0]["kind"] = json!("purchase_return");
    assert!(serde_json::from_value::<frozen12::Snapshot>(value).is_err());
    let mut value = original;
    value["state"]["business_revisions"][0]["payload"]["kind"] = json!("purchase_return");
    assert!(serde_json::from_value::<frozen12::Snapshot>(value).is_err());
}

#[test]
fn frozen_twelve_rejects_future_return_refs_replacements_events_and_traces() {
    let original = fixture();
    let mut value = original.clone();
    value["state"]["correction_records"][0]["plan"]["target"]["kind"] = json!("purchase_return");
    assert!(serde_json::from_value::<frozen12::Snapshot>(value).is_err());
    let mut value = original.clone();
    value["state"]["correction_records"][0]["input"]["action"]["replacement"]["kind"] =
        json!("purchase_return");
    assert!(serde_json::from_value::<frozen12::Snapshot>(value).is_err());
    let mut value = original.clone();
    value["state"]["correction_event_versions"][0]["after"]["kind"] = json!("purchase_return");
    assert!(serde_json::from_value::<frozen12::Snapshot>(value).is_err());
    let mut value = original.clone();
    value["state"]["correction_price_bases"][0]["source"]["entity"] = json!("purchase_return");
    assert!(serde_json::from_value::<frozen12::Snapshot>(value).is_err());
    let mut value = original;
    let trace = value["state"]["receipt_reconciliations"][0]["effect"]["trace"]
        .as_array_mut()
        .unwrap();
    assert!(!trace.is_empty());
    trace[0]["kind"] = json!("purchase_return");
    assert!(serde_json::from_value::<frozen12::Snapshot>(value).is_err());
}

#[test]
fn frozen_twelve_rejects_future_return_relation_and_holding_source() {
    let original = fixture();
    let mut value = original.clone();
    value["state"]["purchase_returns"] = json!([]);
    assert!(serde_json::from_value::<frozen12::Snapshot>(value).is_err());
    let mut value = original;
    value["state"]["holding_versions"][0]["purchase_return_id"] = json!(null);
    assert!(serde_json::from_value::<frozen12::Snapshot>(value).is_err());
}
