use crate::{
    business::{
        ArchiveBusiness, BusinessFilter, BusinessKind, ConfirmBusiness, CreateBusinessDraft,
        PreviewBusiness, ReviseBusinessDraft,
    },
    catalog::{
        ArchiveCatalogMaterial, CatalogFilter, CreateCatalogMaterial, CreateCatalogPrice,
        ReviseCatalogMaterial,
    },
    lifecycle::{Lifecycle, Policy},
    photo_assets::{MAX_PHOTO_BYTES, PhotoAssetError, PhotoUpload, photo_upload},
    production::ProductionFilter,
    profile::{MAX_PROFILE_METADATA_BYTES, ReviseMaterialProfile},
    purchase_order::OrderFilter,
    purchase_return::PurchaseReturnFilter,
    receipt::ReceiptFilter,
    recipe::{CopyRecipe, CreateRecipe, RecipeFilter, ReviseRecipe},
    sale::SaleFilter,
    stocktake::StocktakeFilter,
    store::{CreateMaterial, Store, StoreError},
    waste::WasteFilter,
};
use axum::{
    Json, Router,
    body::Body,
    extract::{
        DefaultBodyLimit, FromRequest, Multipart, Path, Query, State, WebSocketUpgrade,
        multipart::MultipartError,
        rejection::{JsonRejection, QueryRejection},
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, HeaderValue, Method, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant, SystemTime},
};
use tokio::sync::{Notify, Semaphore, watch};

const MAX_PROFILE_BODY_BYTES: usize = 11 * 1024 * 1024;
const PROFILE_BODY_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct AppState(Arc<Inner>);
struct Inner {
    store: Mutex<Store>,
    host: String,
    token: String,
    cookie_name: String,
    lifecycle: Mutex<Lifecycle>,
    started: Instant,
    sequence: AtomicU64,
    stopping: AtomicBool,
    stop: watch::Sender<bool>,
    sockets: AtomicUsize,
    sockets_done: Notify,
    photo_uploads: Arc<Semaphore>,
}
impl AppState {
    pub fn new(store: Store, host: String, token: String, policy: Policy) -> Self {
        let (stop, _) = watch::channel(false);
        let port = host.rsplit(':').next().unwrap_or("local");
        Self(Arc::new(Inner {
            store: Mutex::new(store),
            cookie_name: format!("hansang_{port}"),
            host,
            token,
            lifecycle: Mutex::new(Lifecycle::new(policy)),
            started: Instant::now(),
            sequence: AtomicU64::new(0),
            stopping: AtomicBool::new(false),
            stop,
            sockets: AtomicUsize::new(0),
            sockets_done: Notify::new(),
            photo_uploads: Arc::new(Semaphore::new(1)),
        }))
    }
    fn now(&self) -> u64 {
        self.0.started.elapsed().as_millis().min(u64::MAX as u128) as u64
    }
    pub fn shutdown(&self) {
        self.0.stopping.store(true, Ordering::SeqCst);
        self.0.stop.send_replace(true);
    }
    pub async fn cancelled(&self) {
        let mut stop = self.0.stop.subscribe();
        if !*stop.borrow() {
            let _ = stop.changed().await;
        }
    }
    pub async fn wait_for_sockets(&self) {
        while self.0.sockets.load(Ordering::SeqCst) != 0 {
            self.0.sockets_done.notified().await;
        }
    }
    pub async fn watchdog(&self) {
        let mut tick = tokio::time::interval(Duration::from_millis(100));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut wall = SystemTime::now();
        loop {
            tokio::select! {
                _ = self.cancelled() => break,
                _ = tick.tick() => {
                    let new_wall = SystemTime::now();
                    let wall_gap = new_wall.duration_since(wall).unwrap_or_default().as_millis().min(u64::MAX as u128) as u64;
                    wall = new_wall;
                    if self.0.lifecycle.lock().unwrap().tick(self.now(), wall_gap) { self.shutdown(); break; }
                }
            }
        }
    }
    async fn with_store<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Store) -> Result<T, StoreError> + Send + 'static,
    ) -> Result<T, ApiError> {
        let state = self.clone();
        tokio::task::spawn_blocking(move || {
            let mut store = state.0.store.lock().map_err(|_| {
                ApiError(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "저장소를 다시 열어 주세요.".into(),
                    "unavailable",
                )
            })?;
            // This check under the writer mutex rejects queued work on shutdown.
            // Once past it, the complete DB + journal operation is allowed to finish.
            if state.0.stopping.load(Ordering::SeqCst) {
                return Err(ApiError(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "앱을 종료하는 중입니다.".into(),
                    "unavailable",
                ));
            }
            f(&mut store).map_err(ApiError::from)
        })
        .await
        .map_err(|_| {
            ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                "저장 작업을 마치지 못했습니다.".into(),
                "internal",
            )
        })?
    }
}

