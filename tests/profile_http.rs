use axum::{
    Router,
    body::{Body, Bytes},
    http::{Request, StatusCode},
    response::Response,
};
use http_body_util::BodyExt;
use image::{ExtendedColorType, ImageEncoder};
use serde_json::{Value, json};
use sunchang_hansang::{
    lifecycle::Policy,
    server::{AppState, router},
    store::Store,
};
use tower::ServiceExt;

const HOST: &str = "127.0.0.1:4321";
const ORIGIN: &str = "http://127.0.0.1:4321";
const COOKIE: &str = "hansang_4321=photo-test-session";
fn app() -> (tempfile::TempDir, Router, AppState) {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::new(
        Store::open(directory.path()).unwrap(),
        HOST.into(),
        "photo-test-session".into(),
        Policy::default(),
    );
    (directory, router(state.clone()), state)
}
fn png() -> Vec<u8> {
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(&[13, 31, 59, 255], 1, 1, ExtendedColorType::Rgba8)
        .unwrap();
    bytes
}
async fn body(response: Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
async fn request(app: &Router, method: &str, path: &str, input: Option<&Value>) -> Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("host", HOST)
        .header("origin", ORIGIN)
        .header("cookie", COOKIE);
    let data = if let Some(input) = input {
        builder = builder.header("content-type", "application/json");
        Body::from(input.to_string())
    } else {
        Body::empty()
    };
    app.clone()
        .oneshot(builder.body(data).unwrap())
        .await
        .unwrap()
}
async fn get(app: &Router, path: &str) -> Value {
    let response = request(app, "GET", path, None).await;
    assert_eq!(response.status(), StatusCode::OK, "{path}");
    body(response).await
}
fn meta(state: &Value) -> Value {
    json!({"request_id":uuid::Uuid::new_v4().to_string(),"epoch":state["epoch"],"expected_revision":state["revision"]})
}
async fn material(app: &Router) -> Value {
    let mut input = meta(&get(app, "/api/state").await);
    input["name"] = json!("가상 HTTP 사진 재료");
    input["unit"] = json!("g");
    input["kind"] = json!("raw");
    input["description"] = json!("");
    let response = request(app, "POST", "/api/materials", Some(&input)).await;
    assert_eq!(response.status(), StatusCode::CREATED);
    body(response).await
}
async fn input(app: &Router, item: &Value, upload: bool) -> Value {
    json!({"meta":meta(&get(app,"/api/state").await),"expected_material_version_id":item["version_id"],"expected_profile_version_id":null,
        "photos":if upload { json!([{"source":"upload","caption":"가상 사진"}]) } else {json!([])},
        "primary_position":if upload {json!(0)} else {Value::Null},"suppliers":[]})
}
fn part(
    output: &mut Vec<u8>,
    boundary: &str,
    name: &str,
    filename: Option<&str>,
    mime: Option<&str>,
    bytes: &[u8],
) {
    output
        .extend(format!("--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"").bytes());
    if let Some(filename) = filename {
        output.extend(format!("; filename=\"{filename}\"").bytes());
    }
    output.extend(b"\r\n");
    if let Some(mime) = mime {
        output.extend(format!("Content-Type: {mime}\r\n").bytes());
    }
    output.extend(b"\r\n");
    output.extend(bytes);
    output.extend(b"\r\n");
}
fn multipart(
    input: &Value,
    filename: &str,
    photo: Option<&[u8]>,
    reverse: bool,
    boundary: &str,
) -> Vec<u8> {
    let mut bytes = Vec::new();
    if reverse && let Some(photo) = photo {
        part(
            &mut bytes,
            boundary,
            "photo",
            Some(filename),
            Some("image/png"),
            photo,
        );
    }
    part(
        &mut bytes,
        boundary,
        "metadata",
        None,
        Some("application/json"),
        input.to_string().as_bytes(),
    );
    if !reverse && let Some(photo) = photo {
        part(
            &mut bytes,
            boundary,
            "photo",
            Some(filename),
            Some("image/png"),
            photo,
        );
    }
    bytes.extend(format!("--{boundary}--\r\n").bytes());
    bytes
}
async fn post(app: &Router, path: &str, bytes: Vec<u8>, boundary: &str) -> Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("host", HOST)
                .header("origin", ORIGIN)
                .header("cookie", COOKIE)
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(bytes))
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn profile_upload_history_and_bytes_are_real_and_retry_ignores_transport_names() {
    let (_directory, app, _state) = app();
    let item = material(&app).await;
    let path = format!("/api/materials/{}/profiles", item["id"].as_str().unwrap());
    assert!(
        get(
            &app,
            &format!("/api/materials/{}/profile", item["id"].as_str().unwrap())
        )
        .await["version"]
            .is_null()
    );
    let input = input(&app, &item, true).await;
    let bytes = png();
    let first = post(
        &app,
        &path,
        multipart(&input, "first.png", Some(&bytes), false, "one"),
        "one",
    )
    .await;
    let status = first.status();
    let first = body(first).await;
    assert_eq!(status, StatusCode::CREATED, "{first}");
    assert_eq!(first["version"]["photos"][0]["caption"], "가상 사진");
    let retry = post(
        &app,
        &path,
        multipart(&input, "renamed.PNG", Some(&bytes), true, "other"),
        "other",
    )
    .await;
    assert_eq!(retry.status(), StatusCode::CREATED);
    assert_eq!(body(retry).await, first);
    let photo = &first["version"]["photos"][0]["asset"];
    let asset_path = format!("/api/photo-assets/{}", photo["sha256"].as_str().unwrap());
    let response = request(&app, "GET", &asset_path, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "image/png");
    assert_eq!(
        response.headers()["content-length"],
        bytes.len().to_string()
    );
    assert_eq!(
        response.headers()["cross-origin-resource-policy"],
        "same-origin"
    );
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .as_ref(),
        bytes
    );
    let head = request(&app, "HEAD", &asset_path, None).await;
    assert_eq!(head.status(), StatusCode::OK);
    assert_eq!(head.headers()["content-length"], bytes.len().to_string());
    assert!(
        head.into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .is_empty()
    );
    let history = get(&app, &path).await;
    assert_eq!(history["versions"].as_array().unwrap().len(), 1);
    let version = get(
        &app,
        &format!("{path}/{}", first["version"]["id"].as_str().unwrap()),
    )
    .await;
    assert_eq!(version["version"], first["version"]);
    let mut changed = input.clone();
    changed["photos"][0]["caption"] = json!("changed");
    let conflict = post(
        &app,
        &path,
        multipart(&changed, "first.png", Some(&bytes), false, "one"),
        "one",
    )
    .await;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    assert_eq!(body(conflict).await["code"], "request_conflict");
    let mut stale = input;
    stale["meta"]["request_id"] = json!(uuid::Uuid::new_v4().to_string());
    let conflict = post(
        &app,
        &path,
        multipart(&stale, "first.png", Some(&bytes), false, "one"),
        "one",
    )
    .await;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    assert_eq!(body(conflict).await["code"], "stale_state");
}

