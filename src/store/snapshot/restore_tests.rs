use super::{decode_restore_source, restore_decode::encode_before_restore};
use crate::{restore::RestoreSourceKind, store::Store};
use serde_json::{Value, json};

const OLD: &[(&str, &str)] = &[
    (
        "v1",
        include_str!("../../../tests/fixtures/snapshot-v1.json"),
    ),
    (
        "v3",
        include_str!("../../../tests/fixtures/snapshot-v2-schema3.json"),
    ),
    (
        "v4",
        include_str!("../../../tests/fixtures/snapshot-v2-schema4.json"),
    ),
    (
        "v5",
        include_str!("../../../tests/fixtures/snapshot-v2-schema5.json"),
    ),
    (
        "v6",
        include_str!("../../../tests/fixtures/snapshot-v2-schema6.json"),
    ),
    (
        "v7",
        include_str!("../../../tests/fixtures/snapshot-v2-schema7.json"),
    ),
    (
        "v8",
        include_str!("../../../tests/fixtures/snapshot-v2-schema8.json"),
    ),
    (
        "v9",
        include_str!("../../../tests/fixtures/snapshot-v2-schema9.json"),
    ),
    (
        "v10",
        include_str!("../../../tests/fixtures/snapshot-v2-schema10.json"),
    ),
    (
        "v11",
        include_str!("../../../tests/fixtures/snapshot-v2-schema11.json"),
    ),
    (
        "v12",
        include_str!("../../../tests/fixtures/snapshot-v2-schema12.json"),
    ),
    (
        "v13",
        include_str!("../../../tests/fixtures/snapshot-v2-schema13.json"),
    ),
    (
        "v14",
        include_str!("../../../tests/fixtures/snapshot-v2-schema14.json"),
    ),
];

fn source(index: usize) -> Value {
    serde_json::from_str(OLD[index].1).unwrap()
}

#[test]
fn every_previous_fixture_decodes_without_rewriting_identity_or_request_bytes() {
    for (name, original) in OLD {
        let decoded = decode_restore_source(original.as_bytes())
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let before: Value = serde_json::from_str(original).unwrap();
        let active = serde_json::to_value(&decoded.active).unwrap();
        assert_eq!(
            active["materials"],
            before.get("state").unwrap_or(&before)["materials"],
            "{name}"
        );
        if let Some(old) = before.get("state") {
            for field in [
                "material_versions",
                "material_heads",
                "material_prices",
                "mutation_requests",
                "creation_requests",
            ] {
                assert_eq!(active[field], old[field], "{name}: {field}");
            }
            assert_eq!(
                serde_json::to_value(&decoded.preserved.mutation_requests).unwrap(),
                old["mutation_requests"],
                "{name}: preserved request bytes"
            );
        }
        assert_eq!(decoded.header.source_kind, RestoreSourceKind::Journal);
        assert_eq!(
            decoded.header.request_id.as_deref(),
            before["request_id"].as_str()
        );
    }
}

#[test]
fn v1_technical_expansion_is_stable_and_has_no_invented_business_or_requests() {
    let first = decode_restore_source(OLD[0].1.as_bytes()).unwrap();
    let second = decode_restore_source(OLD[0].1.as_bytes()).unwrap();
    assert_eq!(
        serde_json::to_value(&first.active).unwrap(),
        serde_json::to_value(&second.active).unwrap()
    );
    assert!(
        first
            .active
            .material_versions
            .iter()
            .all(|v| v.seq.is_none() && v.source == "legacy_migration" && v.kind == "unclassified")
    );
    assert!(first.active.material_prices.is_empty());
    assert!(first.active.holding_versions.is_empty());
    assert!(first.active.business_records.is_empty());
    assert!(first.active.creation_requests.is_empty());
    assert!(first.preserved.mutation_requests.is_empty());
    assert!(first.preserved.photo_assets.is_empty());
    assert_eq!(
        first
            .active
            .materials
            .iter()
            .map(|m| m.unit.as_str())
            .collect::<Vec<_>>(),
        ["장", "g"]
    );
}

#[test]
fn restore_rejects_duplicate_identity_and_missing_heads_in_old_documents() {
    let mut duplicate = source(0);
    duplicate["materials"][1]["id"] = duplicate["materials"][0]["id"].clone();
    assert!(decode_restore_source(duplicate.to_string().as_bytes()).is_err());
    let mut headless = source(1);
    headless["state"]["material_heads"] = json!([]);
    assert!(decode_restore_source(headless.to_string().as_bytes()).is_err());
    let mut dimension = source(1);
    dimension["state"]["material_versions"][0]["dimension"] = json!("mass");
    assert!(decode_restore_source(dimension.to_string().as_bytes()).is_err());
}

#[test]
fn restore_rejects_future_unknown_and_escaped_duplicate_json_keys() {
    let mut future = source(1);
    future["schema_version"] = json!(16);
    assert!(decode_restore_source(future.to_string().as_bytes()).is_err());
    let mut unknown = source(1);
    unknown["state"]["future_business"] = json!([]);
    assert!(decode_restore_source(unknown.to_string().as_bytes()).is_err());
    let duplicate = OLD[0].1.replacen(
        "\"format_version\": 1",
        "\"format_version\": 1, \"format_\\u0076ersion\": 1",
        1,
    );
    assert!(decode_restore_source(duplicate.as_bytes()).is_err());
    let trailing = format!("{}{{}}", OLD[0].1);
    assert!(decode_restore_source(trailing.as_bytes()).is_err());
}