#[derive(Debug)]
pub struct ApiError(StatusCode, String, &'static str);
impl From<StoreError> for ApiError {
    fn from(e: StoreError) -> Self {
        let (status, code) = match &e {
            StoreError::Validation(_) => (StatusCode::BAD_REQUEST, "validation"),
            StoreError::Conflict(_) => (StatusCode::CONFLICT, "request_conflict"),
            StoreError::StaleState(_) => (StatusCode::CONFLICT, "stale_state"),
            StoreError::Unavailable(_) => (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
        };
        Self(status, e.to_string(), code)
    }
}
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error": self.1, "code": self.2}))).into_response()
    }
}
fn error(status: StatusCode, message: &str) -> Response {
    let code = match status {
        StatusCode::BAD_REQUEST => "validation",
        StatusCode::UNAUTHORIZED => "unauthorized",
        StatusCode::FORBIDDEN => "forbidden",
        StatusCode::NOT_FOUND => "not_found",
        StatusCode::METHOD_NOT_ALLOWED => "method_not_allowed",
        StatusCode::SERVICE_UNAVAILABLE => "unavailable",
        _ => "internal",
    };
    ApiError(status, message.into(), code).into_response()
}

fn json_input<T>(input: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    input.map(|Json(value)| value).map_err(|error| {
        let status = if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            StatusCode::PAYLOAD_TOO_LARGE
        } else {
            StatusCode::BAD_REQUEST
        };
        ApiError(status, error.body_text(), "validation")
    })
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route(
            "/api/health",
            get(|| async { Json(json!({"status":"ok"})) }),
        )
        .route("/api/session", post(session))
        .route("/api/state", get(catalog_state))
        .route("/api/materials", get(list).post(create))
        .route("/api/materials/{id}", get(detail))
        .route("/api/materials/{id}/versions", post(revise))
        .route("/api/materials/{id}/archive", post(archive))
        .route("/api/materials/{id}/prices", post(add_price))
        .route("/api/materials/{id}/profile", get(profile_current))
        .route(
            "/api/materials/{id}/profiles",
            get(profile_versions)
                .merge(post(profile_revise).layer(DefaultBodyLimit::max(MAX_PROFILE_BODY_BYTES))),
        )
        .route(
            "/api/materials/{id}/profiles/{version}",
            get(profile_version),
        )
        .route("/api/photo-assets/{sha256}", get(photo_asset))
        .route("/api/business", get(business_list))
        .route("/api/business/drafts", post(business_create))
        .route("/api/business/{id}", get(business_detail))
        .route("/api/business/{id}/draft-revisions", post(business_revise))
        .route("/api/business/{id}/preview", post(business_preview))
        .route("/api/business/{id}/confirm", post(business_confirm))
        .route("/api/business/{id}/archive", post(business_archive))
        .route(
            "/api/business/confirmations/{id}/correction-seed",
            get(correction_seed),
        )
        .route("/api/corrections/{id}", get(correction_detail))
        .route(
            "/api/corrections/drafts",
            post(correction_draft_create).layer(DefaultBodyLimit::max(256 * 1024)),
        )
        .route(
            "/api/corrections/{id}/draft-revisions",
            post(correction_draft_revise).layer(DefaultBodyLimit::max(256 * 1024)),
        )
        .route(
            "/api/production-drafts",
            post(production_draft_create).layer(DefaultBodyLimit::max(256 * 1024)),
        )
        .route(
            "/api/production-drafts/{id}/revisions",
            post(production_draft_revise).layer(DefaultBodyLimit::max(256 * 1024)),
        )
        .route("/api/productions", get(production_list))
        .route("/api/productions/{id}", get(production_detail))
        .route("/api/sales", get(sale_list))
        .route("/api/sales/{id}", get(sale_detail))
        .route(
            "/api/stocktake-drafts",
            post(stocktake_draft_create).layer(DefaultBodyLimit::max(256 * 1024)),
        )
        .route(
            "/api/stocktake-drafts/{id}/revisions",
            post(stocktake_draft_revise).layer(DefaultBodyLimit::max(256 * 1024)),
        )
        .route("/api/stocktakes", get(stocktake_list))
        .route("/api/stocktakes/{id}", get(stocktake_detail))
        .route("/api/wastes", get(waste_list))
        .route("/api/wastes/{id}", get(waste_detail))
        .route("/api/purchase-returns", get(purchase_return_list))
        .route("/api/purchase-returns/{id}", get(purchase_return_detail))
        .route("/api/orders", get(order_list))
        .route("/api/orders/{id}", get(order_detail))
        .route("/api/receipts", get(receipt_list))
        .route("/api/receipts/{id}", get(receipt_detail))
        .route(
            "/api/receipts/{id}/return-source",
            get(purchase_return_source),
        )
        .route(
            "/api/materials/{id}/receipt-inclusion-bases",
            get(receipt_inclusion_candidates),
        )
        .route("/api/holdings/{id}", get(holding_detail))
        .route(
            "/api/recipes",
            get(recipe_list).merge(post(recipe_create).layer(DefaultBodyLimit::max(256 * 1024))),
        )
        .route("/api/recipes/{id}", get(recipe_detail))
        .route("/api/recipes/{id}/cost", get(recipe_cost))
        .route("/api/recipes/{id}/versions/{version}", get(recipe_version))
        .route(
            "/api/recipes/{id}/versions/{version}/cost",
            get(recipe_version_cost),
        )
        .route("/api/materials/{id}/recipe-uses", get(recipe_uses))
        .route(
            "/api/recipes/{id}/versions",
            post(recipe_revise).layer(DefaultBodyLimit::max(256 * 1024)),
        )
        .route(
            "/api/recipes/{id}/copies",
            post(recipe_copy).layer(DefaultBodyLimit::max(256 * 1024)),
        )
        .route("/api/heartbeat", get(heartbeat))
        .fallback(static_asset)
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), guard))
        .with_state(state)
}
async fn guard(State(state): State<AppState>, request: Request<Body>, next: Next) -> Response {
    let headers = request.headers();
    if headers.get("host").and_then(|v| v.to_str().ok()) != Some(state.0.host.as_str()) {
        return error(StatusCode::FORBIDDEN, "이 앱의 로컬 주소로 접속해 주세요.");
    }
    let expected_origin = format!("http://{}", state.0.host);
    let origin = headers.get("origin").and_then(|v| v.to_str().ok());
    if origin.is_some_and(|value| value != expected_origin) {
        return error(
            StatusCode::FORBIDDEN,
            "다른 웹사이트의 요청은 허용하지 않습니다.",
        );
    }
    let path = request.uri().path();
    if path.starts_with("/api/") {
        if state.0.stopping.load(Ordering::SeqCst) {
            return error(StatusCode::SERVICE_UNAVAILABLE, "앱을 종료하는 중입니다.");
        }
        if (request.method() != Method::GET || path == "/api/heartbeat")
            && origin != Some(expected_origin.as_str())
        {
            return error(StatusCode::FORBIDDEN, "요청의 출처를 확인할 수 없습니다.");
        }
        if path != "/api/session" && path != "/api/health" {
            let expected = format!("{}={}", state.0.cookie_name, state.0.token);
            let authorized = headers
                .get_all("cookie")
                .iter()
                .filter_map(|h| h.to_str().ok())
                .any(|c| c.split(';').any(|v| v.trim() == expected));
            if !authorized {
                return error(
                    StatusCode::UNAUTHORIZED,
                    "실행한 앱의 주소에서 다시 열어 주세요.",
                );
            }
        }
    }
    let mut response = next.run(request).await;
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
        .headers_mut()
        .insert("x-frame-options", HeaderValue::from_static("DENY"));
    response
        .headers_mut()
        .insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    response
        .headers_mut()
        .insert("cache-control", HeaderValue::from_static("no-store"));
    response
}
#[derive(Deserialize)]
struct Session {
    token: String,
}
async fn session(State(state): State<AppState>, Json(input): Json<Session>) -> Response {
    if input.token != state.0.token {
        return error(
            StatusCode::UNAUTHORIZED,
            "실행한 앱의 주소에서 다시 열어 주세요.",
        );
    }
    let cookie = format!(
        "{}={}; HttpOnly; SameSite=Strict; Path=/",
        state.0.cookie_name, state.0.token
    );
    (StatusCode::NO_CONTENT, [("set-cookie", cookie)]).into_response()
}
async fn catalog_state(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(state.with_store(|s| s.catalog_state()).await?))
}

