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
const COOKIE: &str = "hansang_4321=production-test-session";
const MISSING_ID: &str = "00000000-0000-4000-8000-000000000001";

fn app() -> (tempfile::TempDir, Router) {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::new(
        Store::open(directory.path()).unwrap(),
        HOST.into(),
        "production-test-session".into(),
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

async fn created(app: &Router, path: &str, input: &Value) -> Value {
    let response = send(app, "POST", path, Some(input)).await;
    let status = response.status();
    let body = value(response).await;
    assert_eq!(status, StatusCode::CREATED, "{path}: {body}");
    body
}

fn data() -> Value {
    json!({"inputs":[{"material_id":null,"material_version_id":null,"quantity":"","unit":"g"}],
        "outputs":[{"material_id":null,"material_version_id":null,"quantity":"","unit":"g","allocation_percent":"100"}],
        "note":"가상 생산 HTTP 초안"})
}

fn draft_input(state: &Value, data: Value) -> Value {
    json!({"meta":meta(state),"corrects_confirmation_id":null,
        "payload":{"kind":"production","version":1,"data":data}})
}

async fn material(app: &Router, name: &str, kind: &str) -> Value {
    let mut input = meta(&get(app, "/api/state").await);
    input["name"] = json!(name);
    input["unit"] = json!("g");
    input["kind"] = json!(kind);
    input["description"] = json!("");
    created(app, "/api/materials", &input).await
}

async fn preview(app: &Router, saved: &Value) -> Value {
    let path = format!(
        "/api/business/{}/preview",
        saved["result"]["item"]["id"].as_str().unwrap()
    );
    let response = send(
        app,
        "POST",
        &path,
        Some(&json!({"expected_head_id":saved["result"]["head"]["id"]})),
    )
    .await;
    let status = response.status();
    let body = value(response).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

fn confirm_path(saved: &Value) -> String {
    format!(
        "/api/business/{}/confirm",
        saved["result"]["item"]["id"].as_str().unwrap()
    )
}

async fn receipt(app: &Router, material: &Value, quantity: &str, amount: &str) -> Value {
    let input = json!({"meta":meta(&get(app,"/api/state").await), "corrects_confirmation_id":null,
        "payload":{"kind":"receipt","version":1,"data":{
            "source":{"kind":"direct","material_id":material["id"],"material_version_id":material["version_id"]},
            "quantity":quantity,"unit":"kg","amount":amount,"receipt_date":"2020-02-29","note":"가상 생산 HTTP 입고",
            "stock_effect":{"kind":"unreflected"}}}});
    let draft = created(app, "/api/business/drafts", &input).await;
    let check = preview(app, &draft).await;
    assert_eq!(check["can_confirm"], true, "{check}");
    created(
        app,
        &confirm_path(&draft),
        &json!({"meta":meta(&check["state"]),"expected_head_id":check["head_id"]}),
    )
    .await
}

fn measured_input(material: &Value, quantity: &str, unit: &str) -> Value {
    json!({"material_id":material["id"],"material_version_id":material["version_id"],"quantity":quantity,"unit":unit})
}

fn measured_output(material: &Value, quantity: &str, percent: &str) -> Value {
    let mut row = measured_input(material, quantity, "g");
    row["allocation_percent"] = json!(percent);
    row
}

async fn rejected(app: &Router, path: &str, input: &Value, status: StatusCode, code: &str) {
    let response = send(app, "POST", path, Some(input)).await;
    let actual = response.status();
    let body = value(response).await;
    assert_eq!(actual, status, "{path}: {body}");
    assert_eq!(body["code"], code, "{body}");
}

#[tokio::test]
async fn production_list_has_strict_filter_and_explicit_missing_detail() {
    let (_directory, app) = app();
    let list = get(&app, "/api/productions").await;
    assert_eq!(list["items"], json!([]));
    assert!(list["state"]["epoch"].is_string());
    assert!(list["state"]["revision"].is_string());
    assert_eq!(
        get(&app, &format!("/api/productions?material_id={MISSING_ID}")).await["items"],
        json!([])
    );
    for path in [
        "/api/productions?material_id=bad",
        "/api/productions?extra=true",
    ] {
        let response = send(&app, "GET", path, None).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}");
        assert_eq!(value(response).await["code"], "validation");
    }
    let response = send(&app, "GET", &format!("/api/productions/{MISSING_ID}"), None).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(value(response).await["code"], "not_found");
}

#[tokio::test]
async fn production_spa_routes_allow_get_head_and_reject_unknown_paths() {
    let (_directory, app) = app();
    for path in [
        "/productions".to_owned(),
        format!("/productions/{MISSING_ID}"),
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
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(bytes.is_empty(), method == "HEAD");
        }
    }
    for path in [
        "/productions/not-a-uuid".to_owned(),
        format!("/productions/{MISSING_ID}/extra"),
        "/api/productions/extra/path".to_owned(),
    ] {
        assert_eq!(
            send(&app, "GET", &path, None).await.status(),
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
}

#[tokio::test]
async fn generic_business_route_preserves_incomplete_production_draft() {
    let (_directory, app) = app();
    let raw = data();
    let input = draft_input(&get(&app, "/api/state").await, raw.clone());
    let saved = created(&app, "/api/business/drafts", &input).await;
    assert_eq!(saved["result"]["head"]["payload"]["data"], raw);
    assert_eq!(saved["result"]["item"]["kind"], "production");
    assert_eq!(saved["result"]["item"]["status"], "draft");
    assert!(saved["result"]["confirmation"].is_null());
    assert!(!saved["result"]["issues"].as_array().unwrap().is_empty());
    let path = format!(
        "/api/business/{}",
        saved["result"]["item"]["id"].as_str().unwrap()
    );
    assert_eq!(get(&app, &path).await["head"]["payload"]["data"], raw);
    assert_eq!(get(&app, "/api/productions").await["items"], json!([]));
}

#[tokio::test]
async fn production_alias_accepts_large_draft_and_revisions_without_widening_common_route() {
    let (_directory, app) = app();
    let mut raw = data();
    raw["inputs"] = json!(
        (0..100)
            .map(|_| raw["inputs"][0].clone())
            .collect::<Vec<_>>()
    );
    raw["outputs"] = json!(
        (0..100)
            .map(|position| {
                let mut row = raw["outputs"][0].clone();
                row["allocation_percent"] = json!(if position == 0 { "100" } else { "0" });
                row
            })
            .collect::<Vec<_>>()
    );
    raw["note"] = json!("가".repeat(2000));
    let input = draft_input(&get(&app, "/api/state").await, raw.clone());
    let bytes = input.to_string().len();
    assert!(bytes > 16 * 1024 && bytes < 256 * 1024, "{bytes}");
    let saved = created(&app, "/api/production-drafts", &input).await;
    assert_eq!(saved["result"]["head"]["payload"]["data"], raw);
    let id = saved["result"]["item"]["id"].as_str().unwrap();
    raw["inputs"][17]["quantity"] = json!("1.2");
    let revision = json!({"meta":meta(&saved["state"]),
        "expected_head_id":saved["result"]["head"]["id"],
        "payload":{"kind":"production","version":1,"data":raw}});
    let revised = created(
        &app,
        &format!("/api/production-drafts/{id}/revisions"),
        &revision,
    )
    .await;
    assert_eq!(revised["result"]["head"]["payload"], revision["payload"]);
    let before = get(&app, "/api/state").await;
    for path in [
        "/api/business/drafts".to_owned(),
        format!("/api/business/{id}/draft-revisions"),
    ] {
        let response = send(&app, "POST", &path, Some(&input)).await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE, "{path}");
        assert_eq!(value(response).await["code"], "validation");
    }
    let oversized = json!({"extra":"x".repeat(256 * 1024)});
    for path in [
        "/api/production-drafts".to_owned(),
        format!("/api/production-drafts/{id}/revisions"),
    ] {
        let response = send(&app, "POST", &path, Some(&oversized)).await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE, "{path}");
        assert_eq!(value(response).await["code"], "validation");
    }
    assert_eq!(get(&app, "/api/state").await, before);
}

#[tokio::test]
async fn production_aliases_reject_other_business_kinds_without_changing_them() {
    let (_directory, app) = app();
    let input = json!({"meta":meta(&get(&app,"/api/state").await),
        "corrects_confirmation_id":null,
        "payload":{"kind":"catalog_price","version":1,"data":{
            "material_id":null,"material_version_id":null,
            "quantity":"","unit":"g","amount":"","note":"가상 기존 가격 초안"}}});
    let saved = created(&app, "/api/business/drafts", &input).await;
    let id = saved["result"]["item"]["id"].as_str().unwrap();
    let before = get(&app, "/api/state").await;
    let revise = json!({"meta":meta(&before),
        "expected_head_id":saved["result"]["head"]["id"],"payload":input["payload"]});
    for (path, request) in [
        (format!("/api/production-drafts/{id}/revisions"), &revise),
        ("/api/production-drafts".to_owned(), &input),
    ] {
        let response = send(&app, "POST", &path, Some(request)).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}");
        assert_eq!(value(response).await["code"], "validation");
    }
    assert_eq!(
        get(&app, "/api/business").await["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        get(&app, &format!("/api/business/{id}")).await["head"],
        saved["result"]["head"]
    );
    assert_eq!(get(&app, "/api/state").await, before);
}

#[tokio::test]
async fn production_extension_preserves_session_origin_host_and_existing_16_kib_limits() {
    let (_directory, app) = app();
    let before = get(&app, "/api/state").await;
    for (method, path) in [
        ("GET", "/api/productions"),
        ("POST", "/api/production-drafts"),
        (
            "POST",
            "/api/production-drafts/00000000-0000-4000-8000-000000000001/revisions",
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
                StatusCode::FORBIDDEN
            );
        }
    }
    for path in [
        "/api/materials",
        "/api/business/drafts",
        "/api/business/00000000-0000-4000-8000-000000000001/draft-revisions",
    ] {
        let response = send(
            &app,
            "POST",
            path,
            Some(&json!({"extra":"x".repeat(17 * 1024)})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE, "{path}");
        assert_eq!(value(response).await["code"], "validation");
    }
    assert_eq!(get(&app, "/api/state").await, before);
}

#[tokio::test]
async fn confirmed_production_preserves_value_references_and_cross_alias_retries() {
    let (_directory, app) = app();
    let raw = material(&app, "가상 생산 투입", "raw").await;
    let output = material(&app, "가상 생산 결과", "prepared").await;
    let byproduct = material(&app, "가상 생산 부산물", "stocked").await;
    let received = receipt(&app, &raw, "3", "30000").await;
    let before_holding_id =
        received["result"]["confirmation"]["result"]["holding_version_id"].clone();
    let data = json!({"inputs":[measured_input(&raw,"3","kg")],
        "outputs":[measured_output(&output,"2000","100"), measured_output(&byproduct,"1000","0")],"note":"가상 생산 이력"});
    let create_input = draft_input(&get(&app, "/api/state").await, data.clone());
    let original = created(&app, "/api/production-drafts", &create_input).await;
    assert_eq!(
        created(&app, "/api/business/drafts", &create_input).await,
        original
    );
    let id = original["result"]["item"]["id"].as_str().unwrap();
    let mut changed = data;
    changed["outputs"][0]["allocation_percent"] = json!("70");
    changed["outputs"][1]["allocation_percent"] = json!("30");
    let revision = json!({"meta":meta(&get(&app,"/api/state").await),
        "expected_head_id":original["result"]["head"]["id"],
        "payload":{"kind":"production","version":1,"data":changed}});
    let revised = created(
        &app,
        &format!("/api/production-drafts/{id}/revisions"),
        &revision,
    )
    .await;
    assert_eq!(
        created(
            &app,
            &format!("/api/business/{id}/draft-revisions"),
            &revision
        )
        .await,
        revised
    );
    let check = preview(&app, &revised).await;
    assert_eq!(check["can_confirm"], true, "{check}");
    assert_eq!(check["effect"]["kind"], "production");
    assert_eq!(check["effect"]["total_input_value"], "30000");
    assert_eq!(check["effect"]["outputs"][0]["value"], "21000");
    assert_eq!(check["effect"]["outputs"][1]["value"], "9000");
    let before_effect = check["effect"]["materials"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["material_id"] == raw["id"])
        .unwrap();
    assert_eq!(before_effect["before"]["source_id"], before_holding_id);
    assert!(
        check["effect"]["materials"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["after"]["source_id"].is_null())
    );
    let confirm_input = json!({"meta":meta(&check["state"]),"expected_head_id":check["head_id"]});
    let confirmed = created(&app, &confirm_path(&revised), &confirm_input).await;
    let production_id = confirmed["result"]["confirmation"]["result"]["production_id"]
        .as_str()
        .unwrap();
    let detail_path = format!("/api/productions/{production_id}");
    let detail = get(&app, &detail_path).await;
    assert_eq!(detail["item"]["total_input_value"], "30000");
    assert_eq!(detail["item"]["inputs"][0]["input_quantity"], "3");
    assert_eq!(detail["item"]["inputs"][0]["quantity"], "3000");
    assert_eq!(
        detail["item"]["inputs"][0]["material_version_id"],
        raw["version_id"]
    );
    assert_eq!(detail["item"]["outputs"][0]["allocation_percent"], "70");
    assert_eq!(detail["materials"].as_array().unwrap().len(), 3);
    assert!(
        detail["materials"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["id"].is_string() && row["after"]["source_id"].is_string())
    );
    let input_holding = get(
        &app,
        &format!("/api/holdings/{}", raw["id"].as_str().unwrap()),
    )
    .await;
    assert_eq!(input_holding["settlement_basis"]["quantity"], "0");
    assert_eq!(input_holding["settlement_basis"]["value"], "0");
    assert_eq!(input_holding["price"]["source"], "last_valid");
    assert_eq!(input_holding["price"]["amount"], "30000");
    assert_eq!(input_holding["price"]["quantity"], "3000");
    assert_eq!(input_holding["price"]["source_id"], before_holding_id);
    let output_holding = get(
        &app,
        &format!("/api/holdings/{}", output["id"].as_str().unwrap()),
    )
    .await;
    assert_eq!(output_holding["basis"]["kind"], "production");
    assert!(output_holding["observation"].is_null());
    assert_eq!(output_holding["settlement_basis"]["quantity"], "2000");
    assert_eq!(output_holding["settlement_basis"]["value"], "21000");
    assert_eq!(output_holding["confirmed_changes"], json!([]));
    for material in [&raw, &output, &byproduct] {
        let list = get(
            &app,
            &format!(
                "/api/productions?material_id={}",
                material["id"].as_str().unwrap()
            ),
        )
        .await;
        assert_eq!(list["items"].as_array().unwrap().len(), 1);
        assert_eq!(list["items"][0]["id"], production_id);
        assert_eq!(list["items"][0]["input_count"], 1);
        assert_eq!(list["items"][0]["output_count"], 2);
        assert_eq!(list["items"][0]["first_output_name"], output["name"]);
    }
    receipt(&app, &output, "1", "1000").await;
    let state = get(&app, "/api/state").await;
    assert_eq!(
        created(&app, &confirm_path(&revised), &confirm_input).await,
        confirmed
    );
    assert_eq!(
        created(&app, "/api/business/drafts", &create_input).await,
        original
    );
    assert_eq!(
        created(
            &app,
            &format!("/api/business/{id}/draft-revisions"),
            &revision
        )
        .await,
        revised
    );
    let mut conflict = create_input;
    conflict["payload"]["data"]["note"] = json!("동일 키의 다른 가상 입력");
    rejected(
        &app,
        "/api/production-drafts",
        &conflict,
        StatusCode::CONFLICT,
        "request_conflict",
    )
    .await;
    let preserved = get(&app, &detail_path).await;
    assert_eq!(preserved["item"], detail["item"]);
    assert_eq!(preserved["materials"], detail["materials"]);
    assert_eq!(
        get(&app, "/api/productions").await["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(get(&app, "/api/state").await, state);
}

#[tokio::test]
async fn reconciliation_keeps_actual_input_and_uses_the_new_receipt_basis_once() {
    let (_directory, app) = app();
    let raw = material(&app, "가상 부족 투입", "raw").await;
    let output = material(&app, "가상 부족 후 결과", "prepared").await;
    receipt(&app, &raw, "1", "10000").await;
    let data = json!({"inputs":[measured_input(&raw,"1.2","kg")],
        "outputs":[measured_output(&output,"1000","100")],"note":"가상 실제량 유지"});
    let draft = created(
        &app,
        "/api/production-drafts",
        &draft_input(&get(&app, "/api/state").await, data.clone()),
    )
    .await;
    let blocked = preview(&app, &draft).await;
    assert_eq!(blocked["can_confirm"], false);
    assert!(blocked["effect"].is_null());
    assert!(
        blocked["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["code"] == "needs_reconciliation")
    );
    let blocked_input =
        json!({"meta":meta(&blocked["state"]),"expected_head_id":blocked["head_id"]});
    rejected(
        &app,
        &confirm_path(&draft),
        &blocked_input,
        StatusCode::BAD_REQUEST,
        "validation",
    )
    .await;
    assert_eq!(get(&app, "/api/state").await, blocked["state"]);
    let missing_receipt = receipt(&app, &raw, "0.2", "2000").await;
    rejected(
        &app,
        &confirm_path(&draft),
        &blocked_input,
        StatusCode::CONFLICT,
        "stale_state",
    )
    .await;
    let check = preview(&app, &draft).await;
    assert_eq!(check["can_confirm"], true, "{check}");
    assert_eq!(check["effect"]["inputs"][0]["input_quantity"], "1.2");
    assert_eq!(check["effect"]["total_input_value"], "12000");
    assert_eq!(
        check["effect"]["materials"][0]["before"]["source_id"],
        missing_receipt["result"]["confirmation"]["result"]["holding_version_id"]
    );
    let obsolete = json!({"meta":meta(&check["state"]),"expected_head_id":check["head_id"]});
    material(&app, "가상 다른 탭 변경", "raw").await;
    rejected(
        &app,
        &confirm_path(&draft),
        &obsolete,
        StatusCode::CONFLICT,
        "stale_state",
    )
    .await;
    let fresh = preview(&app, &draft).await;
    let fresh_input = json!({"meta":meta(&fresh["state"]),"expected_head_id":fresh["head_id"]});
    let confirmed = created(&app, &confirm_path(&draft), &fresh_input).await;
    assert_eq!(
        created(&app, &confirm_path(&draft), &fresh_input).await,
        confirmed
    );
    let business = get(
        &app,
        &format!(
            "/api/business/{}",
            draft["result"]["item"]["id"].as_str().unwrap()
        ),
    )
    .await;
    assert_eq!(business["head"]["payload"]["data"], data);
    assert_eq!(
        get(&app, "/api/productions").await["items"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        get(
            &app,
            &format!("/api/holdings/{}", output["id"].as_str().unwrap())
        )
        .await["settlement_basis"]["value"],
        "12000"
    );
}

#[tokio::test]
async fn strict_production_structure_rejects_numbers_unknown_fields_and_invalid_confirmations() {
    let (_directory, app) = app();
    let before = get(&app, "/api/state").await;
    let input = draft_input(&before, data());
    let mut invalid = Vec::new();
    let mut changed = input.clone();
    changed["payload"]["data"]["inputs"][0]["quantity"] = json!(1);
    invalid.push(changed);
    let mut changed = input.clone();
    changed["payload"]["data"]["outputs"][0]["allocation_percent"] = json!(100);
    invalid.push(changed);
    let mut changed = input.clone();
    changed["payload"]["data"]["inputs"][0]["extra"] = json!(true);
    invalid.push(changed);
    let mut changed = input.clone();
    changed["payload"]["data"]["outputs"][0]["extra"] = json!(true);
    invalid.push(changed);
    let mut changed = input.clone();
    changed["payload"]["version"] = json!(2);
    invalid.push(changed);
    let mut changed = input.clone();
    changed["payload"]["data"]["inputs"] =
        json!(vec![input["payload"]["data"]["inputs"][0].clone(); 101]);
    invalid.push(changed);
    for changed in invalid {
        rejected(
            &app,
            "/api/production-drafts",
            &changed,
            StatusCode::BAD_REQUEST,
            "validation",
        )
        .await;
    }
    assert_eq!(get(&app, "/api/state").await, before);
    let raw = material(&app, "가상 검증 투입", "raw").await;
    let output = material(&app, "가상 검증 결과", "prepared").await;
    receipt(&app, &raw, "1", "10000").await;
    for (quantity, unit, percent) in [("0", "g", "100"), ("1", "mL", "100"), ("1", "g", "99")] {
        let data = json!({"inputs":[measured_input(&raw,quantity,unit)],"outputs":[measured_output(&output,"1",percent)],"note":"가상 의미 오류 보존"});
        let saved = created(
            &app,
            "/api/production-drafts",
            &draft_input(&get(&app, "/api/state").await, data.clone()),
        )
        .await;
        assert_eq!(saved["result"]["head"]["payload"]["data"], data);
        let check = preview(&app, &saved).await;
        assert_eq!(check["can_confirm"], false);
        assert!(check["effect"].is_null());
        rejected(
            &app,
            &confirm_path(&saved),
            &json!({"meta":meta(&check["state"]),"expected_head_id":check["head_id"]}),
            StatusCode::BAD_REQUEST,
            "validation",
        )
        .await;
        assert_eq!(get(&app, "/api/state").await, check["state"]);
    }
    assert_eq!(get(&app, "/api/productions").await["items"], json!([]));
    assert_eq!(
        get(
            &app,
            &format!("/api/holdings/{}", raw["id"].as_str().unwrap())
        )
        .await["settlement_basis"]["quantity"],
        "1000"
    );
}
