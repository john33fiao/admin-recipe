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
const COOKIE: &str = "hansang_4321=sale-test-session";
const MISSING_ID: &str = "00000000-0000-4000-8000-000000000001";

fn app() -> (tempfile::TempDir, Router) {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::new(
        Store::open(directory.path()).unwrap(),
        HOST.into(),
        "sale-test-session".into(),
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

fn meta(state: &Value) -> Value {
    json!({"request_id":uuid::Uuid::new_v4().to_string(),
        "epoch":state["epoch"],"expected_revision":state["revision"]})
}

fn draft_input(state: &Value, count: &str) -> Value {
    json!({"meta":meta(state),"corrects_confirmation_id":null,
        "payload":{"kind":"sale","version":1,"data":{
            "material_id":null,"material_version_id":null,"recipe_version_id":null,
            "count":count,"note":"가상 판매 HTTP 초안"}}})
}

async fn created(app: &Router, path: &str, input: &Value) -> Value {
    let response = send(app, "POST", path, Some(input)).await;
    let status = response.status();
    let body = value(response).await;
    assert_eq!(status, StatusCode::CREATED, "{path}: {body}");
    body
}

async fn material(app: &Router, name: &str) -> Value {
    let mut input = meta(&get(app, "/api/state").await);
    input["name"] = json!(name);
    input["unit"] = json!("g");
    input["kind"] = json!("raw");
    input["description"] = json!("");
    created(app, "/api/materials", &input).await
}

async fn price(app: &Router, item: &Value, amount: &str) -> Value {
    let mut input = meta(&get(app, "/api/state").await);
    input["expected_version_id"] = item["version_id"].clone();
    input["quantity"] = json!("1");
    input["unit"] = json!("g");
    input["amount"] = json!(amount);
    created(
        app,
        &format!("/api/materials/{}/prices", item["id"].as_str().unwrap()),
        &input,
    )
    .await
}

fn line(item: &Value, quantity: &str) -> Value {
    json!({"material_id":item["id"],"material_version_id":item["version_id"],"quantity":quantity,"unit":"g","note":"가상 배합"})
}

fn recipe_data(selling_price: Option<&str>, basis: &str, lines: Vec<Value>) -> Value {
    json!({"basis_count":basis,"reference_output":null,"selling_price":selling_price,
        "cooking_description":"","process":"","reference_time":"","lines":lines})
}

async fn recipe(app: &Router, name: &str, data: &Value) -> Value {
    created(
        app,
        "/api/recipes",
        &json!({"meta":meta(&get(app,"/api/state").await),
        "owner":{"mode":"new","name":name,"unit":"개","kind":"menu","description":""},"data":data}),
    )
    .await
}

fn selected_sale(state: &Value, recipe: &Value, count: &str) -> Value {
    let mut input = draft_input(state, count);
    input["payload"]["data"]["material_id"] = recipe["item"]["id"].clone();
    input["payload"]["data"]["material_version_id"] = recipe["item"]["version_id"].clone();
    input["payload"]["data"]["recipe_version_id"] = recipe["version"]["id"].clone();
    input
}

async fn preview(app: &Router, draft: &Value) -> Value {
    let path = format!(
        "/api/business/{}/preview",
        draft["result"]["item"]["id"].as_str().unwrap()
    );
    let response = send(
        app,
        "POST",
        &path,
        Some(&json!({"expected_head_id":draft["result"]["head"]["id"]})),
    )
    .await;
    let status = response.status();
    let body = value(response).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

fn confirm_path(draft: &Value) -> String {
    format!(
        "/api/business/{}/confirm",
        draft["result"]["item"]["id"].as_str().unwrap()
    )
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

fn without_state(mut body: Value) -> Value {
    body.as_object_mut().unwrap().remove("state");
    body
}

async fn rejected(app: &Router, path: &str, input: &Value, expected: StatusCode, code: &str) {
    let response = send(app, "POST", path, Some(input)).await;
    let status = response.status();
    let body = value(response).await;
    assert_eq!(status, expected, "{path}: {body}");
    assert_eq!(body["code"], code, "{body}");
}

#[tokio::test]
async fn sale_list_has_strict_filter_missing_detail_and_known_revenue_subtotal() {
    let (_directory, app) = app();
    let list = get(&app, "/api/sales").await;
    assert_eq!(list["items"], json!([]));
    assert_eq!(list["known_revenue"], "0");
    assert_eq!(list["missing_revenue_count"], 0);
    assert!(list["state"]["epoch"].is_string());
    assert!(list["state"]["revision"].is_string());
    assert_eq!(
        get(&app, &format!("/api/sales?material_id={MISSING_ID}")).await["items"],
        json!([])
    );
    for path in [
        "/api/sales?material_id=bad",
        "/api/sales?extra=true",
        "/api/sales?material_id=bad&material_id=bad",
    ] {
        let response = send(&app, "GET", path, None).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}");
        assert_eq!(value(response).await["code"], "validation");
    }
    let response = send(&app, "GET", &format!("/api/sales/{MISSING_ID}"), None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(value(response).await["code"], "not_found");
}

#[tokio::test]
async fn sale_spa_routes_allow_get_head_and_preserve_unknown_path_rejection() {
    let (_directory, app) = app();
    for path in ["/sales".to_owned(), format!("/sales/{MISSING_ID}")] {
        for method in ["GET", "HEAD"] {
            let response = send(&app, method, &path, None).await;
            assert_eq!(response.status(), StatusCode::OK, "{method} {path}");
            assert!(
                response.headers()["content-type"]
                    .to_str()
                    .unwrap()
                    .contains("text/html")
            );
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(bytes.is_empty(), method == "HEAD");
        }
    }
    for path in [
        "/sales/not-a-uuid".to_owned(),
        format!("/sales/{MISSING_ID}/extra"),
        "/api/sales/extra/path".to_owned(),
    ] {
        assert_eq!(
            send(&app, "GET", &path, None).await.status(),
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
}

#[tokio::test]
async fn sale_is_a_supported_business_filter_without_creating_records() {
    let (_directory, app) = app();
    let before = get(&app, "/api/state").await;
    for path in [
        "/api/business?kind=sale",
        "/api/business?kind=sale&status=all",
    ] {
        assert_eq!(get(&app, path).await["items"], json!([]));
    }
    assert_eq!(get(&app, "/api/state").await, before);
}

#[tokio::test]
async fn sale_drafts_preserve_empty_and_zero_counts_with_original_response_replay() {
    let (_directory, app) = app();
    for count in ["", "0"] {
        let input = draft_input(&get(&app, "/api/state").await, count);
        let response = send(&app, "POST", "/api/business/drafts", Some(&input)).await;
        let status = response.status();
        let saved = value(response).await;
        assert_eq!(status, StatusCode::CREATED, "{saved}");
        assert_eq!(saved["result"]["head"]["payload"], input["payload"]);
        assert_eq!(saved["result"]["item"]["kind"], "sale");
        assert_eq!(saved["result"]["item"]["status"], "draft");
        assert!(saved["result"]["confirmation"].is_null());
        assert!(!saved["result"]["issues"].as_array().unwrap().is_empty());
        let id = saved["result"]["item"]["id"].as_str().unwrap();
        assert_eq!(
            get(&app, &format!("/api/business/{id}")).await["head"]["payload"],
            input["payload"]
        );
        let check = send(
            &app,
            "POST",
            &format!("/api/business/{id}/preview"),
            Some(&json!({"expected_head_id":saved["result"]["head"]["id"]})),
        )
        .await;
        assert_eq!(check.status(), StatusCode::OK);
        assert_eq!(value(check).await["can_confirm"], false);
        let before_retry = get(&app, "/api/state").await;
        let retry = send(&app, "POST", "/api/business/drafts", Some(&input)).await;
        assert_eq!(retry.status(), StatusCode::CREATED);
        assert_eq!(value(retry).await, saved);
        assert_eq!(get(&app, "/api/state").await, before_retry);
        assert_eq!(get(&app, "/api/sales").await["items"], json!([]));
    }
}

#[tokio::test]
async fn sale_routes_preserve_session_host_origin_and_common_16_kib_limit() {
    let (_directory, app) = app();
    let before = get(&app, "/api/state").await;
    for (method, path) in [
        ("GET", "/api/sales"),
        ("POST", "/api/business/drafts"),
        (
            "POST",
            "/api/business/00000000-0000-4000-8000-000000000001/preview",
        ),
        (
            "POST",
            "/api/business/00000000-0000-4000-8000-000000000001/confirm",
        ),
    ] {
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
        ] {
            let mut request = Request::builder()
                .method(method)
                .uri(path)
                .header("host", host)
                .header("content-type", "application/json");
            if let Some(origin) = origin {
                request = request.header("origin", origin);
            }
            if let Some(cookie) = cookie {
                request = request.header("cookie", cookie);
            }
            let response = app
                .clone()
                .oneshot(request.body(Body::from("{}")).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), expected, "{method} {path}");
        }
        if method == "POST" {
            let request = Request::builder()
                .method(method)
                .uri(path)
                .header("host", HOST)
                .header("cookie", COOKIE)
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap();
            assert_eq!(
                app.clone().oneshot(request).await.unwrap().status(),
                StatusCode::FORBIDDEN,
                "missing Origin: {path}"
            );
        }
    }
    let oversized = json!({"extra":"x".repeat(17 * 1024)});
    for path in [
        "/api/business/drafts",
        "/api/business/00000000-0000-4000-8000-000000000001/draft-revisions",
        "/api/business/00000000-0000-4000-8000-000000000001/preview",
        "/api/business/00000000-0000-4000-8000-000000000001/confirm",
    ] {
        let response = send(&app, "POST", path, Some(&oversized)).await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE, "{path}");
        assert_eq!(value(response).await["code"], "validation");
    }
    assert_eq!(get(&app, "/api/state").await, before);
}

#[tokio::test]
async fn sale_freezes_recipe_cost_and_correction_keeps_original_references_and_sequence() {
    let (_directory, app) = app();
    let ingredient = material(&app, "가상 판매 원가 재료").await;
    let registered = price(&app, &ingredient, "10").await;
    let original_recipe = recipe(
        &app,
        "가상 분수 판매",
        &recipe_data(Some("1200"), "3", vec![line(&ingredient, "100")]),
    )
    .await;
    let holding_path = format!("/api/holdings/{}", ingredient["id"].as_str().unwrap());
    let holding_before = without_state(get(&app, &holding_path).await);
    let raw = selected_sale(&get(&app, "/api/state").await, &original_recipe, "3");
    let draft = created(&app, "/api/business/drafts", &raw).await;
    let check = preview(&app, &draft).await;
    assert_eq!(check["can_confirm"], true, "{check}");
    assert_eq!(check["effect"]["sale"]["amount"], "3600");
    assert_eq!(
        check["effect"]["sale"]["recipe_cost"]["total_cost"],
        "333.333333"
    );
    let confirm_input = json!({"meta":meta(&check["state"]),"expected_head_id":check["head_id"]});
    let confirmed = created(&app, &confirm_path(&draft), &confirm_input).await;
    let sale_id = confirmed["result"]["confirmation"]["result"]["sale_id"]
        .as_str()
        .unwrap();
    let detail_path = format!("/api/sales/{sale_id}");
    let original = get(&app, &detail_path).await;
    assert_eq!(original["head"]["count"], "3");
    assert_eq!(original["head"]["amount"], "3600");
    assert_eq!(original["consumption_status"], "unconfirmed");
    assert_eq!(
        original["item"]["recipe_cost"]["rows"][0]["material_name"],
        "가상 판매 원가 재료"
    );
    assert_eq!(
        original["item"]["recipe_cost"]["rows"][0]["price"]["source"],
        "preregistered"
    );
    assert_eq!(
        original["item"]["recipe_cost"]["rows"][0]["price"]["source_id"],
        registered["id"]
    );
    assert_eq!(
        without_state(get(&app, &holding_path).await),
        holding_before
    );
    assert_eq!(get(&app, "/api/receipts").await["items"], json!([]));
    assert_eq!(get(&app, "/api/productions").await["items"], json!([]));

    price(&app, &ingredient, "20").await;
    let owner = original_recipe["item"]["id"].as_str().unwrap();
    created(&app,&format!("/api/recipes/{owner}/versions"),&json!({"meta":meta(&get(&app,"/api/state").await),
        "expected_material_version_id":original_recipe["item"]["version_id"],"expected_recipe_version_id":original_recipe["version"]["id"],
        "data":recipe_data(Some("1500"),"3",vec![line(&ingredient,"200")])})).await;
    let mut rename = meta(&get(&app, "/api/state").await);
    rename["expected_version_id"] = ingredient["version_id"].clone();
    rename["name"] = json!("가상 변경된 재료 이름");
    rename["unit"] = json!("g");
    rename["kind"] = json!("raw");
    rename["description"] = json!("");
    created(
        &app,
        &format!(
            "/api/materials/{}/versions",
            ingredient["id"].as_str().unwrap()
        ),
        &rename,
    )
    .await;
    assert_eq!(get(&app, &detail_path).await["item"], original["item"]);
    assert_eq!(get(&app, &detail_path).await["head"], original["head"]);

    let holding_before_correction = without_state(get(&app, &holding_path).await);
    let mut correction = selected_sale(&get(&app, "/api/state").await, &original_recipe, "1");
    correction["corrects_confirmation_id"] = confirmed["result"]["confirmation"]["id"].clone();
    correction["payload"]["data"]["note"] = json!("가상 개수 오입력 정정");
    let correction_draft = created(&app, "/api/business/drafts", &correction).await;
    let corrected_preview = preview(&app, &correction_draft).await;
    assert_eq!(
        corrected_preview["can_confirm"], true,
        "{corrected_preview}"
    );
    assert_eq!(corrected_preview["effect"]["previous"]["count"], "3");
    assert_eq!(corrected_preview["effect"]["sale"]["amount"], "1200");
    assert_eq!(
        corrected_preview["effect"]["sale"]["recipe_cost"],
        original["item"]["recipe_cost"]
    );
    let corrected = confirm(&app, &correction_draft).await;
    assert_eq!(
        corrected["result"]["confirmation"]["result"]["sale_id"],
        sale_id
    );
    let detail = get(&app, &detail_path).await;
    assert_eq!(detail["item"], original["item"]);
    assert_eq!(detail["head"]["count"], "1");
    assert_eq!(
        detail["head"]["previous_version_id"],
        original["head"]["id"]
    );
    assert_eq!(detail["versions"].as_array().unwrap().len(), 2);
    assert_eq!(
        without_state(get(&app, &holding_path).await),
        holding_before_correction
    );
    assert_eq!(get(&app, "/api/sales").await["known_revenue"], "1200");
    assert_eq!(
        get(&app, &format!("/api/sales?material_id={owner}")).await["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        created(&app, &confirm_path(&draft), &confirm_input).await,
        confirmed
    );
    let mut conflict = confirm_input.clone();
    conflict["expected_head_id"] = json!(MISSING_ID);
    rejected(
        &app,
        &confirm_path(&draft),
        &conflict,
        StatusCode::CONFLICT,
        "request_conflict",
    )
    .await;
    correction["meta"] = meta(&get(&app, "/api/state").await);
    rejected(
        &app,
        "/api/business/drafts",
        &correction,
        StatusCode::CONFLICT,
        "stale_state",
    )
    .await;
}

#[tokio::test]
async fn valid_sales_keep_zero_null_and_aggregate_above_the_per_sale_money_bound() {
    let (_directory, app) = app();
    let maximum = recipe(
        &app,
        "가상 최대 단가 메뉴",
        &recipe_data(Some("9000000000000"), "1", vec![]),
    )
    .await;
    let mut confirmed_sales = Vec::new();
    for _ in 0..2 {
        let input = selected_sale(&get(&app, "/api/state").await, &maximum, "1");
        let draft = created(&app, "/api/business/drafts", &input).await;
        confirmed_sales.push(confirm(&app, &draft).await);
    }
    let ingredient = material(&app, "가상 가격 없는 재료").await;
    let unknown = recipe(
        &app,
        "가상 판매가 없는 메뉴",
        &recipe_data(
            None,
            "1",
            vec![line(&ingredient, "0"), line(&ingredient, "100")],
        ),
    )
    .await;
    let input = selected_sale(&get(&app, "/api/state").await, &unknown, "0");
    let draft = created(&app, "/api/business/drafts", &input).await;
    let confirmed = confirm(&app, &draft).await;
    let id = confirmed["result"]["confirmation"]["result"]["sale_id"]
        .as_str()
        .unwrap();
    let detail = get(&app, &format!("/api/sales/{id}")).await;
    assert_eq!(detail["head"]["count"], "0");
    assert!(detail["head"]["amount"].is_null());
    assert!(detail["item"]["recipe_cost"]["total_cost"].is_null());
    assert_eq!(detail["item"]["recipe_cost"]["rows"][0]["cost"], "0");
    assert!(detail["item"]["recipe_cost"]["rows"][1]["cost"].is_null());
    assert_eq!(
        detail["item"]["recipe_cost"]["missing_price_line_ids"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let zero = recipe(&app, "가상 0원 메뉴", &recipe_data(Some("0"), "1", vec![])).await;
    let input = selected_sale(&get(&app, "/api/state").await, &zero, "1000000000");
    let draft = created(&app, "/api/business/drafts", &input).await;
    confirm(&app, &draft).await;
    let list = get(&app, "/api/sales").await;
    assert_eq!(list["items"].as_array().unwrap().len(), 4);
    assert_eq!(list["known_revenue"], "18000000000000");
    assert_eq!(list["missing_revenue_count"], 1);
    let mut correction = selected_sale(&get(&app, "/api/state").await, &maximum, "0");
    correction["corrects_confirmation_id"] =
        confirmed_sales[0]["result"]["confirmation"]["id"].clone();
    correction["payload"]["data"]["note"] = json!("가상 0개 정정");
    let draft = created(&app, "/api/business/drafts", &correction).await;
    confirm(&app, &draft).await;
    let list = get(&app, "/api/sales").await;
    assert_eq!(list["items"].as_array().unwrap().len(), 4);
    assert_eq!(list["known_revenue"], "9000000000000");
    assert_eq!(list["missing_revenue_count"], 1);
    assert!(
        get(
            &app,
            &format!("/api/holdings/{}", ingredient["id"].as_str().unwrap())
        )
        .await["basis"]
            .is_null()
    );
}

#[tokio::test]
async fn sale_stale_confirm_is_unprocessed_and_payload_conflict_keeps_only_original() {
    let (_directory, app) = app();
    let recipe = recipe(
        &app,
        "가상 동시 판매",
        &recipe_data(Some("10"), "1", vec![]),
    )
    .await;
    let input = selected_sale(&get(&app, "/api/state").await, &recipe, "1");
    let draft = created(&app, "/api/business/drafts", &input).await;
    let check = preview(&app, &draft).await;
    let stale = json!({"meta":meta(&check["state"]),"expected_head_id":check["head_id"]});
    material(&app, "가상 다른 탭 변경").await;
    rejected(
        &app,
        &confirm_path(&draft),
        &stale,
        StatusCode::CONFLICT,
        "stale_state",
    )
    .await;
    assert_eq!(get(&app, "/api/sales").await["items"], json!([]));
    let confirmed = confirm(&app, &draft).await;
    let mut changed = input.clone();
    changed["payload"]["data"]["count"] = json!("2");
    rejected(
        &app,
        "/api/business/drafts",
        &changed,
        StatusCode::CONFLICT,
        "request_conflict",
    )
    .await;
    assert_eq!(created(&app, "/api/business/drafts", &input).await, draft);
    assert_eq!(
        get(&app, "/api/sales").await["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let id = confirmed["result"]["confirmation"]["result"]["sale_id"]
        .as_str()
        .unwrap();
    assert_eq!(
        get(&app, &format!("/api/sales/{id}")).await["head"]["count"],
        "1"
    );
}

#[tokio::test]
async fn sale_rejects_numeric_unknown_input_and_keeps_invalid_semantic_drafts() {
    let (_directory, app) = app();
    let before = get(&app, "/api/state").await;
    let base = draft_input(&before, "0");
    let mut numeric = base.clone();
    numeric["payload"]["data"]["count"] = json!(0);
    let mut unknown = base.clone();
    unknown["payload"]["data"]["inventory_delta"] = json!("0");
    let mut future = base;
    future["payload"]["version"] = json!(2);
    for input in [numeric, unknown, future] {
        rejected(
            &app,
            "/api/business/drafts",
            &input,
            StatusCode::BAD_REQUEST,
            "validation",
        )
        .await;
    }
    assert_eq!(get(&app, "/api/state").await, before);
    let recipe = recipe(&app, "가상 개수 검증", &recipe_data(Some("1"), "1", vec![])).await;
    for count in ["-1", "1.5", "1000000001"] {
        let input = selected_sale(&get(&app, "/api/state").await, &recipe, count);
        let draft = created(&app, "/api/business/drafts", &input).await;
        let check = preview(&app, &draft).await;
        assert_eq!(check["can_confirm"], false, "{check}");
        assert_eq!(draft["result"]["head"]["payload"]["data"]["count"], count);
        rejected(
            &app,
            &confirm_path(&draft),
            &json!({"meta":meta(&check["state"]),"expected_head_id":check["head_id"]}),
            StatusCode::BAD_REQUEST,
            "validation",
        )
        .await;
    }
    assert_eq!(get(&app, "/api/sales").await["items"], json!([]));
}
