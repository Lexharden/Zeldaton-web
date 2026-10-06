//! Organizer API (`/api/admin/*`). Two ways in:
//! * a browser session (cookie from `POST /auth/login`), with the user's role and a CSRF check;
//! * `Authorization: Bearer <ADMIN_TOKEN>`, the emergency/scripting credential (admin role).
//!
//! Every state-changing call is recorded in `audit_log` with who did it: the official session log.

use axum::extract::{DefaultBodyLimit, Extension, Path, Query, Request, State};
use axum::http::header::{AUTHORIZATION, COOKIE, HOST, SET_COOKIE, USER_AGENT};
use axum::http::{HeaderMap, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, patch, post, put};
use axum::{Json, Router};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::accounts::{self, AccountError, Role};
use crate::auth::{
    SESSION_COOKIE, bearer, clear_session_cookie, constant_eq, cookie_value, dummy_hash,
    hash_token, random_token, session_cookie, verify_password,
};
use crate::catalog::{CatalogItem, CatalogObjective};
use crate::db;
use crate::domain::{EventInfo, EventStatus, Racer, RacerStatus};
use crate::engine::{AdminAction, EventPatch, NewRacer, RacerPatch, iso};
use crate::error::ApiError;
use crate::hub::AppState;

/// Who is making the request. Built by [`authenticate`] and read by the handlers.
#[derive(Clone, Debug)]
pub struct Principal {
    /// Recorded in the audit log: a username, or `admin-token`.
    pub actor: String,
    pub role: Role,
    pub user_id: Option<i64>,
    /// The plain session token (only for cookie sessions), so logout can end it.
    pub session_token: Option<String>,
    pub csrf: Option<String>,
}

impl Principal {
    pub fn require(&self, min: Role) -> Result<(), ApiError> {
        if self.role >= min {
            Ok(())
        } else {
            Err(ApiError::Forbidden("your role cannot do this".into()))
        }
    }
}

pub fn router(hub: AppState) -> Router<AppState> {
    let protected = Router::new()
        .route("/me", get(me))
        .route("/me/password", post(change_own_password))
        .route("/auth/logout", post(logout))
        .route("/overview", get(overview))
        .route("/event", put(update_event))
        .route("/event/reset", post(reset_event))
        .route("/event/{action}", post(event_action))
        .route("/racers", get(list_racers).post(create_racer))
        .route("/racers/{id}", patch(update_racer))
        .route("/racers/{id}", delete(delete_racer))
        .route("/racers/{id}/token", post(rotate_token))
        .route("/racers/{id}/actions/{action}", post(racer_action))
        .route("/users", get(list_users).post(create_user))
        .route("/users/{id}", patch(update_user).delete(delete_user))
        .route("/users/{id}/password", post(reset_password))
        .route("/catalog", get(catalog_all))
        .route("/catalog/items/{id}", put(put_item).delete(delete_item))
        .route(
            "/catalog/objectives/{id}",
            put(put_objective).delete(delete_objective),
        )
        .route(
            "/racers/{id}/photo",
            post(crate::media::upload_photo)
                .layer(DefaultBodyLimit::max(crate::media::MAX_BYTES + 4096))
                .delete(crate::media::delete_photo),
        )
        .route("/audit", get(audit))
        .route("/donations", get(donations))
        .route("/donors/visibility", put(set_donors_visibility))
        .route("/donors/hidden", put(set_donor_hidden))
        .layer(middleware::from_fn_with_state(hub, authenticate));
    Router::new()
        .route("/auth/login", post(login))
        .merge(protected)
}

// ---- authentication -----------------------------------------------------------------------------

fn header(headers: &HeaderMap, name: impl axum::http::header::AsHeaderName) -> Option<&str> {
    headers.get(name).and_then(|v| v.to_str().ok())
}

fn is_local_host(host: &str) -> bool {
    let h = host.split(':').next().unwrap_or(host);
    h == "localhost" || h == "127.0.0.1" || h == "[::1]" || host.starts_with("[::1]")
}

/// Whether the session cookie gets the `Secure` flag: forced by COOKIE_SECURE, else on for HTTPS
/// (also behind a proxy) and for any host except plain-http localhost.
fn cookie_secure(hub: &AppState, headers: &HeaderMap) -> bool {
    if let Some(forced) = hub.cfg.cookie_secure {
        return forced;
    }
    if header(headers, "x-forwarded-proto") == Some("https") {
        return true;
    }
    !header(headers, HOST).is_some_and(is_local_host)
}

