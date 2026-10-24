use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use sunchang_hansang::{
    lifecycle::Policy,
    server::{AppState, router},
    store::Store,
};
use tower::ServiceExt;

fn app() -> (tempfile::TempDir, axum::Router, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let state = AppState::new(
        Store::open(dir.path()).unwrap(),
        "127.0.0.1:4321".into(),
        "test-secret".into(),
        Policy::default(),
    );
    (dir, router(state.clone()), state)
}
fn req(method: &str, path: &str, body: &str) -> axum::http::request::Builder {
    let _ = body;
    Request::builder()
        .method(method)
        .uri(path)
        .header("host", "127.0.0.1:4321")
}
#[tokio::test]
async fn untrusted_hosts_origins_and_unauthenticated_reads_are_rejected() {
    let (_dir, app, _) = app();
    for request in [
        Request::builder()
            .uri("/api/health")
            .header("host", "evil.test:4321")
            .body(Body::empty())
            .unwrap(),
        req("GET", "/api/materials", "")
            .body(Body::empty())
            .unwrap(),
        req("POST", "/api/session", "")
            .header("origin", "https://evil.test")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"token":"test-secret"}"#))
            .unwrap(),
    ] {
        let result = app.clone().oneshot(request).await.unwrap();
        assert!(matches!(
            result.status(),
            StatusCode::FORBIDDEN | StatusCode::UNAUTHORIZED
        ));
    }
}
#[tokio::test]
async fn authenticated_register_list_and_retries_use_real_store() {
    let (_dir, app, _) = app();
    let response = app
        .clone()
        .oneshot(
            req("POST", "/api/session", "")
                .header("origin", "http://127.0.0.1:4321")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"token":"test-secret"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let cookie = response.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string();
    let body =
        r#"{"request_id":"6da76a80-21f3-422a-a28a-e7f4adb47e9c","name":"가상 재료","unit":"g"}"#;
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(
                req("POST", "/api/materials", body)
                    .header("origin", "http://127.0.0.1:4321")
                    .header("cookie", &cookie)
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }
    let response = app
        .oneshot(
            req("GET", "/api/materials", "")
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let rows: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["name"], "가상 재료");
}
#[tokio::test]
async fn missing_api_and_assets_do_not_become_spa_successes() {
    let (_dir, app, _) = app();
    for path in [
        "/api/missing",
        "/_nuxt/missing.js",
        "/photos/missing",
        "/db/app.sqlite3",
    ] {
        let response = app
            .clone()
            .oneshot(
                req("GET", path, "")
                    .header("cookie", "hansang_4321=test-secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        assert!(
            !response
                .headers()
                .get("content-type")
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("text/html")
        );
    }
}

#[tokio::test]
async fn session_and_writes_require_exact_origin_even_with_a_valid_cookie() {
    let (_dir, app, _) = app();
    for origin in [
        None,
        Some("null"),
        Some("http://127.0.0.1:4322"),
        Some("https://evil.test"),
    ] {
        for path in ["/api/session", "/api/materials"] {
            let mut request = req("POST", path, "")
                .header("cookie", "hansang_4321=test-secret")
                .header("content-type", "application/json");
            if let Some(origin) = origin {
                request = request.header("origin", origin);
            }
            let response = app
                .clone()
                .oneshot(
                    request
                        .body(Body::from(r#"{"token":"test-secret"}"#))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::FORBIDDEN,
                "{path}: {origin:?}"
            );
        }
    }
}

#[tokio::test]
async fn launch_token_is_required_and_session_cookie_is_not_readable_by_scripts() {
    let (_dir, app, _) = app();
    for (token, expected) in [
        ("", StatusCode::UNAUTHORIZED),
        ("old-token", StatusCode::UNAUTHORIZED),
        ("test-secret", StatusCode::NO_CONTENT),
    ] {
        let response = app
            .clone()
            .oneshot(
                req("POST", "/api/session", "")
                    .header("origin", "http://127.0.0.1:4321")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::json!({"token": token}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        assert!(
            !response
                .headers()
                .contains_key("access-control-allow-origin")
        );
        if expected == StatusCode::NO_CONTENT {
            let cookie = response.headers()["set-cookie"].to_str().unwrap();
            assert!(cookie.contains("HttpOnly"));
            assert!(cookie.contains("SameSite=Strict"));
            assert!(!cookie.contains("Max-Age") && !cookie.contains("Expires"));
        }
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(!String::from_utf8_lossy(&body).contains("test-secret"));
    }
}

#[tokio::test]
async fn oversized_body_is_rejected_and_sql_syntax_is_stored_as_text() {
    let (_dir, app, _) = app();
    let send = |body: String| {
        req("POST", "/api/materials", "")
            .header("origin", "http://127.0.0.1:4321")
            .header("cookie", "hansang_4321=test-secret")
            .header("content-type", "application/json")
            .body(Body::from(body))
            .unwrap()
    };
    let oversized = serde_json::json!({"request_id": uuid::Uuid::new_v4(), "name": "x".repeat(17 * 1024), "unit": "g"});
    assert_eq!(
        app.clone()
            .oneshot(send(oversized.to_string()))
            .await
            .unwrap()
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let name = "가상 '); DROP TABLE materials; --";
    let valid = serde_json::json!({"request_id": uuid::Uuid::new_v4(), "name": name, "unit": "g"});
    assert_eq!(
        app.clone()
            .oneshot(send(valid.to_string()))
            .await
            .unwrap()
            .status(),
        StatusCode::CREATED
    );
    let response = app
        .oneshot(
            req("GET", "/api/materials", "")
                .header("cookie", "hansang_4321=test-secret")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let rows: serde_json::Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["name"], name);
}

#[tokio::test]
async fn traversal_and_private_paths_never_serve_workspace_files() {
    let (_dir, app, _) = app();
    for path in [
        "/../Cargo.toml",
        "/%2e%2e/Cargo.toml",
        "/..%5cCargo.toml",
        "/.env",
        "/db/snapshots/00000000000000000001.json",
        "/.git/config",
    ] {
        let response = app
            .clone()
            .oneshot(req("GET", path, "").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
    }
}
