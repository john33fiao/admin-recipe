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
const COOKIE: &str = "hansang_4321=order-test-session";
const MISSING_ID: &str = "00000000-0000-4000-8000-000000000001";

fn app() -> (tempfile::TempDir, Router) {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::new(
        Store::open(directory.path()).unwrap(),
        HOST.into(),
        "order-test-session".into(),
        Policy::default(),
    );
    (directory, router(state))
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
    json!({"request_id":uuid::Uuid::new_v4().to_string(),
        "epoch":state["epoch"],"expected_revision":state["revision"]})
}

async fn created(app: &Router, path: &str, input: &Value) -> Value {
    let response = send(app, "POST", path, Some(input)).await;
    let status = response.status();
    let body = value(response).await;
    assert_eq!(status, StatusCode::CREATED, "{path}: {body}");
    body
}

fn place_data() -> Value {
    json!({"action":"place","material_id":null,"material_version_id":null,
        "quantity":"","unit":"kg","supplier":"","expected_date":"",
        "estimated_amount":"","note":""})
}

async fn draft(app: &Router, data: Value) -> Value {
    created(
        app,
        "/api/business/drafts",
        &json!({
            "meta":meta(&get(app,"/api/state").await),"corrects_confirmation_id":null,
            "payload":{"kind":"purchase_order","version":1,"data":data}
        }),
    )
    .await
}