fn client_ip(headers: &HeaderMap) -> String {
    header(headers, "x-forwarded-for")
        .and_then(|v| v.split(',').next())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "unknown".into())
}

fn account_error(e: AccountError) -> ApiError {
    match e {
        AccountError::Invalid(m) => ApiError::BadRequest(m),
        AccountError::Taken => ApiError::Conflict("that username is already taken".into()),
        AccountError::NotFound => ApiError::NotFound,
        AccountError::LastAdmin => {
            ApiError::Conflict("there must be at least one active admin".into())
        }
        AccountError::Db(m) => ApiError::Internal(m),
    }
}

async fn authenticate(
    State(hub): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let principal = if let Some(token) = bearer(header(req.headers(), AUTHORIZATION)) {
        // A Bearer header must be the emergency token; a wrong one never falls back to the cookie.
        if constant_eq(&hash_token(token), &hash_token(&hub.cfg.admin_token)) {
            Principal {
                actor: "admin-token".into(),
                role: Role::Admin,
                user_id: None,
                session_token: None,
                csrf: None,
            }
        } else {
            return Err(ApiError::Unauthorized);
        }
    } else {
        let token = cookie_value(header(req.headers(), COOKIE), SESSION_COOKIE)
            .ok_or(ApiError::Unauthorized)?
            .to_string();
        let (user, session) = accounts::session_user(&hub.pool, &token, Utc::now())
            .await
            .map_err(account_error)?
            .ok_or(ApiError::Unauthorized)?;
        // The cookie rides along on any request the browser makes, so every write must also prove
        // it comes from our own page with the token only that page received at login.
        let safe = matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS);
        if !safe {
            let sent = header(req.headers(), "x-csrf-token").unwrap_or("");
            if !constant_eq(&hash_token(sent), &hash_token(&session.csrf)) {
                return Err(ApiError::Forbidden("missing or invalid CSRF token".into()));
            }
        }
        Principal {
            actor: user.username.clone(),
            role: user.role,
            user_id: Some(user.id),
            session_token: Some(token),
            csrf: Some(session.csrf),
        }
    };
    req.extensions_mut().insert(principal);
    Ok(next.run(req).await)
}

#[derive(Deserialize)]
struct LoginBody {
    username: String,
    password: String,
}

async fn login(
    State(hub): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<LoginBody>,
) -> Result<Response, ApiError> {
    let ip = client_ip(&headers);
    let user_key = format!("u:{}", body.username.trim().to_lowercase());
    let ip_key = format!("i:{ip}");
    hub.user_limiter
        .check(&user_key)
        .and_then(|_| hub.ip_limiter.check(&ip_key))
        .map_err(ApiError::TooManyRequests)?;

    let found = accounts::find_for_login(&hub.pool, &body.username)
        .await
        .map_err(account_error)?;
    // Unknown user or wrong password cost the same and answer the same.
    let (hash, user) = match found {
        Some((u, h)) => (h, Some(u)),
        None => (dummy_hash().to_string(), None),
    };
    let password = body.password;
    let ok = tokio::task::spawn_blocking(move || verify_password(&hash, &password))
        .await
        .map_err(ApiError::internal)?;
    let user = match (ok, user) {
        (true, Some(u)) if !u.disabled => u,
        _ => {
            hub.user_limiter.fail(&user_key);
            hub.ip_limiter.fail(&ip_key);
            hub.audit_as(
                "anonymous",
                "auth.login.failed",
                None,
                json!({ "username": body.username.trim(), "ip": ip }),
            );
            return Err(ApiError::Unauthorized);
        }
    };
    hub.user_limiter.success(&user_key);
    accounts::touch_login(&hub.pool, user.id)
        .await
        .map_err(account_error)?;
    let agent = header(&headers, USER_AGENT);
    let (token, session) =
        accounts::create_session(&hub.pool, user.id, Some(&ip), agent, Utc::now())
            .await
            .map_err(account_error)?;
    hub.audit_as(&user.username, "auth.login", None, json!({ "ip": ip }));

    let cookie = session_cookie(
        &token,
        cookie_secure(&hub, &headers),
        accounts::SESSION_IDLE_HOURS * 3600,
    );
    Ok((
        StatusCode::OK,
        [(SET_COOKIE, cookie)],
        Json(json!({
            "user": user,
            "csrfToken": session.csrf,
            "expiresAtUtc": iso(session.expires_at),
        })),
    )
        .into_response())
}

