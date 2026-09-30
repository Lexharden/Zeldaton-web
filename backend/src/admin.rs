//! Organizer API (`/api/admin/*`, `Authorization: Bearer <ADMIN_TOKEN>`).
//! Every state-changing call is recorded in `audit_log`: the official session log.

use axum::extract::{Path, Query, Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{delete, get, patch, post, put};
use axum::{Json, Router};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::{bearer, constant_eq, hash_token, random_token};
use crate::db;
use crate::domain::{EventInfo, Racer};
use crate::engine::{AdminAction, EventPatch, NewRacer, RacerPatch};
use crate::error::ApiError;
use crate::hub::AppState;

pub fn router(hub: AppState) -> Router<AppState> {
    Router::new()
        .route("/event", put(update_event))
        .route("/racers", get(list_racers).post(create_racer))
        .route("/racers/{id}", patch(update_racer))
        .route("/racers/{id}", delete(delete_racer))
        .route("/racers/{id}/token", post(rotate_token))
        .route("/racers/{id}/actions/{action}", post(racer_action))
        .route("/audit", get(audit))
        .layer(middleware::from_fn_with_state(hub, require_admin))
}

async fn require_admin(
    State(hub): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let provided = bearer(
        req.headers()
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok()),
    )
    .ok_or(ApiError::Unauthorized)?;
    if constant_eq(&hash_token(provided), &hash_token(&hub.cfg.admin_token)) {
        Ok(next.run(req).await)
    } else {
        Err(ApiError::Unauthorized)
    }
}

fn bad(e: String) -> ApiError {
    ApiError::BadRequest(e)
}

async fn update_event(
    State(hub): State<AppState>,
    Json(patch): Json<EventPatch>,
) -> Result<Json<EventInfo>, ApiError> {
    let payload = json!({ "patch": patch });
    let event = hub.update_event(patch, Utc::now()).map_err(bad)?;
    hub.audit("event.update", None, payload);
    Ok(Json(event))
}

async fn list_racers(State(hub): State<AppState>) -> Json<Vec<Value>> {
    let now = Utc::now();
    let list = hub.read(|s| {
        s.racers
            .iter()
            .enumerate()
            .map(|(i, r)| {
                json!({
                    "racer": s.view(i, now),
                    "connected": r.ingest.is_some(),
                    "lastHeartbeatUtc": r.last_heartbeat.map(crate::engine::iso),
                })
            })
            .collect()
    });
    Json(list)
}

async fn create_racer(
    State(hub): State<AppState>,
    Json(n): Json<NewRacer>,
) -> Result<(axum::http::StatusCode, Json<Value>), ApiError> {
    let token = random_token();
    let id = n.id.clone();
    let racer: Racer = hub.add_racer(n, &token, Utc::now()).map_err(bad)?;
    hub.audit("racer.create", Some(&id), json!({}));
    // The plain token is returned exactly once; only its hash is stored.
    Ok((
        axum::http::StatusCode::CREATED,
        Json(json!({ "racer": racer, "token": token })),
    ))
}

async fn update_racer(
    State(hub): State<AppState>,
    Path(id): Path<String>,
    Json(p): Json<RacerPatch>,
) -> Result<Json<Racer>, ApiError> {
    let racer = hub.update_racer(&id, p, Utc::now()).map_err(bad)?;
    hub.audit("racer.update", Some(&id), json!({}));
    Ok(Json(racer))
}

async fn delete_racer(
    State(hub): State<AppState>,
    Path(id): Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    hub.delete_racer(&id, Utc::now()).map_err(bad)?;
    hub.audit("racer.delete", Some(&id), json!({}));
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn rotate_token(
    State(hub): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let token = random_token();
    hub.rotate_token(&id, &token, Utc::now()).map_err(bad)?;
    hub.audit("racer.token.rotate", Some(&id), json!({}));
    Ok(Json(json!({ "token": token })))
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ActionBody {
    delta_seconds: Option<i64>,
    final_time_seconds: Option<i64>,
    reason: Option<String>,
}

async fn racer_action(
    State(hub): State<AppState>,
    Path((id, action)): Path<(String, String)>,
    body: Option<Json<ActionBody>>,
) -> Result<Json<Racer>, ApiError> {
    let body = body.map(|b| b.0).unwrap_or_default();
    let act = match action.as_str() {
        "pause" => AdminAction::Pause,
        "resume" => AdminAction::Resume,
        "force-close" => AdminAction::ForceClose,
        "reset-day" => AdminAction::ResetDay,
        "adjust-time" => AdminAction::AdjustTime {
            delta_seconds: body
                .delta_seconds
                .ok_or_else(|| ApiError::BadRequest("deltaSeconds is required".into()))?,
        },
        "finish" => AdminAction::Finish {
            final_time_seconds: body.final_time_seconds,
        },
        _ => return Err(ApiError::NotFound),
    };
    // Time adjustments and manual finishes need a written reason: they affect the ranking.
    if matches!(
        act,
        AdminAction::AdjustTime { .. } | AdminAction::Finish { .. }
    ) && body.reason.as_deref().is_none_or(|r| r.trim().is_empty())
    {
        return Err(ApiError::BadRequest(
            "reason is required for this action".into(),
        ));
    }
    let now = Utc::now();
    hub.admin_action(&id, act, now).map_err(bad)?;
    hub.audit(
        &format!("racer.{action}"),
        Some(&id),
        json!({ "deltaSeconds": body.delta_seconds, "finalTimeSeconds": body.final_time_seconds, "reason": body.reason }),
    );
    hub.racer(&id, now).map(Json).ok_or(ApiError::NotFound)
}

#[derive(Deserialize)]
struct AuditQuery {
    limit: Option<i64>,
}

async fn audit(
    State(hub): State<AppState>,
    Query(q): Query<AuditQuery>,
) -> Result<Json<Vec<Value>>, ApiError> {
    let rows = db::audit_tail(&hub.pool, q.limit.unwrap_or(100).clamp(1, 500))
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(rows))
}
