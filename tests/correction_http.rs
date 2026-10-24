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
const COOKIE: &str = "hansang_4321=correction-http";
fn app() -> (tempfile::TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let app = router(AppState::new(
        Store::open(dir.path()).unwrap(),
        HOST.into(),
        "correction-http".into(),
        Policy::default(),
    ));
    (dir, app)
}
async fn send(app: &Router, method: &str, path: &str, data: Option<&Value>) -> Response {
    let mut req = Request::builder()
        .method(method)
        .uri(path)
        .header("host", HOST)
        .header("cookie", COOKIE);
    let body = if let Some(data) = data {
        req = req
            .header("origin", ORIGIN)
            .header("content-type", "application/json");
        Body::from(data.to_string())
    } else {
        Body::empty()
    };
    app.clone().oneshot(req.body(body).unwrap()).await.unwrap()
}
async fn body(r: Response) -> Value {
    serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
async fn get(app: &Router, path: &str) -> Value {
    let r = send(app, "GET", path, None).await;
    assert_eq!(r.status(), StatusCode::OK, "{path}");
    body(r).await
}
fn meta(state: &Value) -> Value {
    json!({"request_id":uuid::Uuid::new_v4().to_string(),"epoch":state["epoch"],"expected_revision":state["revision"]})
}
async fn created(app: &Router, path: &str, input: &Value) -> Value {
    let r = send(app, "POST", path, Some(input)).await;
    let status = r.status();
    let v = body(r).await;
    assert_eq!(status, StatusCode::CREATED, "{path}: {v}");
    v
}
async fn preview(app: &Router, saved: &Value) -> Value {
    let id = saved["result"]["item"]["id"].as_str().unwrap();
    let r = send(
        app,
        "POST",
        &format!("/api/business/{id}/preview"),
        Some(&json!({"expected_head_id":saved["result"]["head"]["id"]})),
    )
    .await;
    assert_eq!(r.status(), StatusCode::OK);
    body(r).await
}
async fn confirm(app: &Router, saved: &Value) -> Value {
    let p = preview(app, saved).await;
    assert_eq!(p["can_confirm"], true, "{p}");
    let id = saved["result"]["item"]["id"].as_str().unwrap();
    created(
        app,
        &format!("/api/business/{id}/confirm"),
        &json!({"meta":meta(&p["state"]),"expected_head_id":p["head_id"]}),
    )
    .await
}
async fn receipt(app: &Router) -> Value {
    let mut input = meta(&get(app, "/api/state").await);
    input["name"] = json!("가상 정정 HTTP 품목");
    input["unit"] = json!("g");
    input["kind"] = json!("raw");
    input["description"] = json!("");
    let item = created(app, "/api/materials", &input).await;
    let saved=created(app,"/api/business/drafts",&json!({"meta":meta(&get(app,"/api/state").await),"corrects_confirmation_id":null,"payload":{"kind":"receipt","version":1,"data":{"source":{"kind":"direct","material_id":item["id"],"material_version_id":item["version_id"]},"quantity":"10","unit":"g","amount":"100","receipt_date":"2020-02-29","note":"가상 입고","stock_effect":{"kind":"unreflected"}}}})).await;
    confirm(app, &saved).await
}
async fn seed(app: &Router, confirmed: &Value) -> Value {
    get(
        app,
        &format!(
            "/api/business/confirmations/{}/correction-seed",
            confirmed["result"]["confirmation"]["id"].as_str().unwrap()
        ),
    )
    .await
}
fn correction_input(state: &Value, seed: &Value) -> Value {
    json!({"meta":meta(state),"corrects_confirmation_id":seed["original"]["reference"]["original_confirmation_id"],"payload":{"kind":"correction","version":1,"data":{"expected_target_version_id":seed["current"]["reference"]["version_id"],"reason":"가상 오입력 취소","action":{"kind":"void"},"observation_reviews":[],"linked_receipt_changes":[]}}})
}

#[tokio::test]
async fn seed_is_read_only_and_correction_alias_preserves_original_response() {
    let (_dir, app) = app();
    let receipt = receipt(&app).await;
    let before = get(&app, "/api/state").await;
    let seed = seed(&app, &receipt).await;
    assert_eq!(get(&app, "/api/state").await, before);
    let input = correction_input(&before, &seed);
    let saved = created(&app, "/api/corrections/drafts", &input).await;
    let replay = created(&app, "/api/business/drafts", &input).await;
    assert_eq!(replay, saved);
    let p = preview(&app, &saved).await;
    assert_eq!(p["can_confirm"], true, "{p}");
    assert_eq!(
        p["effect"]["correction"]["materials"][0]["after"]["kind"],
        "not_established"
    );
    let confirmed = confirm(&app, &saved).await;
    let id = confirmed["result"]["confirmation"]["result"]["correction_id"]
        .as_str()
        .unwrap();
    let detail = get(&app, &format!("/api/corrections/{id}")).await;
    assert_eq!(detail["item"]["input"]["action"]["kind"], "void");
    let original_seed = get(
        &app,
        &format!(
            "/api/business/confirmations/{}/correction-seed",
            receipt["result"]["confirmation"]["id"].as_str().unwrap()
        ),
    )
    .await;
    assert!(original_seed["original"]["value"].is_object());
    assert!(original_seed["current"]["value"].is_null());
}

#[tokio::test]
async fn correction_alias_has_local_256k_limit_and_common_keeps_16k() {
    let (_dir, app) = app();
    let r = receipt(&app).await;
    let seed = seed(&app, &r).await;
    let mut input = correction_input(&get(&app, "/api/state").await, &seed);
    input["payload"]["data"]["observation_reviews"]=json!((0..250).map(|_|json!({"observation_id":null,"expected_version_id":null,"initial_value":null,"excess":null})).collect::<Vec<_>>());
    assert!(input.to_string().len() > 16 * 1024);
    assert!(input.to_string().len() < 256 * 1024);
    assert_eq!(
        send(&app, "POST", "/api/business/drafts", Some(&input))
            .await
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let saved = created(&app, "/api/corrections/drafts", &input).await;
    assert_eq!(
        saved["result"]["head"]["payload"]["data"]["observation_reviews"]
            .as_array()
            .unwrap()
            .len(),
        250
    );
    let mut huge = input;
    huge["payload"]["data"]["reason"] = json!("x".repeat(256 * 1024));
    assert_eq!(
        send(&app, "POST", "/api/corrections/drafts", Some(&huge))
            .await
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
}

#[tokio::test]
async fn correction_requires_session_origin_and_same_host() {
    let (_dir, app) = app();
    let path = "/api/corrections/drafts";
    for (host, cookie, origin, status) in [
        (HOST, None, ORIGIN, StatusCode::UNAUTHORIZED),
        (
            HOST,
            Some(COOKIE),
            "https://outside.invalid",
            StatusCode::FORBIDDEN,
        ),
        (
            "outside.invalid",
            Some(COOKIE),
            ORIGIN,
            StatusCode::FORBIDDEN,
        ),
    ] {
        let mut req = Request::builder()
            .method("POST")
            .uri(path)
            .header("host", host)
            .header("origin", origin)
            .header("content-type", "application/json");
        if let Some(cookie) = cookie {
            req = req.header("cookie", cookie);
        }
        assert_eq!(
            app.clone()
                .oneshot(req.body(Body::from("{}")).unwrap())
                .await
                .unwrap()
                .status(),
            status
        );
    }
}

#[tokio::test]
async fn correction_rejects_unknown_nested_fields_and_numeric_business_input() {
    let (_dir, app) = app();
    let r = receipt(&app).await;
    let seed = seed(&app, &r).await;
    let before = get(&app, "/api/state").await;
    let mut input = correction_input(&before, &seed);
    input["payload"]["data"]["action"]["quantity"] = json!("0");
    assert_eq!(
        send(&app, "POST", "/api/corrections/drafts", Some(&input))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    input = correction_input(&before, &seed);
    input["payload"]["data"]["action"] =
        json!({"kind":"replace","replacement":{"kind":"sale_count","data":{"count":1}}});
    assert_eq!(
        send(&app, "POST", "/api/corrections/drafts", Some(&input))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(get(&app, "/api/state").await, before);
}

#[tokio::test]
async fn correction_spa_and_unknown_api_keep_distinct_fallbacks() {
    let (_dir, app) = app();
    let id = uuid::Uuid::new_v4();
    assert_eq!(
        send(&app, "GET", &format!("/corrections/{id}"), None)
            .await
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        send(&app, "GET", &format!("/api/corrections/{id}"), None)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(
            &app,
            "GET",
            &format!("/api/business/confirmations/{id}/correction-seed"),
            None
        )
        .await
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        send(&app, "GET", "/corrections/not-a-uuid", None)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn different_body_same_uuid_is_conflict_and_alias_rejects_other_kind() {
    let (_dir, app) = app();
    let r = receipt(&app).await;
    let seed = seed(&app, &r).await;
    let mut input = correction_input(&get(&app, "/api/state").await, &seed);
    let first = created(&app, "/api/corrections/drafts", &input).await;
    input["payload"]["data"]["reason"] = json!("서로 다른 본문");
    let response = send(&app, "POST", "/api/corrections/drafts", Some(&input)).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(body(response).await["code"], "request_conflict");
    let mut other = input;
    other["payload"] = json!({"kind":"sale","version":1,"data":{"material_id":null,"material_version_id":null,"recipe_version_id":null,"count":"0","note":""}});
    assert_eq!(
        send(&app, "POST", "/api/corrections/drafts", Some(&other))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    let detail = get(
        &app,
        &format!(
            "/api/business/{}",
            first["result"]["item"]["id"].as_str().unwrap()
        ),
    )
    .await;
    assert_eq!(detail["revisions"].as_array().unwrap().len(), 1);
}
