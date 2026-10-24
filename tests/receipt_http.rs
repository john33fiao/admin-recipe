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
const COOKIE: &str = "hansang_4321=receipt-test-session";
const MISSING_ID: &str = "00000000-0000-4000-8000-000000000001";

fn app() -> (tempfile::TempDir, Router) {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::new(
        Store::open(directory.path()).unwrap(),
        HOST.into(),
        "receipt-test-session".into(),
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

async fn material(app: &Router) -> Value {
    let mut input = meta(&get(app, "/api/state").await);
    input["name"] = json!("가상 입고 HTTP 재료");
    input["unit"] = json!("g");
    input["kind"] = json!("raw");
    input["description"] = json!("");
    created(app, "/api/materials", &input).await
}

fn direct_data() -> Value {
    json!({"source":{"kind":"direct","material_id":null,"material_version_id":null},
        "quantity":"","unit":"kg","amount":"","receipt_date":"","note":"",
        "stock_effect":{"kind":"unreflected"}})
}

fn draft_input(state: &Value, data: Value) -> Value {
    json!({"meta":meta(state),"corrects_confirmation_id":null,
        "payload":{"kind":"receipt","version":1,"data":data}})
}

fn measured(material: &Value, quantity: &str, amount: &str) -> Value {
    let mut data = direct_data();
    data["source"]["material_id"] = material["id"].clone();
    data["source"]["material_version_id"] = material["version_id"].clone();
    data["quantity"] = json!(quantity);
    data["amount"] = json!(amount);
    data["receipt_date"] = json!("2020-02-29");
    data
}

async fn draft(app: &Router, kind: &str, data: Value) -> Value {
    let mut input = draft_input(&get(app, "/api/state").await, data);
    input["payload"]["kind"] = json!(kind);
    created(app, "/api/business/drafts", &input).await
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

#[tokio::test]
async fn receipt_list_filters_and_missing_detail_have_explicit_contracts() {
    let (_directory, app) = app();
    let list = get(&app, "/api/receipts").await;
    assert_eq!(list["items"], json!([]));
    assert!(list["state"]["epoch"].is_string());
    assert!(list["state"]["revision"].is_string());
    assert_eq!(
        get(&app, &format!("/api/receipts?material_id={MISSING_ID}")).await["items"],
        json!([])
    );
    for path in ["/api/receipts?material_id=bad", "/api/receipts?extra=true"] {
        let response = send(&app, "GET", path, None).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}");
        assert_eq!(value(response).await["code"], "validation");
    }
    let response = send(&app, "GET", &format!("/api/receipts/{MISSING_ID}"), None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(value(response).await["code"], "not_found");
}

#[tokio::test]
async fn existing_material_has_unknown_holding_without_manufactured_zero() {
    let (_directory, app) = app();
    let material = material(&app).await;
    let holding = get(
        &app,
        &format!("/api/holdings/{}", material["id"].as_str().unwrap()),
    )
    .await;
    assert_eq!(holding["material_id"], material["id"]);
    for key in [
        "basis",
        "observation",
        "settlement_basis",
        "last_valid_price",
    ] {
        assert!(
            holding.get(key).is_some_and(Value::is_null),
            "{key}: {holding}"
        );
    }
    assert_eq!(holding["confirmed_changes"], json!([]));
    assert_eq!(holding["price"]["source"], "missing");
    let response = send(&app, "GET", &format!("/api/holdings/{MISSING_ID}"), None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(value(response).await["code"], "not_found");
}

#[tokio::test]
async fn receipt_spa_routes_allow_get_head_without_unknown_fallback() {
    let (_directory, app) = app();
    for path in ["/receipts".to_owned(), format!("/receipts/{MISSING_ID}")] {
        for method in ["GET", "HEAD"] {
            let response = send(&app, method, &path, None).await;
            assert_eq!(response.status(), StatusCode::OK, "{method} {path}");
            assert!(
                response.headers()["content-type"]
                    .to_str()
                    .unwrap()
                    .contains("text/html")
            );
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(bytes.is_empty(), method == "HEAD");
        }
    }
    for path in [
        "/receipts/not-a-uuid",
        "/receipts/extra/path",
        "/api/receipts/extra/path",
    ] {
        assert_eq!(
            send(&app, "GET", path, None).await.status(),
            StatusCode::NOT_FOUND
        );
    }
}

#[tokio::test]
async fn incomplete_direct_and_reconciliation_receipts_remain_drafts() {
    let (_directory, app) = app();
    let mut held = direct_data();
    held["stock_effect"] =
        json!({"kind":"needs_reconciliation","basis_id":null,"included_quantity":""});
    for data in [direct_data(), held] {
        let input = draft_input(&get(&app, "/api/state").await, data.clone());
        let saved = created(&app, "/api/business/drafts", &input).await;
        assert_eq!(saved["result"]["item"]["kind"], "receipt");
        assert_eq!(saved["result"]["item"]["status"], "draft");
        assert_eq!(saved["result"]["head"]["payload"]["data"], data);
        assert!(saved["result"]["confirmation"].is_null());
        let response = send(
            &app,
            "POST",
            &format!(
                "/api/business/{}/preview",
                saved["result"]["item"]["id"].as_str().unwrap()
            ),
            Some(&json!({"expected_head_id":saved["result"]["head"]["id"]})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let preview = value(response).await;
        assert_eq!(preview["can_confirm"], false);
        assert!(preview["effect"].is_null());
    }
    assert_eq!(get(&app, "/api/receipts").await["items"], json!([]));
}

#[tokio::test]
async fn receipt_requests_preserve_host_origin_session_and_size_guards() {
    let (_directory, app) = app();
    let initial = get(&app, "/api/state").await;
    let input = draft_input(&initial, direct_data());
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
                .oneshot(request.body(Body::from(input.to_string())).unwrap())
                .await
                .unwrap()
                .status(),
            expected
        );
    }
    let mut oversized = input;
    oversized["payload"]["data"]["note"] = json!("x".repeat(17 * 1024));
    assert_eq!(
        send(&app, "POST", "/api/business/drafts", Some(&oversized))
            .await
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(get(&app, "/api/state").await, initial);
    for path in [
        "/api/receipts".to_owned(),
        format!("/api/receipts/{MISSING_ID}"),
        format!("/api/holdings/{MISSING_ID}"),
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
                .method("GET")
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
}

#[tokio::test]
async fn direct_receipts_update_exact_holding_once_without_creating_observations_or_prices() {
    let (_directory, app) = app();
    let material = material(&app).await;
    let material_path = format!("/api/materials/{}", material["id"].as_str().unwrap());
    let holding_path = format!("/api/holdings/{}", material["id"].as_str().unwrap());
    let original = get(&app, &material_path).await;
    let mut first_receipt = Value::Null;
    let mut latest_result = Value::Null;
    for (quantity, amount, total_quantity, total_value) in [
        ("1", "10000", "1000", "10000"),
        ("2", "40000", "3000", "50000"),
    ] {
        let saved = draft(&app, "receipt", measured(&material, quantity, amount)).await;
        let before = get(&app, &holding_path).await;
        let state = get(&app, "/api/state").await;
        let check = preview(&app, &saved).await;
        assert_eq!(check["can_confirm"], true, "{check}");
        assert_eq!(check["effect"]["after"]["quantity"], total_quantity);
        assert_eq!(check["effect"]["after"]["value"], total_value);
        assert!(check["effect"]["after"]["source_id"].is_null());
        assert_eq!(get(&app, "/api/state").await, state);
        assert_eq!(get(&app, &holding_path).await, before);
        let (confirmed, input) = confirm(&app, &saved).await;
        assert_eq!(
            created(&app, &confirm_path(&saved), &input).await,
            confirmed
        );
        latest_result = confirmed["result"]["confirmation"]["result"].clone();
        assert_eq!(latest_result["kind"], "receipt");
        assert!(latest_result["order_id"].is_null());
        let receipt = get(
            &app,
            &format!(
                "/api/receipts/{}",
                latest_result["receipt_id"].as_str().unwrap()
            ),
        )
        .await;
        assert_eq!(receipt["item"]["source"], json!({"kind":"direct"}));
        assert_eq!(receipt["item"]["input_quantity"], quantity);
        assert_eq!(receipt["item"]["amount"], amount);
        assert_eq!(receipt["item"]["receipt_date"], "2020-02-29");
        assert!(
            receipt["item"]["recorded_at"]
                .as_str()
                .unwrap()
                .contains('T')
        );
        assert!(receipt["item"]["seq"].is_string());
        assert!(receipt["order_closure"].is_null());
        if first_receipt.is_null() {
            first_receipt = receipt["item"].clone();
        }
        let holding = get(&app, &holding_path).await;
        assert_eq!(holding["basis"]["kind"], "receipt");
        assert_eq!(holding["basis"]["id"], first_receipt["id"]);
        assert_eq!(holding["basis"]["seq"], first_receipt["seq"]);
        assert!(holding["observation"].is_null());
        assert_eq!(holding["settlement_basis"]["quantity"], total_quantity);
        assert_eq!(holding["settlement_basis"]["value"], total_value);
        assert_eq!(holding["price"]["source"], "holding_average");
        assert_eq!(holding["price"]["amount"], total_value);
        assert_eq!(holding["price"]["quantity"], total_quantity);
    }
    let holding = get(&app, &holding_path).await;
    assert_eq!(
        holding["price"]["source_id"],
        latest_result["holding_version_id"]
    );
    assert_eq!(
        holding["last_valid_price"]["source_id"],
        latest_result["holding_version_id"]
    );
    let changes = holding["confirmed_changes"].as_array().unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0]["quantity"], "2000");
    assert_eq!(changes[0]["value"], "40000");
    assert!(changes[0]["seq"].is_string());
    let after = get(&app, &material_path).await;
    assert_eq!(after["prices"], original["prices"]);
    assert_eq!(after["versions"], original["versions"]);
    assert_eq!(after["price"], holding["price"]);
    assert_eq!(
        get(&app, "/api/receipts").await["items"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn real_receipt_closes_revised_order_and_distinguishes_stale_from_conflicting_retries() {
    let (_directory, app) = app();
    let material = material(&app).await;
    let placed_draft = draft(
        &app,
        "purchase_order",
        json!({"action":"place",
        "material_id":material["id"],"material_version_id":material["version_id"],
        "quantity":"10","unit":"kg","supplier":"가상 거래처","expected_date":"",
        "estimated_amount":"0","note":"발주 메모"}),
    )
    .await;
    let (placed, _) = confirm(&app, &placed_draft).await;
    let order_id = placed["result"]["confirmation"]["result"]["order_id"]
        .as_str()
        .unwrap();
    let order_path = format!("/api/orders/{order_id}");
    let initial = get(&app, &order_path).await;
    let revised_draft = draft(&app, "purchase_order", json!({"action":"revise","order_id":order_id,
        "expected_order_version_id":initial["head"]["id"],"material_version_id":material["version_id"],
        "quantity":"12","unit":"kg","supplier":"가상 거래처","expected_date":"",
        "estimated_amount":"0","note":"수정 메모"})).await;
    confirm(&app, &revised_draft).await;
    let ordered = get(&app, &order_path).await;
    let mut data = measured(&material, "11.8", "11800");
    data["source"] = json!({"kind":"order","order_id":order_id,
        "expected_order_version_id":ordered["head"]["id"],"material_version_id":material["version_id"]});
    data["note"] = json!("입고 메모");
    let saved = draft(&app, "receipt", data.clone()).await;
    let check = preview(&app, &saved).await;
    assert_eq!(check["effect"]["order_after"]["difference"], "-200");
    let stale_input = json!({"meta":meta(&check["state"]),"expected_head_id":check["head_id"]});
    draft(&app, "receipt", direct_data()).await;
    let response = send(&app, "POST", &confirm_path(&saved), Some(&stale_input)).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(value(response).await["code"], "stale_state");
    assert_eq!(get(&app, "/api/receipts").await["items"], json!([]));
    assert_eq!(get(&app, &order_path).await["head"], ordered["head"]);
    let (confirmed, input) = confirm(&app, &saved).await;
    let result = &confirmed["result"]["confirmation"]["result"];
    let receipt = get(
        &app,
        &format!("/api/receipts/{}", result["receipt_id"].as_str().unwrap()),
    )
    .await;
    let closure = &receipt["order_closure"];
    assert_eq!(closure["initial_order_version_id"], initial["head"]["id"]);
    assert_eq!(closure["initial_ordered_quantity"], "10000");
    assert_eq!(closure["ordered_version_id"], ordered["head"]["id"]);
    assert_eq!(closure["ordered_quantity"], "12000");
    assert_eq!(closure["received_quantity"], "11800");
    assert_eq!(closure["difference"], "-200");
    assert_eq!(closure["receipt_id"], receipt["item"]["id"]);
    assert_eq!(
        closure["receipt_confirmation_id"],
        receipt["item"]["confirmation_id"]
    );
    let received = get(&app, &order_path).await;
    assert_eq!(received["head"]["action"], "receive");
    assert_eq!(received["head"]["status"], "received");
    assert_eq!(received["head"]["quantity"], "12000");
    assert_eq!(received["head"]["note"], "수정 메모");
    assert_eq!(
        received["head"]["confirmation_id"],
        receipt["item"]["confirmation_id"]
    );
    assert_eq!(
        received["head"]["business_record_id"],
        saved["result"]["item"]["id"]
    );
    assert_eq!(received["versions"][0], initial["head"]);
    assert_eq!(received["versions"][1], ordered["head"]);
    assert_eq!(received["versions"].as_array().unwrap().len(), 3);
    assert_eq!(
        created(&app, &confirm_path(&saved), &input).await,
        confirmed
    );
    let mut conflicting = input;
    conflicting["expected_head_id"] = json!(uuid::Uuid::new_v4().to_string());
    let response = send(&app, "POST", &confirm_path(&saved), Some(&conflicting)).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(value(response).await["code"], "request_conflict");
    let duplicate = draft(&app, "receipt", data).await;
    let state = get(&app, "/api/state").await;
    let response = send(
        &app,
        "POST",
        &confirm_path(&duplicate),
        Some(&json!({"meta":meta(&state),"expected_head_id":duplicate["result"]["head"]["id"]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(value(response).await["code"], "stale_state");
    assert_eq!(get(&app, "/api/state").await, state);
    assert_eq!(
        get(&app, "/api/receipts").await["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        get(&app, "/api/orders?status=received").await["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn receipt_validation_preserves_drafts_and_zero_amount_is_not_missing() {
    let (_directory, app) = app();
    let material = material(&app).await;
    let good = measured(&material, "1", "0");
    let mut invalid_shapes = Vec::new();
    let mut numeric = good.clone();
    numeric["quantity"] = json!(1);
    invalid_shapes.push(numeric);
    let mut numeric_amount = good.clone();
    numeric_amount["amount"] = json!(0);
    invalid_shapes.push(numeric_amount);
    let mut extra = good.clone();
    extra["source"]["order_id"] = json!(MISSING_ID);
    invalid_shapes.push(extra);
    let mut extra_effect = good.clone();
    extra_effect["stock_effect"]["included_quantity"] = json!("1");
    invalid_shapes.push(extra_effect);
    let state = get(&app, "/api/state").await;
    for data in invalid_shapes {
        let response = send(
            &app,
            "POST",
            "/api/business/drafts",
            Some(&draft_input(&state, data)),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(value(response).await["code"], "validation");
    }
    assert_eq!(get(&app, "/api/state").await, state);
    let mut invalid_values = Vec::new();
    for (field, text) in [
        ("quantity", "0"),
        ("amount", ""),
        ("receipt_date", "9999-12-31"),
        ("unit", "mL"),
        ("quantity", "9000000000"),
    ] {
        let mut data = good.clone();
        data[field] = json!(text);
        invalid_values.push(data);
    }
    let mut included = good.clone();
    included["stock_effect"] =
        json!({"kind":"needs_reconciliation","basis_id":null,"included_quantity":""});
    invalid_values.push(included);
    for data in invalid_values {
        let saved = draft(&app, "receipt", data).await;
        let state = get(&app, "/api/state").await;
        let check = preview(&app, &saved).await;
        assert_eq!(check["can_confirm"], false, "{check}");
        assert!(check["effect"].is_null());
        let response = send(
            &app,
            "POST",
            &confirm_path(&saved),
            Some(&json!({"meta":meta(&state),"expected_head_id":saved["result"]["head"]["id"]})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(value(response).await["code"], "validation");
        assert_eq!(get(&app, "/api/state").await, state);
        assert_eq!(get(&app, "/api/receipts").await["items"], json!([]));
    }
    let zero = draft(&app, "receipt", good).await;
    confirm(&app, &zero).await;
    let holding = get(
        &app,
        &format!("/api/holdings/{}", material["id"].as_str().unwrap()),
    )
    .await;
    assert_eq!(holding["settlement_basis"]["value"], "0");
    assert_eq!(holding["price"]["source"], "holding_average");
    assert_eq!(holding["price"]["amount"], "0");
}