async fn logout(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    if let Some(token) = &p.session_token {
        accounts::delete_session(&hub.pool, token)
            .await
            .map_err(account_error)?;
    }
    hub.audit_as(&p.actor, "auth.logout", None, json!({}));
    Ok((
        StatusCode::OK,
        [(
            SET_COOKIE,
            clear_session_cookie(cookie_secure(&hub, &headers)),
        )],
        Json(json!({ "ok": true })),
    )
        .into_response())
}

async fn me(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
) -> Result<Json<Value>, ApiError> {
    let user = match p.user_id {
        Some(id) => accounts::find_by_id(&hub.pool, id)
            .await
            .map_err(account_error)?
            .map(|u| serde_json::to_value(u).unwrap_or_default()),
        None => None,
    };
    Ok(Json(json!({
        "user": user.unwrap_or_else(|| json!({ "id": null, "username": p.actor, "role": p.role })),
        "via": if p.session_token.is_some() { "session" } else { "token" },
        "csrfToken": p.csrf,
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OwnPasswordBody {
    current_password: String,
    password: String,
}

async fn change_own_password(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Json(b): Json<OwnPasswordBody>,
) -> Result<Json<Value>, ApiError> {
    let id = p
        .user_id
        .ok_or_else(|| ApiError::BadRequest("the emergency token has no password".into()))?;
    let hash = accounts::password_hash_of(&hub.pool, id)
        .await
        .map_err(account_error)?
        .ok_or(ApiError::NotFound)?;
    let current = b.current_password;
    let ok = tokio::task::spawn_blocking(move || verify_password(&hash, &current))
        .await
        .map_err(ApiError::internal)?;
    if !ok {
        return Err(ApiError::Forbidden("the current password is wrong".into()));
    }
    accounts::set_password(&hub.pool, id, &b.password, p.session_token.as_deref())
        .await
        .map_err(account_error)?;
    hub.audit_as(&p.actor, "user.password.change", None, json!({}));
    Ok(Json(json!({ "ok": true })))
}

// ---- users (admin) ------------------------------------------------------------------------------

async fn list_users(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
) -> Result<Json<Vec<accounts::User>>, ApiError> {
    p.require(Role::Admin)?;
    Ok(Json(
        accounts::list_users(&hub.pool)
            .await
            .map_err(account_error)?,
    ))
}

#[derive(Deserialize)]
struct NewUserBody {
    username: String,
    password: String,
    role: Role,
}

async fn create_user(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Json(b): Json<NewUserBody>,
) -> Result<(StatusCode, Json<accounts::User>), ApiError> {
    p.require(Role::Admin)?;
    let user = accounts::create_user(&hub.pool, &b.username, &b.password, b.role)
        .await
        .map_err(account_error)?;
    hub.audit_as(
        &p.actor,
        "user.create",
        None,
        json!({ "username": user.username, "role": user.role }),
    );
    Ok((StatusCode::CREATED, Json(user)))
}

#[derive(Deserialize)]
struct UserPatchBody {
    role: Option<Role>,
    disabled: Option<bool>,
}

async fn update_user(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<i64>,
    Json(b): Json<UserPatchBody>,
) -> Result<Json<accounts::User>, ApiError> {
    p.require(Role::Admin)?;
    // Nobody locks themselves out by accident.
    if p.user_id == Some(id) && (b.disabled == Some(true) || b.role == Some(Role::Moderator)) {
        return Err(ApiError::Conflict(
            "you cannot disable or demote your own account".into(),
        ));
    }
    let user = accounts::update_user(&hub.pool, id, b.role, b.disabled)
        .await
        .map_err(account_error)?;
    hub.audit_as(
        &p.actor,
        "user.update",
        None,
        json!({ "username": user.username, "role": user.role, "disabled": user.disabled }),
    );
    Ok(Json(user))
}

async fn delete_user(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    p.require(Role::Admin)?;
    if p.user_id == Some(id) {
        return Err(ApiError::Conflict(
            "you cannot delete your own account".into(),
        ));
    }
    let name = accounts::find_by_id(&hub.pool, id)
        .await
        .map_err(account_error)?
        .map(|u| u.username);
    accounts::delete_user(&hub.pool, id)
        .await
        .map_err(account_error)?;
    hub.audit_as(&p.actor, "user.delete", None, json!({ "username": name }));
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct PasswordBody {
    password: String,
}

async fn reset_password(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<i64>,
    Json(b): Json<PasswordBody>,
) -> Result<Json<Value>, ApiError> {
    p.require(Role::Admin)?;
    // Resetting your own password from here keeps the current session, like changing it yourself.
    let keep = if p.user_id == Some(id) {
        p.session_token.as_deref()
    } else {
        None
    };
    accounts::set_password(&hub.pool, id, &b.password, keep)
        .await
        .map_err(account_error)?;
    hub.audit_as(
        &p.actor,
        "user.password.reset",
        None,
        json!({ "userId": id }),
    );
    Ok(Json(json!({ "ok": true })))
}

// ---- event --------------------------------------------------------------------------------------

/// "The event is live..." refusals are a state conflict (409); anything else is a bad request.
fn state_conflict(e: String) -> ApiError {
    if e.starts_with("the event is live") {
        ApiError::Conflict(e)
    } else {
        ApiError::BadRequest(e)
    }
}

fn bad(e: String) -> ApiError {
    ApiError::BadRequest(e)
}

async fn update_event(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Json(patch): Json<EventPatch>,
) -> Result<Json<EventInfo>, ApiError> {
    p.require(Role::Admin)?;
    let payload = json!({ "patch": patch });
    let event = hub
        .update_event(patch, Utc::now())
        .map_err(state_conflict)?;
    hub.audit_as(&p.actor, "event.update", None, payload);
    Ok(Json(event))
}

/// The word the organizer must type to reset the event.
pub const RESET_CONFIRMATION: &str = "REINICIAR";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResetBody {
    confirm: String,
    /// New start (RFC 3339). Required when the current one is already past.
    start_at_utc: Option<String>,
    /// End rehearsal mode too (default: yes, the usual next step is the real event).
    #[serde(default = "yes")]
    leave_rehearsal: bool,
}

fn yes() -> bool {
    true
}

/// Wipes the test run: progress, items, clocks, donations and activity. Racers, their HiveShock
/// tokens, the catalog, pictures and accounts stay; the audit log keeps this very action.
async fn reset_event(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Json(body): Json<ResetBody>,
) -> Result<Json<EventInfo>, ApiError> {
    p.require(Role::Admin)?;
    if body.confirm != RESET_CONFIRMATION {
        return Err(ApiError::BadRequest(format!(
            "type {RESET_CONFIRMATION} to confirm"
        )));
    }
    let was_rehearsal = hub.read(|s| s.event.rehearsal);
    let event = hub
        .reset_event(body.start_at_utc.clone(), body.leave_rehearsal, Utc::now())
        .map_err(state_conflict)?;
    hub.audit_as(
        &p.actor,
        "event.reset",
        None,
        json!({
            "startAtUtc": event.start_at_utc,
            "wasRehearsal": was_rehearsal,
            "leftRehearsal": body.leave_rehearsal,
        }),
    );
    Ok(Json(event))
}

#[derive(Deserialize, Default)]
struct EventActionBody {
    reason: Option<String>,
}

/// One-click event controls: `start`, `pause`, `resume`, `finish`.
async fn event_action(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(action): Path<String>,
    body: Option<Json<EventActionBody>>,
) -> Result<Json<EventInfo>, ApiError> {
    p.require(Role::Admin)?;
    let now = Utc::now();
    let stored = hub.read(|s| s.event.status);
    let start_in_future = hub.read(|s| {
        chrono::DateTime::parse_from_rfc3339(&s.event.start_at_utc)
            .map(|d| d.with_timezone(&Utc) > now)
            .unwrap_or(false)
    });
    let mut patch = EventPatch::default();
    match (action.as_str(), stored) {
        (_, EventStatus::Finished) => {
            return Err(ApiError::Conflict("the event is already finished".into()));
        }
        ("start", EventStatus::Upcoming | EventStatus::Paused) => {
            patch.status = Some(EventStatus::Live);
            // Starting early: the schedule shown on the site follows.
            if start_in_future {
                patch.start_at_utc = Some(iso(now));
            }
        }
        ("start", _) => return Err(ApiError::Conflict("the event is already live".into())),
        ("pause", EventStatus::Live) => patch.status = Some(EventStatus::Paused),
        ("pause", _) => return Err(ApiError::Conflict("the event is not live".into())),
        ("resume", EventStatus::Paused) => patch.status = Some(EventStatus::Live),
        ("resume", _) => return Err(ApiError::Conflict("the event is not paused".into())),
        ("finish", _) => patch.status = Some(EventStatus::Finished),
        _ => return Err(ApiError::NotFound),
    }
    let event = hub.update_event(patch, now).map_err(bad)?;
    let reason = body.and_then(|b| b.0.reason);
    hub.audit_as(
        &p.actor,
        &format!("event.{action}"),
        None,
        json!({ "reason": reason }),
    );
    Ok(Json(event))
}

// ---- dashboard ----------------------------------------------------------------------------------

fn alert(level: &str, code: &str, racer: Option<&Racer>, message: String) -> Value {
    json!({
        "level": level,
        "code": code,
        "racerId": racer.map(|r| r.id.clone()),
        "racerName": racer.map(|r| r.display_name.clone()),
        "message": message,
    })
}

/// Everything the dashboard needs in one call.
async fn overview(State(hub): State<AppState>) -> Json<Value> {
    let now = Utc::now();
    let event = hub.event(now);
    let rows: Vec<(Racer, bool, Option<String>, Option<i64>)> = hub.read(|s| {
        s.racers
            .iter()
            .enumerate()
            .map(|(i, r)| {
                (
                    s.view(i, now),
                    r.ingest.is_some(),
                    r.last_heartbeat.map(iso),
                    r.last_heartbeat.map(|h| (now - h).num_seconds().max(0)),
                )
            })
            .collect()
    });

    let live_event = event.status == EventStatus::Live;
    let mut alerts = Vec::new();
    if live_event && !rows.iter().any(|r| r.1) {
        alerts.push(alert(
            "warn",
            "nobody_connected",
            None,
            "The event is live but no HiveShock is connected.".into(),
        ));
    }
    for (racer, connected, _, age) in &rows {
        match racer.status {
            RacerStatus::Exhausted => alerts.push(alert(
                "warn",
                "exhausted",
                Some(racer),
                format!("{} ran out of time for today.", racer.display_name),
            )),
            RacerStatus::Live if racer.remaining_seconds <= 600 => alerts.push(alert(
                "warn",
                "low_time",
                Some(racer),
                format!(
                    "{} has {} min left today.",
                    racer.display_name,
                    (racer.remaining_seconds + 59) / 60
                ),
            )),
            RacerStatus::Offline if live_event => alerts.push(alert(
                "info",
                "offline",
                Some(racer),
                format!("{} is offline.", racer.display_name),
            )),
            _ => {}
        }
        if *connected && age.is_some_and(|a| a > 12) {
            alerts.push(alert(
                "warn",
                "weak_signal",
                Some(racer),
                format!(
                    "{} has not sent a heartbeat for {}s.",
                    racer.display_name,
                    age.unwrap_or(0)
                ),
            ));
        }
    }

    let count = |status: RacerStatus| rows.iter().filter(|r| r.0.status == status).count();
    let racers: Vec<Value> = rows
        .iter()
        .map(|(racer, connected, hb, age)| {
            json!({
                "racer": racer,
                "connected": connected,
                "lastHeartbeatUtc": hb,
                "heartbeatAgeSeconds": age,
            })
        })
        .collect();
    let winner = hub.read(|s| s.winner.clone()).map(|w| {
        json!({
            "racerId": w.racer_id,
            "finalTimeSeconds": w.final_time_seconds,
            "finishedAtUtc": w.finished_at_utc,
        })
    });
    Json(json!({
        "serverTimeUtc": iso(now),
        "event": event,
        "summary": {
            "racers": rows.len(),
            "connected": rows.iter().filter(|r| r.1).count(),
            "live": count(RacerStatus::Live),
            "paused": count(RacerStatus::Paused),
            "online": count(RacerStatus::Online),
            "offline": count(RacerStatus::Offline),
            "exhausted": count(RacerStatus::Exhausted),
            "finished": count(RacerStatus::Finished),
        },
        "racers": racers,
        "alerts": alerts,
        "hiveshock": hub.stats(),
        "activity": hub.activity().into_iter().take(15).collect::<Vec<_>>(),
        "winner": winner,
        "catalogVersion": hub.catalog_public().version(),
    }))
}

// ---- racers -------------------------------------------------------------------------------------

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
                    "lastHeartbeatUtc": r.last_heartbeat.map(iso),
                })
            })
            .collect()
    });
    Json(list)
}

