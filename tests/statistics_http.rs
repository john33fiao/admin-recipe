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
const COOKIE: &str = "hansang_4321=statistics-test";
const RANGE: &str = "from=2020-01-01T00%3A00%3A00Z&until=2090-01-01T00%3A00%3A00Z";
fn app() -> (tempfile::TempDir, Router) {
    let d = tempfile::tempdir().unwrap();
    let state = AppState::new(
        Store::open(d.path()).unwrap(),
        HOST.into(),
        "statistics-test".into(),
        Policy::default(),
    );
    (d, router(state))
}
async fn send(app: &Router, method: &str, path: &str, input: Option<&Value>) -> Response {
    let mut r = Request::builder()
        .method(method)
        .uri(path)
        .header("host", HOST)
        .header("cookie", COOKIE);
    if method == "HEAD" {
        r = r.header("origin", ORIGIN);
    }
    let body = if let Some(v) = input {
        r = r
            .header("origin", ORIGIN)
            .header("content-type", "application/json");
        Body::from(v.to_string())
    } else {
        Body::empty()
    };
    app.clone().oneshot(r.body(body).unwrap()).await.unwrap()
}
async fn value(r: Response) -> Value {
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| panic!("expected JSON: {}", String::from_utf8_lossy(&bytes)))
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
    assert_eq!(status, StatusCode::CREATED, "{path}: {v}");
    v
}
async fn material(app: &Router, name: &str) -> Value {
    let mut v = meta(&get(app, "/api/state").await);
    v["name"] = json!(name);
    v["unit"] = json!("g");
    v["kind"] = json!("raw");
    v["description"] = json!("");
    created(app, "/api/materials", &v).await
}
fn data(lines: Vec<Value>) -> Value {
    json!({"basis_count":"1","reference_output":null,"selling_price":"10","cooking_description":"","process":"","reference_time":"","lines":lines})
}
async fn recipe(app: &Router, owner: &Value, lines: Vec<Value>) -> Value {
    created(app,"/api/recipes",&json!({"meta":meta(&get(app,"/api/state").await),"owner":{"mode":"existing","material_id":owner["id"],"expected_material_version_id":owner["version_id"]},"data":data(lines)})).await
}
async fn draft(app: &Router, kind: &str, data: Value, corrects: Option<&str>) -> Value {
    created(app,"/api/business/drafts",&json!({"meta":meta(&get(app,"/api/state").await),"corrects_confirmation_id":corrects,"payload":{"kind":kind,"version":1,"data":data}})).await
}
async fn confirm(app: &Router, draft: &Value) -> Value {
    let id = draft["result"]["item"]["id"].as_str().unwrap();
    let r = send(
        app,
        "POST",
        &format!("/api/business/{id}/preview"),
        Some(&json!({"expected_head_id":draft["result"]["head"]["id"]})),
    )
    .await;
    assert_eq!(r.status(), StatusCode::OK);
    let p = value(r).await;
    assert_eq!(p["can_confirm"], true, "{p}");
    created(
        app,
        &format!("/api/business/{id}/confirm"),
        &json!({"meta":meta(&p["state"]),"expected_head_id":p["head_id"]}),
    )
    .await["result"]["confirmation"]
        .clone()
}
async fn event(app: &Router, kind: &str, data: Value) -> Value {
    let d = draft(app, kind, data, None).await;
    confirm(app, &d).await
}

#[tokio::test]
async fn empty_statistics_and_head_are_one_state_and_do_not_write() {
    let (directory, app) = app();
    let before = get(&app, "/api/state").await;
    let db = std::fs::read(directory.path().join("app.sqlite3")).unwrap();
    let report = get(&app, &format!("/api/statistics?{RANGE}")).await;
    assert_eq!(report["state"], before);
    assert_eq!(report["range"]["time_basis"], "original_recorded_at");
    assert_eq!(report["range"]["correction_basis"], "current");
    assert_eq!(report["sales"]["rows"], json!([]));
    assert_eq!(report["sales"]["summary"]["known_revenue"], "0");
    assert_eq!(report["sales"]["coverage"]["all_time_records"], 0);
    assert_eq!(report["observations"]["rows"], json!([]));
    assert_eq!(report["operations"]["rows"], json!([]));
    let head = send(&app, "HEAD", &format!("/api/statistics?{RANGE}"), None).await;
    assert_eq!(head.status(), StatusCode::OK);
    assert_eq!(head.headers()["cache-control"], "no-store");
    assert!(
        head.into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .is_empty()
    );
    assert_eq!(get(&app, "/api/state").await, before);
    assert_eq!(
        std::fs::read(directory.path().join("app.sqlite3")).unwrap(),
        db
    );
}

