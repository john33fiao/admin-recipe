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
const COOKIE: &str = "hansang_4321=waste-test-session";
const MISSING_ID: &str = "00000000-0000-4000-8000-000000000001";

fn app() -> (tempfile::TempDir, Router) {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::new(
        Store::open(directory.path()).unwrap(),
        HOST.into(),
        "waste-test-session".into(),
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

fn draft(state: &Value, reason: &str) -> Value {
    json!({"meta":{"request_id":uuid::Uuid::new_v4().to_string(),"epoch":state["epoch"],"expected_revision":state["revision"]},
        "corrects_confirmation_id":null,"payload":{"kind":"waste","version":1,"data":{
            "material_id":null,"material_version_id":null,"quantity":"","unit":"g","reason":reason,"note":"가상 폐기 초안"}}})
}

#[tokio::test]
async fn waste_list_and_detail_have_strict_filters() {
    let (_directory, app) = app();
    let list = get(&app, "/api/wastes").await;
    assert_eq!(list["items"], json!([]));
    assert!(list["state"]["epoch"].is_string());
    assert!(list["state"]["revision"].is_string());
    assert_eq!(
        get(&app, &format!("/api/wastes?material_id={MISSING_ID}")).await["items"],
        json!([])
    );
    for path in ["/api/wastes?material_id=bad", "/api/wastes?extra=true"] {
        let response = send(&app, "GET", path, None).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}");
        assert_eq!(value(response).await["code"], "validation");
    }
    let response = send(&app, "GET", &format!("/api/wastes/{MISSING_ID}"), None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(value(response).await["code"], "not_found");
}

#[tokio::test]
async fn waste_spa_supports_get_head_and_rejects_undocumented_paths() {
    let (_directory, app) = app();
    for path in ["/wastes".to_owned(), format!("/wastes/{MISSING_ID}")] {
        for method in ["GET", "HEAD"] {
            let response = send(&app, method, &path, None).await;
            assert_eq!(response.status(), StatusCode::OK, "{method} {path}");
            assert!(
                response.headers()["content-type"]
                    .to_str()
                    .unwrap()
                    .contains("text/html")
            );
            assert_eq!(
                response
                    .into_body()
                    .collect()
                    .await
                    .unwrap()
                    .to_bytes()
                    .is_empty(),
                method == "HEAD"
            );
        }
    }
    for path in [
        "/wastes/new",
        "/wastes/not-a-uuid",
        "/api/wastes/extra/path",
    ] {
        assert_eq!(
            send(&app, "GET", path, None).await.status(),
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
}

#[tokio::test]
async fn waste_kind_can_be_filtered_without_changing_state() {
    let (_directory, app) = app();
    let before = get(&app, "/api/state").await;
    assert_eq!(
        get(&app, "/api/business?kind=waste&status=all").await["items"],
        json!([])
    );
    assert_eq!(get(&app, "/api/state").await, before);
}

#[tokio::test]
async fn waste_and_staff_meal_drafts_are_preserved_without_effect() {
    let (_directory, app) = app();
    for reason in ["discard", "staff_meal"] {
        let input = draft(&get(&app, "/api/state").await, reason);
        let response = send(&app, "POST", "/api/business/drafts", Some(&input)).await;
        let status = response.status();
        let saved = value(response).await;
        assert_eq!(status, StatusCode::CREATED, "{saved}");
        assert_eq!(saved["result"]["head"]["payload"], input["payload"]);
        assert_eq!(saved["result"]["item"]["kind"], "waste");
        assert!(!saved["result"]["issues"].as_array().unwrap().is_empty());
        let id = saved["result"]["item"]["id"].as_str().unwrap();
        let response = send(
            &app,
            "POST",
            &format!("/api/business/{id}/preview"),
            Some(&json!({"expected_head_id":saved["result"]["head"]["id"]})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(value(response).await["can_confirm"], false);
        let before_retry = get(&app, "/api/state").await;
        let replay = send(&app, "POST", "/api/business/drafts", Some(&input)).await;
        assert_eq!(replay.status(), StatusCode::CREATED);
        assert_eq!(value(replay).await, saved);
        assert_eq!(get(&app, "/api/state").await, before_retry);
        assert_eq!(get(&app, "/api/wastes").await["items"], json!([]));
    }
}

#[tokio::test]
async fn waste_preserves_existing_security_and_16_kib_limit() {
    let (_directory, app) = app();
    let before = get(&app, "/api/state").await;
    for (host, origin, cookie, expected) in [
        (
            "foreign.example",
            Some(ORIGIN),
            Some(COOKIE),
            StatusCode::FORBIDDEN,
        ),
        (
            HOST,
            Some("https://foreign.example"),
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
                .oneshot(
                    request
                        .body(Body::from(draft(&before, "discard").to_string()))
                        .unwrap()
                )
                .await
                .unwrap()
                .status(),
            expected
        );
    }
    let oversized = json!({"extra":"x".repeat(17 * 1024)});
    assert_eq!(
        send(&app, "POST", "/api/business/drafts", Some(&oversized))
            .await
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(get(&app, "/api/state").await, before);
}

fn meta(state: &Value) -> Value {
    json!({"request_id":uuid::Uuid::new_v4().to_string(),"epoch":state["epoch"],"expected_revision":state["revision"]})
}
async fn created(app: &Router, path: &str, input: &Value) -> Value {
    let response = send(app, "POST", path, Some(input)).await;
    let status = response.status();
    let value = value(response).await;
    assert_eq!(status, StatusCode::CREATED, "{path}: {value}");
    value
}
async fn material(app: &Router) -> Value {
    let mut input = meta(&get(app, "/api/state").await);
    input["name"] = json!("가상 전량 폐기");
    input["unit"] = json!("g");
    input["kind"] = json!("raw");
    input["description"] = json!("");
    created(app, "/api/materials", &input).await
}
async fn business(app: &Router, kind: &str, data: Value) -> Value {
    created(app,"/api/business/drafts",&json!({"meta":meta(&get(app,"/api/state").await),"corrects_confirmation_id":null,"payload":{"kind":kind,"version":1,"data":data}})).await
}
fn confirm_path(draft: &Value) -> String {
    format!(
        "/api/business/{}/confirm",
        draft["result"]["item"]["id"].as_str().unwrap()
    )
}
async fn preview(app: &Router, draft: &Value) -> Value {
    let response = send(
        app,
        "POST",
        &format!(
            "/api/business/{}/preview",
            draft["result"]["item"]["id"].as_str().unwrap()
        ),
        Some(&json!({"expected_head_id":draft["result"]["head"]["id"]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    value(response).await
}
async fn confirm(app: &Router, draft: &Value) -> Value {
    let check = preview(app, draft).await;
    assert_eq!(check["can_confirm"], true, "{check}");
    created(
        app,
        &confirm_path(draft),
        &json!({"meta":meta(&check["state"]),"expected_head_id":check["head_id"]}),
    )
    .await
}
async fn receipt(app: &Router, item: &Value, q: &str, v: &str) {
    let draft=business(app,"receipt",json!({"source":{"kind":"direct","material_id":item["id"],"material_version_id":item["version_id"]},"quantity":q,"unit":"g","amount":v,"receipt_date":"2020-02-29","note":"가상 폐기 입고","stock_effect":{"kind":"unreflected"}})).await;
    confirm(app, &draft).await;
}
fn waste_data(item: &Value, q: &str) -> Value {
    json!({"material_id":item["id"],"material_version_id":item["version_id"],"quantity":q,"unit":"g","reason":"staff_meal","note":"가상 직원식"})
}

#[tokio::test]
async fn partial_then_full_waste_preserves_loss_without_carrying_it_into_the_next_receipt() {
    let (_directory, app) = app();
    let item = material(&app).await;
    receipt(&app, &item, "10", "100").await;
    let holding_path = format!("/api/holdings/{}", item["id"].as_str().unwrap());
    let partial = business(&app, "waste", waste_data(&item, "2")).await;
    let check = preview(&app, &partial).await;
    assert_eq!(check["effect"]["waste"]["after"]["quantity"], "8");
    assert_eq!(check["effect"]["waste"]["after"]["value"], "100");
    assert_eq!(check["effect"]["waste"]["loss_value"], "0");
    confirm(&app, &partial).await;
    let after_partial = get(&app, &holding_path).await;
    assert_eq!(after_partial["confirmed_changes"][0]["kind"], "waste");
    assert!(after_partial["observation"].is_null());
    let full = business(&app, "waste", waste_data(&item, "8")).await;
    let check = preview(&app, &full).await;
    assert_eq!(check["effect"]["waste"]["loss_value"], "100");
    let input = json!({"meta":meta(&check["state"]),"expected_head_id":check["head_id"]});
    let saved = created(&app, &confirm_path(&full), &input).await;
    let id = saved["result"]["confirmation"]["result"]["waste_id"]
        .as_str()
        .unwrap();
    let detail = get(&app, &format!("/api/wastes/{id}")).await;
    assert_eq!(detail["item"]["effect"]["reason"], "staff_meal");
    assert_eq!(detail["item"]["effect"]["loss_value"], "100");
    assert!(detail["item"]["effect"]["after"]["source_id"].is_string());
    let empty = get(&app, &holding_path).await;
    assert_eq!(empty["settlement_basis"]["quantity"], "0");
    assert_eq!(empty["settlement_basis"]["value"], "0");
    assert_eq!(empty["price"]["source"], "last_valid");
    assert_eq!(empty["price"]["amount"], "100");
    assert_eq!(empty["price"]["quantity"], "8");
    receipt(&app, &item, "2", "30").await;
    let next = get(&app, &holding_path).await;
    assert_eq!(next["settlement_basis"]["value"], "30");
    assert_eq!(next["settlement_basis"]["quantity"], "2");
    let before = get(&app, "/api/state").await;
    assert_eq!(created(&app, &confirm_path(&full), &input).await, saved);
    assert_eq!(get(&app, "/api/state").await, before);
    assert_eq!(
        get(
            &app,
            &format!("/api/wastes?material_id={}", item["id"].as_str().unwrap())
        )
        .await["items"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn insufficient_waste_is_saved_and_stale_confirmation_cannot_consume_a_new_receipt() {
    let (_directory, app) = app();
    let item = material(&app).await;
    receipt(&app, &item, "1", "0").await;
    let draft = business(&app, "waste", waste_data(&item, "2")).await;
    let stale = preview(&app, &draft).await;
    assert_eq!(stale["can_confirm"], false);
    assert!(
        stale["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["code"] == "needs_reconciliation")
    );
    receipt(&app, &item, "1", "0").await;
    let input = json!({"meta":meta(&stale["state"]),"expected_head_id":stale["head_id"]});
    let response = send(&app, "POST", &confirm_path(&draft), Some(&input)).await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(value(response).await["code"], "stale_state");
    assert_eq!(get(&app, "/api/wastes").await["items"], json!([]));
    let check = preview(&app, &draft).await;
    assert_eq!(check["can_confirm"], true);
    assert_eq!(check["effect"]["waste"]["quantity"], "2");
    assert_eq!(check["effect"]["waste"]["loss_value"], "0");
    confirm(&app, &draft).await;
    assert_eq!(
        get(
            &app,
            &format!("/api/holdings/{}", item["id"].as_str().unwrap())
        )
        .await["settlement_basis"]["quantity"],
        "0"
    );
}