async fn create_racer(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Json(n): Json<NewRacer>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    p.require(Role::Admin)?;
    let token = random_token();
    let id = n.id.clone();
    let racer: Racer = hub.add_racer(n, &token, Utc::now()).map_err(bad)?;
    hub.audit_as(&p.actor, "racer.create", Some(&id), json!({}));
    // The plain token is returned exactly once; only its hash is stored.
    Ok((
        StatusCode::CREATED,
        Json(json!({ "racer": racer, "token": token })),
    ))
}

async fn update_racer(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<String>,
    Json(patch): Json<RacerPatch>,
) -> Result<Json<Racer>, ApiError> {
    p.require(Role::Admin)?;
    let racer = hub.update_racer(&id, patch, Utc::now()).map_err(bad)?;
    hub.audit_as(&p.actor, "racer.update", Some(&id), json!({}));
    Ok(Json(racer))
}

async fn delete_racer(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    p.require(Role::Admin)?;
    hub.delete_racer(&id, Utc::now()).map_err(bad)?;
    hub.audit_as(&p.actor, "racer.delete", Some(&id), json!({}));
    Ok(StatusCode::NO_CONTENT)
}

async fn rotate_token(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    p.require(Role::Admin)?;
    let token = random_token();
    hub.rotate_token(&id, &token, Utc::now()).map_err(bad)?;
    hub.audit_as(&p.actor, "racer.token.rotate", Some(&id), json!({}));
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
    Extension(p): Extension<Principal>,
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
    // Moderators run the race day; declaring a winner by hand is an admin decision.
    p.require(if matches!(act, AdminAction::Finish { .. }) {
        Role::Admin
    } else {
        Role::Moderator
    })?;
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
    hub.audit_as(
        &p.actor,
        &format!("racer.{action}"),
        Some(&id),
        json!({ "deltaSeconds": body.delta_seconds, "finalTimeSeconds": body.final_time_seconds, "reason": body.reason }),
    );
    hub.racer(&id, now).map(Json).ok_or(ApiError::NotFound)
}