fn profile_missing() -> ApiError {
    ApiError(
        StatusCode::NOT_FOUND,
        "품목 또는 사진·구입처 버전을 찾을 수 없습니다.".into(),
        "not_found",
    )
}
async fn profile_current(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(
        state
            .with_store(move |store| store.profile_current(&id))
            .await?
            .ok_or_else(profile_missing)?,
    ))
}
async fn profile_versions(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(
        state
            .with_store(move |store| store.profile_versions(&id))
            .await?
            .ok_or_else(profile_missing)?,
    ))
}
async fn profile_version(
    State(state): State<AppState>,
    Path((id, version)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(
        state
            .with_store(move |store| store.profile_version(&id, &version))
            .await?
            .ok_or_else(profile_missing)?,
    ))
}
fn photo_error(error: PhotoAssetError) -> ApiError {
    match error {
        PhotoAssetError::Validation(message) => {
            ApiError(StatusCode::BAD_REQUEST, message, "validation")
        }
        PhotoAssetError::Unavailable(message) => {
            ApiError(StatusCode::SERVICE_UNAVAILABLE, message, "unavailable")
        }
    }
}
fn multipart_error(error: MultipartError) -> ApiError {
    let status = if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
        StatusCode::PAYLOAD_TOO_LARGE
    } else {
        StatusCode::BAD_REQUEST
    };
    ApiError(
        status,
        "사진 전송 본문 또는 크기를 확인해 주세요.".into(),
        "validation",
    )
}
fn profile_invalid(message: &str) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, message.into(), "validation")
}
async fn profile_body(
    request: Request<Body>,
    state: &AppState,
) -> Result<(ReviseMaterialProfile, Option<PhotoUpload>), ApiError> {
    // Read the entire bounded envelope, including any multipart epilogue. A parser
    // may stop at the closing boundary before a chunked body's remaining bytes.
    let (parts, body) = request.into_parts();
    let mut stream = body.into_data_stream();
    let mut envelope = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|_| profile_invalid("사진 전송 본문을 끝까지 읽지 못했습니다."))?;
        if envelope
            .len()
            .checked_add(chunk.len())
            .is_none_or(|size| size > MAX_PROFILE_BODY_BYTES)
        {
            return Err(ApiError(
                StatusCode::PAYLOAD_TOO_LARGE,
                "전체 사진 전송 크기가 11MiB를 넘었습니다.".into(),
                "validation",
            ));
        }
        envelope.try_reserve(chunk.len()).map_err(|_| {
            ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "사진 전송을 위한 메모리를 확보하지 못했습니다.".into(),
                "unavailable",
            )
        })?;
        envelope.extend_from_slice(&chunk);
    }
    let request = Request::from_parts(parts, Body::from(envelope));
    let mut multipart = Multipart::from_request(request, state)
        .await
        .map_err(|_| profile_invalid("multipart 사진 전송 형식이 올바르지 않습니다."))?;
    let mut metadata = None;
    let mut photo = None;
    while let Some(mut field) = multipart.next_field().await.map_err(multipart_error)? {
        let name = field
            .name()
            .ok_or_else(|| profile_invalid("전송 항목의 이름이 없습니다."))?
            .to_owned();
        let filename = field.file_name().map(str::to_owned);
        let mime = field.content_type().map(str::to_owned);
        let limit = match name.as_str() {
            "metadata" if metadata.is_none() && filename.is_none() => MAX_PROFILE_METADATA_BYTES,
            "photo" if photo.is_none() && filename.is_some() => MAX_PHOTO_BYTES,
            _ => return Err(profile_invalid("알 수 없거나 중복된 사진 전송 항목입니다.")),
        };
        let mut bytes = Vec::new();
        while let Some(chunk) = field.chunk().await.map_err(multipart_error)? {
            if bytes
                .len()
                .checked_add(chunk.len())
                .is_none_or(|size| size > limit)
            {
                return Err(ApiError(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "사진 또는 설명 전송 크기가 한도를 넘었습니다.".into(),
                    "validation",
                ));
            }
            bytes.try_reserve(chunk.len()).map_err(|_| {
                ApiError(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "사진 전송을 위한 메모리를 확보하지 못했습니다.".into(),
                    "unavailable",
                )
            })?;
            bytes.extend_from_slice(&chunk);
        }
        if name == "metadata" {
            metadata = Some(
                serde_json::from_slice(&bytes)
                    .map_err(|_| profile_invalid("사진·구입처 입력 형식을 확인해 주세요."))?,
            );
        } else {
            photo = Some(
                photo_upload(
                    filename.as_deref().unwrap_or_default(),
                    mime.as_deref(),
                    bytes,
                )
                .map_err(photo_error)?,
            );
        }
    }
    Ok((
        metadata.ok_or_else(|| profile_invalid("사진·구입처 입력 정보가 없습니다."))?,
        photo,
    ))
}
async fn profile_revise(
    State(state): State<AppState>,
    Path(id): Path<String>,
    request: Request<Body>,
) -> Result<impl IntoResponse, ApiError> {
    // Admission precedes body polling. Moving the permit into blocking work also
    // retains it if the HTTP client disconnects after the Store accepted the write.
    let permit = state
        .0
        .photo_uploads
        .clone()
        .try_acquire_owned()
        .map_err(|_| {
            ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "다른 사진 저장을 마친 뒤 같은 요청을 다시 시도해 주세요.".into(),
                "unavailable",
            )
        })?;
    let (input, photo) = tokio::select! {
        _ = state.cancelled() => return Err(ApiError(StatusCode::SERVICE_UNAVAILABLE, "앱을 종료하는 중입니다.".into(), "unavailable")),
        result = tokio::time::timeout(PROFILE_BODY_TIMEOUT, profile_body(request, &state)) => {
            result.map_err(|_| ApiError(StatusCode::REQUEST_TIMEOUT, "사진 전송 시간이 초과되었습니다. 같은 요청을 다시 시도해 주세요.".into(), "unavailable"))??
        }
    };
    let detail = state
        .with_store(move |store| {
            let _permit = permit;
            store.profile_revise(&id, input, photo)
        })
        .await?;
    Ok((StatusCode::CREATED, Json(detail)))
}
async fn photo_asset(
    State(state): State<AppState>,
    Path(sha256): Path<String>,
) -> Result<Response, ApiError> {
    let photo = state
        .with_store(move |store| store.photo_asset_bytes(&sha256))
        .await?
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                "등록된 사진을 찾을 수 없습니다.".into(),
                "not_found",
            )
        })?;
    let filename = format!(
        "inline; filename=\"{}.{}\"",
        photo.descriptor.sha256,
        photo.descriptor.format.extension()
    );
    Ok((
        [
            ("content-type", photo.descriptor.format.mime().to_owned()),
            ("content-length", photo.bytes.len().to_string()),
            ("content-disposition", filename),
            ("cross-origin-resource-policy", "same-origin".to_owned()),
        ],
        photo.bytes,
    )
        .into_response())
}
async fn list(
    State(state): State<AppState>,
    input: Result<Query<CatalogFilter>, QueryRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Query(filter) = input
        .map_err(|error| ApiError(StatusCode::BAD_REQUEST, error.body_text(), "validation"))?;
    Ok(Json(
        state.with_store(move |s| s.catalog_list(filter)).await?,
    ))
}
async fn detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let material = state
        .with_store(move |s| s.catalog_detail(&id))
        .await?
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                "재료를 찾을 수 없습니다.".into(),
                "not_found",
            )
        })?;
    Ok(Json(material))
}
async fn create(
    State(state): State<AppState>,
    input: Result<Json<Value>, JsonRejection>,
) -> Result<Response, ApiError> {
    let input = json_input(input)?;
    let object = input.as_object().ok_or_else(|| {
        ApiError(
            StatusCode::BAD_REQUEST,
            "재료 입력은 JSON 객체여야 합니다.".into(),
            "validation",
        )
    })?;
    // Only the original narrow shape can bypass metadata. Extended inputs must
    // never silently fall back when their revision or epoch is missing.
    if object
        .keys()
        .all(|key| matches!(key.as_str(), "request_id" | "name" | "unit"))
    {
        let input: CreateMaterial = serde_json::from_value(input)
            .map_err(|error| ApiError(StatusCode::BAD_REQUEST, error.to_string(), "validation"))?;
        return Ok((
            StatusCode::CREATED,
            Json(state.with_store(move |s| s.create(input)).await?),
        )
            .into_response());
    }
    let input: CreateCatalogMaterial = serde_json::from_value(input)
        .map_err(|error| ApiError(StatusCode::BAD_REQUEST, error.to_string(), "validation"))?;
    Ok((
        StatusCode::CREATED,
        Json(state.with_store(move |s| s.catalog_create(input)).await?),
    )
        .into_response())
}
async fn revise(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<ReviseCatalogMaterial>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = json_input(input)?;
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .with_store(move |s| s.catalog_revise(&id, input))
                .await?,
        ),
    ))
}
async fn archive(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<ArchiveCatalogMaterial>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = json_input(input)?;
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .with_store(move |s| s.catalog_archive(&id, input))
                .await?,
        ),
    ))
}
async fn add_price(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<CreateCatalogPrice>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = json_input(input)?;
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .with_store(move |s| s.catalog_add_price(&id, input))
                .await?,
        ),
    ))
}