async fn preview(app: &Router, draft: &Value) -> Value {
    let id = draft["result"]["item"]["id"].as_str().unwrap();
    let response = send(
        app,
        "POST",
        &format!("/api/business/{id}/preview"),
        Some(&json!({"expected_head_id":draft["result"]["head"]["id"]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    value(response).await
}

async fn confirm(app: &Router, draft: &Value) -> (Value, Value) {
    let check = preview(app, draft).await;
    assert_eq!(check["can_confirm"], true, "{check}");
    let input = json!({"meta":meta(&check["state"]),"expected_head_id":check["head_id"]});
    let path = format!(
        "/api/business/{}/confirm",
        check["record_id"].as_str().unwrap()
    );
    (created(app, &path, &input).await, input)
}

async fn material(app: &Router) -> Value {
    let mut input = meta(&get(app, "/api/state").await);
    input["name"] = json!("가상 발주 HTTP 재료");
    input["unit"] = json!("g");
    input["kind"] = json!("raw");
    input["description"] = json!("");
    created(app, "/api/materials", &input).await
}

fn selected_place(material: &Value) -> Value {
    let mut data = place_data();
    data["material_id"] = material["id"].clone();
    data["material_version_id"] = material["version_id"].clone();
    data["quantity"] = json!("10");
    data
}

#[tokio::test]
async fn order_get_filters_and_missing_detail_keep_the_nested_contract() {
    let (_directory, app) = app();
    for path in [
        "/api/orders",
        "/api/orders?status=received",
        "/api/orders?status=all",
    ] {
        let list = get(&app, path).await;
        assert_eq!(list["items"], json!([]));
        assert!(list["state"]["epoch"].is_string());
        assert!(list["state"]["revision"].is_string());
    }
    for path in [
        "/api/orders?status=draft",
        "/api/orders?unknown=true",
        "/api/orders?status=open&status=all",
        "/api/orders/not-a-uuid",
    ] {
        let response = send(&app, "GET", path, None).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}");
        assert_eq!(value(response).await["code"], "validation");
    }
    let response = send(&app, "GET", &format!("/api/orders/{MISSING_ID}"), None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(value(response).await["code"], "not_found");
}

#[tokio::test]
async fn order_spa_routes_support_get_and_head_without_unknown_fallback() {
    let (_directory, app) = app();
    for path in ["/orders".to_owned(), format!("/orders/{MISSING_ID}")] {
        for method in ["GET", "HEAD"] {
            let response = send(&app, method, &path, None).await;
            assert_eq!(response.status(), StatusCode::OK, "{method} {path}");
            assert!(
                response.headers()["content-type"]
                    .to_str()
                    .unwrap()
                    .starts_with("text/html")
            );
            let body = response.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(body.is_empty(), method == "HEAD");
        }
    }
    for path in [
        "/orders/not-a-uuid",
        "/orders/settings",
        "/orders/missing.js",
        "/api/orders/extra/path",
    ] {
        assert_eq!(
            send(&app, "GET", path, None).await.status(),
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
}

#[tokio::test]
async fn place_revise_cancel_keep_one_order_history_and_catalog_unchanged() {
    let (_directory, app) = app();
    let material = material(&app).await;
    let material_path = format!("/api/materials/{}", material["id"].as_str().unwrap());
    let before = get(&app, &material_path).await;
    let first_draft = draft(&app, selected_place(&material)).await;
    let (placed, _) = confirm(&app, &first_draft).await;
    let result = &placed["result"]["confirmation"]["result"];
    assert_eq!(result["kind"], "purchase_order");
    assert_eq!(result["action"], "place");
    let order_id = result["order_id"].as_str().unwrap();
    let path = format!("/api/orders/{order_id}");
    let first = get(&app, &path).await;
    assert_eq!(first["head"]["quantity"], "10000");
    assert_eq!(first["head"]["unit"], "g");
    assert_eq!(first["head"]["estimated_amount"], Value::Null);
    assert_eq!(first["item"]["status"], "open");
    let revised = draft(
        &app,
        json!({"action":"revise","order_id":order_id,
        "expected_order_version_id":first["head"]["id"],
        "material_version_id":material["version_id"],"quantity":"12","unit":"kg",
        "supplier":"가상 거래처","expected_date":"2028-02-29","estimated_amount":"0","note":""}),
    )
    .await;
    let check = preview(&app, &revised).await;
    assert_eq!(check["effect"]["before"]["quantity"], "10000");
    assert_eq!(check["effect"]["after"]["quantity"], "12000");
    confirm(&app, &revised).await;
    let updated = get(&app, &path).await;
    assert_eq!(updated["head"]["estimated_amount"], "0");
    assert_eq!(updated["head"]["material_id"], material["id"]);
    let cancelled = draft(
        &app,
        json!({"action":"cancel","order_id":order_id,
        "expected_order_version_id":updated["head"]["id"],"note":""}),
    )
    .await;
    let (closed, close_input) = confirm(&app, &cancelled).await;
    let final_detail = get(&app, &path).await;
    assert_eq!(final_detail["item"]["id"], order_id);
    assert_eq!(final_detail["item"]["status"], "cancelled");
    assert_eq!(final_detail["head"]["quantity"], "12000");
    assert_eq!(final_detail["head"]["supplier"], "가상 거래처");
    assert_eq!(final_detail["head"]["expected_date"], "2028-02-29");
    assert_eq!(final_detail["head"]["estimated_amount"], "0");
    let versions = final_detail["versions"].as_array().unwrap();
    assert_eq!(versions.len(), 3);
    assert_eq!(versions[0], first["head"]);
    assert_eq!(versions[1], updated["head"]);
    assert_eq!(versions[2]["previous_version_id"], versions[1]["id"]);
    for version in versions {
        assert!(version["seq"].is_string());
        assert!(version["confirmation_id"].is_string());
        assert!(version["business_record_id"].is_string());
    }
    let confirm_path = format!(
        "/api/business/{}/confirm",
        cancelled["result"]["item"]["id"].as_str().unwrap()
    );
    assert_eq!(created(&app, &confirm_path, &close_input).await, closed);
    assert_eq!(get(&app, "/api/orders").await["items"], json!([]));
    assert_eq!(
        get(&app, "/api/orders?status=cancelled").await["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let after = get(&app, &material_path).await;
    assert_eq!(after["price"], before["price"]);
    assert_eq!(after["prices"], before["prices"]);
    assert_eq!(after["versions"], before["versions"]);
}

#[tokio::test]
async fn revise_and_cancel_missing_references_remain_drafts_without_order_effects() {
    let (_directory, app) = app();
    for data in [
        json!({"action":"revise","order_id":MISSING_ID,"expected_order_version_id":MISSING_ID,
            "material_version_id":null,"quantity":"1.","unit":"kg","supplier":"",
            "expected_date":"2026-","estimated_amount":"","note":""}),
        json!({"action":"cancel","order_id":MISSING_ID,"expected_order_version_id":MISSING_ID,"note":""}),
    ] {
        let saved = draft(&app, data.clone()).await;
        assert_eq!(saved["result"]["head"]["payload"]["data"], data);
        assert_eq!(saved["result"]["item"]["status"], "draft");
        assert!(!saved["result"]["issues"].as_array().unwrap().is_empty());
        assert_eq!(preview(&app, &saved).await["can_confirm"], false);
    }
    assert_eq!(
        get(&app, "/api/orders?status=all").await["items"],
        json!([])
    );
    let drafts = get(&app, "/api/business?kind=purchase_order").await;
    assert_eq!(drafts["items"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn stale_order_revision_is_unprocessed_and_conflicting_retry_keeps_original() {
    let (_directory, app) = app();
    let material = material(&app).await;
    let placed = draft(&app, selected_place(&material)).await;
    let (placed, _) = confirm(&app, &placed).await;
    let order_id = placed["result"]["confirmation"]["result"]["order_id"]
        .as_str()
        .unwrap();
    let path = format!("/api/orders/{order_id}");
    let order = get(&app, &path).await;
    let pending_revision = draft(&app, json!({"action":"revise","order_id":order_id,
        "expected_order_version_id":order["head"]["id"],"material_version_id":material["version_id"],
        "quantity":"20","unit":"kg","supplier":"","expected_date":"",
        "estimated_amount":"","note":""})).await;
    let old_preview = preview(&app, &pending_revision).await;
    assert_eq!(old_preview["can_confirm"], true);
    let cancellation = draft(
        &app,
        json!({"action":"cancel","order_id":order_id,
        "expected_order_version_id":order["head"]["id"],"note":""}),
    )
    .await;
    let (closed, close_input) = confirm(&app, &cancellation).await;
    let revision_path = format!(
        "/api/business/{}/confirm",
        old_preview["record_id"].as_str().unwrap()
    );
    for basis in [old_preview["state"].clone(), get(&app, "/api/state").await] {
        let input = json!({"meta":meta(&basis),"expected_head_id":old_preview["head_id"]});
        let response = send(&app, "POST", &revision_path, Some(&input)).await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(value(response).await["code"], "stale_state");
    }
    let revision_check = preview(&app, &pending_revision).await;
    assert_eq!(revision_check["can_confirm"], false);
    let close_path = format!(
        "/api/business/{}/confirm",
        cancellation["result"]["item"]["id"].as_str().unwrap()
    );
    let mut conflicting = close_input.clone();
    conflicting["expected_head_id"] = json!(uuid::Uuid::new_v4().to_string());
    let response = send(&app, "POST", &close_path, Some(&conflicting)).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(value(response).await["code"], "request_conflict");
    assert_eq!(created(&app, &close_path, &close_input).await, closed);
    let retained = get(&app, &path).await;
    assert_eq!(retained["head"]["quantity"], "10000");
    assert_eq!(retained["head"]["status"], "cancelled");
    assert_eq!(retained["versions"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn order_boundaries_reject_unknown_input_and_preserve_access_guards() {
    let (directory, app) = app();
    let initial = get(&app, "/api/state").await;
    let valid = json!({"meta":meta(&initial),"corrects_confirmation_id":null,
        "payload":{"kind":"purchase_order","version":1,"data":place_data()}});
    let mut invalid = vec![];
    let mut numeric = valid.clone();
    numeric["payload"]["data"]["quantity"] = json!(10);
    invalid.push(numeric);
    let mut numeric_amount = valid.clone();
    numeric_amount["payload"]["data"]["estimated_amount"] = json!(0);
    invalid.push(numeric_amount);
    let mut extra = valid.clone();
    extra["payload"]["data"]["inventory_delta"] = json!("10");
    invalid.push(extra);
    let mut future = valid.clone();
    future["payload"]["version"] = json!(2);
    invalid.push(future);
    let mut action = valid.clone();
    action["payload"]["data"]["action"] = json!("receive");
    invalid.push(action);
    let mut wrong_correction = valid.clone();
    wrong_correction["corrects_confirmation_id"] = json!(MISSING_ID);
    invalid.push(wrong_correction);
    let mut mixed_action = valid.clone();
    mixed_action["payload"]["data"] = json!({"action":"cancel","order_id":MISSING_ID,
        "expected_order_version_id":MISSING_ID,"note":"","quantity":"1"});
    invalid.push(mixed_action);
    for input in invalid {
        let response = send(&app, "POST", "/api/business/drafts", Some(&input)).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(value(response).await["code"], "validation");
    }
    for (method, path) in [
        ("GET", "/api/orders".to_owned()),
        ("GET", format!("/api/orders/{MISSING_ID}")),
        ("POST", "/api/business/drafts".to_owned()),
    ] {
        for (host, origin, cookie, expected) in [
            (
                "external.test",
                Some(ORIGIN),
                Some(COOKIE),
                StatusCode::FORBIDDEN,
            ),
            (
                HOST,
                Some("https://external.test"),
                Some(COOKIE),
                StatusCode::FORBIDDEN,
            ),
            (HOST, Some(ORIGIN), None, StatusCode::UNAUTHORIZED),
        ] {
            let mut request = Request::builder()
                .method(method)
                .uri(&path)
                .header("host", host);
            if let Some(origin) = origin {
                request = request.header("origin", origin);
            }
            if let Some(cookie) = cookie {
                request = request.header("cookie", cookie);
            }
            assert_eq!(
                app.clone()
                    .oneshot(request.body(Body::empty()).unwrap())
                    .await
                    .unwrap()
                    .status(),
                expected
            );
        }
    }
    let no_origin = Request::builder()
        .method("POST")
        .uri("/api/business/drafts")
        .header("host", HOST)
        .header("cookie", COOKIE)
        .body(Body::from(valid.to_string()))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(no_origin).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    let mut oversized = valid;
    oversized["payload"]["data"]["note"] = json!("x".repeat(17 * 1024));
    assert_eq!(
        send(&app, "POST", "/api/business/drafts", Some(&oversized))
            .await
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(get(&app, "/api/state").await, initial);
    assert_eq!(
        std::fs::read_dir(directory.path().join("snapshots"))
            .unwrap()
            .count(),
        0
    );
}