// ---- catalog (admin) ----------------------------------------------------------------------------

/// The whole catalog, including disabled entries.
async fn catalog_all(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
) -> Result<Json<Value>, ApiError> {
    p.require(Role::Admin)?;
    let c = hub.catalog_all();
    Ok(Json(
        json!({ "version": c.version(), "items": c.items, "objectives": c.objectives }),
    ))
}

async fn put_item(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<String>,
    Json(item): Json<CatalogItem>,
) -> Result<Json<CatalogItem>, ApiError> {
    p.require(Role::Admin)?;
    if item.id != id {
        return Err(ApiError::BadRequest(
            "the id in the body must match the URL".into(),
        ));
    }
    hub.catalog_upsert_item(item.clone(), Utc::now())
        .map_err(bad)?;
    hub.audit_as(
        &p.actor,
        "catalog.item.save",
        None,
        json!({ "id": id, "enabled": item.enabled }),
    );
    Ok(Json(item))
}

async fn delete_item(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    p.require(Role::Admin)?;
    hub.catalog_delete_item(&id, Utc::now()).map_err(bad)?;
    hub.audit_as(&p.actor, "catalog.item.delete", None, json!({ "id": id }));
    Ok(StatusCode::NO_CONTENT)
}

async fn put_objective(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<String>,
    Json(objective): Json<CatalogObjective>,
) -> Result<Json<CatalogObjective>, ApiError> {
    p.require(Role::Admin)?;
    if objective.id != id {
        return Err(ApiError::BadRequest(
            "the id in the body must match the URL".into(),
        ));
    }
    hub.catalog_upsert_objective(objective.clone(), Utc::now())
        .map_err(bad)?;
    hub.audit_as(
        &p.actor,
        "catalog.objective.save",
        None,
        json!({ "id": id, "enabled": objective.enabled }),
    );
    Ok(Json(objective))
}