async fn business_list(
    State(state): State<AppState>,
    input: Result<Query<BusinessFilter>, QueryRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Query(filter) = input
        .map_err(|error| ApiError(StatusCode::BAD_REQUEST, error.body_text(), "validation"))?;
    Ok(Json(
        state.with_store(move |s| s.business_list(filter)).await?,
    ))
}

async fn business_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let detail = state
        .with_store(move |s| s.business_detail(&id))
        .await?
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                "업무 기록을 찾을 수 없습니다.".into(),
                "not_found",
            )
        })?;
    Ok(Json(detail))
}

async fn business_create(
    State(state): State<AppState>,
    input: Result<Json<CreateBusinessDraft>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = json_input(input)?;
    Ok((
        StatusCode::CREATED,
        Json(state.with_store(move |s| s.business_create(input)).await?),
    ))
}

async fn business_revise(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<ReviseBusinessDraft>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = json_input(input)?;
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .with_store(move |s| s.business_revise(&id, input))
                .await?,
        ),
    ))
}

async fn business_preview(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<PreviewBusiness>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = json_input(input)?;
    Ok(Json(
        state
            .with_store(move |s| s.business_preview(&id, input))
            .await?,
    ))
}

async fn business_confirm(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<ConfirmBusiness>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = json_input(input)?;
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .with_store(move |s| s.business_confirm(&id, input))
                .await?,
        ),
    ))
}

