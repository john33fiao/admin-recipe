use axum::{
    Router,
    body::{Body, Bytes},
    http::{Request, StatusCode},
    response::Response,
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use sunchang_hansang::{
    lifecycle::Policy,
    server::{AppState, router},
    store::Store,
};
use tower::ServiceExt;

const HOST: &str = "127.0.0.1:4321";
const ORIGIN: &str = "http://127.0.0.1:4321";
const COOKIE: &str = "hansang_4321=restore-test";
fn app() -> (tempfile::TempDir, Router) {
    let root = tempfile::tempdir().unwrap();
    let state = AppState::new(
        Store::open(root.path()).unwrap(),
        HOST.into(),
        "restore-test".into(),
        Policy::default(),
    );
    (root, router(state))
}
async fn raw(app: &Router, method: &str, path: &str, bytes: Vec<u8>) -> Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("host", HOST)
                .header("origin", ORIGIN)
                .header("cookie", COOKIE)
                .header("content-type", "application/json")
                .body(Body::from(bytes))
                .unwrap(),
        )
        .await
        .unwrap()
}
async fn send(app: &Router, method: &str, path: &str, input: Option<&Value>) -> Response {
    raw(
        app,
        method,
        path,
        input
            .map(|v| v.to_string().into_bytes())
            .unwrap_or_default(),
    )
    .await
}
async fn body(response: Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| panic!("expected JSON: {}", String::from_utf8_lossy(&bytes)))
}
async fn get(app: &Router, path: &str) -> Value {
    let response = send(app, "GET", path, None).await;
    let status = response.status();
    let value = body(response).await;
    assert_eq!(status, StatusCode::OK, "{path}: {value}");
    value
}
async fn created(app: &Router, path: &str, input: &Value) -> Value {
    let response = send(app, "POST", path, Some(input)).await;
    let status = response.status();
    let value = body(response).await;
    assert_eq!(status, StatusCode::CREATED, "{path}: {value}");
    value
}
fn meta(state: &Value) -> Value {
    json!({"request_id":uuid::Uuid::new_v4().to_string(),"epoch":state["epoch"],"expected_revision":state["revision"]})
}
async fn material_input(app: &Router, name: &str) -> Value {
    let mut input = meta(&get(app, "/api/state").await);
    input["name"] = json!(name);
    input["unit"] = json!("g");
    input["kind"] = json!("raw");
    input["description"] = json!("");
    input
}
fn target_input(target: &Value) -> Value {
    json!({"target_id":target["target_id"],"expected_sha256":target["sha256"]})
}
fn disk(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, directory: &Path, result: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, result);
            } else if path != root.join(".writer.lock") {
                result.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}

#[tokio::test]
async fn full_restore_and_original_request_status_are_distinct_and_retry_is_not_a_second_restore() {
    let (root, app) = app();
    let first_input = material_input(&app, "가상 복원 A").await;
    let first = created(&app, "/api/materials", &first_input).await;
    let target = get(&app, "/api/restore-targets").await["rows"][0].clone();
    let old_bytes = std::fs::read(root.path().join("snapshots/00000000000000000001.json")).unwrap();
    let second_input = material_input(&app, "가상 복원 B").await;
    created(&app, "/api/materials", &second_input).await;
    let before = get(&app, "/api/state").await;
    let old_unsent = material_input(&app, "가상 미전송 C").await;
    let preview = send(
        &app,
        "POST",
        "/api/restore/preview",
        Some(&target_input(&target)),
    )
    .await;
    assert_eq!(preview.status(), StatusCode::OK);
    let preview = body(preview).await;
    assert_eq!(preview["state"], before);
    assert_eq!(preview["can_restore"], true);
    assert_eq!(preview["before"]["areas"].as_array().unwrap().len(), 15);
    let mut command = target_input(&target);
    command["meta"] = meta(&before);
    let restored = created(&app, "/api/restores", &command).await;
    assert_ne!(restored["state"]["epoch"], before["epoch"]);
    assert_eq!(restored["previous_target"]["source_kind"], "before_restore");
    let current = get(&app, "/api/materials").await;
    assert_eq!(current.as_array().unwrap().len(), 1);
    assert_eq!(current[0]["id"], first["id"]);
    assert_eq!(created(&app, "/api/restores", &command).await, restored);
    for (input, expected) in [
        (&second_input, "already_processed"),
        (&old_unsent, "not_processed"),
    ] {
        let response = send(&app, "POST", "/api/materials", Some(input)).await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let error = body(response).await;
        assert_eq!(error["code"], "stale_generation");
        assert_eq!(error["request_status"], expected);
        assert_eq!(error["state"], restored["state"]);
    }
    let mut conflict = second_input.clone();
    conflict["name"] = json!("다른 원본문");
    let response = send(&app, "POST", "/api/materials", Some(&conflict)).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(body(response).await["code"], "request_conflict");
    let new_input = material_input(&app, "가상 새 세대").await;
    created(&app, "/api/materials", &new_input).await;
    assert_eq!(created(&app, "/api/restores", &command).await, restored);
    assert_eq!(
        std::fs::read(root.path().join("snapshots/00000000000000000001.json")).unwrap(),
        old_bytes
    );
}