async fn delete_objective(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    p.require(Role::Admin)?;
    hub.catalog_delete_objective(&id, Utc::now()).map_err(bad)?;
    hub.audit_as(
        &p.actor,
        "catalog.objective.delete",
        None,
        json!({ "id": id }),
    );
    Ok(StatusCode::NO_CONTENT)
}

// ---- audit --------------------------------------------------------------------------------------

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

// ---- time donations -------------------------------------------------------------------------------

#[derive(Deserialize)]
struct DonationsQuery {
    racer: Option<String>,
    limit: Option<i64>,
}

/// Time donations reported by HiveShock: the ledger (newest first), totals per racer for the whole
/// event and for today (what counts against the daily limits), and the current policy.
async fn donations(
    State(hub): State<AppState>,
    Query(q): Query<DonationsQuery>,
) -> Result<Json<Value>, ApiError> {
    let racer = q.racer.as_deref().filter(|r| !r.is_empty());
    let recent = db::donations_tail(&hub.pool, racer, q.limit.unwrap_or(200).clamp(1, 1000))
        .await
        .map_err(ApiError::internal)?;
    let totals = db::donation_totals(&hub.pool)
        .await
        .map_err(ApiError::internal)?;
    let donors = db::top_donors(&hub.pool, 20, true)
        .await
        .map_err(ApiError::internal)?;
    let donors_public = donors_public(&hub.pool).await;
    let (policy, today) = hub.read(|s| {
        let today: Vec<Value> = s
            .racers
            .iter()
            .map(|r| {
                json!({
                    "racerId": r.racer.id,
                    "addedSeconds": r.donation_added_ms / 1000,
                    "removedSeconds": r.donation_removed_ms / 1000,
                })
            })
            .collect();
        (s.event.donation_time.clone(), today)
    });
    Ok(Json(
        json!({ "policy": policy, "today": today, "totals": totals, "donors": donors,
                "donorsPublic": donors_public, "recent": recent }),
    ))
}

