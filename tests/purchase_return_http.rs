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
const COOKIE: &str = "hansang_4321=return-session";
const MISSING: &str = "00000000-0000-4000-8000-000000000001";
fn app() -> (tempfile::TempDir, Router) {
    let d = tempfile::tempdir().unwrap();
    let state = AppState::new(
        Store::open(d.path()).unwrap(),
        HOST.into(),
        "return-session".into(),
        Policy::default(),
    );
    (d, router(state))
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
    serde_json::from_slice(&bytes).unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&bytes)))
}
async fn get(app: &Router, path: &str) -> Value {
    let r = send(app, "GET", path, None).await;
    assert_eq!(r.status(), StatusCode::OK, "{path}");
    value(r).await
}
fn meta(state: &Value) -> Value {
    json!({"request_id":uuid::Uuid::new_v4().to_string(),"epoch":state["epoch"],"expected_revision":state["revision"]})
}
async fn created(app: &Router, path: &str, input: &Value) -> Value {
    let r = send(app, "POST", path, Some(input)).await;
    let status = r.status();
    let v = value(r).await;
    assert_eq!(status, StatusCode::CREATED, "{v}");
    v
}
async fn draft(app: &Router, kind: &str, data: Value) -> Value {
    created(app,"/api/business/drafts",&json!({"meta":meta(&get(app,"/api/state").await),"corrects_confirmation_id":null,"payload":{"kind":kind,"version":1,"data":data}})).await
}
async fn preview(app: &Router, draft: &Value) -> Value {
    let r = send(
        app,
        "POST",
        &format!(
            "/api/business/{}/preview",
            draft["result"]["item"]["id"].as_str().unwrap()
        ),
        Some(&json!({"expected_head_id":draft["result"]["head"]["id"]})),
    )
    .await;
    assert_eq!(r.status(), StatusCode::OK);
    value(r).await
}
async fn confirm(app: &Router, draft: &Value) -> Value {
    let p = preview(app, draft).await;
    assert_eq!(p["can_confirm"], true, "{p}");
    created(
        app,
        &format!(
            "/api/business/{}/confirm",
            draft["result"]["item"]["id"].as_str().unwrap()
        ),
        &json!({"meta":meta(&p["state"]),"expected_head_id":p["head_id"]}),
    )
    .await
}
async fn material(app: &Router) -> Value {
    let mut input = meta(&get(app, "/api/state").await);
    input["name"] = json!("가상 구매 반품");
    input["unit"] = json!("g");
    input["kind"] = json!("raw");
    input["description"] = json!("");
    created(app, "/api/materials", &input).await
}
async fn receipt(app: &Router, m: &Value, q: &str, a: &str) -> String {
    let d=draft(app,"receipt",json!({"source":{"kind":"direct","material_id":m["id"],"material_version_id":m["version_id"]},"quantity":q,"unit":"g","amount":a,"receipt_date":"2020-02-29","note":"가상 입고","stock_effect":{"kind":"unreflected"}})).await;
    confirm(app, &d).await["result"]["confirmation"]["result"]["receipt_id"]
        .as_str()
        .unwrap()
        .into()
}
async fn return_data(app: &Router, id: &str, q: &str) -> Value {
    let source = get(app, &format!("/api/receipts/{id}/return-source")).await;
    json!({"receipt_id":id,"expected_receipt_version_id":source["current"]["reference"]["version_id"],"quantity":q,"unit":"g","note":"가상 반품"})
}

