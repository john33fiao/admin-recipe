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
