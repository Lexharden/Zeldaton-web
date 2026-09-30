//! Router assembly, shared by the binary and the integration tests.

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Method, header};
use axum::routing::{get, post};
use tower_http::compression::CompressionLayer;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::hub::AppState;
use crate::{admin, api, ws_ingest, ws_public};

pub fn build(hub: AppState) -> Router {
    let origins: Vec<HeaderValue> = hub
        .cfg
        .cors_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    Router::new()
        .nest(
            "/api",
            api::router().nest("/admin", admin::router(hub.clone())),
        )
        .route("/health", get(|| async { "ok" }))
        .route("/ws", get(ws_public::handler))
        .route("/ingest", get(ws_ingest::ws_handler))
        .route("/ingest/events", post(ws_ingest::post_events))
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(hub)
}