#[tokio::test]
async fn multipart_limits_are_local_and_unknown_duplicate_or_mismatched_parts_have_no_effect() {
    let (_directory, app, _state) = app();
    let item = material(&app).await;
    let path = format!("/api/materials/{}/profiles", item["id"].as_str().unwrap());
    let input = input(&app, &item, false).await;
    let before = get(&app, "/api/state").await;
    let mut cases = Vec::new();
    for name in ["metadata", "unknown"] {
        let mut bytes = multipart(&input, "a.png", None, false, "x");
        bytes.truncate(bytes.len() - 7);
        part(&mut bytes, "x", name, None, None, b"{}");
        bytes.extend(b"--x--\r\n");
        cases.push((bytes, StatusCode::BAD_REQUEST));
    }
    cases.push((
        multipart(&input, "a.png", Some(&png()), false, "x"),
        StatusCode::BAD_REQUEST,
    ));
    let mut needs_photo = input.clone();
    needs_photo["photos"] = json!([{"source":"upload","caption":""}]);
    needs_photo["primary_position"] = json!(0);
    cases.push((
        multipart(&needs_photo, "a.png", None, false, "x"),
        StatusCode::BAD_REQUEST,
    ));
    cases.push((
        multipart(
            &needs_photo,
            "a.png",
            Some(&vec![0; 10 * 1024 * 1024 + 1]),
            false,
            "x",
        ),
        StatusCode::PAYLOAD_TOO_LARGE,
    ));
    let mut too_much = Vec::new();
    part(
        &mut too_much,
        "x",
        "metadata",
        None,
        None,
        &vec![b' '; 256 * 1024 + 1],
    );
    too_much.extend(b"--x--\r\n");
    cases.push((too_much, StatusCode::PAYLOAD_TOO_LARGE));
    let mut oversized_envelope = multipart(&input, "a.png", None, false, "x");
    oversized_envelope.resize(11 * 1024 * 1024 + 1, b' ');
    cases.push((oversized_envelope, StatusCode::PAYLOAD_TOO_LARGE));
    for (bytes, status) in cases {
        let response = post(&app, &path, bytes, "x").await;
        let actual = response.status();
        let detail = body(response).await;
        assert_eq!(actual, status, "{detail}");
        assert_eq!(detail["code"], "validation");
    }
    assert_eq!(get(&app, "/api/state").await, before);
    // A valid >16KiB metadata request fits this route's 256KiB limit.
    let mut large = input;
    large["suppliers"] = json!(
        (0..20)
            .map(|_| json!({"url":null,"description":"가".repeat(500)}))
            .collect::<Vec<_>>()
    );
    let response = post(
        &app,
        &path,
        multipart(&large, "a.png", None, false, "x"),
        "x",
    )
    .await;
    let status = response.status();
    let detail = body(response).await;
    assert_eq!(status, StatusCode::CREATED, "{detail}");
    let response = request(
        &app,
        "POST",
        "/api/business/drafts",
        Some(&json!({"padding":"x".repeat(16*1024)})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn registered_assets_require_session_and_exact_origin_and_unregistered_files_are_not_served()
{
    use sunchang_hansang::photo_assets::{photo_upload, publish_photo, validate_photo};
    let (directory, app, _state) = app();
    let photo =
        validate_photo(photo_upload("orphan.png", Some("image/png"), png()).unwrap()).unwrap();
    publish_photo(directory.path(), &photo).unwrap();
    let path = format!("/api/photo-assets/{}", photo.descriptor().sha256);
    assert_eq!(
        request(&app, "GET", &path, None).await.status(),
        StatusCode::NOT_FOUND
    );
    for (host, origin, cookie, expected) in [
        (HOST, None, false, StatusCode::UNAUTHORIZED),
        ("foreign.example", None, true, StatusCode::FORBIDDEN),
        (
            HOST,
            Some("https://foreign.example"),
            true,
            StatusCode::FORBIDDEN,
        ),
    ] {
        let mut builder = Request::builder().uri(&path).header("host", host);
        if cookie {
            builder = builder.header("cookie", COOKIE);
        }
        if let Some(origin) = origin {
            builder = builder.header("origin", origin);
        }
        assert_eq!(
            app.clone()
                .oneshot(builder.body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            expected
        );
    }
    for path in [
        "/assets/photos/anything.png",
        "/db/assets/photos/anything.png",
        "/api/photo-assets/unknown/nested",
    ] {
        assert_eq!(
            request(&app, "GET", path, None).await.status(),
            StatusCode::NOT_FOUND
        );
    }
}

#[tokio::test]
async fn changed_registered_asset_is_unavailable_without_overwrite_or_false_success() {
    let (directory, app, _state) = app();
    let item = material(&app).await;
    let path = format!("/api/materials/{}/profiles", item["id"].as_str().unwrap());
    let input = input(&app, &item, true).await;
    let first = post(
        &app,
        &path,
        multipart(&input, "a.png", Some(&png()), false, "x"),
        "x",
    )
    .await;
    assert_eq!(first.status(), StatusCode::CREATED);
    let first = body(first).await;
    let asset = &first["version"]["photos"][0]["asset"];
    let file = directory.path().join(asset["asset_key"].as_str().unwrap());
    std::fs::write(&file, b"tampered").unwrap();
    let response = request(
        &app,
        "GET",
        &format!("/api/photo-assets/{}", asset["sha256"].as_str().unwrap()),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let retry = post(
        &app,
        &path,
        multipart(&input, "a.png", Some(&png()), false, "x"),
        "x",
    )
    .await;
    assert_eq!(retry.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(std::fs::read(&file).unwrap(), b"tampered");
    std::fs::write(&file, png()).unwrap();
    // Deliberate corruption, after normal API creation: a registered descriptor
    // fault is an unavailable asset, not a new user's validation error.
    let connection = rusqlite::Connection::open(directory.path().join("app.sqlite3")).unwrap();
    connection
        .execute(
            "UPDATE photo_assets SET asset_key='../outside.png' WHERE sha256=?1",
            [asset["sha256"].as_str().unwrap()],
        )
        .unwrap();
    let response = request(
        &app,
        "GET",
        &format!("/api/photo-assets/{}", asset["sha256"].as_str().unwrap()),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body(response).await["code"], "unavailable");
    assert_eq!(std::fs::read(file).unwrap(), png());
}

#[tokio::test]
async fn upload_admission_rejects_a_second_body_and_shutdown_cancels_only_unaccepted_upload() {
    let (_directory, app, state) = app();
    let item = material(&app).await;
    let path = format!("/api/materials/{}/profiles", item["id"].as_str().unwrap());
    let before = get(&app, "/api/state").await;
    let started = std::sync::Arc::new(tokio::sync::Notify::new());
    let signal = started.clone();
    let stream = futures_util::stream::once(async move {
        signal.notify_one();
        std::future::pending::<Result<Bytes, std::io::Error>>().await
    });
    let first = app.clone().oneshot(
        Request::builder()
            .method("POST")
            .uri(&path)
            .header("host", HOST)
            .header("origin", ORIGIN)
            .header("cookie", COOKIE)
            .header("content-type", "multipart/form-data; boundary=x")
            .body(Body::from_stream(stream))
            .unwrap(),
    );
    let task = tokio::spawn(first);
    started.notified().await;
    let response = post(&app, &path, vec![], "x").await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body(response).await["code"], "unavailable");
    assert_eq!(get(&app, "/api/state").await, before);
    state.shutdown();
    let response = tokio::time::timeout(std::time::Duration::from_secs(3), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn upload_guards_reject_before_polling_an_untrusted_body() {
    let (_directory, app, _state) = app();
    let path = "/api/materials/00000000-0000-4000-8000-000000000001/profiles";
    for (host, origin, cookie, expected) in [
        ("foreign.example", Some(ORIGIN), true, StatusCode::FORBIDDEN),
        (
            HOST,
            Some("https://foreign.example"),
            true,
            StatusCode::FORBIDDEN,
        ),
        (HOST, None, true, StatusCode::FORBIDDEN),
        (HOST, Some(ORIGIN), false, StatusCode::UNAUTHORIZED),
    ] {
        let stream = futures_util::stream::poll_fn(
            |_| -> std::task::Poll<Option<Result<Bytes, std::io::Error>>> {
                panic!("guard must not poll this body")
            },
        );
        let mut request = Request::builder()
            .method("POST")
            .uri(path)
            .header("host", host)
            .header("content-type", "multipart/form-data; boundary=x");
        if let Some(origin) = origin {
            request = request.header("origin", origin);
        }
        if cookie {
            request = request.header("cookie", COOKIE);
        }
        let response = app
            .clone()
            .oneshot(request.body(Body::from_stream(stream)).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
    }
}