#[tokio::test]
async fn raw_import_preserves_bytes_and_hash_but_unregistered_archives_are_not_targets() {
    use sunchang_hansang::restore_files::{publish_restore_import, verify_restore_bytes};
    let (root, app) = app();
    let input = material_input(&app, "가상 원문").await;
    created(&app, "/api/materials", &input).await;
    let state = get(&app, "/api/state").await;
    let bytes = std::fs::read(root.path().join("snapshots/00000000000000000001.json")).unwrap();
    let response = raw(&app, "POST", "/api/restore-targets/import", bytes.clone()).await;
    let status = response.status();
    let target = body(response).await;
    assert_eq!(status, StatusCode::CREATED, "{target}");
    let response = raw(&app, "POST", "/api/restore-targets/import", bytes.clone()).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(body(response).await, target);
    assert_eq!(get(&app, "/api/state").await, state);
    assert_eq!(
        std::fs::read(
            root.path()
                .join("imports")
                .join(format!("{}.json", target["sha256"].as_str().unwrap()))
        )
        .unwrap(),
        bytes
    );
    let orphan = verify_restore_bytes(b"{\"synthetic_orphan\":true}".to_vec()).unwrap();
    publish_restore_import(root.path(), &orphan).unwrap();
    let page = get(&app, "/api/restore-targets?source=import").await;
    assert_eq!(page["rows"].as_array().unwrap().len(), 1);
    let response = send(&app, "POST", "/api/restore/preview", Some(&json!({"target_id":format!("import:{}",orphan.sha256()),"expected_sha256":orphan.sha256()}))).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        send(
            &app,
            "GET",
            &format!("/imports/{}.json", orphan.sha256()),
            None
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    // A registered archive that is subsequently damaged is unavailable, not a
    // new client validation error; internal filesystem/SQL details stay private.
    std::fs::write(
        root.path()
            .join("imports")
            .join(format!("{}.json", target["sha256"].as_str().unwrap())),
        b"damaged",
    )
    .unwrap();
    let response = send(
        &app,
        "POST",
        "/api/restore/preview",
        Some(&target_input(&target)),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let error = body(response).await;
    assert_eq!(error["code"], "unavailable");
    assert!(
        !error
            .to_string()
            .contains(&root.path().to_string_lossy().to_string())
    );
    assert!(error["error"].as_str().unwrap().contains("같은 요청"));
}

#[tokio::test]
async fn target_query_and_preview_are_strict_and_leave_database_and_journals_unchanged() {
    let (root, app) = app();
    let input = material_input(&app, "가상 조회").await;
    created(&app, "/api/materials", &input).await;
    let before = disk(root.path());
    for query in [
        "source=all",
        "source=",
        "cursor=",
        "cursor=01",
        "cursor=-1",
        "cursor=0",
        "source=import&cursor=1",
        "source=journal&source=import",
        "cursor=1&cursor=2",
        "path=x",
        "cursor=%GG",
        "cursor=%FF",
    ] {
        let response = send(&app, "GET", &format!("/api/restore-targets?{query}"), None).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{query}");
    }
    assert_eq!(
        send(
            &app,
            "GET",
            &format!("/api/restore-targets?x={}", "x".repeat(4096)),
            None
        )
        .await
        .status(),
        StatusCode::URI_TOO_LONG
    );
    for input in [
        json!({"target_id":"../outside","expected_sha256":"a".repeat(64)}),
        json!({"target_id":"journal:1","expected_sha256":"A".repeat(64)}),
        json!({"target_id":"journal:1","expected_sha256":"a".repeat(64),"path":"ignored"}),
    ] {
        assert_eq!(
            send(&app, "POST", "/api/restore/preview", Some(&input))
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(disk(root.path()), before);
}

#[tokio::test]
async fn malformed_import_and_small_body_limits_do_not_register_or_mutate_anything() {
    let (root, app) = app();
    let before = disk(root.path());
    for bytes in [
        br#"{"format_version":1,"format_version":2}"#.to_vec(),
        br#"{"outer":[{"x":1,"\u0078":2}]}"#.to_vec(),
        b"{} {}".to_vec(),
        b"\xff".to_vec(),
        vec![],
        br#"{"format_version":999}"#.to_vec(),
        br#"{}"#.to_vec(),
    ] {
        assert_eq!(
            raw(&app, "POST", "/api/restore-targets/import", bytes)
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        raw(
            &app,
            "POST",
            "/api/restore/preview",
            vec![b' '; 16 * 1024 + 1]
        )
        .await
        .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(
        raw(
            &app,
            "POST",
            "/api/restores",
            br#"{"meta":{},"meta":{}}"#.to_vec()
        )
        .await
        .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(disk(root.path()), before);
}

#[tokio::test]
async fn host_origin_session_and_oversized_header_reject_before_polling_import_body() {
    let (_root, app) = app();
    for (host, origin, cookie, length, status) in [
        (
            "external.invalid",
            ORIGIN,
            Some(COOKIE),
            None,
            StatusCode::FORBIDDEN,
        ),
        (
            HOST,
            "http://external.invalid",
            Some(COOKIE),
            None,
            StatusCode::FORBIDDEN,
        ),
        (HOST, ORIGIN, None, None, StatusCode::UNAUTHORIZED),
        (
            HOST,
            ORIGIN,
            Some(COOKIE),
            Some("67108865"),
            StatusCode::PAYLOAD_TOO_LARGE,
        ),
    ] {
        let polled = Arc::new(AtomicUsize::new(0));
        let count = polled.clone();
        let stream = futures_util::stream::poll_fn(move |_| {
            count.fetch_add(1, Ordering::SeqCst);
            std::task::Poll::Ready(Some(Ok::<Bytes, std::io::Error>(Bytes::from_static(b"{}"))))
        });
        let mut request = Request::builder()
            .method("POST")
            .uri("/api/restore-targets/import")
            .header("host", host)
            .header("origin", origin)
            .header("content-type", "application/json");
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        if let Some(length) = length {
            request = request.header("content-length", length);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::from_stream(stream)).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(polled.load(Ordering::SeqCst), 0);
    }
    let response = send(&app, "HEAD", "/api/restore-targets", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .is_empty()
    );
    assert_eq!(
        send(&app, "GET", "/restore", None).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        send(&app, "GET", "/restore/not-a-route", None)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&app, "GET", "/api/restores", None).await.status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
}

#[tokio::test]
async fn chunked_limit_and_compressed_or_wrong_mime_are_explicitly_rejected() {
    let (_root, app) = app();
    for (mime, encoding) in [("text/plain", None), ("application/json", Some("gzip"))] {
        let mut r = Request::builder()
            .method("POST")
            .uri("/api/restore-targets/import")
            .header("host", HOST)
            .header("origin", ORIGIN)
            .header("cookie", COOKIE)
            .header("content-type", mime);
        if let Some(encoding) = encoding {
            r = r.header("content-encoding", encoding);
        }
        assert_eq!(
            app.clone()
                .oneshot(r.body(Body::from("{}")).unwrap())
                .await
                .unwrap()
                .status(),
            StatusCode::UNSUPPORTED_MEDIA_TYPE
        );
    }
    let chunk = Bytes::from(vec![b' '; 1024 * 1024]);
    let chunks = (0..65)
        .map(|_| Ok::<_, std::io::Error>(chunk.clone()))
        .collect::<Vec<_>>();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/restore-targets/import")
                .header("host", HOST)
                .header("origin", ORIGIN)
                .header("cookie", COOKIE)
                .header("content-type", "application/json")
                .body(Body::from_stream(futures_util::stream::iter(chunks)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/restore-targets/import")
                .header("host", HOST)
                .header("origin", ORIGIN)
                .header("cookie", COOKIE)
                .header("content-type", "application/json")
                .header("content-length", "3")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(get(&app, "/api/state").await["revision"], "0");
}

#[tokio::test(start_paused = true)]
async fn one_import_admission_and_timeout_release_leave_no_partial_target() {
    let (_root, app) = app();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let mut entered_tx = Some(entered_tx);
    let waiting = futures_util::stream::poll_fn(move |_| {
        if let Some(tx) = entered_tx.take() {
            let _ = tx.send(());
        }
        std::task::Poll::Pending::<Option<Result<Bytes, std::io::Error>>>
    });
    let first = tokio::spawn(
        app.clone().oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/restore-targets/import")
                .header("host", HOST)
                .header("origin", ORIGIN)
                .header("cookie", COOKIE)
                .header("content-type", "application/json")
                .body(Body::from_stream(waiting))
                .unwrap(),
        ),
    );
    entered_rx.await.unwrap();
    let polled = Arc::new(AtomicUsize::new(0));
    let count = polled.clone();
    let second_body = futures_util::stream::poll_fn(move |_| {
        count.fetch_add(1, Ordering::SeqCst);
        std::task::Poll::Ready(Some(Ok::<Bytes, std::io::Error>(Bytes::from_static(b"{}"))))
    });
    let second = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/restore-targets/import")
                .header("host", HOST)
                .header("origin", ORIGIN)
                .header("cookie", COOKIE)
                .header("content-type", "application/json")
                .body(Body::from_stream(second_body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(polled.load(Ordering::SeqCst), 0);
    tokio::time::advance(std::time::Duration::from_secs(61)).await;
    assert_eq!(
        first.await.unwrap().unwrap().status(),
        StatusCode::REQUEST_TIMEOUT
    );
    assert_eq!(
        raw(&app, "POST", "/api/restore-targets/import", b"{}".to_vec())
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert!(
        get(&app, "/api/restore-targets?source=import").await["rows"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(get(&app, "/api/state").await["revision"], "0");
}
