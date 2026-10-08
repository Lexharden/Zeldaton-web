//! Public REST API consumed by the frontend (`src/services/api/HttpRaceApi.ts`).

use axum::extract::{Path, Query, State};
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
    live.route("/media/racers/{file}", get(crate::media::serve))
        .route("/donors", get(donors))
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
    let now = Utc::now();
    let required = hub.read(|s| s.event.rules.required_objective_ids.clone());
    Json(standings::compute(&hub.racers(now), &required))
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

#[derive(serde::Deserialize)]
struct DonorsQuery {
    limit: Option<i64>,
}

/// What the public donors board exposes: ranked donors with only the fields meant to be shown
/// (no racers, dates or gifts) and event-wide totals. `rows` come from `db::top_donors`.
pub fn donors_payload(enabled: bool, rows: Vec<Value>, totals: Value) -> Value {
    let donors: Vec<Value> = rows
        .into_iter()
        .enumerate()
        .map(|(i, d)| {
            json!({
                "rank": i + 1,
                "viewer": d["viewer"],
                "platform": d["platform"],
                "currency": d["currency"],
                "donations": d["donations"],
                "amount": d["amount"],
                "addedSeconds": d["addedSeconds"],
                "removedSeconds": d["removedSeconds"],
            })
        })
        .collect();
    json!({ "enabled": enabled, "totals": totals, "donors": donors })
}

/// The public donors board. Donors the organizer hid are left out, and the whole board is empty
/// while the organizer has it switched off. Cacheable for a few seconds.
async fn donors(
    State(hub): State<AppState>,
    Query(q): Query<DonorsQuery>,
) -> Result<impl axum::response::IntoResponse, ApiError> {
    let enabled = crate::admin::donors_public(&hub.pool).await;
    let body = if enabled {
        let limit = q.limit.unwrap_or(10).clamp(1, 20);
        let rows = crate::db::top_donors(&hub.pool, limit, false)
            .await
            .map_err(ApiError::internal)?;
        let totals = crate::db::donation_summary(&hub.pool)
            .await
            .map_err(ApiError::internal)?;
        donors_payload(true, rows, totals)
    } else {
        let totals = json!({ "donations": 0, "addedSeconds": 0, "removedSeconds": 0 });
        donors_payload(false, vec![], totals)
    };
    Ok((
        [(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=15"),
        )],
        Json(body),
    ))
}
