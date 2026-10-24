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

fn app() -> (tempfile::TempDir, Router) {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::new(
        Store::open(directory.path()).unwrap(),
        "127.0.0.1:4321".into(),
        "catalog-test-session".into(),
        Policy::default(),
    );
    (directory, router(state))
}

async fn send(app: &Router, method: &str, path: &str, input: Option<&Value>) -> Response {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("host", "127.0.0.1:4321")
        .header("cookie", "hansang_4321=catalog-test-session");
    let body = if let Some(input) = input {
        request = request
            .header("origin", "http://127.0.0.1:4321")
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
    let result = value(response).await;
    assert!(result["epoch"].is_string());
    assert!(result["revision"].is_string());
    result
}

fn new_material(state: &Value, name: &str) -> Value {
    json!({
        "request_id": uuid::Uuid::new_v4().to_string(),
        "epoch": state["epoch"],
        "expected_revision": state["revision"],
        "name": name,
        "unit": "g",
        "kind": "raw",
        "description": "가상 HTTP 검증 자료"
    })
}

async fn create(app: &Router, name: &str) -> Value {
    let input = new_material(&state(app).await, name);
    let response = send(app, "POST", "/api/materials", Some(&input)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    value(response).await
}

async fn detail(app: &Router, id: &str) -> Value {
    let response = send(app, "GET", &format!("/api/materials/{id}"), None).await;
    assert_eq!(response.status(), StatusCode::OK);
    value(response).await
}

fn mutation(detail: &Value) -> Value {
    json!({
        "request_id": uuid::Uuid::new_v4().to_string(),
        "epoch": detail["epoch"],
        "expected_revision": detail["revision"],
        "expected_version_id": detail["version_id"]
    })
}

#[tokio::test]
async fn stale_state_is_unprocessed_but_request_conflict_preserves_the_original() {
    let (_directory, app) = app();
    let initial = state(&app).await;
    let input = new_material(&initial, "가상 최초 재료");
    let response = send(&app, "POST", "/api/materials", Some(&input)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let original = value(response).await;

    let mut conflict = input.clone();
    conflict["name"] = json!("가상 바뀐 본문");
    let response = send(&app, "POST", "/api/materials", Some(&conflict)).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let error = value(response).await;
    assert_eq!(error["code"], "request_conflict");
    assert!(error["error"].is_string());

    let stale = new_material(&initial, "가상 미처리 재료");
    let response = send(&app, "POST", "/api/materials", Some(&stale)).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(value(response).await["code"], "stale_state");

    let response = send(&app, "POST", "/api/materials", Some(&input)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(value(response).await, original);
    let rows = value(send(&app, "GET", "/api/materials", None).await).await;
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["name"], "가상 최초 재료");
    assert_eq!(rows[0]["id"], original["id"]);
}

#[tokio::test]
async fn zero_price_versions_and_archive_preserve_history_and_literal_search() {
    let (_directory, app) = app();
    let first = create(&app, "가상 %_ 재료").await;
    create(&app, "가상 비교 재료").await;
    let id = first["id"].as_str().unwrap();
    let before = detail(&app, id).await;
    let mut price = mutation(&before);
    price["quantity"] = json!("1.25");
    price["unit"] = json!("kg");
    price["amount"] = json!("0");
    let response = send(
        &app,
        "POST",
        &format!("/api/materials/{id}/prices"),
        Some(&price),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let price_result = value(response).await;
    assert_eq!(price_result["amount"], "0");
    assert_eq!(price_result["quantity"], "1250");
    let priced = detail(&app, id).await;
    assert_eq!(priced["prices"].as_array().unwrap().len(), 1);
    assert_eq!(priced["price"]["source"], "preregistered");
    assert_eq!(priced["price"]["amount"], "0");
    assert!(priced.get("holding_quantity").is_none());

    let mut revision = mutation(&priced);
    revision["name"] = json!("가상 %_ 수정 재료");
    revision["unit"] = json!("kg");
    revision["kind"] = json!("prepared");
    revision["description"] = json!("가상 수정 설명");
    let response = send(
        &app,
        "POST",
        &format!("/api/materials/{id}/versions"),
        Some(&revision),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let revised = detail(&app, id).await;
    assert_eq!(revised["id"], first["id"]);
    assert_eq!(revised["unit"], "kg");
    assert_eq!(revised["canonical_unit"], "g");
    let versions = revised["versions"].as_array().unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0]["name"], "가상 %_ 재료");
    assert_eq!(versions[0]["unit"], "g");

    let found = value(send(&app, "GET", "/api/materials?query=%20%25_%20", None).await).await;
    assert_eq!(found.as_array().unwrap().len(), 1);
    assert_eq!(found[0]["id"], first["id"]);
    let archive = mutation(&revised);
    let response = send(
        &app,
        "POST",
        &format!("/api/materials/{id}/archive"),
        Some(&archive),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let active = value(send(&app, "GET", "/api/materials?query=%25_", None).await).await;
    assert!(active.as_array().unwrap().is_empty());
    let archived = value(
        send(
            &app,
            "GET",
            "/api/materials?archive=archived&query=%25_",
            None,
        )
        .await,
    )
    .await;
    assert_eq!(archived.as_array().unwrap().len(), 1);
    let preserved = detail(&app, id).await;
    assert_eq!(preserved["archived"], true);
    assert_eq!(preserved["prices"].as_array().unwrap().len(), 1);
    assert_eq!(preserved["versions"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn extended_inputs_do_not_fall_back_to_legacy_or_accept_numeric_json() {
    let (_directory, app) = app();
    let legacy = json!({"request_id":uuid::Uuid::new_v4(),"name":"가상 옛 입력","unit":"g"});
    let response = send(&app, "POST", "/api/materials", Some(&legacy)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let created = value(response).await;
    let mut extended = legacy.clone();
    extended["request_id"] = json!(uuid::Uuid::new_v4());
    extended["kind"] = json!("raw");
    let response = send(&app, "POST", "/api/materials", Some(&extended)).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(value(response).await["code"], "validation");

    let id = created["id"].as_str().unwrap();
    let current = detail(&app, id).await;
    let mut input = mutation(&current);
    input["quantity"] = json!("100");
    input["unit"] = json!("g");
    input["amount"] = json!(10);
    let response = send(
        &app,
        "POST",
        &format!("/api/materials/{id}/prices"),
        Some(&input),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(value(response).await["code"], "validation");
    assert!(
        detail(&app, id).await["prices"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let rows = value(send(&app, "GET", "/api/materials", None).await).await;
    assert_eq!(rows.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn catalog_routes_keep_host_origin_session_and_body_size_guards() {
    let (_directory, app) = app();
    let id = uuid::Uuid::new_v4();
    for (method, path) in [
        ("GET", "/api/state".to_string()),
        ("POST", format!("/api/materials/{id}/versions")),
        ("POST", format!("/api/materials/{id}/prices")),
        ("POST", format!("/api/materials/{id}/archive")),
    ] {
        for (host, origin, cookie, expected) in [
            (
                "evil.test:4321",
                "http://127.0.0.1:4321",
                true,
                StatusCode::FORBIDDEN,
            ),
            (
                "127.0.0.1:4321",
                "https://evil.test",
                true,
                StatusCode::FORBIDDEN,
            ),
            (
                "127.0.0.1:4321",
                "http://127.0.0.1:4321",
                false,
                StatusCode::UNAUTHORIZED,
            ),
        ] {
            let mut request = Request::builder()
                .method(method)
                .uri(&path)
                .header("host", host)
                .header("origin", origin)
                .header("content-type", "application/json");
            if cookie {
                request = request.header("cookie", "hansang_4321=catalog-test-session");
            }
            let response = app
                .clone()
                .oneshot(request.body(Body::from("{}")).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), expected, "{method} {path}");
        }
    }
    let mut large = new_material(&state(&app).await, "가상 크기 제한");
    large["description"] = json!("x".repeat(17 * 1024));
    let response = send(&app, "POST", "/api/materials", Some(&large)).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let rows = value(send(&app, "GET", "/api/materials", None).await).await;
    assert!(rows.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn journal_failure_cannot_become_success_or_allow_another_mutation() {
    let (directory, app) = app();
    let initial = state(&app).await;
    let input = new_material(&initial, "가상 기록 복구");
    let snapshots = directory.path().join("snapshots");
    std::fs::remove_dir(&snapshots).unwrap();
    std::fs::write(&snapshots, b"blocked").unwrap();
    for _ in 0..2 {
        let response = send(&app, "POST", "/api/materials", Some(&input)).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(value(response).await["code"], "unavailable");
    }
    let next = new_material(&state(&app).await, "가상 차단할 후속 입력");
    let response = send(&app, "POST", "/api/materials", Some(&next)).await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(value(response).await["code"], "unavailable");
    let rows = value(send(&app, "GET", "/api/materials", None).await).await;
    assert_eq!(rows.as_array().unwrap().len(), 1);

    std::fs::remove_file(&snapshots).unwrap();
    std::fs::create_dir(&snapshots).unwrap();
    let response = send(&app, "POST", "/api/materials", Some(&input)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(value(response).await["id"], rows[0]["id"]);
    let files = std::fs::read_dir(&snapshots)
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(files.len(), 1);
    let first_bytes = std::fs::read(files[0].path()).unwrap();
    let response = send(&app, "POST", "/api/materials", Some(&input)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(std::fs::read(files[0].path()).unwrap(), first_bytes);
}
