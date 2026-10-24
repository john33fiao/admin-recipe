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
const COOKIE: &str = "hansang_4321=stocktake-test-session";
const MISSING_ID: &str = "00000000-0000-4000-8000-000000000001";

fn app() -> (tempfile::TempDir, Router) {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::new(
        Store::open(directory.path()).unwrap(),
        HOST.into(),
        "stocktake-test-session".into(),
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
    json!({"request_id":uuid::Uuid::new_v4().to_string(),"epoch":state["epoch"],"expected_revision":state["revision"]})
}

fn row(quantity: &str) -> Value {
    json!({"material_id":null,"material_version_id":null,"quantity":quantity,"unit":"g","initial_value":null,"excess":null})
}

fn draft(state: &Value) -> Value {
    json!({"meta":meta(state),"corrects_confirmation_id":null,"payload":{
        "kind":"stocktake","version":1,"data":{"rows":[row(""),row("0")],"note":"가상 실사 초안"}}})
}

async fn created(app: &Router, path: &str, input: &Value) -> Value {
    let response = send(app, "POST", path, Some(input)).await;
    let status = response.status();
    let body = value(response).await;
    assert_eq!(status, StatusCode::CREATED, "{path}: {body}");
    body
}

#[tokio::test]
async fn stocktake_list_and_detail_have_strict_filters() {
    let (_directory, app) = app();
    let list = get(&app, "/api/stocktakes").await;
    assert_eq!(list["items"], json!([]));
    assert!(list["state"]["epoch"].is_string());
    assert!(list["state"]["revision"].is_string());
    assert_eq!(
        get(&app, &format!("/api/stocktakes?material_id={MISSING_ID}")).await["items"],
        json!([])
    );
    for path in [
        "/api/stocktakes?material_id=bad",
        "/api/stocktakes?extra=true",
    ] {
        let response = send(&app, "GET", path, None).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}");
        assert_eq!(value(response).await["code"], "validation");
    }
    let response = send(&app, "GET", &format!("/api/stocktakes/{MISSING_ID}"), None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(value(response).await["code"], "not_found");
}

#[tokio::test]
async fn stocktake_spa_allows_get_and_head_only_for_documented_paths() {
    let (_directory, app) = app();
    for path in [
        "/stocktakes".to_owned(),
        format!("/stocktakes/{MISSING_ID}"),
    ] {
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
        "/stocktakes/new",
        "/stocktakes/not-a-uuid",
        "/api/stocktakes/extra/path",
    ] {
        assert_eq!(
            send(&app, "GET", path, None).await.status(),
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
}

#[tokio::test]
async fn stocktake_kind_can_be_filtered_without_changing_state() {
    let (_directory, app) = app();
    let before = get(&app, "/api/state").await;
    assert_eq!(
        get(&app, "/api/business?kind=stocktake&status=all").await["items"],
        json!([])
    );
    assert_eq!(get(&app, "/api/state").await, before);
}

#[tokio::test]
async fn stocktake_draft_preserves_blank_and_zero_as_different_inputs() {
    let (_directory, app) = app();
    let input = draft(&get(&app, "/api/state").await);
    let saved = created(&app, "/api/business/drafts", &input).await;
    assert_eq!(saved["result"]["head"]["payload"], input["payload"]);
    assert_eq!(saved["result"]["item"]["kind"], "stocktake");
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
    assert_eq!(created(&app, "/api/business/drafts", &input).await, saved);
    assert_eq!(get(&app, "/api/state").await, before_retry);
    assert_eq!(get(&app, "/api/stocktakes").await["items"], json!([]));
}

#[tokio::test]
async fn stocktake_alias_shares_create_identity_and_accepts_revision() {
    let (_directory, app) = app();
    let input = draft(&get(&app, "/api/state").await);
    let saved = created(&app, "/api/stocktake-drafts", &input).await;
    let before_retry = get(&app, "/api/state").await;
    assert_eq!(created(&app, "/api/business/drafts", &input).await, saved);
    assert_eq!(get(&app, "/api/state").await, before_retry);
    let id = saved["result"]["item"]["id"].as_str().unwrap();
    let revision = json!({"meta":meta(&before_retry),"expected_head_id":saved["result"]["head"]["id"],
        "payload":{"kind":"stocktake","version":1,"data":{"rows":[],"note":"가상 수정"}}});
    let changed = created(
        &app,
        &format!("/api/stocktake-drafts/{id}/revisions"),
        &revision,
    )
    .await;
    assert_eq!(changed["result"]["head"]["payload"], revision["payload"]);
}

#[tokio::test]
async fn stocktake_preserves_existing_security_and_common_body_limit() {
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
                        .body(Body::from(draft(&before).to_string()))
                        .unwrap()
                )
                .await
                .unwrap()
                .status(),
            expected
        );
    }
    let oversized = json!({"extra":"x".repeat(17 * 1024)});
    for path in [
        "/api/business/drafts".to_owned(),
        format!("/api/business/{MISSING_ID}/draft-revisions"),
        format!("/api/business/{MISSING_ID}/preview"),
        format!("/api/business/{MISSING_ID}/confirm"),
    ] {
        assert_eq!(
            send(&app, "POST", &path, Some(&oversized)).await.status(),
            StatusCode::PAYLOAD_TOO_LARGE,
            "{path}"
        );
    }
    assert_eq!(get(&app, "/api/state").await, before);
}