#[tokio::test]
async fn list_source_and_detail_routes_validate_filters_and_missing_references() {
    let (_d, app) = app();
    let before = get(&app, "/api/state").await;
    assert_eq!(get(&app, "/api/purchase-returns").await["items"], json!([]));
    assert_eq!(
        get(
            &app,
            &format!("/api/purchase-returns?receipt_id={MISSING}&material_id={MISSING}")
        )
        .await["items"],
        json!([])
    );
    for path in [
        "/api/purchase-returns?receipt_id=bad",
        "/api/purchase-returns?extra=true",
        "/api/receipts/bad/return-source",
    ] {
        let r = send(&app, "GET", path, None).await;
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        assert_eq!(value(r).await["code"], "validation");
    }
    for path in [
        format!("/api/purchase-returns/{MISSING}"),
        format!("/api/receipts/{MISSING}/return-source"),
    ] {
        let r = send(&app, "GET", &path, None).await;
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
        assert_eq!(value(r).await["code"], "not_found");
    }
    assert_eq!(get(&app, "/api/state").await, before);
}
#[tokio::test]
async fn return_spa_get_head_and_unknown_paths_preserve_asset_boundary() {
    let (_d, app) = app();
    for path in [
        "/purchase-returns".into(),
        format!("/purchase-returns/{MISSING}"),
    ] {
        for method in ["GET", "HEAD"] {
            let r = send(&app, method, &path, None).await;
            assert_eq!(r.status(), StatusCode::OK);
            assert!(
                r.headers()["content-type"]
                    .to_str()
                    .unwrap()
                    .contains("text/html")
            );
            assert_eq!(
                r.into_body().collect().await.unwrap().to_bytes().is_empty(),
                method == "HEAD"
            );
        }
    }
    for path in [
        "/purchase-returns/new",
        "/purchase-returns/bad",
        "/api/purchase-returns/extra/path",
    ] {
        assert_eq!(
            send(&app, "GET", path, None).await.status(),
            StatusCode::NOT_FOUND
        );
    }
}
#[tokio::test]
async fn incomplete_draft_roundtrips_and_replays_without_stock_effect() {
    let (_d, app) = app();
    let before = get(&app, "/api/state").await;
    let input = json!({"meta":meta(&before),"corrects_confirmation_id":null,"payload":{"kind":"purchase_return","version":1,"data":{"receipt_id":null,"expected_receipt_version_id":null,"quantity":"","unit":"g","note":""}}});
    let saved = created(&app, "/api/business/drafts", &input).await;
    assert_eq!(saved["result"]["head"]["payload"], input["payload"]);
    assert_eq!(preview(&app, &saved).await["can_confirm"], false);
    let state = get(&app, "/api/state").await;
    assert_eq!(created(&app, "/api/business/drafts", &input).await, saved);
    assert_eq!(get(&app, "/api/state").await, state);
    assert_eq!(get(&app, "/api/purchase-returns").await["items"], json!([]));
}
#[tokio::test]
async fn security_limits_and_string_numbers_remain_strict() {
    let (_d, app) = app();
    let state = get(&app, "/api/state").await;
    let input = json!({"meta":meta(&state),"corrects_confirmation_id":null,"payload":{"kind":"purchase_return","version":1,"data":{"receipt_id":null,"expected_receipt_version_id":null,"quantity":1,"unit":"g","note":""}}});
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
        let mut req = Request::builder()
            .method("POST")
            .uri("/api/business/drafts")
            .header("host", host)
            .header("content-type", "application/json");
        if let Some(origin) = origin {
            req = req.header("origin", origin);
        }
        if let Some(cookie) = cookie {
            req = req.header("cookie", cookie);
        }
        assert_eq!(
            app.clone()
                .oneshot(req.body(Body::from(input.to_string())).unwrap())
                .await
                .unwrap()
                .status(),
            expected
        );
    }
    let r = send(&app, "POST", "/api/business/drafts", Some(&input)).await;
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    assert_eq!(value(r).await["code"], "validation");
    assert_eq!(
        send(
            &app,
            "POST",
            "/api/business/drafts",
            Some(&json!({"extra":"x".repeat(17*1024)}))
        )
        .await
        .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(get(&app, "/api/state").await, state);
}
#[tokio::test]
async fn mixed_average_return_and_exact_full_remainder_preserve_source_and_replay() {
    let (_d, app) = app();
    let m = material(&app).await;
    let a = receipt(&app, &m, "2000", "20000").await;
    let b = receipt(&app, &m, "1000", "20000").await;
    let first = draft(&app, "purchase_return", return_data(&app, &b, "1000").await).await;
    let p = preview(&app, &first).await;
    assert_eq!(
        p["effect"]["purchase_return"]["deducted_value"],
        "13333.333333"
    );
    assert_eq!(
        p["effect"]["purchase_return"]["source"]["original_unit"],
        "g"
    );
    let path = format!(
        "/api/business/{}/confirm",
        first["result"]["item"]["id"].as_str().unwrap()
    );
    let request = json!({"meta":meta(&p["state"]),"expected_head_id":p["head_id"]});
    let done = created(&app, &path, &request).await;
    let state = get(&app, "/api/state").await;
    assert_eq!(created(&app, &path, &request).await, done);
    assert_eq!(get(&app, "/api/state").await, state);
    let id = done["result"]["confirmation"]["result"]["purchase_return_id"]
        .as_str()
        .unwrap();
    let detail = get(&app, &format!("/api/purchase-returns/{id}")).await;
    assert_eq!(detail["item"]["effect"]["source"]["reference"]["id"], b);
    assert_eq!(
        detail["item"]["effect"]["source"]["original_amount"],
        "20000"
    );
    let last = draft(&app, "purchase_return", return_data(&app, &a, "2000").await).await;
    assert_eq!(
        preview(&app, &last).await["effect"]["purchase_return"]["deducted_value"],
        "26666.666667"
    );
    confirm(&app, &last).await;
    let holding = get(
        &app,
        &format!("/api/holdings/{}", m["id"].as_str().unwrap()),
    )
    .await;
    assert_eq!(holding["settlement_basis"]["quantity"], "0");
    assert_eq!(holding["settlement_basis"]["value"], "0");
    assert_eq!(
        get(&app, &format!("/api/receipts/{a}/return-source")).await["current"]["remaining_quantity"],
        "0"
    );
    assert_eq!(
        get(&app, &format!("/api/purchase-returns?receipt_id={b}")).await["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
#[tokio::test]
async fn receipt_limit_blocks_second_return_even_with_other_current_stock() {
    let (_d, app) = app();
    let m = material(&app).await;
    let a = receipt(&app, &m, "3", "30").await;
    receipt(&app, &m, "10", "100").await;
    let first = draft(&app, "purchase_return", return_data(&app, &a, "2").await).await;
    confirm(&app, &first).await;
    let second = draft(&app, "purchase_return", return_data(&app, &a, "2").await).await;
    let p = preview(&app, &second).await;
    assert_eq!(p["can_confirm"], false);
    assert!(!p["issues"].as_array().unwrap().is_empty());
    assert_eq!(
        get(&app, &format!("/api/receipts/{a}/return-source")).await["current"]["remaining_quantity"],
        "1"
    );
}
#[tokio::test]
async fn return_confirmation_has_unified_correction_seed_and_original_source_amount() {
    let (_d, app) = app();
    let m = material(&app).await;
    let a = receipt(&app, &m, "3", "0").await;
    let ret = draft(&app, "purchase_return", return_data(&app, &a, "1").await).await;
    let done = confirm(&app, &ret).await;
    let conf = done["result"]["confirmation"]["id"].as_str().unwrap();
    let seed = get(
        &app,
        &format!("/api/business/confirmations/{conf}/correction-seed"),
    )
    .await;
    assert_eq!(seed["current"]["reference"]["kind"], "purchase_return");
    assert_eq!(seed["replacement_template"]["kind"], "purchase_return");
    let receipt = get(&app, &format!("/api/receipts/{a}")).await;
    assert_eq!(receipt["item"]["amount"], "0");
}
