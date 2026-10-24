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
const COOKIE: &str = "hansang_4321=recipe-test-session";
const MISSING_ID: &str = "00000000-0000-4000-8000-000000000001";

fn app() -> (tempfile::TempDir, Router) {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::new(
        Store::open(directory.path()).unwrap(),
        HOST.into(),
        "recipe-test-session".into(),
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

async fn material(app: &Router) -> Value {
    let mut input = meta(&get(app, "/api/state").await);
    input["name"] = json!("가상 레시피 HTTP 재료");
    input["unit"] = json!("g");
    input["kind"] = json!("raw");
    input["description"] = json!("");
    created(app, "/api/materials", &input).await
}

fn empty_data() -> Value {
    json!({"basis_count":"1","reference_output":null,"selling_price":null,
        "cooking_description":"","process":"","reference_time":"","lines":[]})
}

fn component(item: &Value, quantity: &str) -> Value {
    json!({"material_id":item["id"],"material_version_id":item["version_id"],
        "quantity":quantity,"unit":"g","note":"적당량"})
}

async fn new_recipe_input(app: &Router, data: Value) -> Value {
    json!({"meta":meta(&get(app,"/api/state").await),
        "owner":{"mode":"new","name":"가상 HTTP 메뉴","unit":"개","kind":"menu","description":""},
        "data":data})
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

async fn rejected(app: &Router, path: &str, input: &Value, status: StatusCode, code: &str) {
    let response = send(app, "POST", path, Some(input)).await;
    let actual = response.status();
    let body = value(response).await;
    assert_eq!(actual, status, "{path}: {body}");
    assert_eq!(body["code"], code, "{body}");
}

#[tokio::test]
async fn recipe_list_filters_and_missing_versions_have_explicit_contracts() {
    let (_directory, app) = app();
    let list = get(&app, "/api/recipes").await;
    assert_eq!(list["items"], json!([]));
    assert!(list["state"]["epoch"].is_string());
    assert!(list["state"]["revision"].is_string());
    assert_eq!(
        get(&app, "/api/recipes?query=absent&archive=all").await["items"],
        json!([])
    );
    for path in ["/api/recipes?extra=true", "/api/recipes?archive=invalid"] {
        let response = send(&app, "GET", path, None).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}");
        assert_eq!(value(response).await["code"], "validation");
    }
    for path in [
        format!("/api/recipes/{MISSING_ID}"),
        format!("/api/recipes/{MISSING_ID}/cost"),
        format!("/api/recipes/{MISSING_ID}/versions/{MISSING_ID}"),
        format!("/api/recipes/{MISSING_ID}/versions/{MISSING_ID}/cost"),
    ] {
        let response = send(&app, "GET", &path, None).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        assert_eq!(value(response).await["code"], "not_found");
    }
}

#[tokio::test]
async fn existing_material_without_recipe_uses_is_distinct_from_missing_material() {
    let (_directory, app) = app();
    let item = material(&app).await;
    let path = format!(
        "/api/materials/{}/recipe-uses",
        item["id"].as_str().unwrap()
    );
    let uses = get(&app, &path).await;
    assert_eq!(uses["items"], json!([]));
    assert!(uses["state"]["revision"].is_string());
    let response = send(
        &app,
        "GET",
        &format!("/api/materials/{MISSING_ID}/recipe-uses"),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(value(response).await["code"], "not_found");
}

#[tokio::test]
async fn recipe_spa_routes_allow_get_head_and_reject_unknown_paths() {
    let (_directory, app) = app();
    for path in [
        "/recipes".to_owned(),
        "/recipes/new".to_owned(),
        format!("/recipes/{MISSING_ID}"),
        format!("/recipes/{MISSING_ID}/versions/{MISSING_ID}"),
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
        "/recipes/not-a-uuid".to_owned(),
        format!("/recipes/{MISSING_ID}/versions/not-a-uuid"),
        format!("/recipes/{MISSING_ID}/extra"),
        "/api/recipes/extra/path".to_owned(),
    ] {
        assert_eq!(
            send(&app, "GET", &path, None).await.status(),
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
}

#[tokio::test]
async fn existing_raw_owner_can_store_empty_recipe_and_read_separate_cost() {
    let (_directory, app) = app();
    let item = material(&app).await;
    let saved = created(
        &app,
        "/api/recipes",
        &json!({"meta":meta(&get(&app,"/api/state").await),
            "owner":{"mode":"existing","material_id":item["id"],
                "expected_material_version_id":item["version_id"]},
            "data":empty_data()}),
    )
    .await;
    assert_eq!(saved["item"]["id"], item["id"]);
    assert_eq!(saved["item"]["kind"], "raw");
    assert_eq!(saved["version"]["lines"], json!([]));
    assert_eq!(saved["version"]["basis_count"], "1");
    assert!(saved["version"]["selling_price"].is_null());
    let owner = item["id"].as_str().unwrap();
    let version = saved["version"]["id"].as_str().unwrap();
    let detail = get(&app, &format!("/api/recipes/{owner}")).await;
    assert_eq!(detail["head"], saved["version"]);
    assert!(detail.get("cost").is_none());
    let frozen = get(&app, &format!("/api/recipes/{owner}/versions/{version}")).await;
    assert_eq!(frozen, saved["version"]);
    let cost = get(
        &app,
        &format!("/api/recipes/{owner}/versions/{version}/cost"),
    )
    .await;
    assert_eq!(cost["recipe_version_id"], version);
    assert_eq!(cost["price_mode"], "current");
    assert_eq!(cost["total_cost"], "0");
    assert_eq!(cost["ratio_unavailable_reason"], "missing_selling_price");
    assert_eq!(get(&app, "/api/state").await, saved["state"]);
}

#[tokio::test]
async fn recipe_post_accepts_valid_large_content_but_rejects_over_256_kib() {
    let (_directory, app) = app();
    let ingredient = material(&app).await;
    let mut data = empty_data();
    data["lines"] = json!(
        (0..100)
            .map(|_| json!({
                "material_id":ingredient["id"],"material_version_id":ingredient["version_id"],
                "quantity":"0","unit":"g","note":"가".repeat(200)
            }))
            .collect::<Vec<_>>()
    );
    let input = json!({"meta":meta(&get(&app,"/api/state").await),
        "owner":{"mode":"new","name":"가상 HTTP 큰 레시피","unit":"개", "kind":"menu","description":""},
        "data":data});
    let bytes = input.to_string().len();
    assert!(bytes > 16 * 1024 && bytes < 256 * 1024, "{bytes}");
    let saved = created(&app, "/api/recipes", &input).await;
    assert_eq!(saved["version"]["lines"].as_array().unwrap().len(), 100);
    let before = get(&app, "/api/state").await;
    let mut oversized = input;
    oversized["meta"] = meta(&before);
    oversized["data"]["process"] = json!("x".repeat(256 * 1024));
    let response = send(&app, "POST", "/api/recipes", Some(&oversized)).await;
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(value(response).await["code"], "validation");
    assert_eq!(get(&app, "/api/state").await, before);
}

#[tokio::test]
async fn existing_guards_and_small_body_limits_survive_recipe_extension() {
    let (_directory, app) = app();
    let before = get(&app, "/api/state").await;
    for (method, path) in [
        ("GET", "/api/recipes"),
        ("POST", "/api/recipes"),
        (
            "GET",
            "/api/materials/00000000-0000-4000-8000-000000000001/recipe-uses",
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
    }
    let missing_origin = Request::builder()
        .method("POST")
        .uri("/api/recipes")
        .header("host", HOST)
        .header("cookie", COOKIE)
        .header("content-type", "application/json")
        .body(Body::from("{}"))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(missing_origin).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    for path in ["/api/materials", "/api/business/drafts"] {
        let response = send(
            &app,
            "POST",
            path,
            Some(&json!({"extra":"x".repeat(17*1024)})),
        )
        .await;
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE, "{path}");
    }
    assert_eq!(get(&app, "/api/state").await, before);
}

#[tokio::test]
async fn recipe_versions_copy_retries_and_current_cost_keep_their_distinct_references() {
    let (_directory, app) = app();
    let ingredient = material(&app).await;
    let initial_price = price(&app, &ingredient, "10").await;
    let mut data = empty_data();
    data["basis_count"] = json!("3");
    data["selling_price"] = json!("1000");
    data["lines"] = json!([component(&ingredient, "100")]);
    let create_input = new_recipe_input(&app, data.clone()).await;
    let original = created(&app, "/api/recipes", &create_input).await;
    let owner = original["item"]["id"].as_str().unwrap();
    let first_id = original["version"]["id"].as_str().unwrap();
    let first_path = format!("/api/recipes/{owner}/versions/{first_id}");
    let first_cost = get(&app, &format!("{first_path}/cost")).await;
    assert_eq!(first_cost["total_cost"], "333.333333");
    assert_eq!(first_cost["cost_ratio"]["numerator"], "333333333");
    assert_eq!(first_cost["cost_ratio"]["denominator"], "1000000000");
    assert_eq!(first_cost["rows"][0]["price"]["source"], "preregistered");
    assert_eq!(
        first_cost["rows"][0]["price"]["source_id"],
        initial_price["id"]
    );
    assert_eq!(get(&app, "/api/state").await, original["state"]);

    let copy_path = format!("/api/recipes/{owner}/copies");
    let copy_input = json!({"meta":meta(&get(&app,"/api/state").await),
        "expected_material_version_id":original["item"]["version_id"],
        "source_recipe_version_id":first_id});
    let copied = created(&app, &copy_path, &copy_input).await;
    let copied_owner = copied["item"]["id"].as_str().unwrap();
    assert_ne!(copied_owner, owner);
    assert_eq!(copied["item"]["name"], "가상 HTTP 메뉴 (2)");
    assert_eq!(copied["version"]["copied_from_version_id"], first_id);
    assert!(copied["version"]["previous_version_id"].is_null());
    assert_ne!(
        copied["version"]["lines"][0]["id"],
        original["version"]["lines"][0]["id"]
    );
    assert_eq!(
        copied["version"]["lines"][0]["material_id"],
        ingredient["id"]
    );

    let mut rename = meta(&get(&app, "/api/state").await);
    rename["expected_version_id"] = ingredient["version_id"].clone();
    rename["name"] = json!("가상 재료 새 표시");
    rename["unit"] = json!("kg");
    rename["kind"] = json!("prepared");
    rename["description"] = json!("");
    let newer_ingredient = created(
        &app,
        &format!(
            "/api/materials/{}/versions",
            ingredient["id"].as_str().unwrap()
        ),
        &rename,
    )
    .await;
    data["basis_count"] = json!("6");
    data["lines"] = json!([component(&ingredient, "250"), component(&ingredient, "0")]);
    let revise_path = format!("/api/recipes/{owner}/versions");
    let revise_input = json!({"meta":meta(&get(&app,"/api/state").await),
        "expected_material_version_id":original["item"]["version_id"],
        "expected_recipe_version_id":first_id,"data":data});
    let revised = created(&app, &revise_path, &revise_input).await;
    assert_eq!(revised["version"]["previous_version_id"], first_id);
    assert_eq!(
        revised["version"]["lines"][0]["material_version_id"],
        ingredient["version_id"]
    );
    assert_ne!(ingredient["version_id"], newer_ingredient["version_id"]);
    assert_eq!(get(&app, &first_path).await, original["version"]);
    assert_eq!(
        get(&app, &format!("/api/recipes/{copied_owner}")).await["head"],
        copied["version"]
    );
    let uses = get(
        &app,
        &format!(
            "/api/materials/{}/recipe-uses",
            ingredient["id"].as_str().unwrap()
        ),
    )
    .await;
    assert_eq!(uses["items"].as_array().unwrap().len(), 2);

    let before = get(&app, "/api/state").await;
    assert_eq!(created(&app, "/api/recipes", &create_input).await, original);
    assert_eq!(created(&app, &copy_path, &copy_input).await, copied);
    assert_eq!(created(&app, &revise_path, &revise_input).await, revised);
    let mut conflicting = revise_input.clone();
    conflicting["data"]["process"] = json!("다른 본문");
    rejected(
        &app,
        &revise_path,
        &conflicting,
        StatusCode::CONFLICT,
        "request_conflict",
    )
    .await;
    let mut stale = revise_input;
    stale["meta"] = meta(&before);
    rejected(
        &app,
        &revise_path,
        &stale,
        StatusCode::CONFLICT,
        "stale_state",
    )
    .await;
    let mut stale_copy = copy_input;
    stale_copy["meta"] = meta(&before);
    rejected(
        &app,
        &copy_path,
        &stale_copy,
        StatusCode::CONFLICT,
        "stale_state",
    )
    .await;
    assert_eq!(get(&app, "/api/state").await, before);
    let foreign_version = format!(
        "/api/recipes/{owner}/versions/{}",
        copied["version"]["id"].as_str().unwrap()
    );
    assert_eq!(
        send(&app, "GET", &foreign_version, None).await.status(),
        StatusCode::NOT_FOUND
    );

    price(&app, &newer_ingredient, "2").await;
    let historical_composition_cost = get(&app, &format!("{first_path}/cost")).await;
    assert_eq!(historical_composition_cost["recipe_version_id"], first_id);
    assert_eq!(historical_composition_cost["price_mode"], "current");
    assert_eq!(historical_composition_cost["total_cost"], "66.666667");
    assert_eq!(
        get(&app, &format!("/api/recipes/{owner}/cost")).await["total_cost"],
        "83.333333"
    );
    assert_eq!(get(&app, &first_path).await, original["version"]);
}

#[tokio::test]
async fn malformed_or_invalid_recipe_inputs_leave_no_partial_owner_or_recipe() {
    let (_directory, app) = app();
    let ingredient = material(&app).await;
    let mut data = empty_data();
    data["lines"] = json!([component(&ingredient, "0")]);
    let good = new_recipe_input(&app, data).await;
    let before = get(&app, "/api/state").await;
    let mut invalids = Vec::new();
    for pointer in [
        "/meta/expected_revision",
        "/data/basis_count",
        "/data/lines/0/quantity",
        "/data/selling_price",
    ] {
        let mut invalid = good.clone();
        *invalid.pointer_mut(pointer).unwrap() = json!(1);
        invalids.push(invalid);
    }
    for pointer in ["", "/meta", "/owner", "/data", "/data/lines/0"] {
        let mut invalid = good.clone();
        invalid
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("extra".into(), json!(true));
        invalids.push(invalid);
    }
    for quantity in ["", "-1", "0.0000001", "9000000001"] {
        let mut invalid = good.clone();
        invalid["data"]["lines"][0]["quantity"] = json!(quantity);
        invalids.push(invalid);
    }
    let mut dimension = good.clone();
    dimension["data"]["lines"][0]["unit"] = json!("mL");
    invalids.push(dimension);
    let mut reference = good.clone();
    reference["data"]["reference_output"] = json!({"quantity":"1","unit":"개","extra":true});
    invalids.push(reference);
    let mut count = good.clone();
    count["data"]["lines"] = json!(vec![good["data"]["lines"][0].clone(); 101]);
    invalids.push(count);
    let mut note = good.clone();
    note["data"]["lines"][0]["note"] = json!("가".repeat(201));
    invalids.push(note);
    let mut legacy_kind = good.clone();
    legacy_kind["owner"]["kind"] = json!("unclassified");
    invalids.push(legacy_kind);
    let mut cycle = good;
    cycle["owner"] = json!({"mode":"existing","material_id":ingredient["id"],
        "expected_material_version_id":ingredient["version_id"]});
    invalids.push(cycle);
    for input in invalids {
        rejected(
            &app,
            "/api/recipes",
            &input,
            StatusCode::BAD_REQUEST,
            "validation",
        )
        .await;
        assert_eq!(get(&app, "/api/state").await, before);
    }
    assert_eq!(get(&app, "/api/recipes").await["items"], json!([]));
    let materials = get(&app, "/api/materials").await;
    assert_eq!(materials.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn overflowing_cost_does_not_prevent_reading_or_revising_saved_composition() {
    let (_directory, app) = app();
    let ingredient = material(&app).await;
    price(&app, &ingredient, "9000000000000").await;
    let mut data = empty_data();
    data["selling_price"] = json!("0");
    data["lines"] = json!([component(&ingredient, "9000000000")]);
    let input = new_recipe_input(&app, data.clone()).await;
    let saved = created(&app, "/api/recipes", &input).await;
    let owner = saved["item"]["id"].as_str().unwrap();
    let version = saved["version"]["id"].as_str().unwrap();
    let before = get(&app, "/api/state").await;
    let response = send(
        &app,
        "GET",
        &format!("/api/recipes/{owner}/versions/{version}/cost"),
        None,
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(value(response).await["code"], "validation");
    assert_eq!(
        get(&app, &format!("/api/recipes/{owner}")).await["head"],
        saved["version"]
    );
    assert_eq!(get(&app, "/api/state").await, before);
    data["lines"][0]["quantity"] = json!("0");
    let revised = created(
        &app,
        &format!("/api/recipes/{owner}/versions"),
        &json!({
            "meta":meta(&before),"expected_material_version_id":saved["item"]["version_id"],
            "expected_recipe_version_id":version,"data":data
        }),
    )
    .await;
    let cost = get(&app, &format!("/api/recipes/{owner}/cost")).await;
    assert_eq!(cost["recipe_version_id"], revised["version"]["id"]);
    assert_eq!(cost["total_cost"], "0");
    assert_eq!(cost["ratio_unavailable_reason"], "zero_selling_price");
    assert!(cost["cost_ratio"].is_null());
}