async fn business_archive(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<ArchiveBusiness>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = json_input(input)?;
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .with_store(move |s| s.business_archive(&id, input))
                .await?,
        ),
    ))
}

async fn correction_seed(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .with_store(move |s| s.correction_seed(&id))
        .await?
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                "정정할 원기록을 찾을 수 없습니다.".into(),
                "not_found",
            )
        })?;
    Ok(Json(result))
}

async fn correction_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let result = state
        .with_store(move |s| s.correction_detail(&id))
        .await?
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                "정정 내역을 찾을 수 없습니다.".into(),
                "not_found",
            )
        })?;
    Ok(Json(result))
}

async fn correction_draft_create(
    State(state): State<AppState>,
    input: Result<Json<CreateBusinessDraft>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = json_input(input)?;
    if input.payload.kind() != BusinessKind::Correction {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "이 경로에는 정정 초안만 저장할 수 있습니다.".into(),
            "validation",
        ));
    }
    Ok((
        StatusCode::CREATED,
        Json(state.with_store(move |s| s.business_create(input)).await?),
    ))
}

async fn correction_draft_revise(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<ReviseBusinessDraft>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = json_input(input)?;
    if input.payload.kind() != BusinessKind::Correction {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "이 경로에는 정정 초안만 저장할 수 있습니다.".into(),
            "validation",
        ));
    }
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .with_store(move |s| s.business_revise(&id, input))
                .await?,
        ),
    ))
}

async fn production_draft_create(
    State(state): State<AppState>,
    input: Result<Json<CreateBusinessDraft>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = json_input(input)?;
    if input.payload.kind() != BusinessKind::Production {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "이 경로에는 생산 초안만 저장할 수 있습니다.".into(),
            "validation",
        ));
    }
    Ok((
        StatusCode::CREATED,
        Json(state.with_store(move |s| s.business_create(input)).await?),
    ))
}

async fn production_draft_revise(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<ReviseBusinessDraft>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = json_input(input)?;
    if input.payload.kind() != BusinessKind::Production {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "이 경로에는 생산 초안만 저장할 수 있습니다.".into(),
            "validation",
        ));
    }
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .with_store(move |s| s.business_revise(&id, input))
                .await?,
        ),
    ))
}

async fn production_list(
    State(state): State<AppState>,
    input: Result<Query<ProductionFilter>, QueryRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Query(filter) = input
        .map_err(|error| ApiError(StatusCode::BAD_REQUEST, error.body_text(), "validation"))?;
    Ok(Json(
        state.with_store(move |s| s.production_list(filter)).await?,
    ))
}

async fn production_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let detail = state
        .with_store(move |s| s.production_detail(&id))
        .await?
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                "생산 내역을 찾을 수 없습니다.".into(),
                "not_found",
            )
        })?;
    Ok(Json(detail))
}

