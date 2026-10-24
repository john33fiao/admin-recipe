//! Corrupt in-memory copies of snapshots produced by normal Store commands.
//! These tests never manufacture successful evidence or patch a live database.
use super::validate_source;
use crate::store::Store;
use rusqlite::Connection;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use tempfile::TempDir;
use uuid::Uuid;

fn decode<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).unwrap()
}
fn encode(value: impl Serialize) -> Value {
    serde_json::to_value(value).unwrap()
}
fn meta(store: &Store) -> Value {
    let state = store.catalog_state().unwrap();
    json!({"request_id":Uuid::new_v4().to_string(),"epoch":state.epoch,
        "expected_revision":state.revision})
}
fn confirm(store: &mut Store, payload: Value, target: Option<&str>) -> Value {
    let request = json!({"meta":meta(store),"payload":payload,
        "corrects_confirmation_id":target});
    let draft = encode(store.business_create(decode(request)).unwrap());
    let id = draft["result"]["item"]["id"].as_str().unwrap();
    let request = json!({"meta":meta(store),
        "expected_head_id":draft["result"]["head"]["id"]});
    encode(store.business_confirm(id, decode(request)).unwrap())
}
fn fixture(twice: bool) -> Value {
    let root = TempDir::new().unwrap();
    let mut store = Store::open(root.path()).unwrap();
    let mut request = meta(&store);
    request["name"] = json!("가상 정정 검증 품목");
    request["unit"] = json!("g");
    request["kind"] = json!("raw");
    request["description"] = json!("");
    let material = store.catalog_create(decode(request)).unwrap().value;
    let mut receipt = json!({"source":{"kind":"direct","material_id":material.id,
        "material_version_id":material.version_id},"quantity":"10","unit":"g",
        "amount":"100","receipt_date":"2020-01-01","note":"가상 입고",
        "stock_effect":{"kind":"unreflected"}});
    let original = confirm(
        &mut store,
        json!({"kind":"receipt","version":1,"data":receipt}),
        None,
    );
    let target = original["result"]["confirmation"]["id"].as_str().unwrap();
    let original_id = original["result"]["confirmation"]["result"]["receipt_id"]
        .as_str()
        .unwrap();
    receipt["quantity"] = json!("8");
    receipt["amount"] = json!("80");
    confirm(
        &mut store,
        json!({"kind":"correction","version":1,"data":{
        "expected_target_version_id":original_id,"reason":"가상 수량 정정",
        "action":{"kind":"replace","replacement":{"kind":"receipt","data":receipt}},
        "observation_reviews":[],"linked_receipt_changes":[]}}),
        Some(target),
    );
    let db = Connection::open(root.path().join("app.sqlite3")).unwrap();
    if twice {
        let current: String = db
            .query_row(
                "SELECT current_version_id FROM correction_event_heads WHERE original_id=?1",
                [original_id],
                |r| r.get(0),
            )
            .unwrap();
        receipt["amount"] = json!("88");
        confirm(
            &mut store,
            json!({"kind":"correction","version":1,"data":{
            "expected_target_version_id":current,"reason":"가상 금액 정정",
            "action":{"kind":"replace","replacement":{"kind":"receipt","data":receipt}},
            "observation_reviews":[],"linked_receipt_changes":[]}}),
            Some(target),
        );
    }
    let source: String = db
        .query_row(
            "SELECT payload FROM snapshot_outbox ORDER BY sequence DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let value: Value = serde_json::from_str(&source).unwrap();
    accepts(&value).unwrap();
    value
}
fn accepts(value: &Value) -> Result<(), crate::store::StoreError> {
    validate_source(
        &value.to_string(),
        value["sequence"].as_str().unwrap().parse().unwrap(),
        crate::store::SCHEMA_VERSION,
    )
}
fn rejects(original: &Value, label: &str, change: impl FnOnce(&mut Value)) {
    let mut corrupted = original.clone();
    change(&mut corrupted);
    assert!(
        accepts(&corrupted).is_err(),
        "accepted damaged evidence: {label}"
    );
}

fn replace_reference(value: &mut Value, old: &Value, replacement: &Value) {
    match value {
        Value::String(_) if value == old => *value = replacement.clone(),
        Value::Array(rows) => rows
            .iter_mut()
            .for_each(|v| replace_reference(v, old, replacement)),
        Value::Object(rows) => rows
            .values_mut()
            .for_each(|v| replace_reference(v, old, replacement)),
        _ => {}
    }
}

#[test]
fn corrections_reject_rewritten_input_plan_and_event_identity() {
    let original = fixture(false);
    rejects(
        &original,
        "new event version reuses original identity",
        |v| {
            let old = v["state"]["correction_event_versions"][0]["id"].clone();
            let replacement =
                v["state"]["correction_event_versions"][0]["reference"]["original_id"].clone();
            replace_reference(v, &old, &replacement);
        },
    );
    rejects(&original, "reason detached from input", |v| {
        v["state"]["correction_records"][0]["reason"] = json!("덮어쓴 이유");
    });
    rejects(&original, "plan changed after confirmation", |v| {
        v["state"]["correction_records"][0]["plan"]["materials"][0]["after"]["amounts"]["value"] =
            json!("81");
    });
    rejects(&original, "event moved to correction time", |v| {
        let current = v["sequence"].clone();
        v["state"]["correction_event_versions"][0]["reference"]["logical_seq"] = current;
    });
    rejects(
        &original,
        "revision metadata detached from confirmation",
        |v| {
            let confirmation_id = v["state"]["correction_records"][0]["confirmation_id"].clone();
            let revision_id = v["state"]["business_confirmations"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["id"] == confirmation_id)
                .unwrap()["revision_id"]
                .clone();
            let revision = v["state"]["business_revisions"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| r["id"] == revision_id)
                .unwrap();
            revision["recorded_at"] = json!("2020-01-01T00:00:00Z");
        },
    );
}

#[test]
fn corrections_reject_stale_event_and_projection_heads() {
    let original = fixture(true);
    rejects(&original, "stale event head", |v| {
        let old = v["state"]["correction_event_versions"][0]["id"].clone();
        v["state"]["correction_event_heads"][0]["current_version_id"] = old;
    });
    rejects(&original, "stale material projection head", |v| {
        let old = v["state"]["correction_projection_versions"][0]["id"].clone();
        v["state"]["correction_projection_heads"][0]["current_version_id"] = old;
    });
    rejects(&original, "lost previous event", |v| {
        v["state"]["correction_event_versions"][1]["previous_version_id"] = Value::Null;
    });
}

#[test]
fn corrections_reject_orphaned_projection_and_misdirected_actual_reference() {
    let original = fixture(false);
    rejects(
        &original,
        "actual holding reuses original receipt identity",
        |v| {
            let old =
                v["state"]["correction_projection_versions"][0]["projection"]["holding_version_id"]
                    .clone();
            let replacement = v["state"]["correction_records"][0]["original_id"].clone();
            replace_reference(v, &old, &replacement);
        },
    );
    rejects(&original, "projection owner missing", |v| {
        v["state"]["correction_projection_versions"][0]["correction_id"] =
            json!(Uuid::new_v4().to_string());
    });
    rejects(&original, "resolved reference points to original", |v| {
        let old = v["state"]["correction_records"][0]["original_id"].clone();
        v["state"]["correction_records"][0]["resolved_refs"][0]["version_id"] = old;
    });
    rejects(&original, "derived holding missing", |v| {
        v["state"]["correction_projection_versions"][0]["projection"]["holding_version_id"] =
            json!(Uuid::new_v4().to_string());
    });
}

#[test]
fn corrections_reject_lost_record_and_extra_price_evidence() {
    let original = fixture(false);
    rejects(&original, "record missing", |v| {
        v["state"]["correction_records"] = json!([]);
    });
    rejects(&original, "unowned duplicated price", |v| {
        let mut row = v["state"]["correction_price_bases"][0].clone();
        row["id"] = json!(Uuid::new_v4().to_string());
        v["state"]["correction_price_bases"]
            .as_array_mut()
            .unwrap()
            .push(row);
    });
}
