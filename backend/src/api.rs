//! Public REST API consumed by the frontend (`src/services/api/HttpRaceApi.ts`).

use axum::extract::{Path, State};
use axum::http::header::{CACHE_CONTROL, HeaderValue};
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use serde_json::{Value, json};
use tower_http::set_header::SetResponseHeaderLayer;

use crate::domain::*;
use crate::error::ApiError;
use crate::hub::AppState;
use crate::standings;

pub fn router() -> Router<AppState> {
    let live = Router::new()
        .route("/health", get(health))
        .route("/event", get(event))
        .route("/racers", get(racers))
        .route("/racers/{id}", get(racer))
        .route("/standings", get(standings_route))
        .route("/streams", get(streams))
        .route("/activity", get(activity))
        .route("/hiveshock/stats", get(hiveshock_stats))
        .route("/clocks", get(clocks))
        .route("/catalog", get(catalog))
        // Live race data must never be served from a stale cache.
        .layer(SetResponseHeaderLayer::overriding(
            CACHE_CONTROL,
            HeaderValue::from_static("no-store"),
        ));
    // Pictures are cacheable (they set their own Cache-Control), unlike the live race data.
    live.route("/media/items/{file}", get(crate::media::serve))
}

async fn health(State(hub): State<AppState>) -> Json<Value> {
    let now = Utc::now();
    Json(
        json!({ "ok": true, "serverTimeUtc": crate::engine::iso(now), "eventStatus": hub.event(now).status }),
    )
}

async fn event(State(hub): State<AppState>) -> Json<EventInfo> {
    Json(hub.event(Utc::now()))
}

async fn racers(State(hub): State<AppState>) -> Json<Vec<Racer>> {
    Json(hub.racers(Utc::now()))
}

async fn racer(
    State(hub): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Racer>, ApiError> {
    hub.racer(&id, Utc::now())
        .map(Json)
        .ok_or(ApiError::NotFound)
}

async fn standings_route(State(hub): State<AppState>) -> Json<Vec<StandingEntry>> {
    Json(standings::compute(&hub.racers(Utc::now())))
}

async fn streams(State(hub): State<AppState>) -> Json<Vec<StreamInfo>> {
    let list = hub
        .racers(Utc::now())
        .into_iter()
        .map(|r| {
            let s = r.stream.unwrap_or_default();
            StreamInfo {
                racer_id: r.id,
                is_live: s.is_live,
                thumbnail_url: s.thumbnail_url,
                viewers: s.viewers,
            }
        })
        .collect();
    Json(list)
}

async fn activity(State(hub): State<AppState>) -> Json<Vec<ActivityItem>> {
    Json(hub.activity())
}

async fn hiveshock_stats(State(hub): State<AppState>) -> Json<HiveShockStats> {
    Json(hub.stats())
}

/// Items and objectives (enabled only, in display order) plus a content `version` clients can compare.
async fn catalog(State(hub): State<AppState>) -> Json<Value> {
    let c = hub.catalog_public();
    Json(json!({ "version": c.version(), "items": c.items, "objectives": c.objectives }))
}

async fn clocks(State(hub): State<AppState>) -> Json<Vec<ClockState>> {
    Json(hub.clocks(Utc::now()))
}