async fn sale_list(
    State(state): State<AppState>,
    input: Result<Query<SaleFilter>, QueryRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Query(filter) = input
        .map_err(|error| ApiError(StatusCode::BAD_REQUEST, error.body_text(), "validation"))?;
    Ok(Json(state.with_store(move |s| s.sale_list(filter)).await?))
}

async fn stocktake_draft_create(
    State(state): State<AppState>,
    input: Result<Json<CreateBusinessDraft>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = json_input(input)?;
    if input.payload.kind() != BusinessKind::Stocktake {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "이 경로에는 실사 초안만 저장할 수 있습니다.".into(),
            "validation",
        ));
    }
    Ok((
        StatusCode::CREATED,
        Json(state.with_store(move |s| s.business_create(input)).await?),
    ))
}

async fn stocktake_draft_revise(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<ReviseBusinessDraft>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = json_input(input)?;
    if input.payload.kind() != BusinessKind::Stocktake {
        return Err(ApiError(
            StatusCode::BAD_REQUEST,
            "이 경로에는 실사 초안만 저장할 수 있습니다.".into(),
            "validation",
        ));
    }
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .with_store(move |s| s.business_revise(&id, input))
                .await?,
        ),
    ))
}

async fn stocktake_list(
    State(state): State<AppState>,
    input: Result<Query<StocktakeFilter>, QueryRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Query(filter) = input
        .map_err(|error| ApiError(StatusCode::BAD_REQUEST, error.body_text(), "validation"))?;
    Ok(Json(
        state.with_store(move |s| s.stocktake_list(filter)).await?,
    ))
}

async fn stocktake_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let detail = state
        .with_store(move |s| s.stocktake_detail(&id))
        .await?
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                "실사 내역을 찾을 수 없습니다.".into(),
                "not_found",
            )
        })?;
    Ok(Json(detail))
}

async fn waste_list(
    State(state): State<AppState>,
    input: Result<Query<WasteFilter>, QueryRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Query(filter) = input
        .map_err(|error| ApiError(StatusCode::BAD_REQUEST, error.body_text(), "validation"))?;
    Ok(Json(state.with_store(move |s| s.waste_list(filter)).await?))
}

async fn purchase_return_list(
    State(state): State<AppState>,
    input: Result<Query<PurchaseReturnFilter>, QueryRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Query(filter) = input
        .map_err(|error| ApiError(StatusCode::BAD_REQUEST, error.body_text(), "validation"))?;
    Ok(Json(
        state
            .with_store(move |s| s.purchase_return_list(filter))
            .await?,
    ))
}
async fn purchase_return_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let detail = state
        .with_store(move |s| s.purchase_return_detail(&id))
        .await?
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                "반품 내역을 찾을 수 없습니다.".into(),
                "not_found",
            )
        })?;
    Ok(Json(detail))
}
async fn purchase_return_source(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let source = state
        .with_store(move |s| s.purchase_return_source(&id))
        .await?
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                "원입고 내역을 찾을 수 없습니다.".into(),
                "not_found",
            )
        })?;
    Ok(Json(source))
}

async fn waste_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let detail = state
        .with_store(move |s| s.waste_detail(&id))
        .await?
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                "폐기 내역을 찾을 수 없습니다.".into(),
                "not_found",
            )
        })?;
    Ok(Json(detail))
}

async fn sale_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let detail = state
        .with_store(move |s| s.sale_detail(&id))
        .await?
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                "판매 내역을 찾을 수 없습니다.".into(),
                "not_found",
            )
        })?;
    Ok(Json(detail))
}

async fn order_list(
    State(state): State<AppState>,
    input: Result<Query<OrderFilter>, QueryRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Query(filter) = input
        .map_err(|error| ApiError(StatusCode::BAD_REQUEST, error.body_text(), "validation"))?;
    Ok(Json(state.with_store(move |s| s.order_list(filter)).await?))
}

async fn order_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let detail = state
        .with_store(move |s| s.order_detail(&id))
        .await?
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                "주문 내역을 찾을 수 없습니다.".into(),
                "not_found",
            )
        })?;
    Ok(Json(detail))
}

async fn receipt_list(
    State(state): State<AppState>,
    input: Result<Query<ReceiptFilter>, QueryRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Query(filter) = input
        .map_err(|error| ApiError(StatusCode::BAD_REQUEST, error.body_text(), "validation"))?;
    Ok(Json(
        state.with_store(move |s| s.receipt_list(filter)).await?,
    ))
}

async fn receipt_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let detail = state
        .with_store(move |s| s.receipt_detail(&id))
        .await?
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                "입고 내역을 찾을 수 없습니다.".into(),
                "not_found",
            )
        })?;
    Ok(Json(detail))
}

async fn receipt_inclusion_candidates(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    Ok(Json(
        state
            .with_store(move |s| s.receipt_inclusion_candidates(&id))
            .await?,
    ))
}

async fn holding_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let detail = state
        .with_store(move |s| s.holding_detail(&id))
        .await?
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                "재료를 찾을 수 없습니다.".into(),
                "not_found",
            )
        })?;
    Ok(Json(detail))
}

async fn recipe_list(
    State(state): State<AppState>,
    input: Result<Query<RecipeFilter>, QueryRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let Query(filter) = input
        .map_err(|error| ApiError(StatusCode::BAD_REQUEST, error.body_text(), "validation"))?;
    Ok(Json(
        state.with_store(move |s| s.recipe_list(filter)).await?,
    ))
}

