//! Schema13 evidence comes from the preserved pre-profile release's normal API.
//! Adversarial changes below affect in-memory copies only.
use super::{frozen13, validate_source};
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/snapshot-v2-schema13.json"
    ))
    .unwrap()
}

#[test]
fn frozen_thirteen_preserves_all_original_wire_fields() {
    let expected = fixture();
    let decoded: frozen13::Snapshot = serde_json::from_value(expected.clone()).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), expected);
}

#[test]
fn schema_thirteen_original_return_and_correction_evidence_remains_valid() {
    let value = fixture();
    let seq = value["sequence"].as_str().unwrap().parse().unwrap();
    assert_eq!(
        value["state"]["purchase_returns"].as_array().unwrap().len(),
        1
    );
    validate_source(&value.to_string(), seq, 13).unwrap();
    validate_source(&value.to_string(), seq, 14).unwrap();
}

#[test]
fn frozen_thirteen_rejects_each_future_profile_relation() {
    for field in [
        "photo_assets",
        "material_profile_versions",
        "material_profile_heads",
        "material_profile_photos",
        "material_profile_suppliers",
    ] {
        let mut value = fixture();
        value["state"][field] = json!([]);
        assert!(
            serde_json::from_value::<frozen13::Snapshot>(value).is_err(),
            "{field}"
        );
    }
}

#[test]
fn frozen_thirteen_preserves_legacy_catalog_tolerance_and_strict_return_fields() {
    let original = fixture();
    let mut value = fixture();
    value["state"]["material_versions"][0]["profile_version_id"] = json!(null);
    // The source13 catalog DTO intentionally had no deny_unknown_fields.
    // Preserve that reader behavior without adopting a future profile field.
    let decoded: frozen13::Snapshot = serde_json::from_value(value).unwrap();
    assert_eq!(serde_json::to_value(decoded).unwrap(), original);
    let mut value = fixture();
    value["state"]["purchase_returns"][0]["photo_assets"] = json!([]);
    assert!(serde_json::from_value::<frozen13::Snapshot>(value).is_err());
}

#[test]
fn schema_thirteen_source_rejects_future_snapshot_before_migration() {
    let mut value = fixture();
    let seq = value["sequence"].as_str().unwrap().parse().unwrap();
    value["schema_version"] = json!(14);
    for field in [
        "photo_assets",
        "material_profile_versions",
        "material_profile_heads",
        "material_profile_photos",
        "material_profile_suppliers",
    ] {
        value["state"][field] = json!([]);
    }
    assert!(validate_source(&value.to_string(), seq, 13).is_err());
}
