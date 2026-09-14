use axum::{
    extract::{State, DefaultBodyLimit},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;
use tower_governor::{governor::GovernorConfigBuilder, GovernorLayer, key_extractor::SmartIpKeyExtractor};

use crate::state::{AppState, PlaybackState, get_current_state};

#[derive(Clone)]
pub struct SharedData {
    pub state: AppState,
    pub start_time: Instant,
    pub api_key: String,
}

pub fn router(shared_data: SharedData) -> Router {
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_millisecond(2000) // 1 petición cada 2 segundos = 30 por minuto
            .burst_size(30) // Ráfaga permitida de 30
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .unwrap(),
    );

    let read_router = Router::new()
        .route("/", get(root_handler))
        .route("/api/current", get(current_handler))
        .layer(GovernorLayer { config: governor_conf });

    let write_router = Router::new()
        .route("/api/update", post(update_handler))
        .layer(DefaultBodyLimit::max(512 * 1024)); // HTTP 413 Payload Too Large > 512 KB

    let health_router = Router::new().route("/health", get(health_handler));

    Router::new()
        .merge(read_router)
        .merge(write_router)
        .merge(health_router)
        .with_state(shared_data)
}

async fn health_handler(State(data): State<SharedData>) -> impl IntoResponse {
    let uptime = data.start_time.elapsed().as_secs();
    (StatusCode::OK, Json(json!({ "status": "nominal", "uptime_sec": uptime })))
}

async fn root_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CACHE_CONTROL, "public, max-age=0, s-maxage=60, stale-while-revalidate=30")],
        "Renderizador SVG pendiente",
    )
}

async fn current_handler(State(data): State<SharedData>) -> impl IntoResponse {
    if let Some(winner) = get_current_state(&data.state).await {
        (StatusCode::OK, Json(json!(winner))).into_response()
    } else {
        // Fallback visual si el estado está vacío
        (StatusCode::OK, Json(json!({ "is_playing": false }))).into_response()
    }
}

async fn update_handler(
    State(data): State<SharedData>,
    headers: HeaderMap,
    Json(payload): Json<PlaybackState>,
) -> impl IntoResponse {
    let api_key = headers.get("X-API-Key").and_then(|h| h.to_str().ok()).unwrap_or_default();
    
    if api_key != data.api_key {
        return StatusCode::FORBIDDEN;
    }

    let mut map = data.state.write().await;
    map.insert(payload.device_id.clone(), payload);

    StatusCode::OK
}