#[test]
fn preserved_request_json_is_strict_even_when_encoded_inside_a_string() {
    let mut invalid = source(1);
    invalid["state"]["mutation_requests"][0]["payload"] = json!("{\"id\":1,\"\\u0069d\":2}");
    assert!(decode_restore_source(invalid.to_string().as_bytes()).is_err());
}

#[test]
fn empty_before_restore_capture_round_trips_without_creating_a_journal() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open(directory.path()).unwrap();
    let raw = encode_before_restore(&store.connection, "2026-10-25T00:00:00.000Z").unwrap();
    let decoded = decode_restore_source(raw.as_bytes()).unwrap();
    assert_eq!(decoded.header.sequence, 0);
    assert_eq!(decoded.header.source_kind, RestoreSourceKind::BeforeRestore);
    assert!(decoded.header.request_id.is_none());
    assert!(decoded.active.materials.is_empty());
    let count: i64 = store
        .connection
        .query_row("SELECT COUNT(*) FROM snapshot_outbox", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
    assert_eq!(
        std::fs::read_dir(directory.path().join("snapshots"))
            .unwrap()
            .count(),
        0
    );
    let mut invalid: Value = serde_json::from_str(&raw).unwrap();
    invalid["source_kind"] = json!("journal");
    assert!(decode_restore_source(invalid.to_string().as_bytes()).is_err());
}

#[test]
fn schema_fifteen_requires_matching_preserved_request_facts() {
    let old = decode_restore_source(OLD[1].1.as_bytes()).unwrap();
    let mut value = json!({
        "format_version":2,"schema_version":15,"source_kind":"journal",
        "sequence":old.header.sequence.to_string(),"request_id":old.header.request_id,
        "recorded_at":old.header.recorded_at,"state":old.active,"preserved":old.preserved
    });
    decode_restore_source(value.to_string().as_bytes()).unwrap();
    value["preserved"]["mutation_requests"][0]["response"] = json!("{}");
    assert!(decode_restore_source(value.to_string().as_bytes()).is_err());
}

#[test]
fn schema_fifteen_does_not_drop_nested_unknown_fields_from_shared_dtos() {
    let old = decode_restore_source(OLD[1].1.as_bytes()).unwrap();
    let value = json!({
        "format_version":2,"schema_version":15,"source_kind":"journal",
        "sequence":old.header.sequence.to_string(),"request_id":old.header.request_id,
        "recorded_at":old.header.recorded_at,"state":old.active,"preserved":old.preserved
    });
    for pointer in [
        "/state/materials/0",
        "/state/material_versions/0",
        "/state/catalog_state",
        "/preserved/creation_requests/0/material",
    ] {
        let mut changed = value.clone();
        changed.pointer_mut(pointer).unwrap()["future_field"] =
            json!("must not be silently dropped");
        assert!(
            decode_restore_source(changed.to_string().as_bytes()).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn old_restore_documents_do_not_silently_discard_nested_unknown_fields() {
    for (name, original) in OLD {
        let path = if *name == "v1" {
            "/materials/0"
        } else {
            "/state/materials/0"
        };
        for unknown in [Value::Null, json!("unsupported")] {
            let mut value: Value = serde_json::from_str(original).unwrap();
            value.pointer_mut(path).unwrap()["future_field"] = unknown;
            assert!(
                decode_restore_source(value.to_string().as_bytes()).is_err(),
                "{name}"
            );
        }
    }
}

#[test]
fn schema_fifteen_journal_header_must_identify_its_actual_latest_request() {
    let old = decode_restore_source(OLD[1].1.as_bytes()).unwrap();
    let mut value = json!({
        "format_version":2,"schema_version":15,"source_kind":"journal",
        "sequence":old.header.sequence.to_string(),"request_id":old.header.request_id,
        "recorded_at":old.header.recorded_at,"state":old.active,"preserved":old.preserved
    });
    decode_restore_source(value.to_string().as_bytes()).unwrap();
    value["request_id"] = json!("00000000-0000-4000-8000-000000000999");
    assert!(decode_restore_source(value.to_string().as_bytes()).is_err());
}

#[test]
fn imported_invalid_data_and_internal_journal_corruption_keep_distinct_error_kinds() {
    let old = decode_restore_source(OLD[1].1.as_bytes()).unwrap();
    let sequence = old.header.sequence;
    let mut value = json!({
        "format_version":2,"schema_version":15,"source_kind":"journal",
        "sequence":sequence.to_string(),"request_id":old.header.request_id,
        "recorded_at":old.header.recorded_at,"state":old.active,"preserved":old.preserved
    });
    value["preserved"]["mutation_requests"][0]["response"] = json!("{}");
    let raw = value.to_string();
    assert!(matches!(
        decode_restore_source(raw.as_bytes()),
        Err(crate::store::StoreError::Validation(_))
    ));
    assert!(matches!(
        super::validate_source(&raw, sequence, 15),
        Err(crate::store::StoreError::Unavailable(_))
    ));
}