async fn material(app: &Router, name: &str) -> Value {
    let mut input = meta(&get(app, "/api/state").await);
    input["name"] = json!(name);
    input["unit"] = json!("g");
    input["kind"] = json!("raw");
    input["description"] = json!("");
    created(app, "/api/materials", &input).await
}

async fn business(app: &Router, kind: &str, data: Value) -> Value {
    created(
        app,
        "/api/business/drafts",
        &json!({"meta":meta(&get(app,"/api/state").await),"corrects_confirmation_id":null,
        "payload":{"kind":kind,"version":1,"data":data}}),
    )
    .await
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

async fn receipt(app: &Router, item: &Value, quantity: &str, amount: &str) {
    let draft=business(app,"receipt",json!({"source":{"kind":"direct","material_id":item["id"],"material_version_id":item["version_id"]},
        "quantity":quantity,"unit":"g","amount":amount,"receipt_date":"2020-02-29","note":"가상 실사 입고","stock_effect":{"kind":"unreflected"}})).await;
    confirm(app, &draft).await;
}

fn measured(item: &Value, quantity: &str) -> Value {
    let mut input = row(quantity);
    input["material_id"] = item["id"].clone();
    input["material_version_id"] = item["version_id"].clone();
    input
}

async fn sale(
    app: &Router,
    ingredient: &Value,
    name: &str,
    count: &str,
    recipe_quantity: &str,
) -> Value {
    let recipe=created(app,"/api/recipes",&json!({"meta":meta(&get(app,"/api/state").await),
        "owner":{"mode":"new","name":name,"unit":"개","kind":"menu","description":""},
        "data":{"basis_count":"1","reference_output":null,"selling_price":"1000","cooking_description":"","process":"","reference_time":"",
            "lines":[{"material_id":ingredient["id"],"material_version_id":ingredient["version_id"],"quantity":recipe_quantity,"unit":"g","note":""}]}})).await;
    let draft=business(app,"sale",json!({"material_id":recipe["item"]["id"],"material_version_id":recipe["item"]["version_id"],
        "recipe_version_id":recipe["version"]["id"],"count":count,"note":"가상 판매"})).await;
    confirm(app, &draft).await
}

async fn holding(app: &Router, item: &Value) -> Value {
    get(
        app,
        &format!("/api/holdings/{}", item["id"].as_str().unwrap()),
    )
    .await
}

#[tokio::test]
async fn actual_batch_settles_consumption_excluding_waste_and_keeps_unchecked_material() {
    let (_directory, app) = app();
    let ingredient = material(&app, "가상 실사 원물").await;
    let zero = material(&app, "가상 처음 확인0").await;
    let unchecked = material(&app, "가상 미확인").await;
    receipt(&app, &ingredient, "10000", "10000").await;
    let waste=business(&app,"waste",json!({"material_id":ingredient["id"],"material_version_id":ingredient["version_id"],"quantity":"1000","unit":"g","reason":"discard","note":""})).await;
    confirm(&app, &waste).await;
    let sale_a = sale(&app, &ingredient, "가상 메뉴 A", "3", "0").await;
    let sale_b = sale(&app, &ingredient, "가상 메뉴 B", "1", "500").await;
    assert_eq!(
        holding(&app, &ingredient).await["settlement_basis"]["quantity"],
        "9000"
    );
    let mut first_zero = measured(&zero, "0");
    first_zero["initial_value"] = json!({"kind":"manual_amount","amount":"0"});
    let draft=business(&app,"stocktake",json!({"rows":[measured(&ingredient,"6000"),first_zero,measured(&unchecked," ")],"note":"가상 일괄 실사"})).await;
    let check = preview(&app, &draft).await;
    assert_eq!(check["can_confirm"], true, "{check}");
    assert_eq!(check["effect"]["unchecked_positions"], json!([2]));
    let calc = &check["effect"]["rows"][0]["calculation"];
    assert_eq!(calc["mode"], "settlement");
    assert_eq!(calc["consumption_quantity"], "3000");
    assert_eq!(calc["consumption_value"], "3333.333333");
    assert_eq!(calc["allocation"]["candidates"][0]["quantity"], "2250");
    assert_eq!(calc["allocation"]["candidates"][1]["quantity"], "750");
    assert_eq!(check["effect"]["rows"][1]["calculation"]["mode"], "initial");
    assert!(
        check["effect"]["rows"][1]["calculation"]
            .get("consumption_quantity")
            .is_none()
    );
    let confirm_input = json!({"meta":meta(&check["state"]),"expected_head_id":check["head_id"]});
    let saved = created(&app, &confirm_path(&draft), &confirm_input).await;
    let stocktake_id = saved["result"]["confirmation"]["result"]["stocktake_id"]
        .as_str()
        .unwrap();
    let detail = get(&app, &format!("/api/stocktakes/{stocktake_id}")).await;
    assert_eq!(detail["observations"].as_array().unwrap().len(), 2);
    let current = holding(&app, &ingredient).await;
    assert_eq!(current["settlement_basis"]["quantity"], "6000");
    assert_eq!(current["settlement_basis"]["value"], "6666.666667");
    assert_eq!(current["basis"]["kind"], "observation");
    assert_eq!(current["observation"]["quantity"], "6000");
    assert_eq!(current["confirmed_changes"], json!([]));
    let zero_holding = holding(&app, &zero).await;
    assert_eq!(zero_holding["settlement_basis"]["quantity"], "0");
    assert!(zero_holding["last_valid_price"].is_null());
    assert!(holding(&app, &unchecked).await["settlement_basis"].is_null());
    for (sale, quantity) in [(&sale_a, "2250"), (&sale_b, "750")] {
        let id = sale["result"]["confirmation"]["result"]["sale_id"]
            .as_str()
            .unwrap();
        let detail = get(&app, &format!("/api/sales/{id}")).await;
        assert_eq!(detail["consumption_status"], "settled");
        assert_eq!(detail["consumption"][0]["quantity"], quantity);
        assert_eq!(detail["consumption"][0]["stocktake_id"], stocktake_id);
    }
    let before_retry = get(&app, "/api/state").await;
    assert_eq!(
        created(&app, &confirm_path(&draft), &confirm_input).await,
        saved
    );
    assert_eq!(get(&app, "/api/state").await, before_retry);
    receipt(&app, &ingredient, "1000", "1000").await;
    let second = business(
        &app,
        "stocktake",
        json!({"rows":[measured(&ingredient,"7000")],"note":"가상 재실사"}),
    )
    .await;
    let check = preview(&app, &second).await;
    assert_eq!(
        check["effect"]["rows"][0]["calculation"]["consumption_quantity"],
        "0"
    );
    confirm(&app, &second).await;
    assert_eq!(
        holding(&app, &ingredient).await["settlement_basis"]["value"],
        "7666.666667"
    );
}

#[tokio::test]
async fn real_stocktake_reconciles_a_preserved_production_and_stale_preview_has_no_effect() {
    let (_directory, app) = app();
    let raw = material(&app, "가상 생산 보정 원물").await;
    let output = material(&app, "가상 생산 결과").await;
    receipt(&app, &raw, "1000", "10000").await;
    let production=business(&app,"production",json!({"inputs":[{"material_id":raw["id"],"material_version_id":raw["version_id"],"quantity":"1.2","unit":"kg"}],
        "outputs":[{"material_id":output["id"],"material_version_id":output["version_id"],"quantity":"1000","unit":"g","allocation_percent":"100"}],"note":"가상 부족 입력 보존"})).await;
    let stale = preview(&app, &production).await;
    assert_eq!(stale["can_confirm"], false);
    let mut row = measured(&raw, "1200");
    row["excess"] =
        json!({"reviewed":true,"reason":"가상 누락 여부 확인","value":{"kind":"existing_price"}});
    let correction = business(
        &app,
        "stocktake",
        json!({"rows":[row],"note":"가상 실제 실사"}),
    )
    .await;
    confirm(&app, &correction).await;
    let response = send(
        &app,
        "POST",
        &confirm_path(&production),
        Some(&json!({"meta":meta(&stale["state"]),"expected_head_id":stale["head_id"]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(value(response).await["code"], "stale_state");
    assert_eq!(get(&app, "/api/productions").await["items"], json!([]));
    let before = holding(&app, &raw).await;
    let check = preview(&app, &production).await;
    assert_eq!(check["can_confirm"], true, "{check}");
    let effect = check["effect"]["materials"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["material_id"] == raw["id"])
        .unwrap();
    assert_eq!(effect["observation"]["id"], before["observation"]["id"]);
    assert_eq!(
        effect["before"]["source_id"],
        before["settlement_basis"]["source_id"]
    );
    assert_eq!(check["effect"]["inputs"][0]["input_quantity"], "1.2");
    confirm(&app, &production).await;
    assert_eq!(
        holding(&app, &raw).await["settlement_basis"]["quantity"],
        "0"
    );
    assert_eq!(
        holding(&app, &output).await["settlement_basis"]["value"],
        "12000"
    );
    assert_eq!(
        get(&app, "/api/receipts").await["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn stocktake_large_alias_is_local_strict_and_cannot_bypass_security() {
    let (_directory, app) = app();
    let state = get(&app, "/api/state").await;
    let mut input = draft(&state);
    input["payload"]["data"]["rows"] = json!(
        (0..100)
            .map(|_| {
                let mut r = row("");
                r["excess"] = json!({"reviewed":false,"reason":"x".repeat(180),"value":null});
                r
            })
            .collect::<Vec<_>>()
    );
    assert!(input.to_string().len() > 16 * 1024);
    assert_eq!(
        send(&app, "POST", "/api/business/drafts", Some(&input))
            .await
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let saved = created(&app, "/api/stocktake-drafts", &input).await;
    assert_eq!(saved["result"]["head"]["payload"], input["payload"]);
    let too_large = json!({"extra":"x".repeat(257*1024)});
    assert_eq!(
        send(&app, "POST", "/api/stocktake-drafts", Some(&too_large))
            .await
            .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let wrong = json!({"meta":meta(&get(&app,"/api/state").await),"corrects_confirmation_id":null,"payload":{"kind":"waste","version":1,"data":{"material_id":null,"material_version_id":null,"quantity":"","unit":"g","reason":"discard","note":""}}});
    assert_eq!(
        send(&app, "POST", "/api/stocktake-drafts", Some(&wrong))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    for (cookie, origin, expected) in [
        (
            Some(COOKIE),
            "https://foreign.example",
            StatusCode::FORBIDDEN,
        ),
        (None, ORIGIN, StatusCode::UNAUTHORIZED),
    ] {
        let mut request = Request::builder()
            .method("POST")
            .uri("/api/stocktake-drafts")
            .header("host", HOST)
            .header("origin", origin)
            .header("content-type", "application/json");
        if let Some(cookie) = cookie {
            request = request.header("cookie", cookie);
        }
        assert_eq!(
            app.clone()
                .oneshot(request.body(Body::from(input.to_string())).unwrap())
                .await
                .unwrap()
                .status(),
            expected
        );
    }
}

#[tokio::test]
async fn invalid_batch_blocks_every_row_and_zero_weight_sales_keep_unallocated_evidence() {
    let (_directory, app) = app();
    let item = material(&app, "가상 미배분 근거").await;
    receipt(&app, &item, "1000", "100").await;
    let zero_sale = sale(&app, &item, "가상0개 판매", "0", "0").await;
    let bad = business(
        &app,
        "stocktake",
        json!({"rows":[measured(&item,"500"),measured(&item,"abc")],"note":""}),
    )
    .await;
    let check = preview(&app, &bad).await;
    assert_eq!(check["can_confirm"], false);
    let response = send(
        &app,
        "POST",
        &confirm_path(&bad),
        Some(&json!({"meta":meta(&check["state"]),"expected_head_id":check["head_id"]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(get(&app, "/api/stocktakes").await["items"], json!([]));
    assert_eq!(
        holding(&app, &item).await["settlement_basis"]["quantity"],
        "1000"
    );
    let good = business(
        &app,
        "stocktake",
        json!({"rows":[measured(&item,"500")],"note":""}),
    )
    .await;
    let check = preview(&app, &good).await;
    let allocation = &check["effect"]["rows"][0]["calculation"]["allocation"];
    assert_eq!(allocation["reason"], "zero_weight");
    assert_eq!(allocation["unallocated_quantity"], "500");
    assert!(allocation["candidates"][0]["quantity"].is_null());
    confirm(&app, &good).await;
    let sale_id = zero_sale["result"]["confirmation"]["result"]["sale_id"]
        .as_str()
        .unwrap();
    let before = get(&app, &format!("/api/sales/{sale_id}")).await;
    assert_eq!(before["consumption"][0]["status"], "unallocated");
    let mut payload = zero_sale["result"]["head"]["payload"].clone();
    payload["data"]["count"] = json!("3");
    payload["data"]["note"] = json!("가상 정정 이유");
    let correction=created(&app,"/api/business/drafts",&json!({"meta":meta(&get(&app,"/api/state").await),"corrects_confirmation_id":zero_sale["result"]["confirmation"]["id"],"payload":payload})).await;
    let check = preview(&app, &correction).await;
    assert_eq!(check["can_confirm"], false);
    assert!(
        check["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["code"] == "requires_allocation_correction")
    );
    let response = send(
        &app,
        "POST",
        &confirm_path(&correction),
        Some(&json!({"meta":meta(&check["state"]),"expected_head_id":check["head_id"]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        get(&app, &format!("/api/sales/{sale_id}")).await["head"],
        before["head"]
    );
    assert_eq!(
        holding(&app, &item).await["settlement_basis"]["value"],
        "50"
    );
}