fn recipe_not_found() -> ApiError {
    ApiError(
        StatusCode::NOT_FOUND,
        "레시피 또는 버전을 찾을 수 없습니다.".into(),
        "not_found",
    )
}

async fn recipe_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let detail = state
        .with_store(move |s| s.recipe_detail(&id))
        .await?
        .ok_or_else(recipe_not_found)?;
    Ok(Json(detail))
}

async fn recipe_version(
    State(state): State<AppState>,
    Path((id, version)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let version = state
        .with_store(move |s| s.recipe_version(&id, &version))
        .await?
        .ok_or_else(recipe_not_found)?;
    Ok(Json(version))
}

async fn recipe_cost(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let cost = state
        .with_store(move |s| s.recipe_cost(&id, None))
        .await?
        .ok_or_else(recipe_not_found)?;
    Ok(Json(cost))
}

async fn recipe_version_cost(
    State(state): State<AppState>,
    Path((id, version)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let cost = state
        .with_store(move |s| s.recipe_cost(&id, Some(&version)))
        .await?
        .ok_or_else(recipe_not_found)?;
    Ok(Json(cost))
}

async fn recipe_uses(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let uses = state
        .with_store(move |s| s.recipe_uses(&id))
        .await?
        .ok_or_else(|| {
            ApiError(
                StatusCode::NOT_FOUND,
                "재료를 찾을 수 없습니다.".into(),
                "not_found",
            )
        })?;
    Ok(Json(uses))
}

async fn recipe_create(
    State(state): State<AppState>,
    input: Result<Json<CreateRecipe>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = json_input(input)?;
    Ok((
        StatusCode::CREATED,
        Json(state.with_store(move |s| s.recipe_create(input)).await?),
    ))
}

async fn recipe_revise(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<ReviseRecipe>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = json_input(input)?;
    Ok((
        StatusCode::CREATED,
        Json(
            state
                .with_store(move |s| s.recipe_revise(&id, input))
                .await?,
        ),
    ))
}

async fn recipe_copy(
    State(state): State<AppState>,
    Path(id): Path<String>,
    input: Result<Json<CopyRecipe>, JsonRejection>,
) -> Result<impl IntoResponse, ApiError> {
    let input = json_input(input)?;
    Ok((
        StatusCode::CREATED,
        Json(state.with_store(move |s| s.recipe_copy(&id, input)).await?),
    ))
}

struct SocketGuard(AppState);
impl Drop for SocketGuard {
    fn drop(&mut self) {
        self.0.0.sockets.fetch_sub(1, Ordering::SeqCst);
        self.0.0.sockets_done.notify_one();
    }
}
async fn heartbeat(State(state): State<AppState>, upgrade: WebSocketUpgrade) -> Response {
    // Register before on_upgrade spawns, so shutdown waits for pending upgrades too.
    state.0.sockets.fetch_add(1, Ordering::SeqCst);
    let guard = SocketGuard(state.clone());
    upgrade
        .max_message_size(1024)
        .max_frame_size(1024)
        .on_upgrade(move |socket| websocket(state, socket, guard))
}
async fn websocket(state: AppState, mut socket: WebSocket, _guard: SocketGuard) {
    if state.0.stopping.load(Ordering::SeqCst) {
        return;
    }
    let id = state.0.sequence.fetch_add(1, Ordering::Relaxed);
    state.0.lifecycle.lock().unwrap().connect(id, state.now());
    let ping_ms = state.0.lifecycle.lock().unwrap().policy.ping_ms;
    let mut timer = tokio::time::interval(Duration::from_millis(ping_ms));
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            biased;
            _ = state.cancelled() => { break; }
            message = socket.recv() => match message {
                Some(Ok(Message::Pong(_))) => { state.0.lifecycle.lock().unwrap().pong(id, state.now()); }
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                _ => {},
            },
            _ = timer.tick() => {
                if !state.0.lifecycle.lock().unwrap().contains(id) { break; }
                if !matches!(tokio::time::timeout(Duration::from_secs(2), socket.send(Message::Ping(Vec::new().into()))).await, Ok(Ok(()))) { break; }
            }
        }
    }
    state
        .0
        .lifecycle
        .lock()
        .unwrap()
        .disconnect(id, state.now());
    let _ = tokio::time::timeout(Duration::from_secs(1), socket.close()).await;
}
#[derive(rust_embed::RustEmbed)]
#[folder = "frontend/.output/public/"]
struct Assets;

