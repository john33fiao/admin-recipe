use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    response::Response,
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};
use sunchang_hansang::{
    lifecycle::Policy,
    server::{AppState, router},
    store::Store,
};
use tower::ServiceExt;

const HOST: &str = "127.0.0.1:4321";
const ORIGIN: &str = "http://127.0.0.1:4321";
const COOKIE: &str = "hansang_4321=overview-test";
const SECTIONS: [&str; 10] = [
    "materials",
    "material_changes",
    "open_orders",
    "received_orders",
    "receipts",
    "sales",
    "observations",
    "unallocated",
    "unconfirmed",
    "drafts",
];

fn app() -> (tempfile::TempDir, Router) {
    let directory = tempfile::tempdir().unwrap();
    let state = AppState::new(
        Store::open(directory.path()).unwrap(),
        HOST.into(),
        "overview-test".into(),
        Policy::default(),
    );
    (directory, router(state))
}
async fn send(app: &Router, method: &str, path: &str, input: Option<&Value>) -> Response {
    let mut r = Request::builder()
        .method(method)
        .uri(path)
        .header("host", HOST)
        .header("cookie", COOKIE);
    if method != "GET" {
        r = r.header("origin", ORIGIN);
    }
    let body = if let Some(v) = input {
        r = r.header("content-type", "application/json");
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
    let status = r.status();
    let body = value(r).await;
    assert_eq!(status, StatusCode::OK, "{path}: {body}");
    body
}
fn meta(state: &Value) -> Value {
    json!({"request_id":uuid::Uuid::new_v4().to_string(),"epoch":state["epoch"],"expected_revision":state["revision"]})
}
async fn created(app: &Router, path: &str, input: &Value) -> Value {
    let r = send(app, "POST", path, Some(input)).await;
    let status = r.status();
    let body = value(r).await;
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
fn list_path(section: &str, offset: &str, state: &Value, material: Option<&str>) -> String {
    let mut path = format!(
        "/api/overview/list?section={section}&offset={offset}&epoch={}&revision={}",
        state["epoch"].as_str().unwrap(),
        state["revision"].as_str().unwrap()
    );
    if let Some(id) = material {
        path.push_str(&format!("&material_id={id}"));
    }
    path
}
fn disk(path: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, path: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            // Windows denies reads of the live exclusive lock; persisted data stays covered.
            if entry.path() == root.join(".writer.lock") {
                continue;
            }
            if entry.file_type().unwrap().is_dir() {
                visit(root, &entry.path(), out);
            } else {
                out.insert(
                    entry
                        .path()
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into(),
                    std::fs::read(entry.path()).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(path, path, &mut out);
    out
}
async fn draft(app: &Router, kind: &str, data: Value, corrects: Option<&str>) -> Value {
    created(app, "/api/business/drafts", &json!({"meta":meta(&get(app,"/api/state").await),"corrects_confirmation_id":corrects,"payload":{"kind":kind,"version":1,"data":data}})).await
}
async fn confirm(app: &Router, draft: &Value) -> Value {
    let id = draft["result"]["item"]["id"].as_str().unwrap();
    let response = send(
        app,
        "POST",
        &format!("/api/business/{id}/preview"),
        Some(&json!({"expected_head_id":draft["result"]["head"]["id"]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let preview = value(response).await;
    assert_eq!(preview["can_confirm"], true, "{preview}");
    created(
        app,
        &format!("/api/business/{id}/confirm"),
        &json!({"meta":meta(&preview["state"]),"expected_head_id":preview["head_id"]}),
    )
    .await["result"]["confirmation"]
        .clone()
}
async fn event(app: &Router, kind: &str, data: Value) -> Value {
    confirm(app, &draft(app, kind, data, None).await).await
}

#[tokio::test]
async fn overview_and_all_ten_sections_share_state_and_preserve_every_file() {
    let (directory, app) = app();
    let item = material(&app, "가상 미확인 품목").await;
    let state = get(&app, "/api/state").await;
    let before = disk(directory.path());
    let summary = get(&app, "/api/overview").await;
    assert_eq!(summary["state"], state);
    assert_eq!(summary["materials"]["total_count"], 1);
    let row = &summary["materials"]["rows"][0];
    assert_eq!(row["material"]["id"], item["id"]);
    assert!(row["last_observation"].is_null());
    assert_eq!(row["changes_boundary"]["kind"], "not_observed");
    assert_eq!(row["changes_after_observation"]["total_count"], 0);
    assert!(row.get("holding").is_none());
    assert!(row.get("current_quantity").is_none());
    for section in SECTIONS {
        let material = (section == "material_changes").then(|| item["id"].as_str().unwrap());
        let path = list_path(section, "0", &state, material);
        let result = get(&app, &path).await;
        assert_eq!(result["state"], state);
        assert_eq!(result["scope"]["section"], section);
        assert_eq!(result["scope"]["material_id"], json!(material));
        assert_eq!(result["list"]["section"], section);
        let page = &result["list"]["page"];
        assert_eq!(page["offset"], 0);
        assert_eq!(
            page["page_size"],
            if section == "materials" { 25 } else { 20 }
        );
        assert_eq!(
            page["returned_count"],
            page["rows"].as_array().unwrap().len()
        );
        assert!(page["next_offset"].is_null());
        let head = send(&app, "HEAD", &path, None).await;
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
    }
    let head = send(&app, "HEAD", "/api/overview", None).await;
    assert_eq!(head.status(), StatusCode::OK);
    assert!(
        head.into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .is_empty()
    );
    assert_eq!(get(&app, "/api/state").await, state);
    assert_eq!(disk(directory.path()), before);
}

#[tokio::test]
async fn list_query_is_closed_canonical_required_and_transport_bounded() {
    let (_directory, app) = app();
    let item = material(&app, "가상 조회 조건 품목").await;
    let state = get(&app, "/api/state").await;
    let valid = list_path("materials", "0", &state, None);
    let query = valid.split_once('?').unwrap().1;
    let mut invalid = vec![
        "/api/overview/list".to_owned(),
        format!("{valid}&other=1"),
        format!("{valid}&section=materials"),
        format!("{valid}&%6fffset=0"),
        format!("{valid}&epoch={}", state["epoch"].as_str().unwrap()),
        format!("{valid}&revision=0"),
        format!("{valid}&material_id={}", item["id"].as_str().unwrap()),
        list_path("material_changes", "0", &state, None),
        list_path("other", "0", &state, None),
        format!("{valid}&extra=%GG"),
        format!("{valid}&extra=%FF"),
        format!("{valid}&extra=%A"),
    ];
    for key in ["section", "offset", "epoch", "revision"] {
        invalid.push(format!(
            "/api/overview/list?{}",
            query
                .split('&')
                .filter(|part| !part.starts_with(&format!("{key}=")))
                .collect::<Vec<_>>()
                .join("&")
        ));
    }
    for offset in [
        "",
        "-1",
        "+1",
        "01",
        "%200",
        "0%20",
        "1.0",
        "184467440737095516160",
    ] {
        invalid.push(list_path("materials", offset, &state, None));
    }
    for revision in ["", "-1", "%2B1", "01", "1.0", "184467440737095516160"] {
        let mut changed = state.clone();
        changed["revision"] = json!(revision);
        invalid.push(list_path("materials", "0", &changed, None));
    }
    for epoch in [
        "not-a-uuid".to_owned(),
        "00000000-0000-4000-8000-00000000000A".to_owned(),
        state["epoch"].as_str().unwrap().replace('-', ""),
    ] {
        let mut changed = state.clone();
        changed["epoch"] = json!(epoch);
        invalid.push(list_path("materials", "0", &changed, None));
    }
    for id in ["bad", "00000000000040008000000000000001"] {
        invalid.push(list_path("material_changes", "0", &state, Some(id)));
    }
    for path in invalid {
        let response = send(&app, "GET", &path, None).await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{path}");
        assert_eq!(value(response).await["code"], "validation");
    }
    for base in ["/api/overview", "/api/overview/list"] {
        for (length, status) in [
            (4096, StatusCode::BAD_REQUEST),
            (4097, StatusCode::URI_TOO_LONG),
        ] {
            let response = send(
                &app,
                "GET",
                &format!("{base}?x={}", "a".repeat(length - 2)),
                None,
            )
            .await;
            assert_eq!(response.status(), status);
            assert_eq!(value(response).await["code"], "validation");
        }
    }
    for query in ["section=materials", "x=%GG", "x=%FF"] {
        assert_eq!(
            send(&app, "GET", &format!("/api/overview?{query}"), None)
                .await
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(get(&app, "/api/state").await, state);
}

#[tokio::test]
async fn changed_state_rejects_page_continuation_without_mutating_or_retargeting() {
    let (directory, app) = app();
    let old = get(&app, "/api/overview").await["state"].clone();
    material(&app, "가상 새 조회 상태").await;
    let current = get(&app, "/api/state").await;
    let before = disk(directory.path());
    let mut other_epoch = current.clone();
    other_epoch["epoch"] = json!(uuid::Uuid::new_v4().to_string());
    for state in [&old, &other_epoch] {
        let response = send(&app, "GET", &list_path("materials", "0", state, None), None).await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body = value(response).await;
        assert_eq!(body["code"], "stale_state");
        assert!(body.get("list").is_none());
    }
    let fresh = get(&app, "/api/overview").await;
    let page = get(&app, &list_path("materials", "0", &fresh["state"], None)).await;
    assert_eq!(page["state"], current);
    assert_eq!(page["list"]["page"], fresh["materials"]);
    assert_eq!(get(&app, "/api/state").await, current);
    assert_eq!(disk(directory.path()), before);
}

#[tokio::test]
async fn overview_keeps_session_host_origin_head_and_method_guards() {
    let (_directory, app) = app();
    let state = get(&app, "/api/state").await;
    for path in [
        "/api/overview".to_owned(),
        list_path("materials", "0", &state, None),
    ] {
        for (method, host, cookie, origin, expected) in [
            ("GET", HOST, "", None, StatusCode::UNAUTHORIZED),
            (
                "GET",
                "foreign.example",
                COOKIE,
                None,
                StatusCode::FORBIDDEN,
            ),
            (
                "GET",
                HOST,
                COOKIE,
                Some("https://foreign.example"),
                StatusCode::FORBIDDEN,
            ),
            ("HEAD", HOST, COOKIE, None, StatusCode::FORBIDDEN),
        ] {
            let mut request = Request::builder()
                .method(method)
                .uri(&path)
                .header("host", host)
                .header("cookie", cookie);
            if let Some(origin) = origin {
                request = request.header("origin", origin);
            }
            let result = app
                .clone()
                .oneshot(request.body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(result.status(), expected);
        }
        assert_eq!(
            send(&app, "POST", &path, Some(&json!({}))).await.status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
    }
}

#[tokio::test]
async fn overview_list_spa_is_exact_and_does_not_hide_missing_api_or_assets() {
    let (_directory, app) = app();
    for method in ["GET", "HEAD"] {
        let response = send(
            &app,
            method,
            "/overview/list?section=materials&offset=0",
            None,
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
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
    for path in [
        "/overview/list/extra",
        "/api/overview/missing",
        "/overview/list/app.sqlite3",
        "/overview/list/.env",
    ] {
        assert_eq!(
            send(&app, "GET", path, None).await.status(),
            StatusCode::NOT_FOUND,
            "{path}"
        );
    }
    assert_eq!(
        send(&app, "POST", "/overview/list", Some(&json!({})))
            .await
            .status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
}

#[tokio::test]
async fn material_pages_cover_all_rows_once_and_past_end_is_explicitly_empty() {
    let (_directory, app) = app();
    let mut ids = Vec::new();
    for n in 0..26 {
        ids.push(material(&app, &format!("가상 목록 {n:02}")).await["id"].clone());
    }
    let summary = get(&app, "/api/overview").await;
    let state = &summary["state"];
    assert_eq!(summary["materials"]["total_count"], 26);
    assert_eq!(summary["materials"]["returned_count"], 25);
    assert_eq!(summary["materials"]["next_offset"], 25);
    let last = get(&app, &list_path("materials", "25", state, None)).await;
    assert_eq!(last["list"]["page"]["total_count"], 26);
    assert_eq!(last["list"]["page"]["returned_count"], 1);
    assert!(last["list"]["page"]["next_offset"].is_null());
    let got = summary["materials"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .chain(last["list"]["page"]["rows"].as_array().unwrap())
        .map(|row| row["material"]["id"].clone())
        .collect::<Vec<_>>();
    assert_eq!(got, ids);
    for offset in ["26".to_owned(), usize::MAX.to_string()] {
        let beyond = get(&app, &list_path("materials", &offset, state, None)).await;
        assert_eq!(beyond["list"]["page"]["rows"], json!([]));
        assert_eq!(beyond["list"]["page"]["total_count"], 26);
        assert!(beyond["list"]["page"]["next_offset"].is_null());
    }
}

#[tokio::test]
async fn mixed_overview_rows_link_real_originals_and_corrections_without_writes() {
    let (directory, app) = app();
    let raw = material(&app, "가상 개요 원료").await;
    let output = material(&app, "가상 개요 결과").await;
    let menu = material(&app, "가상 개요 메뉴").await;
    let recipe=created(&app,"/api/recipes",&json!({"meta":meta(&get(&app,"/api/state").await),"owner":{"mode":"existing","material_id":menu["id"],"expected_material_version_id":menu["version_id"]},"data":{"basis_count":"1","reference_output":null,"selling_price":"10","cooking_description":"","process":"","reference_time":"","lines":[{"material_id":raw["id"],"material_version_id":raw["version_id"],"quantity":"1","unit":"g","note":""}]}})).await;
    let receipt=event(&app,"receipt",json!({"source":{"kind":"direct","material_id":raw["id"],"material_version_id":raw["version_id"]},"quantity":"10","unit":"g","amount":"100","receipt_date":"2020-02-29","note":"","stock_effect":{"kind":"unreflected"}})).await;
    let sale=event(&app,"sale",json!({"material_id":menu["id"],"material_version_id":menu["version_id"],"recipe_version_id":recipe["version"]["id"],"count":"2","note":""})).await;
    let stocktake=event(&app,"stocktake",json!({"rows":[{"material_id":raw["id"],"material_version_id":raw["version_id"],"quantity":"8","unit":"g","initial_value":null,"excess":null}],"note":""})).await;
    event(&app,"waste",json!({"material_id":raw["id"],"material_version_id":raw["version_id"],"quantity":"1","unit":"g","reason":"discard","note":""})).await;
    let receipt_id = receipt["result"]["receipt_id"].as_str().unwrap();
    let source = get(&app, &format!("/api/receipts/{receipt_id}/return-source")).await;
    event(&app,"purchase_return",json!({"receipt_id":receipt_id,"expected_receipt_version_id":source["current"]["reference"]["version_id"],"quantity":"1","unit":"g","note":""})).await;
    event(&app,"production",json!({"inputs":[{"material_id":raw["id"],"material_version_id":raw["version_id"],"quantity":"1","unit":"g"}],"outputs":[{"material_id":output["id"],"material_version_id":output["version_id"],"quantity":"1","unit":"g","allocation_percent":"100"}],"note":""})).await;
    let confirmation = sale["id"].as_str().unwrap();
    let seed = get(
        &app,
        &format!("/api/business/confirmations/{confirmation}/correction-seed"),
    )
    .await;
    let mut change = seed["draft"].clone();
    change["reason"] = json!("가상 개수 정정");
    change["action"] =
        json!({"kind":"replace","replacement":{"kind":"sale_count","data":{"count":"1"}}});
    let correction = confirm(
        &app,
        &draft(&app, "correction", change, Some(confirmation)).await,
    )
    .await;
    let saved=draft(&app,"waste",json!({"material_id":null,"material_version_id":null,"quantity":"","unit":"g","reason":"discard","note":""}),None).await;
    let state = get(&app, "/api/state").await;
    let stats_path = "/api/statistics?from=2020-01-01T00%3A00%3A00Z&until=2090-01-01T00%3A00%3A00Z";
    let stats = get(&app, stats_path).await;
    let before = disk(directory.path());
    let report = get(&app, "/api/overview").await;
    assert_eq!(report["state"], state);
    let raw_row = report["materials"]["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["material"]["id"] == raw["id"])
        .unwrap();
    assert_eq!(raw_row["last_observation"]["quantity"], "8");
    assert_eq!(
        raw_row["last_observation"]["origin"]["id"],
        stocktake["result"]["stocktake_id"]
    );
    assert_eq!(raw_row["changes_after_observation"]["total_count"], 3);
    for change in raw_row["changes_after_observation"]["rows"]
        .as_array()
        .unwrap()
    {
        let origin = &change["origin"];
        let route = match origin["kind"].as_str().unwrap() {
            "production" => "productions",
            "waste" => "wastes",
            "purchase_return" => "purchase-returns",
            other => panic!("unexpected later event {other}"),
        };
        let detail = get(
            &app,
            &format!("/api/{route}/{}", origin["id"].as_str().unwrap()),
        )
        .await;
        assert_eq!(detail["item"]["id"], origin["id"]);
        assert_eq!(
            detail["item"]["business_record_id"],
            origin["business_record_id"]
        );
    }
    let sale_row = &report["sales"]["rows"][0];
    assert_eq!(sale_row["origin"]["id"], sale["result"]["sale_id"]);
    assert_eq!(sale_row["count"], "1");
    let change = sale_row["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["kind"] == "correction")
        .unwrap();
    assert_eq!(change["id"], correction["result"]["correction_id"]);
    assert_eq!(
        get(
            &app,
            &format!("/api/corrections/{}", change["id"].as_str().unwrap())
        )
        .await["item"]["id"],
        change["id"]
    );
    assert_eq!(
        report["drafts"]["rows"][0]["id"],
        saved["result"]["item"]["id"]
    );
    assert_eq!(report["drafts"]["rows"][0]["validation"], "not_evaluated");
    for (section, route) in [
        ("receipts", "receipts"),
        ("sales", "sales"),
        ("observations", "stocktakes"),
    ] {
        for row in report[section]["rows"].as_array().unwrap() {
            let origin = &row["origin"];
            assert_eq!(
                get(
                    &app,
                    &format!("/api/{route}/{}", origin["id"].as_str().unwrap())
                )
                .await["item"]["id"],
                origin["id"]
            );
            assert_eq!(
                get(
                    &app,
                    &format!(
                        "/api/business/{}",
                        origin["business_record_id"].as_str().unwrap()
                    )
                )
                .await["item"]["id"],
                origin["business_record_id"]
            );
        }
    }
    for section in SECTIONS {
        let selected = (section == "material_changes").then(|| raw["id"].as_str().unwrap());
        let page = get(&app, &list_path(section, "0", &state, selected)).await;
        if section != "material_changes" {
            assert_eq!(page["list"]["page"], report[section]);
        } else {
            assert_eq!(page["list"]["page"], raw_row["changes_after_observation"]);
        }
    }
    assert_eq!(get(&app, stats_path).await, stats);
    assert_eq!(get(&app, "/api/state").await, state);
    assert_eq!(disk(directory.path()), before);
}