/// Whether the public site shows the donors board (on unless the organizer turned it off).
pub async fn donors_public(pool: &sqlx::SqlitePool) -> bool {
    !matches!(
        db::setting(pool, "donors_public").await,
        Ok(Some(v)) if v == "false"
    )
}

#[derive(Deserialize)]
struct VisibilityBody {
    enabled: bool,
}

/// Turns the public donors board on or off.
async fn set_donors_visibility(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Json(body): Json<VisibilityBody>,
) -> Result<Json<Value>, ApiError> {
    p.require(Role::Admin)?;
    db::set_setting(
        &hub.pool,
        "donors_public",
        if body.enabled { "true" } else { "false" },
    )
    .await
    .map_err(ApiError::internal)?;
    hub.audit_as(
        &p.actor,
        "donors.visibility",
        None,
        json!({ "enabled": body.enabled }),
    );
    Ok(Json(json!({ "donorsPublic": body.enabled })))
}

#[derive(Deserialize)]
struct HiddenBody {
    platform: String,
    viewer: String,
    hidden: bool,
}

/// Hides or shows one donor on the public board (they keep counting in the ledger and the panel).
async fn set_donor_hidden(
    State(hub): State<AppState>,
    Extension(p): Extension<Principal>,
    Json(body): Json<HiddenBody>,
) -> Result<Json<Value>, ApiError> {
    p.require(Role::Admin)?;
    if !matches!(body.platform.as_str(), "tiktok" | "twitch" | "youtube")
        || body.viewer.trim().is_empty()
    {
        return Err(ApiError::BadRequest(
            "platform (tiktok, twitch, youtube) and viewer are required".into(),
        ));
    }
    db::set_donor_hidden(&hub.pool, &body.platform, &body.viewer, body.hidden)
        .await
        .map_err(ApiError::internal)?;
    hub.audit_as(
        &p.actor,
        "donors.hide",
        None,
        json!({ "platform": body.platform, "viewer": body.viewer, "hidden": body.hidden }),
    );
    Ok(Json(json!({ "ok": true })))
}
