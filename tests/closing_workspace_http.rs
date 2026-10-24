use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use http_body_util::BodyExt;
use serde_json::Value;
use sunchang_hansang::{
    lifecycle::Policy,
    server::{AppState, router},
    store::Store,
};
use tower::ServiceExt;

const HOST: &str = "127.0.0.1:4321";
const ORIGIN: &str = "http://127.0.0.1:4321";
const COOKIE: &str = "hansang_4321=workspace-test-session";

fn app() -> (tempfile::TempDir, Router) {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::new(
        Store::open(directory.path()).unwrap(),
        HOST.into(),
        "workspace-test-session".into(),
        Policy::default(),
    );
    (directory, router(state))
}

async fn send(app: &Router, method: &str, path: &str) -> Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("host", HOST)
                .header("cookie", COOKIE)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn state(app: &Router) -> Value {
    let response = send(app, "GET", "/api/state").await;
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

#[tokio::test]
async fn workspace_get_and_head_serve_the_app_shell_without_creating_records() {
    let (_directory, app) = app();
    let before = state(&app).await;
    let home = send(&app, "GET", "/")
        .await
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    assert!(!home.is_empty());
    for path in ["/workspace", "/workspace?selected=local-input"] {
        for method in ["GET", "HEAD"] {
            let response = send(&app, method, path).await;
            assert_eq!(response.status(), StatusCode::OK, "{method} {path}");
            assert!(
                response.headers()["content-type"]
                    .to_str()
                    .unwrap()
                    .starts_with("text/html")
            );
            assert_eq!(response.headers()["cache-control"], "no-store");
            assert_eq!(response.headers()["x-content-type-options"], "nosniff");
            let body = response.into_body().collect().await.unwrap().to_bytes();
            if method == "HEAD" {
                assert!(body.is_empty());
            } else {
                assert_eq!(body, home);
            }
        }
    }
    assert_eq!(state(&app).await, before);
}

#[tokio::test]
async fn workspace_is_an_exact_spa_path_not_a_new_api_or_nested_route() {
    let (_directory, app) = app();
    for path in [
        "/workspace/",
        "/workspace/unlisted",
        "/workspace/00000000-0000-4000-8000-000000000001",
        "/workspaces",
        "/api/workspace",
        "/api/workspace/confirm",
    ] {
        assert_eq!(
            send(&app, "GET", path).await.status(),
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
    for method in ["POST", "PUT", "DELETE"] {
        assert_eq!(
            send(&app, method, "/workspace").await.status(),
            StatusCode::METHOD_NOT_ALLOWED,
            "{method}"
        );
    }
}

#[tokio::test]
async fn workspace_keeps_bootstrap_and_existing_api_security_boundaries() {
    let (_directory, app) = app();
    let before = state(&app).await;
    for (path, host, origin, cookie, expected) in [
        ("/workspace", HOST, None, false, StatusCode::OK),
        (
            "/workspace",
            "foreign.example",
            None,
            true,
            StatusCode::FORBIDDEN,
        ),
        (
            "/workspace",
            HOST,
            Some("https://foreign.example"),
            true,
            StatusCode::FORBIDDEN,
        ),
        ("/api/business", HOST, None, false, StatusCode::UNAUTHORIZED),
        (
            "/api/business",
            HOST,
            Some("https://foreign.example"),
            true,
            StatusCode::FORBIDDEN,
        ),
        ("/api/business", HOST, None, true, StatusCode::OK),
    ] {
        let mut request = Request::builder().uri(path).header("host", host);
        if let Some(origin) = origin {
            request = request.header("origin", origin);
        }
        if cookie {
            request = request.header("cookie", COOKIE);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected, "{path} {host} {origin:?}");
    }
    for (origin, expected) in [
        (None, StatusCode::FORBIDDEN),
        (Some("https://foreign.example"), StatusCode::FORBIDDEN),
        (Some(ORIGIN), StatusCode::PAYLOAD_TOO_LARGE),
    ] {
        let mut request = Request::builder()
            .method("POST")
            .uri("/api/business/drafts")
            .header("host", HOST)
            .header("cookie", COOKIE)
            .header("content-type", "application/json");
        if let Some(origin) = origin {
            request = request.header("origin", origin);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::from("x".repeat(16 * 1024 + 1))).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected, "{origin:?}");
    }
    assert_eq!(state(&app).await, before);
}