#[tokio::test]
async fn query_keys_encoding_time_and_exact_transport_limit_are_strict() {
    let (_directory, app) = app();
    for query in [
        "".to_owned(),
        "from=2026-01-01".into(),
        format!("{RANGE}&other=1"),
        format!("{RANGE}&%66rom=2020-01-01T00:00:00Z"),
        format!("{RANGE}&until=2090-01-01T00:00:00Z"),
        "from=%GG&until=2090-01-01T00:00:00Z".into(),
        "from=%FF&until=2090-01-01T00:00:00Z".into(),
        "from=%A&until=2090-01-01T00:00:00Z".into(),
        "from=2026-01-01T00:00:00&until=2090-01-01T00:00:00Z".into(),
        "from=2026-01-01T00:00:00Z&until=2026-01-01T00:00:00Z".into(),
        "from=2090-01-01T00:00:00Z&until=2020-01-01T00:00:00Z".into(),
    ] {
        let r = send(&app, "GET", &format!("/api/statistics?{query}"), None).await;
        assert_eq!(r.status(), StatusCode::BAD_REQUEST, "{query}");
        assert_eq!(value(r).await["code"], "validation");
    }
    for (length, status) in [
        (4096, StatusCode::BAD_REQUEST),
        (4097, StatusCode::URI_TOO_LONG),
    ] {
        let query = format!("{RANGE}&extra={}", "x".repeat(length - RANGE.len() - 7));
        assert_eq!(query.len(), length);
        assert_eq!(
            send(&app, "GET", &format!("/api/statistics?{query}"), None)
                .await
                .status(),
            status
        );
    }
    let a = get(
        &app,
        "/api/statistics?from=2026-01-01T09:00:00%2B09:00&until=2026-01-02T09:00:00%2B09:00",
    )
    .await;
    let b = get(
        &app,
        "/api/statistics?from=2026-01-01T00:00:00Z&until=2026-01-02T00:00:00Z",
    )
    .await;
    assert_eq!(a, b);
}

#[tokio::test]
async fn statistics_and_connections_keep_existing_host_origin_session_guards() {
    let (_directory, app) = app();
    let id = uuid::Uuid::new_v4();
    for path in [
        format!("/api/statistics?{RANGE}"),
        format!("/api/recipes/{id}/connections"),
    ] {
        for (host, cookie, origin, status) in [
            (HOST, "", None, StatusCode::UNAUTHORIZED),
            ("foreign.example", COOKIE, None, StatusCode::FORBIDDEN),
            (
                HOST,
                COOKIE,
                Some("https://foreign.example"),
                StatusCode::FORBIDDEN,
            ),
        ] {
            let mut r = Request::builder()
                .uri(&path)
                .header("host", host)
                .header("cookie", cookie);
            if let Some(o) = origin {
                r = r.header("origin", o)
            }
            assert_eq!(
                app.clone()
                    .oneshot(r.body(Body::empty()).unwrap())
                    .await
                    .unwrap()
                    .status(),
                status
            );
        }
    }
}

