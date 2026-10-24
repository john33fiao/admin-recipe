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
const COOKIE: &str = "hansang_4321=business-test-session";
const MISSING_ID: &str = "00000000-0000-4000-8000-000000000001";

fn app() -> (tempfile::TempDir, Router) {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::new(
        Store::open(directory.path()).unwrap(),
        HOST.into(),
        "business-test-session".into(),
        Policy::default(),
    );
    (directory, router(state))
}

async fn send(app: &Router, method: &str, path: &str, input: Option<&Value>) -> Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("host", HOST)
        .header("cookie", COOKIE);
    let body = if let Some(input) = input {
        builder = builder
            .header("origin", ORIGIN)
            .header("content-type", "application/json");
        Body::from(input.to_string())
    } else {
        Body::empty()
    };
    app.clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap()
}

async fn value(response: Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap_or_else(|_| {
        panic!(
            "expected JSON, received {}",
            String::from_utf8_lossy(&bytes)
        )
    })
}

async fn state(app: &Router) -> Value {
    let response = send(app, "GET", "/api/state", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    value(response).await
}

fn meta(state: &Value) -> Value {
    json!({"request_id":uuid::Uuid::new_v4().to_string(),
        "epoch":state["epoch"],"expected_revision":state["revision"]})
}

fn draft_input(state: &Value) -> Value {
    json!({"meta":meta(state),"corrects_confirmation_id":null,
        "payload":{"kind":"catalog_price","version":1,"data":{
            "material_id":null,"material_version_id":null,
            "quantity":"","unit":"g","amount":"0","note":"가상 HTTP 초안"}}})
}

async fn created(app: &Router, path: &str, input: &Value) -> Value {
    let response = send(app, "POST", path, Some(input)).await;
    let status = response.status();
    let result = value(response).await;
    assert_eq!(status, StatusCode::CREATED, "{path}: {result}");
    result
}

async fn material(app: &Router) -> Value {
    let current = state(app).await;
    let mut input = meta(&current);
    input["name"] = json!("가상 업무 HTTP 재료");
    input["unit"] = json!("g");
    input["kind"] = json!("raw");
    input["description"] = json!("");
    created(app, "/api/materials", &input).await
}

fn selected_payload(material: &Value, quantity: &str, amount: &str, note: &str) -> Value {
    json!({"kind":"catalog_price","version":1,"data":{
        "material_id":material["id"],"material_version_id":material["version_id"],
        "quantity":quantity,"unit":"kg","amount":amount,"note":note}})
}

async fn new_draft(app: &Router, payload: Value, target: Value) -> Value {
    created(
        app,
        "/api/business/drafts",
        &json!({
            "meta":meta(&state(app).await),"payload":payload,"corrects_confirmation_id":target
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

async fn confirm(app: &Router, draft: &Value) -> Value {
    let check = preview(app, draft).await;
    assert_eq!(check["can_confirm"], true, "{check}");
    let id = draft["result"]["item"]["id"].as_str().unwrap();
    created(
        app,
        &format!("/api/business/{id}/confirm"),
        &json!({
            "meta":meta(&check["state"]),"expected_head_id":check["head_id"]
        }),
    )
    .await
}

#[tokio::test]
async fn business_list_filters_and_missing_detail_keep_the_nested_contract() {
    let (_directory, app) = app();
    let response = send(&app, "GET", "/api/business", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let list = value(response).await;
    assert_eq!(list["items"], json!([]));
    assert!(list["state"]["epoch"].is_string());
    assert!(list["state"]["revision"].is_string());
    for query in [
        "status=deleted",
        "kind=inventory",
        "unrecognized=true",
        "status=draft&status=all",
    ] {
        let response = send(&app, "GET", &format!("/api/business?{query}"), None).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{query}");
        assert_eq!(value(response).await["code"], "validation");
    }
    let response = send(&app, "GET", &format!("/api/business/{MISSING_ID}"), None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(value(response).await["code"], "not_found");
    let catalog = value(send(&app, "GET", "/api/materials", None).await).await;
    assert!(catalog.is_array());
}

#[tokio::test]
async fn draft_envelopes_reject_unknown_fields_and_numeric_input_without_writes() {
    let (directory, app) = app();
    let before = state(&app).await;
    let input = draft_input(&before);
    let mut invalid = Vec::new();
    let mut extra = input.clone();
    extra["inventory_delta"] = json!(1);
    invalid.push(extra);
    let mut extra = input.clone();
    extra["meta"]["confirmed"] = json!(true);
    invalid.push(extra);
    let mut extra = input.clone();
    extra["payload"]["effect"] = json!(true);
    invalid.push(extra);
    let mut extra = input.clone();
    extra["payload"]["data"]["inventory_delta"] = json!(1);
    invalid.push(extra);
    let mut numeric = input.clone();
    numeric["payload"]["data"]["amount"] = json!(0);
    invalid.push(numeric);
    let mut future = input.clone();
    future["payload"]["version"] = json!(2);
    invalid.push(future);
    let mut unsupported = input.clone();
    unsupported["payload"]["kind"] = json!("sale");
    invalid.push(unsupported);
    for input in invalid {
        let response = send(&app, "POST", "/api/business/drafts", Some(&input)).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{input}");
        assert_eq!(value(response).await["code"], "validation");
    }
    assert_eq!(state(&app).await, before);
    assert_eq!(
        std::fs::read_dir(directory.path().join("snapshots"))
            .unwrap()
            .count(),
        0
    );
}

#[tokio::test]
async fn business_preview_and_writes_keep_origin_session_host_and_size_guards() {
    let (_directory, app) = app();
    let paths = [
        ("GET", "/api/business".to_owned()),
        ("GET", format!("/api/business/{MISSING_ID}")),
        ("POST", "/api/business/drafts".to_owned()),
        (
            "POST",
            format!("/api/business/{MISSING_ID}/draft-revisions"),
        ),
        ("POST", format!("/api/business/{MISSING_ID}/preview")),
        ("POST", format!("/api/business/{MISSING_ID}/confirm")),
        ("POST", format!("/api/business/{MISSING_ID}/archive")),
    ];
    for (method, path) in &paths {
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
            let mut builder = Request::builder()
                .method(*method)
                .uri(path)
                .header("host", host);
            if let Some(origin) = origin {
                builder = builder.header("origin", origin);
            }
            if let Some(cookie) = cookie {
                builder = builder.header("cookie", cookie);
            }
            let response = app
                .clone()
                .oneshot(builder.body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), expected, "{method} {path}");
        }
        if *method == "POST" {
            let no_origin = Request::builder()
                .method("POST")
                .uri(path)
                .header("host", HOST)
                .header("cookie", COOKIE)
                .body(Body::from("{}"))
                .unwrap();
            assert_eq!(
                app.clone().oneshot(no_origin).await.unwrap().status(),
                StatusCode::FORBIDDEN
            );
            let oversized = json!({"expected_head_id": "x".repeat(17 * 1024)});
            let response = send(&app, "POST", path, Some(&oversized)).await;
            assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE, "{path}");
        }
    }
}

#[tokio::test]
async fn business_direct_routes_are_spa_but_unknown_paths_stay_missing() {
    let (_directory, app) = app();
    for path in ["/business".to_owned(), format!("/business/{MISSING_ID}")] {
        for method in ["GET", "HEAD"] {
            let response = send(&app, method, &path, None).await;
            assert_eq!(response.status(), StatusCode::OK, "{method} {path}");
            assert!(
                response.headers()["content-type"]
                    .to_str()
                    .unwrap()
                    .starts_with("text/html")
            );
            if method == "HEAD" {
                assert!(
                    response
                        .into_body()
                        .collect()
                        .await
                        .unwrap()
                        .to_bytes()
                        .is_empty()
                );
            }
        }
    }
    for path in [
        "/business/not-a-uuid",
        "/business/settings",
        "/api/business/missing/extra",
        "/missing-business.js",
    ] {
        assert_eq!(
            send(&app, "GET", path, None).await.status(),
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
}

#[tokio::test]
async fn saved_draft_preview_and_confirmation_preserve_ids_and_retry_once() {
    let (directory, app) = app();
    let item = material(&app).await;
    let draft = new_draft(&app, selected_payload(&item, "", "0", ""), Value::Null).await;
    let id = draft["result"]["item"]["id"].as_str().unwrap();
    assert_eq!(draft["result"]["item"]["status"], "draft");
    assert_eq!(draft["result"]["head"]["payload"]["data"]["quantity"], "");
    assert!(draft["result"]["head"]["seq"].is_string());
    assert!(draft["result"]["confirmation"].is_null());
    let before = state(&app).await;
    let file_count = std::fs::read_dir(directory.path().join("snapshots"))
        .unwrap()
        .count();
    let incomplete = preview(&app, &draft).await;
    assert_eq!(incomplete["can_confirm"], false);
    assert!(
        incomplete["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["field"] == "quantity")
    );
    assert_eq!(state(&app).await, before);
    assert_eq!(
        std::fs::read_dir(directory.path().join("snapshots"))
            .unwrap()
            .count(),
        file_count
    );

    let revised = created(
        &app,
        &format!("/api/business/{id}/draft-revisions"),
        &json!({
            "meta":meta(&before),"expected_head_id":draft["result"]["head"]["id"],
            "payload":selected_payload(&item,"1","0","")
        }),
    )
    .await;
    assert_eq!(
        revised["result"]["item"]["id"],
        draft["result"]["item"]["id"]
    );
    assert_eq!(
        revised["result"]["head"]["previous_revision_id"],
        draft["result"]["head"]["id"]
    );
    let checked = preview(&app, &revised).await;
    assert_eq!(checked["can_confirm"], true);
    assert_eq!(checked["effect"]["quantity"], "1000");
    assert_eq!(checked["effect"]["amount"], "0");
    assert_eq!(checked["effect"]["unit"], "g");
    assert!(checked["effect"]["previous_price"].is_null());
    assert_eq!(state(&app).await, revised["state"]);
    let input = json!({"meta":meta(&checked["state"]),"expected_head_id":checked["head_id"]});
    let path = format!("/api/business/{id}/confirm");
    let confirmed = created(&app, &path, &input).await;
    assert_eq!(confirmed["result"]["item"]["status"], "confirmed");
    assert_eq!(
        confirmed["result"]["confirmation"]["seq"],
        confirmed["state"]["revision"]
    );
    assert_eq!(
        confirmed["result"]["confirmation"]["request_id"],
        input["meta"]["request_id"]
    );
    assert_eq!(created(&app, &path, &input).await, confirmed);

    let mut conflict = input.clone();
    conflict["expected_head_id"] = json!(uuid::Uuid::new_v4().to_string());
    let response = send(&app, "POST", &path, Some(&conflict)).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(value(response).await["code"], "request_conflict");
    let details = value(
        send(
            &app,
            "GET",
            &format!("/api/materials/{}", item["id"].as_str().unwrap()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(details["prices"].as_array().unwrap().len(), 1);
    assert_eq!(
        details["prices"][0]["id"],
        confirmed["result"]["confirmation"]["result"]["price_id"]
    );
    assert_eq!(details["price"]["amount"], "0");
    assert!(details.get("holding_quantity").is_none());

    let next = new_draft(&app, selected_payload(&item, "1", "10", ""), Value::Null).await;
    let stale_preview = preview(&app, &next).await;
    let unrelated = new_draft(&app, selected_payload(&item, "", "", ""), Value::Null).await;
    let next_id = next["result"]["item"]["id"].as_str().unwrap();
    let stale =
        json!({"meta":meta(&stale_preview["state"]),"expected_head_id":stale_preview["head_id"]});
    let response = send(
        &app,
        "POST",
        &format!("/api/business/{next_id}/confirm"),
        Some(&stale),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(value(response).await["code"], "stale_state");
    assert_eq!(state(&app).await, unrelated["state"]);
    assert_eq!(created(&app, &path, &input).await, confirmed);
    let detail = value(send(&app, "GET", &format!("/api/business/{next_id}"), None).await).await;
    assert_eq!(detail["item"]["status"], "draft");
    assert!(detail["confirmations"].as_array().unwrap().is_empty());
    let archived = created(
        &app,
        &format!("/api/business/{next_id}/archive"),
        &json!({"meta":meta(&state(&app).await),"expected_head_id":detail["head"]["id"]}),
    )
    .await;
    assert_eq!(archived["result"]["item"]["status"], "archived");
    let active = value(send(&app, "GET", "/api/business", None).await).await;
    assert_eq!(active["items"].as_array().unwrap().len(), 1);
    assert_eq!(active["items"][0]["id"], unrelated["result"]["item"]["id"]);
    let archived_list = value(
        send(
            &app,
            "GET",
            "/api/business?status=archived&kind=catalog_price",
            None,
        )
        .await,
    )
    .await;
    assert_eq!(archived_list["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        archived_list["items"][0]["id"],
        next["result"]["item"]["id"]
    );
}

#[tokio::test]
async fn correction_preview_distinguishes_target_price_and_current_price_and_retains_chain() {
    let (_directory, app) = app();
    let item = material(&app).await;
    let original = new_draft(&app, selected_payload(&item, "1", "10", ""), Value::Null).await;
    let confirmed = confirm(&app, &original).await;
    let confirmation_id = confirmed["result"]["confirmation"]["id"].clone();
    let mut independent = meta(&state(&app).await);
    independent["expected_version_id"] = item["version_id"].clone();
    independent["quantity"] = json!("1");
    independent["unit"] = json!("kg");
    independent["amount"] = json!("20");
    created(
        &app,
        &format!("/api/materials/{}/prices", item["id"].as_str().unwrap()),
        &independent,
    )
    .await;
    let correction = new_draft(
        &app,
        selected_payload(&item, "1", "30", "가상 입력 금액 정정"),
        confirmation_id.clone(),
    )
    .await;
    let check = preview(&app, &correction).await;
    assert_eq!(check["effect"]["corrected_price"]["amount"], "10");
    assert_eq!(check["effect"]["previous_price"]["amount"], "20");
    assert_eq!(check["effect"]["amount"], "30");
    let replacement = confirm(&app, &correction).await;
    let root_id = original["result"]["item"]["id"].as_str().unwrap();
    let detail = value(send(&app, "GET", &format!("/api/business/{root_id}"), None).await).await;
    assert_eq!(detail["confirmations"].as_array().unwrap().len(), 2);
    assert_eq!(detail["corrections"].as_array().unwrap().len(), 1);
    assert_eq!(
        detail["item"]["effective_confirmation_id"],
        replacement["result"]["confirmation"]["id"]
    );
    assert_eq!(
        detail["corrections"][0]["target_confirmation_id"],
        confirmation_id
    );
    assert_eq!(
        detail["confirmations"][1]["record_id"],
        correction["result"]["item"]["id"]
    );

    let missing_reason = new_draft(
        &app,
        selected_payload(&item, "1", "40", ""),
        replacement["result"]["confirmation"]["id"].clone(),
    )
    .await;
    let invalid = preview(&app, &missing_reason).await;
    assert_eq!(invalid["can_confirm"], false);
    assert!(
        invalid["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["field"] == "note")
    );
    let prices = value(
        send(
            &app,
            "GET",
            &format!("/api/materials/{}", item["id"].as_str().unwrap()),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(prices["prices"].as_array().unwrap().len(), 3);
    assert_eq!(prices["price"]["amount"], "30");
}
