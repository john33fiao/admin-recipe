use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sunchang_hansang::{
    lifecycle::Policy,
    server::{AppState, router},
    store::Store,
};
use tower::ServiceExt;

const HOST: &str = "127.0.0.1:4321";
const ORIGIN: &str = "http://127.0.0.1:4321";
const COOKIE: &str = "hansang_4321=reconciliation-test-session";
fn app() -> (tempfile::TempDir, Router) {
    let directory = tempfile::tempdir().unwrap();
    let app = router(AppState::new(
        Store::open(directory.path()).unwrap(),
        HOST.into(),
        "reconciliation-test-session".into(),
        Policy::default(),
    ));
    (directory, app)
}
async fn send(app: &Router, method: &str, path: &str, input: Option<&Value>) -> Response {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("host", HOST)
        .header("cookie", COOKIE);
    let body = if let Some(input) = input {
        request = request
            .header("origin", ORIGIN)
            .header("content-type", "application/json");
        Body::from(input.to_string())
    } else {
        Body::empty()
    };
    app.clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap()
}
async fn value(response: Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| panic!("expected JSON: {}", String::from_utf8_lossy(&bytes)))
}
async fn get(app: &Router, path: &str) -> Value {
    let response = send(app, "GET", path, None).await;
    assert_eq!(response.status(), StatusCode::OK, "{path}");
    value(response).await
}
fn meta(state: &Value) -> Value {
    json!({"request_id":uuid::Uuid::new_v4().to_string(),"epoch":state["epoch"],"expected_revision":state["revision"]})
}
async fn created(app: &Router, path: &str, input: &Value) -> Value {
    let response = send(app, "POST", path, Some(input)).await;
    let status = response.status();
    let body = value(response).await;
    assert_eq!(status, StatusCode::CREATED, "{path}: {body}");
    body
}
async fn material(app: &Router) -> Value {
    let mut input = meta(&get(app, "/api/state").await);
    input["name"] = json!("가상 포함 입고 HTTP 재료");
    input["unit"] = json!("g");
    input["kind"] = json!("raw");
    input["description"] = json!("");
    created(app, "/api/materials", &input).await
}
async fn draft(app: &Router, kind: &str, data: Value) -> Value {
    created(app,"/api/business/drafts",&json!({"meta":meta(&get(app,"/api/state").await),"corrects_confirmation_id":null,"payload":{"kind":kind,"version":1,"data":data}})).await
}
async fn preview(app: &Router, saved: &Value) -> Value {
    let response = send(
        app,
        "POST",
        &format!(
            "/api/business/{}/preview",
            saved["result"]["item"]["id"].as_str().unwrap()
        ),
        Some(&json!({"expected_head_id":saved["result"]["head"]["id"]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    value(response).await
}
fn confirm_path(saved: &Value) -> String {
    format!(
        "/api/business/{}/confirm",
        saved["result"]["item"]["id"].as_str().unwrap()
    )
}
async fn confirm(app: &Router, saved: &Value) -> (Value, Value) {
    let check = preview(app, saved).await;
    assert_eq!(check["can_confirm"], true, "{check}");
    let input = json!({"meta":meta(&check["state"]),"expected_head_id":check["head_id"]});
    (created(app, &confirm_path(saved), &input).await, input)
}
fn receipt_data(item: &Value, quantity: &str, amount: &str, selections: Value) -> Value {
    json!({"source":{"kind":"direct","material_id":item["id"],"material_version_id":item["version_id"]},"quantity":quantity,"unit":"g","amount":amount,"receipt_date":"2020-02-29","note":"가상 지연 입고","stock_effect":{"kind":"reconciled","selections":selections}})
}
fn selection(observation: &Value, quantity: &str) -> Value {
    json!({"observation_version_id":observation["version_id"],"source":{"kind":"all_available"},"included_quantity":quantity})
}
async fn observe(
    app: &Router,
    item: &Value,
    quantity: &str,
    initial: Option<&str>,
) -> (Value, Value) {
    let saved=draft(app,"stocktake",json!({"rows":[{"material_id":item["id"],"material_version_id":item["version_id"],"quantity":quantity,"unit":"g","initial_value":initial.map(|amount|json!({"kind":"manual_amount","amount":amount})),"excess":null}],"note":"가상 기준"})).await;
    let (confirmed, _) = confirm(app, &saved).await;
    let id = confirmed["result"]["confirmation"]["result"]["stocktake_id"]
        .as_str()
        .unwrap();
    let detail = get(app, &format!("/api/stocktakes/{id}")).await;
    (detail["observations"][0].clone(), detail)
}
async fn holding(app: &Router, item: &Value) -> Value {
    get(
        app,
        &format!("/api/holdings/{}", item["id"].as_str().unwrap()),
    )
    .await
}
async fn candidates(app: &Router, item: &Value) -> Value {
    get(
        app,
        &format!(
            "/api/materials/{}/receipt-inclusion-bases",
            item["id"].as_str().unwrap()
        ),
    )
    .await
}
async fn receipt_detail(app: &Router, confirmed: &Value) -> Value {
    let receipt = confirmed["result"]["confirmation"]["result"]["receipt_id"]
        .as_str()
        .unwrap();
    get(app, &format!("/api/receipts/{receipt}")).await
}

#[tokio::test]
async fn candidates_read_without_mutation_and_reject_invalid_material_ids() {
    let (_dir, app) = app();
    let item = material(&app).await;
    let before = get(&app, "/api/state").await;
    assert_eq!(candidates(&app, &item).await["items"], json!([]));
    assert_eq!(get(&app, "/api/state").await, before);
    let response = send(
        &app,
        "GET",
        "/api/materials/bad/receipt-inclusion-bases",
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(value(response).await["code"], "validation");
}

#[tokio::test]
async fn full_inclusion_updates_value_once_and_preserves_original_observation() {
    let (_dir, app) = app();
    let item = material(&app).await;
    let (observation, original) = observe(&app, &item, "10", Some("10")).await;
    let offered = candidates(&app, &item).await;
    assert_eq!(
        offered["items"][0]["observation_version_id"],
        observation["version_id"]
    );
    assert_eq!(offered["items"][0]["available_quantity"], "10");
    let saved = draft(
        &app,
        "receipt",
        receipt_data(&item, "5", "10", json!([selection(&observation, "5")])),
    )
    .await;
    let check = preview(&app, &saved).await;
    assert_eq!(check["can_confirm"], true, "{check}");
    assert_eq!(check["effect"]["kind"], "reconciled_receipt");
    assert_eq!(check["effect"]["receipt"]["unreflected_quantity"], "0");
    let (confirmed, input) = confirm(&app, &saved).await;
    assert_eq!(
        confirmed["result"]["confirmation"]["result"]["kind"],
        "reconciled_receipt"
    );
    let detail = receipt_detail(&app, &confirmed).await;
    assert_eq!(detail["item"]["quantity"], "5");
    assert_eq!(detail["item"]["amount"], "10");
    assert!(detail["holding_version_id"].is_string());
    assert_eq!(detail["reconciliation"]["effect"]["value_difference"], "5");
    assert_eq!(
        holding(&app, &item).await["settlement_basis"]["quantity"],
        "10"
    );
    assert_eq!(
        holding(&app, &item).await["settlement_basis"]["value"],
        "15"
    );
    let state = get(&app, "/api/state").await;
    assert_eq!(
        created(&app, &confirm_path(&saved), &input).await,
        confirmed
    );
    assert_eq!(get(&app, "/api/state").await, state);
    let mut conflict = input.clone();
    conflict["expected_head_id"] = json!(uuid::Uuid::new_v4().to_string());
    let response = send(&app, "POST", &confirm_path(&saved), Some(&conflict)).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(value(response).await["code"], "request_conflict");
    let stocktake = observation["stocktake_id"].as_str().unwrap();
    assert_eq!(
        get(&app, &format!("/api/stocktakes/{stocktake}")).await["observations"],
        original["observations"]
    );
    assert_eq!(
        candidates(&app, &item).await["items"][0]["available_quantity"],
        "5"
    );
}

#[tokio::test]
async fn prior_anchor_correction_and_later_anchor_correction_keep_frozen_bv() {
    let (_dir, app) = app();
    let item = material(&app).await;
    let (first, _) = observe(&app, &item, "10", Some("10")).await;
    let (second, original) = observe(&app, &item, "5", None).await;
    let one = draft(
        &app,
        "receipt",
        receipt_data(&item, "5", "10", json!([selection(&first, "5")])),
    )
    .await;
    let (one, _) = confirm(&app, &one).await;
    assert_eq!(
        receipt_detail(&app, &one).await["reconciliation"]["effect"]["separate_adjustment"],
        "2.5"
    );
    assert_eq!(
        holding(&app, &item).await["settlement_basis"]["value"],
        "7.5"
    );
    let offered = candidates(&app, &item).await;
    let later = offered["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["observation_version_id"] == second["version_id"])
        .unwrap();
    assert_eq!(later["available_quantity"], "2.5");
    let two = draft(
        &app,
        "receipt",
        receipt_data(&item, "2.5", "5", json!([selection(&second, "2.5")])),
    )
    .await;
    let (two, _) = confirm(&app, &two).await;
    let detail = receipt_detail(&app, &two).await;
    assert_eq!(detail["reconciliation"]["effect"]["booked_value"], "2.5");
    assert_eq!(
        detail["reconciliation"]["effect"]["separate_adjustment"],
        "0"
    );
    assert_eq!(
        holding(&app, &item).await["settlement_basis"]["value"],
        "10"
    );
    let id = second["stocktake_id"].as_str().unwrap();
    assert_eq!(
        get(&app, &format!("/api/stocktakes/{id}")).await["observations"],
        original["observations"]
    );
}

#[tokio::test]
async fn partial_inclusion_and_explicit_zero_inclusion_preserve_actual_receipt_amounts() {
    let (_dir, app) = app();
    let item = material(&app).await;
    let (basis, _) = observe(&app, &item, "10", Some("10")).await;
    let saved = draft(
        &app,
        "receipt",
        receipt_data(&item, "6", "9", json!([selection(&basis, "3")])),
    )
    .await;
    let (confirmed, _) = confirm(&app, &saved).await;
    let effect = receipt_detail(&app, &confirmed).await["reconciliation"]["effect"].clone();
    assert_eq!(effect["unreflected_quantity"], "3");
    assert_eq!(effect["included_amount"], "4.5");
    assert_eq!(effect["unreflected_amount"], "4.5");
    let state = holding(&app, &item).await;
    assert_eq!(state["settlement_basis"]["quantity"], "13");
    assert_eq!(state["settlement_basis"]["value"], "16");
    let saved = draft(&app, "receipt", receipt_data(&item, "1", "0", json!([]))).await;
    let (confirmed, _) = confirm(&app, &saved).await;
    assert_eq!(
        receipt_detail(&app, &confirmed).await["reconciliation"]["effect"]["included_quantity"],
        "0"
    );
    let state = holding(&app, &item).await;
    assert_eq!(state["settlement_basis"]["quantity"], "14");
    assert_eq!(state["settlement_basis"]["value"], "16");
}

#[tokio::test]
async fn fully_consumed_inclusion_has_separate_adjustment_without_fake_holding() {
    let (_dir, app) = app();
    let item = material(&app).await;
    let (basis, _) = observe(&app, &item, "10", Some("10")).await;
    let waste=draft(&app,"waste",json!({"material_id":item["id"],"material_version_id":item["version_id"],"quantity":"10","unit":"g","reason":"discard","note":"가상 전량 폐기"})).await;
    confirm(&app, &waste).await;
    let before = holding(&app, &item).await;
    let saved = draft(
        &app,
        "receipt",
        receipt_data(&item, "5", "10", json!([selection(&basis, "5")])),
    )
    .await;
    let (confirmed, _) = confirm(&app, &saved).await;
    let detail = receipt_detail(&app, &confirmed).await;
    assert!(detail["holding_version_id"].is_null());
    assert_eq!(detail["reconciliation"]["material_results"], json!([]));
    assert_eq!(
        detail["reconciliation"]["effect"]["separate_adjustment"],
        "5"
    );
    let after = holding(&app, &item).await;
    assert_eq!(after["settlement_basis"], before["settlement_basis"]);
    assert_eq!(after["confirmed_changes"], before["confirmed_changes"]);
}

#[tokio::test]
async fn new_mode_preserves_incomplete_rows_strict_nested_values_and_security_boundaries() {
    let (_dir, app) = app();
    let item = material(&app).await;
    let raw = receipt_data(
        &item,
        "1",
        "0",
        json!([{"observation_version_id":null,"source":null,"included_quantity":""}]),
    );
    let saved = draft(&app, "receipt", raw.clone()).await;
    assert_eq!(saved["result"]["head"]["payload"]["data"], raw);
    assert_eq!(preview(&app, &saved).await["can_confirm"], false);
    let before = get(&app, "/api/state").await;
    for bad in [
        json!({"kind":"all_available","extra":true}),
        json!({"kind":"source","source_key":{"kind":"receipt_inflow","id":uuid::Uuid::new_v4().to_string(),"position":0,"extra":true}}),
    ] {
        let mut raw = raw.clone();
        raw["stock_effect"]["selections"][0]["source"] = bad;
        let response=send(&app,"POST","/api/business/drafts",Some(&json!({"meta":meta(&before),"corrects_confirmation_id":null,"payload":{"kind":"receipt","version":1,"data":raw}}))).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    let oversized = json!({"extra":"x".repeat(17*1024)});
    assert_eq!(
        send(&app, "POST", "/api/business/drafts", Some(&oversized))
            .await
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let payload = json!({"meta":meta(&before),"corrects_confirmation_id":null,"payload":{"kind":"receipt","version":1,"data":raw}});
    for (host, origin, cookie, expected) in [
        (
            "evil.example",
            Some(ORIGIN),
            Some(COOKIE),
            StatusCode::FORBIDDEN,
        ),
        (
            HOST,
            Some("https://evil.example"),
            Some(COOKIE),
            StatusCode::FORBIDDEN,
        ),
        (HOST, Some(ORIGIN), None, StatusCode::UNAUTHORIZED),
        (HOST, None, Some(COOKIE), StatusCode::FORBIDDEN),
    ] {
        let mut request = Request::builder()
            .method("POST")
            .uri("/api/business/drafts")
            .header("host", host)
            .header("content-type", "application/json");
        if let Some(origin) = origin {
            request = request.header("origin", origin);
        }
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        assert_eq!(
            app.clone()
                .oneshot(request.body(Body::from(payload.to_string())).unwrap())
                .await
                .unwrap()
                .status(),
            expected
        );
    }
    assert_eq!(get(&app, "/api/state").await, before);
}

#[tokio::test]
async fn stale_preview_is_unprocessed_and_rechecks_reservations_after_another_confirmation() {
    let (_dir, app) = app();
    let item = material(&app).await;
    let (basis, _) = observe(&app, &item, "10", Some("10")).await;
    let first = draft(
        &app,
        "receipt",
        receipt_data(&item, "6", "12", json!([selection(&basis, "6")])),
    )
    .await;
    let second = draft(
        &app,
        "receipt",
        receipt_data(&item, "6", "12", json!([selection(&basis, "6")])),
    )
    .await;
    let old = preview(&app, &first).await;
    assert_eq!(old["can_confirm"], true);
    confirm(&app, &second).await;
    let input = json!({"meta":meta(&old["state"]),"expected_head_id":old["head_id"]});
    let response = send(&app, "POST", &confirm_path(&first), Some(&input)).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(value(response).await["code"], "stale_state");
    assert_eq!(preview(&app, &first).await["can_confirm"], false);
    assert_eq!(
        get(&app, "/api/receipts").await["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        holding(&app, &item).await["settlement_basis"]["value"],
        "16"
    );
}