async fn static_asset(_headers: HeaderMap, request: Request<Body>) -> Response {
    if request.method() != Method::GET && request.method() != Method::HEAD {
        return error(
            StatusCode::METHOD_NOT_ALLOWED,
            "이 경로에서는 조회만 가능합니다.",
        );
    }
    let path = request.uri().path();
    let spa_route = path == "/"
        || path == "/overview"
        || path == "/workspace"
        || path == "/materials"
        || path == "/business"
        || path == "/orders"
        || path == "/receipts"
        || path == "/productions"
        || path == "/sales"
        || path == "/stocktakes"
        || path == "/wastes"
        || path == "/purchase-returns"
        || path == "/recipes"
        || path == "/recipes/new"
        || path
            .strip_prefix("/materials/")
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
        || path
            .strip_prefix("/business/")
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
        || path
            .strip_prefix("/orders/")
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
        || path
            .strip_prefix("/receipts/")
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
        || path
            .strip_prefix("/productions/")
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
        || path
            .strip_prefix("/sales/")
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
        || path
            .strip_prefix("/stocktakes/")
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
        || path
            .strip_prefix("/wastes/")
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
        || path
            .strip_prefix("/purchase-returns/")
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
        || path
            .strip_prefix("/corrections/")
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_ok())
        || path.strip_prefix("/recipes/").is_some_and(|part| {
            uuid::Uuid::parse_str(part).is_ok()
                || part.split_once("/versions/").is_some_and(|(id, version)| {
                    uuid::Uuid::parse_str(id).is_ok() && uuid::Uuid::parse_str(version).is_ok()
                })
        });
    let asset_path = if spa_route {
        "index.html"
    } else {
        path.trim_start_matches('/')
    };
    if path.starts_with("/api/")
        || asset_path
            .split('/')
            .any(|part| part == "." || part == "..")
    {
        return error(StatusCode::NOT_FOUND, "요청한 경로를 찾을 수 없습니다.");
    }
    match Assets::get(asset_path) {
        Some(file) => {
            let mime = mime_guess::from_path(asset_path)
                .first_or_octet_stream()
                .to_string();
            let body = if request.method() == Method::HEAD {
                Body::empty()
            } else {
                Body::from(file.data.into_owned())
            };
            ([("content-type", mime)], body).into_response()
        }
        None => error(StatusCode::NOT_FOUND, "요청한 파일을 찾을 수 없습니다."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn accepted_photo_write_keeps_admission_after_disconnect_and_finishes_during_shutdown() {
        use image::ImageEncoder;
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        let item = store
            .create(CreateMaterial {
                request_id: uuid::Uuid::new_v4().to_string(),
                name: "가상 사진 종료".into(),
                unit: "g".into(),
            })
            .unwrap();
        let version = store
            .catalog_detail(&item.id)
            .unwrap()
            .unwrap()
            .item
            .version_id;
        let catalog_state = store.catalog_state().unwrap();
        let input: ReviseMaterialProfile = serde_json::from_value(json!({
            "meta":{"request_id":uuid::Uuid::new_v4().to_string(),"epoch":catalog_state.epoch,"expected_revision":catalog_state.revision},
            "expected_material_version_id":version,"expected_profile_version_id":null,
            "photos":[{"source":"upload","caption":"가상"}],"primary_position":0,"suppliers":[]
        })).unwrap();
        let mut bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut bytes)
            .write_image(&[1, 2, 3, 255], 1, 1, image::ExtendedColorType::Rgba8)
            .unwrap();
        let photo = photo_upload("synthetic.png", Some("image/png"), bytes).unwrap();
        let state = AppState::new(
            store,
            "127.0.0.1:4321".into(),
            "photo".into(),
            Policy::default(),
        );
        let permit = state.0.photo_uploads.clone().try_acquire_owned().unwrap();
        let worker = state.clone();
        let material_id = item.id.clone();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        let client = tokio::spawn(async move {
            worker
                .with_store(move |store| {
                    let _permit = permit;
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    let result = store.profile_revise(&material_id, input, Some(photo));
                    done_tx.send(result.is_ok()).unwrap();
                    result.map(|_| ())
                })
                .await
        });
        entered_rx.await.unwrap();
        client.abort();
        state.shutdown();
        assert_eq!(state.0.photo_uploads.available_permits(), 0);
        release_tx.send(()).unwrap();
        assert!(done_rx.await.unwrap());
        // Taking the Store lock waits for the accepted closure and its permit drop.
        let saved = state
            .0
            .store
            .lock()
            .unwrap()
            .profile_current(&item.id)
            .unwrap()
            .unwrap();
        assert_eq!(saved.version.unwrap().photos.len(), 1);
        assert_eq!(state.0.photo_uploads.available_permits(), 1);
        assert!(state.with_store(|store| store.list()).await.is_err());
        drop(state);
        let reopened = Store::open(dir.path()).unwrap();
        assert_eq!(
            reopened
                .profile_current(&item.id)
                .unwrap()
                .unwrap()
                .version
                .unwrap()
                .photos
                .len(),
            1
        );
        assert_eq!(
            std::fs::read_dir(dir.path().join("snapshots"))
                .unwrap()
                .count(),
            2
        );
    }
    #[tokio::test]
    async fn shutdown_allows_accepted_write_to_finish_but_refuses_later_work() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState::new(
            Store::open(dir.path()).unwrap(),
            "127.0.0.1:4321".into(),
            "secret".into(),
            Policy::default(),
        );
        let worker = state.clone();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let accepted = tokio::spawn(async move {
            worker
                .with_store(move |store| {
                    entered_tx.send(()).unwrap();
                    release_rx.recv().unwrap();
                    store.create(CreateMaterial {
                        request_id: uuid::Uuid::new_v4().to_string(),
                        name: "가상 종료 테스트".into(),
                        unit: "g".into(),
                    })
                })
                .await
        });
        entered_rx.await.unwrap();
        state.shutdown();
        assert!(!accepted.is_finished());
        release_tx.send(()).unwrap();
        assert_eq!(accepted.await.unwrap().unwrap().name, "가상 종료 테스트");
        assert!(state.with_store(|s| s.list()).await.is_err());
        drop(state);
        let reopened = Store::open(dir.path()).unwrap();
        assert_eq!(reopened.list().unwrap().len(), 1);
        assert_eq!(
            std::fs::read_dir(dir.path().join("snapshots"))
                .unwrap()
                .count(),
            1
        );
    }
}