#[tokio::test]
async fn statistics_spa_get_head_are_exact_and_do_not_expand_fallbacks() {
    let (_directory, app) = app();
    for method in ["GET", "HEAD"] {
        let r = send(&app, method, "/statistics", None).await;
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
    for path in [
        "/statistics/unknown",
        "/api/statistics/missing",
        "/statistics/app.sqlite3",
    ] {
        assert_eq!(
            send(&app, "GET", path, None).await.status(),
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        send(&app, "POST", "/statistics", Some(&json!({})))
            .await
            .status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
}

#[tokio::test]
async fn current_connections_have_real_recipe_links_and_reject_historical_query() {
    let (_directory, app) = app();
    let raw = material(&app, "가상 연결 잎").await;
    let owner = material(&app, "가상 연결 메뉴").await;
    let raw_path = format!("/api/recipes/{}/connections", raw["id"].as_str().unwrap());
    assert_eq!(
        send(&app, "GET", &raw_path, None).await.status(),
        StatusCode::NOT_FOUND
    );
    let saved=recipe(&app,&owner,vec![json!({"material_id":raw["id"],"material_version_id":raw["version_id"],"quantity":"0","unit":"g","note":"적당량"})]).await;
    let path = format!("/api/recipes/{}/connections", owner["id"].as_str().unwrap());
    let graph = get(&app, &path).await;
    assert_eq!(graph["root_recipe_version_id"], saved["version"]["id"]);
    assert_eq!(graph["basis"], "current_root_current_children");
    assert_eq!(graph["direct"][0]["basis_count"], "1");
    assert!(graph["direct"][0]["child_recipe_version_id"].is_null());
    for query in [
        format!("version_id={}", saved["version"]["id"].as_str().unwrap()),
        "extra=true".into(),
    ] {
        let r = send(&app, "GET", &format!("{path}?{query}"), None).await;
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        assert_eq!(value(r).await["code"], "validation");
    }
    let step = &graph["direct"][0];
    let detail = get(
        &app,
        &format!(
            "/api/recipes/{}/versions/{}",
            step["recipe_material_id"].as_str().unwrap(),
            step["recipe_version_id"].as_str().unwrap()
        ),
    )
    .await;
    assert_eq!(detail["id"], saved["version"]["id"]);
    assert_eq!(
        get(
            &app,
            &format!("/api/materials/{}/recipe-uses", raw["id"].as_str().unwrap())
        )
        .await["items"][0]["item"]["id"],
        owner["id"]
    );
}

#[tokio::test]
async fn mixed_report_links_resolve_originals_current_correction_and_observation_owners() {
    let (_directory, app) = app();
    let raw = material(&app, "가상 통계 재료").await;
    let output = material(&app, "가상 통계 결과").await;
    let menu = material(&app, "가상 통계 메뉴").await;
    let recipe=recipe(&app,&menu,vec![json!({"material_id":raw["id"],"material_version_id":raw["version_id"],"quantity":"1","unit":"g","note":""})]).await;
    let received=event(&app,"receipt",json!({"source":{"kind":"direct","material_id":raw["id"],"material_version_id":raw["version_id"]},"quantity":"10","unit":"g","amount":"100","receipt_date":"2020-02-29","note":"","stock_effect":{"kind":"unreflected"}})).await;
    let sale=event(&app,"sale",json!({"material_id":menu["id"],"material_version_id":menu["version_id"],"recipe_version_id":recipe["version"]["id"],"count":"2","note":""})).await;
    event(&app,"stocktake",json!({"rows":[{"material_id":raw["id"],"material_version_id":raw["version_id"],"quantity":"8","unit":"g","initial_value":null,"excess":null}],"note":""})).await;
    event(&app,"waste",json!({"material_id":raw["id"],"material_version_id":raw["version_id"],"quantity":"1","unit":"g","reason":"discard","note":""})).await;
    let receipt_id = received["result"]["receipt_id"].as_str().unwrap();
    let source = get(&app, &format!("/api/receipts/{receipt_id}/return-source")).await;
    event(&app,"purchase_return",json!({"receipt_id":receipt_id,"expected_receipt_version_id":source["current"]["reference"]["version_id"],"quantity":"1","unit":"g","note":""})).await;
    event(&app,"production",json!({"inputs":[{"material_id":raw["id"],"material_version_id":raw["version_id"],"quantity":"1","unit":"g"}],"outputs":[{"material_id":output["id"],"material_version_id":output["version_id"],"quantity":"1","unit":"g","allocation_percent":"100"}],"note":""})).await;
    let original_confirmation = sale["id"].as_str().unwrap();
    let seed = get(
        &app,
        &format!("/api/business/confirmations/{original_confirmation}/correction-seed"),
    )
    .await;
    let mut change = seed["draft"].clone();
    change["reason"] = json!("가상 판매 개수 정정");
    change["action"] =
        json!({"kind":"replace","replacement":{"kind":"sale_count","data":{"count":"1"}}});
    let corrected = draft(&app, "correction", change, Some(original_confirmation)).await;
    let correction = confirm(&app, &corrected).await;
    let state = get(&app, "/api/state").await;
    let report = get(&app, &format!("/api/statistics?{RANGE}")).await;
    assert_eq!(report["state"], state);
    assert_eq!(report["sales"]["summary"]["count"], "1");
    let row = &report["sales"]["rows"][0];
    assert_eq!(row["origin"]["id"], sale["result"]["sale_id"]);
    assert_eq!(row["origin"]["business_record_id"], sale["record_id"]);
    assert_eq!(
        get(
            &app,
            &format!("/api/sales/{}", row["origin"]["id"].as_str().unwrap())
        )
        .await["item"]["id"],
        row["origin"]["id"]
    );
    let changed = row["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["kind"] == "correction")
        .unwrap();
    assert_eq!(changed["id"], correction["result"]["correction_id"]);
    assert_eq!(
        get(
            &app,
            &format!("/api/corrections/{}", changed["id"].as_str().unwrap())
        )
        .await["item"]["id"],
        changed["id"]
    );
    for operation in report["operations"]["rows"].as_array().unwrap() {
        let origin = &operation["origin"];
        let prefix = match origin["kind"].as_str().unwrap() {
            "receipt" => "receipts",
            "production" => "productions",
            "waste" => "wastes",
            "purchase_return" => "purchase-returns",
            "stocktake" => "stocktakes",
            other => panic!("unexpected operation {other}"),
        };
        let detail = get(
            &app,
            &format!("/api/{prefix}/{}", origin["id"].as_str().unwrap()),
        )
        .await;
        assert_eq!(detail["item"]["id"], origin["id"]);
        assert_eq!(
            detail["item"]["business_record_id"],
            origin["business_record_id"]
        );
    }
    assert_eq!(report["operations"]["rows"].as_array().unwrap().len(), 5);
    for observation in report["observations"]["rows"].as_array().unwrap() {
        let detail = get(
            &app,
            &format!(
                "/api/stocktakes/{}",
                observation["origin"]["id"].as_str().unwrap()
            ),
        )
        .await;
        assert!(
            detail["observations"]
                .as_array()
                .unwrap()
                .iter()
                .any(|o| o["id"] == observation["observation_id"])
        );
        assert_ne!(
            observation["origin"]["id"],
            observation["effective_observation_version_id"]
        );
    }
    assert_eq!(get(&app, "/api/state").await, state);
}
