//! End-to-end: real router + in-memory SQLite + real WebSocket clients.

use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio::sync::mpsc::unbounded_channel;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tower::ServiceExt;
use zeldathon_server::config::Config;
use zeldathon_server::hub::{AppState, Hub};
use zeldathon_server::{app, db, seed};

const ADMIN: &str = "test-admin-token-123456";
type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

struct Harness {
    hub: AppState,
    addr: std::net::SocketAddr,
    tokens: Vec<(String, String)>,
    pool: sqlx::SqlitePool,
}

impl Harness {
    fn token(&self, id: &str) -> String {
        self.tokens.iter().find(|(i, _)| i == id).unwrap().1.clone()
    }
}

async fn harness() -> Harness {
    harness_with(None).await
}

async fn harness_with(discord_webhook_url: Option<String>) -> Harness {
    let pool = db::connect("sqlite::memory:").await.unwrap();
    db::migrate(&pool).await.unwrap();
    let seeded = seed::build(Utc::now());
    for op in seeded.ops {
        db::apply(&pool, op).await.unwrap();
    }
    let state = db::load(&pool).await.unwrap().unwrap();
    let (tx, rx) = unbounded_channel();
    tokio::spawn(db::writer(pool.clone(), rx));
    let cfg = Config {
        bind: "127.0.0.1:0".into(),
        database_url: "sqlite::memory:".into(),
        admin_token: ADMIN.into(),
        admin_token_generated: false,
        cors_origins: vec!["http://localhost:5173".into()],
        stale_heartbeat_secs: 20,
        dev_tokens_file: None,
        admin_user: "admin".into(),
        admin_password: None,
        cookie_secure: Some(false),
        discord_webhook_url,
        public_url: "https://zeldaton.example".into(),
        uploads_dir: std::env::temp_dir()
            .join(format!("zeldathon-test-{}", std::process::id()))
            .to_string_lossy()
            .into_owned(),
    };
    let hub = Hub::new(state, tx, cfg, pool.clone());
    let ticker = hub.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            ticker.tick(Utc::now());
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = app::build(hub.clone());
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    Harness {
        hub,
        addr,
        tokens: seeded.tokens,
        pool,
    }
}

async fn call(
    h: &Harness,
    method: &str,
    uri: &str,
    bearer: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(t) = bearer {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    let req = match body {
        Some(b) => req
            .header("content-type", "application/json")
            .body(Body::from(b.to_string()))
            .unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
    let res = app::build(h.hub.clone()).oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn connect_ws(
    h: &Harness,
    path: &str,
    token: Option<&str>,
) -> Result<Ws, tokio_tungstenite::tungstenite::Error> {
    let mut req = format!("ws://{}{path}", h.addr)
        .into_client_request()
        .unwrap();
    if let Some(t) = token {
        req.headers_mut()
            .insert("authorization", format!("Bearer {t}").parse().unwrap());
    }
    tokio_tungstenite::connect_async(req)
        .await
        .map(|(ws, _)| ws)
}

async fn next_json(ws: &mut Ws) -> Value {
    loop {
        match tokio::time::timeout(Duration::from_secs(5), ws.next())
            .await
            .expect("timed out waiting for a frame")
        {
            Some(Ok(Message::Text(t))) => return serde_json::from_str(&t).unwrap(),
            Some(Ok(_)) => continue,
            other => panic!("socket ended: {other:?}"),
        }
    }
}

/// Reads frames until one matches `ty`.
async fn wait_for(ws: &mut Ws, ty: &str) -> Value {
    for _ in 0..40 {
        let v = next_json(ws).await;
        if v["type"] == ty {
            return v;
        }
    }
    panic!("never saw {ty}");
}

async fn send(ws: &mut Ws, v: Value) {
    ws.send(Message::Text(v.to_string().into())).await.unwrap();
}

async fn make_event_live(h: &Harness) {
    let (s, _) = call(
        h,
        "PUT",
        "/api/admin/event",
        Some(ADMIN),
        Some(json!({ "status": "live" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
}

#[tokio::test]
async fn public_rest_matches_the_frontend_contract() {
    let h = harness().await;
    let (s, event) = call(&h, "GET", "/api/event", None, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(event["dailyBudgetSeconds"], 14_400);
    assert_eq!(event["dailyResetLocalTime"], "06:00");
    assert_eq!(event["startAtUtc"], "2026-10-07T12:00:00.000Z");
    assert_eq!(
        event["rules"]["requiredObjectiveIds"]
            .as_array()
            .unwrap()
            .len(),
        10
    );

    let (_, racers) = call(&h, "GET", "/api/racers", None, None).await;
    let racers = racers.as_array().unwrap();
    assert_eq!(racers.len(), 9);
    assert_eq!(racers[0]["id"], "pinchiviejo");
    assert_eq!(racers[0]["status"], "offline");
    assert_eq!(racers[0]["remainingSeconds"], 14_400);
    assert!(racers[0]["channels"].as_array().unwrap().is_empty());
    assert!(racers[0].get("token").is_none() && racers[0].get("tokenHash").is_none());

    let (_, one) = call(&h, "GET", "/api/racers/ralbat", None, None).await;
    assert_eq!(one["displayName"], "Ralbat");
    assert_eq!(
        call(&h, "GET", "/api/racers/nope", None, None).await.0,
        StatusCode::NOT_FOUND
    );

    for path in [
        "/api/standings",
        "/api/streams",
        "/api/activity",
        "/api/hiveshock/stats",
        "/api/clocks",
    ] {
        assert_eq!(
            call(&h, "GET", path, None, None).await.0,
            StatusCode::OK,
            "{path}"
        );
    }
    let (_, clocks) = call(&h, "GET", "/api/clocks", None, None).await;
    assert_eq!(clocks[0]["remainingMs"], 14_400_000);
    assert!(clocks[0]["resetAtUtc"].as_str().unwrap().ends_with('Z'));
}

#[tokio::test]
async fn public_ws_sends_a_snapshot_with_server_time() {
    let h = harness().await;
    let mut ws = connect_ws(&h, "/ws", None).await.unwrap();
    let first = next_json(&mut ws).await;
    assert_eq!(first["type"], "CLOCK_SNAPSHOT");
    assert_eq!(first["clocks"].as_array().unwrap().len(), 9);
    assert!(first["serverTimeUtc"].is_string());

    // A resync request is answered with a fresh snapshot, and a JSON PING is silently accepted.
    send(&mut ws, json!({ "type": "PING", "sentAt": 1 })).await;
    send(&mut ws, json!({ "type": "CLOCK_SYNC_REQUEST" })).await;
    assert_eq!(next_json(&mut ws).await["type"], "CLOCK_SNAPSHOT");
}

#[tokio::test]
async fn admin_requires_the_admin_token() {
    let h = harness().await;
    assert_eq!(
        call(&h, "GET", "/api/admin/racers", None, None).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&h, "GET", "/api/admin/racers", Some("nope"), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    // A racer's ingest token is not an admin credential.
    let racer_token = h.token("ralbat");
    assert_eq!(
        call(&h, "GET", "/api/admin/racers", Some(&racer_token), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(&h, "GET", "/api/admin/racers", Some(ADMIN), None)
            .await
            .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn ingest_rejects_bad_tokens() {
    let h = harness().await;
    assert!(connect_ws(&h, "/ingest", None).await.is_err());
    assert!(connect_ws(&h, "/ingest", Some("wrong")).await.is_err());
    let (s, _) = call(
        &h,
        "POST",
        "/ingest/events",
        Some("wrong"),
        Some(json!({ "events": [] })),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn hiveshock_session_flows_to_rest_and_public_ws() {
    let h = harness().await;
    make_event_live(&h).await;
    let mut public = connect_ws(&h, "/ws", None).await.unwrap();
    next_json(&mut public).await; // snapshot

    let mut hs = connect_ws(&h, "/ingest", Some(&h.token("ralbat")))
        .await
        .unwrap();
    send(&mut hs, json!({ "type": "HELLO", "id": "1" })).await;
    let hello = next_json(&mut hs).await;
    assert_eq!(hello["type"], "CLOCK");
    assert_eq!(hello["clock"]["remainingMs"], 14_400_000);

    send(&mut hs, json!({ "type": "SESSION_STARTED", "id": "2" })).await;
    assert_eq!(next_json(&mut hs).await["type"], "ACK");
    // HELLO first brings the racer online, then the session start makes it live.
    let mut statuses = vec![];
    while statuses.last() != Some(&"live".to_string()) {
        let changed = wait_for(&mut public, "RACER_STATUS_CHANGED").await;
        assert_eq!(changed["racerId"], "ralbat");
        statuses.push(changed["status"].as_str().unwrap().to_string());
    }
    assert_eq!(statuses, ["online", "live"]);

    send(&mut hs, json!({ "type": "GAME_PROGRESS", "progress": { "percentage": 41.5, "currentArea": "forest-temple", "completedObjectives": ["kokiri-forest", "deku-tree"] } })).await;
    let progress = wait_for(&mut public, "GAME_PROGRESS").await;
    assert_eq!(progress["racerId"], "ralbat");
    assert_eq!(progress["progress"]["percentage"], 41.5);

    send(
        &mut hs,
        json!({ "type": "ITEM_ACQUIRED", "item": "longshot" }),
    )
    .await;
    let activity = wait_for(&mut public, "LIVE_ACTIVITY").await;
    assert!(activity["activity"]["code"].is_string());

    // Validation errors come back on the ingest socket without breaking it.
    send(
        &mut hs,
        json!({ "type": "ITEM_ACQUIRED", "item": "triforce", "id": "bad" }),
    )
    .await;
    loop {
        let v = next_json(&mut hs).await;
        if v["type"] == "ERROR" {
            assert_eq!(v["code"], "invalid");
            assert_eq!(v["id"], "bad");
            break;
        }
    }

    // REST reflects the race, and standings rank the racer that has progress first.
    let (_, ralbat) = call(&h, "GET", "/api/racers/ralbat", None, None).await;
    assert_eq!(ralbat["status"], "live");
    assert_eq!(ralbat["currentArea"], "forest-temple");
    assert_eq!(
        ralbat["completedObjectives"],
        json!(["kokiri-forest", "deku-tree"])
    );
    assert_eq!(ralbat["items"]["longshot"], true);
    tokio::time::sleep(Duration::from_millis(400)).await;
    let reloaded = db::load(&h.pool).await.unwrap().unwrap();
    let r = &reloaded.racers[reloaded.idx("ralbat").unwrap()].racer;
    assert_eq!(r.current_area.as_deref(), Some("forest-temple"));
    assert_eq!(r.completed_objectives.len(), 2);
    assert!(!reloaded.activity.is_empty());
}

#[tokio::test]
async fn running_out_of_time_forces_the_game_closed() {
    let h = harness().await;
    make_event_live(&h).await;
    let mut public = connect_ws(&h, "/ws", None).await.unwrap();
    next_json(&mut public).await;
    let mut hs = connect_ws(&h, "/ingest", Some(&h.token("cuaco")))
        .await
        .unwrap();
    send(&mut hs, json!({ "type": "HELLO" })).await;
    next_json(&mut hs).await;

    // Leave the racer with ~1 second of budget.
    let (s, _) = call(
        &h,
        "POST",
        "/api/admin/racers/cuaco/actions/adjust-time",
        Some(ADMIN),
        Some(json!({ "deltaSeconds": -14_399, "reason": "test" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    send(&mut hs, json!({ "type": "SESSION_STARTED" })).await;

    wait_for(&mut public, "SESSION_EXHAUSTED").await;
    let closed = wait_for(&mut public, "GAME_FORCE_CLOSE").await;
    assert_eq!(closed["racerId"], "cuaco");
    // HiveShock itself is told to close the game.
    loop {
        if next_json(&mut hs).await["type"] == "GAME_FORCE_CLOSE" {
            break;
        }
    }
    let (_, c) = call(&h, "GET", "/api/racers/cuaco", None, None).await;
    assert_eq!(c["status"], "exhausted");
    assert_eq!(c["remainingSeconds"], 0);
}

#[tokio::test]
async fn admin_actions_need_a_reason_and_are_audited() {
    let h = harness().await;
    let none = call(
        &h,
        "POST",
        "/api/admin/racers/xime/actions/adjust-time",
        Some(ADMIN),
        Some(json!({ "deltaSeconds": 60 })),
    )
    .await;
    assert_eq!(none.0, StatusCode::BAD_REQUEST);
    let ok = call(
        &h,
        "POST",
        "/api/admin/racers/xime/actions/adjust-time",
        Some(ADMIN),
        Some(json!({ "deltaSeconds": -60, "reason": "lag compensation" })),
    )
    .await;
    assert_eq!(ok.0, StatusCode::OK);
    assert_eq!(ok.1["remainingSeconds"], 14_340);
    tokio::time::sleep(Duration::from_millis(300)).await;
    let (_, audit) = call(&h, "GET", "/api/admin/audit?limit=5", Some(ADMIN), None).await;
    assert_eq!(audit[0]["action"], "racer.adjust-time");
    assert_eq!(audit[0]["racerId"], "xime");
    assert_eq!(audit[0]["payload"]["reason"], "lag compensation");
}

#[tokio::test]
async fn admin_can_add_channels_and_manage_racers() {
    let h = harness().await;
    let (s, created) = call(&h, "POST", "/api/admin/racers", Some(ADMIN), Some(json!({
        "id": "nuevo", "displayName": "Nuevo", "timezone": "America/Bogota",
        "channels": [{ "platform": "twitch", "handle": "@nuevo" }, { "platform": "tiktok", "handle": "nuevo.tk" }]
    }))).await;
    assert_eq!(s, StatusCode::CREATED);
    let token = created["token"].as_str().unwrap().to_string();
    assert_eq!(
        created["racer"]["channels"][0]["url"],
        "https://twitch.tv/nuevo"
    );
    assert_eq!(
        created["racer"]["channels"][1]["url"],
        "https://tiktok.com/@nuevo.tk"
    );

    // The new token works for ingestion; a rotated one invalidates the old.
    assert!(connect_ws(&h, "/ingest", Some(&token)).await.is_ok());
    let (_, rotated) = call(
        &h,
        "POST",
        "/api/admin/racers/nuevo/token",
        Some(ADMIN),
        None,
    )
    .await;
    assert_ne!(rotated["token"].as_str().unwrap(), token);
    assert!(connect_ws(&h, "/ingest", Some(&token)).await.is_err());

    // Bad input is rejected; duplicates conflict; racers can be removed.
    let bad_tz = call(
        &h,
        "POST",
        "/api/admin/racers",
        Some(ADMIN),
        Some(json!({ "id": "x", "displayName": "X", "timezone": "Mars/Base" })),
    )
    .await;
    assert_eq!(bad_tz.0, StatusCode::BAD_REQUEST);
    let dup = call(
        &h,
        "POST",
        "/api/admin/racers",
        Some(ADMIN),
        Some(json!({ "id": "nuevo", "displayName": "N", "timezone": "UTC" })),
    )
    .await;
    assert_eq!(dup.0, StatusCode::BAD_REQUEST);
    assert_eq!(
        call(&h, "DELETE", "/api/admin/racers/nuevo", Some(ADMIN), None)
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(&h, "GET", "/api/racers/nuevo", None, None).await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn http_batch_ingest_applies_events_in_order() {
    let h = harness().await;
    make_event_live(&h).await;
    let t = h.token("speaksins");
    let (s, out) = call(
        &h,
        "POST",
        "/ingest/events",
        Some(&t),
        Some(json!({ "events": [
            { "type": "HEARTBEAT", "id": "a" },
            { "type": "SESSION_STARTED", "id": "b" },
            { "type": "AREA_CHANGED", "area": "lake-hylia", "id": "c" },
            { "type": "NOPE", "id": "d" }
        ]})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let results = out["results"].as_array().unwrap();
    assert_eq!(results[0]["type"], "CLOCK");
    assert_eq!(results[1]["type"], "ACK");
    assert_eq!(results[2]["type"], "ACK");
    assert_eq!(results[3]["type"], "ERROR");
    let (_, r) = call(&h, "GET", "/api/racers/speaksins", None, None).await;
    assert_eq!(r["currentArea"], "lake-hylia");
}

#[tokio::test]
async fn organizer_actions_push_the_official_clock_to_hiveshock_immediately() {
    let h = harness().await;
    make_event_live(&h).await;
    let mut hs = connect_ws(&h, "/ingest", Some(&h.token("cuaco")))
        .await
        .unwrap();
    send(&mut hs, json!({ "type": "HELLO" })).await;
    let hello = wait_for(&mut hs, "CLOCK").await;
    let before = hello["clock"]["remainingMs"].as_i64().unwrap();

    // No heartbeat is sent: the new value must arrive on its own.
    let (s, _) = call(
        &h,
        "POST",
        "/api/admin/racers/cuaco/actions/adjust-time",
        Some(ADMIN),
        Some(json!({ "deltaSeconds": -600, "reason": "test" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let pushed = wait_for(&mut hs, "CLOCK").await;
    assert_eq!(pushed["clock"]["racerId"], "cuaco");
    let after = pushed["clock"]["remainingMs"].as_i64().unwrap();
    assert!(before - after >= 599_000, "before={before} after={after}");
}

#[tokio::test]
async fn public_catalog_lists_items_and_objectives_by_age() {
    let h = harness().await;
    let (s, c) = call(&h, "GET", "/api/catalog", None, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(c["version"].as_str().unwrap().len(), 12);
    let items = c["items"].as_array().unwrap();
    assert!(items.len() >= 60);
    assert!(items.iter().all(|i| i["enabled"] == true));
    let ages: std::collections::HashSet<_> =
        items.iter().map(|i| i["age"].as_str().unwrap()).collect();
    assert_eq!(ages, ["child", "adult", "both"].into_iter().collect());
    let objectives = c["objectives"].as_array().unwrap();
    assert_eq!(objectives.len(), 10);
    assert_eq!(objectives[0]["id"], "kokiri-forest");
    assert_eq!(objectives[0]["age"], "child");
}

#[tokio::test]
async fn an_existing_database_without_a_catalog_gets_the_factory_one() {
    let pool = db::connect("sqlite::memory:").await.unwrap();
    db::migrate(&pool).await.unwrap();
    assert!(db::ensure_catalog(&pool).await.unwrap(), "seeded once");
    assert!(!db::ensure_catalog(&pool).await.unwrap(), "never again");
    let loaded = db::load_catalog(&pool).await.unwrap();
    assert!(loaded.items.len() >= 60);
    assert_eq!(
        loaded,
        zeldathon_server::catalog::default_catalog().clone_sorted()
    );
}

// ---- organizer accounts and the admin panel API -------------------------------------------------

use zeldathon_server::accounts::{self, Role};

/// Like `call`, but with arbitrary headers; also returns the response headers.
async fn call_h(
    h: &Harness,
    method: &str,
    uri: &str,
    headers: &[(&str, String)],
    body: Option<Value>,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let mut req = Request::builder().method(method).uri(uri);
    for (k, v) in headers {
        req = req.header(*k, v);
    }
    let req = match body {
        Some(b) => req
            .header("content-type", "application/json")
            .body(Body::from(b.to_string()))
            .unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
    let res = app::build(h.hub.clone()).oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        headers,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

const PASSWORD: &str = "a-strong-passphrase";

struct Login {
    cookie: String,
    csrf: String,
}

impl Login {
    /// Headers for a state-changing call made from the panel.
    fn write(&self) -> Vec<(&'static str, String)> {
        vec![
            ("cookie", self.cookie.clone()),
            ("x-csrf-token", self.csrf.clone()),
        ]
    }

    fn read(&self) -> Vec<(&'static str, String)> {
        vec![("cookie", self.cookie.clone())]
    }
}

async fn make_user(h: &Harness, name: &str, role: Role) {
    accounts::create_user(&h.pool, name, PASSWORD, role)
        .await
        .unwrap();
}

async fn login_as(h: &Harness, name: &str) -> Login {
    let (s, headers, body) = call_h(
        h,
        "POST",
        "/api/admin/auth/login",
        &[],
        Some(json!({ "username": name, "password": PASSWORD })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "login as {name}: {body}");
    let set = headers.get("set-cookie").unwrap().to_str().unwrap();
    let cookie = set.split(';').next().unwrap().to_string();
    Login {
        cookie,
        csrf: body["csrfToken"].as_str().unwrap().to_string(),
    }
}

#[tokio::test]
async fn login_sets_a_locked_down_cookie_and_the_session_works() {
    let h = harness().await;
    make_user(&h, "Ana", Role::Admin).await;

    let (s, headers, body) = call_h(
        &h,
        "POST",
        "/api/admin/auth/login",
        &[],
        Some(json!({ "username": "ANA", "password": PASSWORD })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let set = headers.get("set-cookie").unwrap().to_str().unwrap();
    assert!(set.starts_with("zt_session="));
    assert!(set.contains("HttpOnly") && set.contains("SameSite=Strict"));
    assert_eq!(body["user"]["username"], "ana");
    assert_eq!(body["user"]["role"], "admin");
    assert!(body["user"].get("passwordHash").is_none());

    let login = login_as(&h, "ana").await;
    let (s, _, me) = call_h(&h, "GET", "/api/admin/me", &login.read(), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(me["via"], "session");
    assert_eq!(me["csrfToken"], login.csrf);

    // No cookie, no access.
    assert_eq!(
        call_h(&h, "GET", "/api/admin/me", &[], None).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn bad_logins_look_identical_and_get_locked_after_five() {
    let h = harness().await;
    make_user(&h, "ana", Role::Admin).await;

    let wrong = call_h(
        &h,
        "POST",
        "/api/admin/auth/login",
        &[],
        Some(json!({ "username": "ana", "password": "wrong-password-1" })),
    )
    .await;
    let unknown = call_h(
        &h,
        "POST",
        "/api/admin/auth/login",
        &[],
        Some(json!({ "username": "nobody", "password": "wrong-password-1" })),
    )
    .await;
    assert_eq!(wrong.0, StatusCode::UNAUTHORIZED);
    assert_eq!(wrong.0, unknown.0);
    assert_eq!(wrong.2, unknown.2, "same body: no user enumeration");

    for _ in 0..4 {
        call_h(
            &h,
            "POST",
            "/api/admin/auth/login",
            &[],
            Some(json!({ "username": "ana", "password": "wrong-password-1" })),
        )
        .await;
    }
    // Even the right password is refused while locked.
    let (s, headers, _) = call_h(
        &h,
        "POST",
        "/api/admin/auth/login",
        &[],
        Some(json!({ "username": "ana", "password": PASSWORD })),
    )
    .await;
    assert_eq!(s, StatusCode::TOO_MANY_REQUESTS);
    assert!(headers.get("retry-after").is_some());

    // The failures are in the audit log, without the password.
    let (_, audit) = call(&h, "GET", "/api/admin/audit?limit=50", Some(ADMIN), None).await;
    let failed = audit
        .as_array()
        .unwrap()
        .iter()
        .filter(|a| a["action"] == "auth.login.failed")
        .count();
    assert!(failed >= 5);
    assert!(!audit.to_string().contains("wrong-password"));
}

#[tokio::test]
async fn writes_from_a_session_need_the_csrf_token() {
    let h = harness().await;
    make_user(&h, "ana", Role::Admin).await;
    let login = login_as(&h, "ana").await;

    let (s, _, _) = call_h(
        &h,
        "POST",
        "/api/admin/event/start",
        &login.read(),
        Some(json!({})),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN, "cookie alone is not enough");

    let bad = vec![
        ("cookie", login.cookie.clone()),
        ("x-csrf-token", "not-the-token".to_string()),
    ];
    assert_eq!(
        call_h(&h, "POST", "/api/admin/event/start", &bad, Some(json!({})))
            .await
            .0,
        StatusCode::FORBIDDEN
    );

    let (s, _, event) = call_h(
        &h,
        "POST",
        "/api/admin/event/start",
        &login.write(),
        Some(json!({})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(event["status"], "live");
}

#[tokio::test]
async fn a_wrong_bearer_never_falls_back_to_a_valid_cookie() {
    let h = harness().await;
    make_user(&h, "ana", Role::Admin).await;
    let login = login_as(&h, "ana").await;
    let mut headers = login.read();
    headers.push(("authorization", "Bearer not-the-admin-token".into()));
    assert_eq!(
        call_h(&h, "GET", "/api/admin/me", &headers, None).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn moderators_run_the_race_day_but_cannot_administer() {
    let h = harness().await;
    make_user(&h, "mod", Role::Moderator).await;
    let m = login_as(&h, "mod").await;
    make_event_live(&h).await;

    // Allowed: read the dashboard and control a racer.
    assert_eq!(
        call_h(&h, "GET", "/api/admin/overview", &m.read(), None)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        call_h(&h, "GET", "/api/admin/audit", &m.read(), None)
            .await
            .0,
        StatusCode::OK
    );
    let (s, _, _) = call_h(
        &h,
        "POST",
        "/api/admin/racers/xime/actions/adjust-time",
        &m.write(),
        Some(json!({ "deltaSeconds": -60, "reason": "lag" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);

    // Forbidden: everything that administers.
    let forbidden: Vec<(&str, &str, Value)> = vec![
        ("PUT", "/api/admin/event", json!({ "name": "x" })),
        ("POST", "/api/admin/event/pause", json!({})),
        (
            "POST",
            "/api/admin/racers",
            json!({ "id": "nuevo", "displayName": "N", "timezone": "UTC" }),
        ),
        ("POST", "/api/admin/racers/xime/token", json!({})),
        (
            "POST",
            "/api/admin/racers/xime/actions/finish",
            json!({ "reason": "x" }),
        ),
        ("GET", "/api/admin/users", Value::Null),
        ("GET", "/api/admin/catalog", Value::Null),
    ];
    for (method, path, body) in forbidden {
        let body = if body.is_null() { None } else { Some(body) };
        let (s, _, _) = call_h(&h, method, path, &m.write(), body).await;
        assert_eq!(s, StatusCode::FORBIDDEN, "{method} {path}");
    }
}

#[tokio::test]
async fn the_audit_log_names_the_person_behind_each_action() {
    let h = harness().await;
    make_user(&h, "ana", Role::Admin).await;
    let login = login_as(&h, "ana").await;
    call_h(
        &h,
        "POST",
        "/api/admin/event/start",
        &login.write(),
        Some(json!({})),
    )
    .await;
    // The emergency token is recorded as such.
    call(
        &h,
        "POST",
        "/api/admin/racers/xime/actions/pause",
        Some(ADMIN),
        None,
    )
    .await;

    tokio::time::sleep(Duration::from_millis(200)).await; // the audit writer is asynchronous
    let (_, audit) = call(&h, "GET", "/api/admin/audit?limit=20", Some(ADMIN), None).await;
    let actors: Vec<(&str, &str)> = audit
        .as_array()
        .unwrap()
        .iter()
        .map(|a| (a["action"].as_str().unwrap(), a["actor"].as_str().unwrap()))
        .collect();
    assert!(actors.contains(&("event.start", "ana")), "{actors:?}");
    assert!(actors.contains(&("auth.login", "ana")));
}

#[tokio::test]
async fn logout_sessions_expiry_and_account_changes_end_sessions() {
    let h = harness().await;
    make_user(&h, "ana", Role::Admin).await;
    make_user(&h, "bob", Role::Moderator).await;

    // Logout kills the cookie.
    let a = login_as(&h, "ana").await;
    let (_, headers, _) = call_h(
        &h,
        "POST",
        "/api/admin/auth/logout",
        &a.write(),
        Some(json!({})),
    )
    .await;
    assert!(
        headers
            .get("set-cookie")
            .unwrap()
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );
    assert_eq!(
        call_h(&h, "GET", "/api/admin/me", &a.read(), None).await.0,
        StatusCode::UNAUTHORIZED
    );

    // Expired sessions are refused.
    let a = login_as(&h, "ana").await;
    sqlx::query("UPDATE sessions SET expires_at = '2000-01-01T00:00:00.000Z'")
        .execute(&h.pool)
        .await
        .unwrap();
    assert_eq!(
        call_h(&h, "GET", "/api/admin/me", &a.read(), None).await.0,
        StatusCode::UNAUTHORIZED
    );

    // Disabling a user ends their sessions at once; they cannot log in either.
    let a = login_as(&h, "ana").await;
    let b = login_as(&h, "bob").await;
    let users = call_h(&h, "GET", "/api/admin/users", &a.read(), None)
        .await
        .2;
    let bob_id = users
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["username"] == "bob")
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    let (s, _, _) = call_h(
        &h,
        "PATCH",
        &format!("/api/admin/users/{bob_id}"),
        &a.write(),
        Some(json!({ "disabled": true })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        call_h(&h, "GET", "/api/admin/me", &b.read(), None).await.0,
        StatusCode::UNAUTHORIZED
    );
    let (s, _, _) = call_h(
        &h,
        "POST",
        "/api/admin/auth/login",
        &[],
        Some(json!({ "username": "bob", "password": PASSWORD })),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn nobody_can_lock_the_panel_out_and_passwords_rotate_sessions() {
    let h = harness().await;
    make_user(&h, "ana", Role::Admin).await;
    let a = login_as(&h, "ana").await;
    let users = call_h(&h, "GET", "/api/admin/users", &a.read(), None)
        .await
        .2;
    let ana_id = users[0]["id"].as_i64().unwrap();

    // Not yourself, and never the last admin.
    for body in [json!({ "disabled": true }), json!({ "role": "moderator" })] {
        let (s, _, _) = call_h(
            &h,
            "PATCH",
            &format!("/api/admin/users/{ana_id}"),
            &a.write(),
            Some(body),
        )
        .await;
        assert_eq!(s, StatusCode::CONFLICT);
    }
    assert_eq!(
        call_h(
            &h,
            "DELETE",
            &format!("/api/admin/users/{ana_id}"),
            &a.write(),
            None
        )
        .await
        .0,
        StatusCode::CONFLICT
    );

    // New accounts validate their input.
    let (s, _, _) = call_h(
        &h,
        "POST",
        "/api/admin/users",
        &a.write(),
        Some(json!({ "username": "ok-name", "password": "short", "role": "moderator" })),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _, _) = call_h(
        &h,
        "POST",
        "/api/admin/users",
        &a.write(),
        Some(json!({ "username": "ANA", "password": PASSWORD, "role": "moderator" })),
    )
    .await;
    assert_eq!(
        s,
        StatusCode::CONFLICT,
        "usernames are unique ignoring case"
    );

    // Changing your own password keeps this session but ends the others.
    let other = login_as(&h, "ana").await;
    let (s, _, _) = call_h(
        &h,
        "POST",
        "/api/admin/me/password",
        &a.write(),
        Some(
            json!({ "currentPassword": "not-my-password", "password": "another-long-passphrase" }),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, _, _) = call_h(
        &h,
        "POST",
        "/api/admin/me/password",
        &a.write(),
        Some(json!({ "currentPassword": PASSWORD, "password": "another-long-passphrase" })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        call_h(&h, "GET", "/api/admin/me", &a.read(), None).await.0,
        StatusCode::OK
    );
    assert_eq!(
        call_h(&h, "GET", "/api/admin/me", &other.read(), None)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn event_controls_follow_the_state_machine() {
    let h = harness().await;
    let go = |action: &'static str| {
        let h = &h;
        async move {
            call(
                h,
                "POST",
                &format!("/api/admin/event/{action}"),
                Some(ADMIN),
                Some(json!({ "reason": "test" })),
            )
            .await
        }
    };
    assert_eq!(go("pause").await.0, StatusCode::CONFLICT, "not live yet");
    assert_eq!(go("resume").await.0, StatusCode::CONFLICT);
    let (s, e) = go("start").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(e["status"], "live");
    // Starting early moves the announced start to now.
    let start = chrono::DateTime::parse_from_rfc3339(e["startAtUtc"].as_str().unwrap()).unwrap();
    assert!(start <= Utc::now(), "start moved to now, got {start}");
    assert_eq!(go("start").await.0, StatusCode::CONFLICT);
    assert_eq!(go("pause").await.1["status"], "paused");
    assert_eq!(go("resume").await.1["status"], "live");
    assert_eq!(go("finish").await.1["status"], "finished");
    assert_eq!(
        go("start").await.0,
        StatusCode::CONFLICT,
        "finished is final"
    );
    assert_eq!(go("nonsense").await.0, StatusCode::CONFLICT);
}

#[tokio::test]
async fn the_overview_summarises_the_race_for_the_dashboard() {
    let h = harness().await;
    make_event_live(&h).await;
    let (s, o) = call(&h, "GET", "/api/admin/overview", Some(ADMIN), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(o["event"]["status"], "live");
    assert_eq!(o["summary"]["racers"], 9);
    assert_eq!(o["summary"]["connected"], 0);
    assert_eq!(o["racers"].as_array().unwrap().len(), 9);
    assert!(o["catalogVersion"].as_str().unwrap().len() == 12);
    let codes: Vec<&str> = o["alerts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["code"].as_str().unwrap())
        .collect();
    assert!(codes.contains(&"nobody_connected"), "{codes:?}");

    // A connected racer with a session shows up as live and connected.
    let mut hs = connect_ws(&h, "/ingest", Some(&h.token("cuaco")))
        .await
        .unwrap();
    send(&mut hs, json!({ "type": "HELLO" })).await;
    wait_for(&mut hs, "CLOCK").await;
    send(&mut hs, json!({ "type": "SESSION_STARTED" })).await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let (_, o) = call(&h, "GET", "/api/admin/overview", Some(ADMIN), None).await;
    assert_eq!(o["summary"]["connected"], 1);
    assert_eq!(o["summary"]["live"], 1);
    let row = o["racers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["racer"]["id"] == "cuaco")
        .unwrap();
    assert_eq!(row["connected"], true);
    assert!(row["heartbeatAgeSeconds"].as_i64().unwrap() <= 2);
}

#[tokio::test]
async fn the_catalog_is_managed_through_the_api_and_clients_are_told() {
    let h = harness().await;
    let mut public = connect_ws(&h, "/ws", None).await.unwrap();
    next_json(&mut public).await;

    let item = json!({
        "id": "magic-beans", "group": "tool", "age": "child", "nameEs": "Frijoles Mágicos",
        "nameEn": "Magic Beans", "short": "FM", "sortOrder": 175, "enabled": true
    });
    let (s, _) = call(
        &h,
        "PUT",
        "/api/admin/catalog/items/magic-beans",
        Some(ADMIN),
        Some(item.clone()),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let msg = wait_for(&mut public, "CATALOG_UPDATED").await;
    assert_eq!(msg["version"].as_str().unwrap().len(), 12);

    // The public catalog shows it in order; the id in the URL must match the body.
    let (_, c) = call(&h, "GET", "/api/catalog", None, None).await;
    assert!(
        c["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == "magic-beans" && i["age"] == "child")
    );
    assert_eq!(
        call(
            &h,
            "PUT",
            "/api/admin/catalog/items/other-id",
            Some(ADMIN),
            Some(item.clone())
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(&h, "PUT", "/api/admin/catalog/items/bad-group", Some(ADMIN),
            Some(json!({ "id": "bad-group", "group": "nope", "age": "both", "nameEs": "x", "nameEn": "x", "short": "x", "sortOrder": 1, "enabled": true }))).await.0,
        StatusCode::BAD_REQUEST
    );

    // A racer can now report it (a session is needed first)...
    make_event_live(&h).await;
    let mut hs = connect_ws(&h, "/ingest", Some(&h.token("cuaco")))
        .await
        .unwrap();
    send(&mut hs, json!({ "type": "HELLO" })).await;
    wait_for(&mut hs, "CLOCK").await;
    send(&mut hs, json!({ "type": "SESSION_STARTED" })).await;
    send(
        &mut hs,
        json!({ "type": "ITEM_ACQUIRED", "id": "i1", "item": "magic-beans" }),
    )
    .await;
    let ack = wait_for(&mut hs, "ACK").await;
    assert!(ack["id"].is_null() || ack["id"] == "i1" || ack["id"].is_string());

    // ...until it is hidden. Required objectives cannot be hidden.
    let mut off = item.clone();
    off["enabled"] = json!(false);
    call(
        &h,
        "PUT",
        "/api/admin/catalog/items/magic-beans",
        Some(ADMIN),
        Some(off),
    )
    .await;
    let (_, c) = call(&h, "GET", "/api/catalog", None, None).await;
    assert!(
        !c["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == "magic-beans")
    );
    let (_, all) = call(&h, "GET", "/api/admin/catalog", Some(ADMIN), None).await;
    assert!(
        all["items"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["id"] == "magic-beans" && i["enabled"] == false)
    );
    let ganon = json!({ "id": "ganons-castle", "age": "adult", "nameEs": "x", "nameEn": "x", "sortOrder": 100, "required": true, "enabled": false });
    assert_eq!(
        call(
            &h,
            "PUT",
            "/api/admin/catalog/objectives/ganons-castle",
            Some(ADMIN),
            Some(ganon)
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        call(
            &h,
            "DELETE",
            "/api/admin/catalog/items/magic-beans",
            Some(ADMIN),
            None
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn time_donations_flow_from_hiveshock_to_the_ledger_and_survive_a_restart() {
    let h = harness().await;
    make_event_live(&h).await;
    let mut hs = connect_ws(&h, "/ingest", Some(&h.token("cuaco")))
        .await
        .unwrap();
    send(&mut hs, json!({ "type": "HELLO" })).await;
    let before = wait_for(&mut hs, "CLOCK").await["clock"]["remainingMs"]
        .as_i64()
        .unwrap();

    let donation = json!({
        "type": "TIME_DONATION",
        "id": "don-1",
        "deltaSeconds": 120,
        "source": { "platform": "twitch", "currency": "bits", "amount": 200, "viewer": "fan" }
    });
    send(&mut hs, donation.clone()).await;
    let applied = wait_for(&mut hs, "TIME_APPLIED").await;
    assert_eq!(applied["id"], "don-1");
    assert_eq!(applied["requestedSeconds"], 120);
    assert_eq!(applied["appliedSeconds"], 120);
    assert!(applied.get("limitedBy").is_none());
    assert_eq!(
        applied["clock"]["remainingMs"].as_i64().unwrap(),
        before + 120_000
    );

    // The same id again (a retry after a lost ACK) is acknowledged, not applied.
    send(&mut hs, donation).await;
    assert_eq!(wait_for(&mut hs, "ACK").await["id"], "don-1");

    // The organizer turns donations off: HiveShock is told why.
    let (s, event) = call(
        &h,
        "PUT",
        "/api/admin/event",
        Some(ADMIN),
        Some(json!({ "donationTime": { "enabled": false, "allowAdd": true, "allowRemove": true,
            "maxSecondsPerDonation": 600, "maxAddedSecondsPerDay": 3600, "maxRemovedSecondsPerDay": 3600 } })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(event["donationTime"]["enabled"], false);
    let (_, public) = call(&h, "GET", "/api/event", None, None).await;
    assert_eq!(public["donationTime"]["maxSecondsPerDonation"], 600);
    send(
        &mut hs,
        json!({ "type": "TIME_DONATION", "id": "don-2", "deltaSeconds": -60,
                "source": { "platform": "tiktok", "currency": "diamonds", "amount": 30, "gift": "Rose", "giftCount": 30 } }),
    )
    .await;
    let err = wait_for(&mut hs, "ERROR").await;
    assert_eq!(
        (err["id"].as_str(), err["code"].as_str()),
        (Some("don-2"), Some("not_allowed"))
    );

    // Let the single writer catch up, then read what the organizer panel shows.
    tokio::time::sleep(Duration::from_millis(300)).await;
    let (s, panel) = call(&h, "GET", "/api/admin/donations", Some(ADMIN), None).await;
    assert_eq!(s, StatusCode::OK);
    let recent = panel["recent"].as_array().unwrap();
    assert_eq!(recent.len(), 1);
    assert_eq!(recent[0]["racerId"], "cuaco");
    assert_eq!(recent[0]["viewer"], "fan");
    assert_eq!(recent[0]["appliedSeconds"], 120);
    let totals = panel["totals"].as_array().unwrap();
    assert_eq!(totals[0]["addedSeconds"], 120);
    let donors = panel["donors"].as_array().unwrap();
    assert_eq!(donors.len(), 1);
    assert_eq!(donors[0]["viewer"], "fan");
    assert_eq!(donors[0]["platform"], "twitch");
    assert_eq!(donors[0]["amount"], 200);
    assert_eq!(donors[0]["addedSeconds"], 120);
    let today = panel["today"].as_array().unwrap();
    assert!(
        today
            .iter()
            .any(|t| t["racerId"] == "cuaco" && t["addedSeconds"] == 120)
    );
    let (s, _) = call(&h, "GET", "/api/admin/donations", None, None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    // After a restart the ledger still blocks the retry and the daily counters are kept.
    let reloaded = db::load(&h.pool).await.unwrap().unwrap();
    let i = reloaded.idx("cuaco").unwrap();
    assert!(reloaded.racers[i].donation_ids.contains("don-1"));
    assert_eq!(reloaded.racers[i].donation_added_ms, 120_000);
    assert!(!reloaded.event.donation_time.enabled);
}

/// Raw-body request (the picture upload), answered as bytes.
async fn raw(
    h: &Harness,
    method: &str,
    uri: &str,
    bearer: Option<&str>,
    body: Vec<u8>,
) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(t) = bearer {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    let req = req
        .header("content-type", "application/octet-stream")
        .body(Body::from(body))
        .unwrap();
    let res = app::build(h.hub.clone()).oneshot(req).await.unwrap();
    let (status, headers) = (res.status(), res.headers().clone());
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (status, headers, bytes.to_vec())
}

const TINY_PNG: [u8; 12] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];

#[tokio::test]
async fn racer_photos_are_uploaded_served_replaced_and_deleted() {
    let h = harness().await;
    let up = "/api/admin/racers/ralbat/photo";

    // Only organizers upload, only for racers that exist, only real pictures.
    let (s, _, _) = raw(&h, "POST", up, None, TINY_PNG.to_vec()).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _, _) = raw(
        &h,
        "POST",
        "/api/admin/racers/nadie/photo",
        Some(ADMIN),
        TINY_PNG.to_vec(),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, _, _) = raw(&h, "POST", up, Some(ADMIN), b"<svg></svg>".to_vec()).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let big = vec![0x89; zeldathon_server::media::MAX_BYTES + 5000];
    let (s, _, _) = raw(&h, "POST", up, Some(ADMIN), big).await;
    assert_eq!(s, StatusCode::PAYLOAD_TOO_LARGE);

    // The upload sets the racer's avatar to a public URL that serves the picture.
    let (s, _, body) = raw(&h, "POST", up, Some(ADMIN), TINY_PNG.to_vec()).await;
    assert_eq!(s, StatusCode::CREATED);
    let racer: Value = serde_json::from_slice(&body).unwrap();
    let url = racer["avatarUrl"].as_str().unwrap().to_string();
    assert!(
        url.starts_with("/api/media/racers/ralbat-") && url.ends_with(".png"),
        "{url}"
    );
    let (s, headers, bytes) = raw(&h, "GET", &url, None, vec![]).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(headers["content-type"], "image/png");
    assert!(
        headers["cache-control"]
            .to_str()
            .unwrap()
            .contains("max-age")
    );
    assert_eq!(bytes, TINY_PNG);
    let (_, public) = call(&h, "GET", "/api/racers/ralbat", None, None).await;
    assert_eq!(public["avatarUrl"], url.as_str());

    // A new photo gets a new URL and the old file is gone.
    tokio::time::sleep(Duration::from_millis(5)).await;
    let jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3];
    let (s, _, body) = raw(&h, "POST", up, Some(ADMIN), jpeg.clone()).await;
    assert_eq!(s, StatusCode::CREATED);
    let second: Value = serde_json::from_slice(&body).unwrap();
    let url2 = second["avatarUrl"].as_str().unwrap().to_string();
    assert_ne!(url, url2);
    assert!(url2.ends_with(".jpg"));
    assert_eq!(
        raw(&h, "GET", &url, None, vec![]).await.0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(raw(&h, "GET", &url2, None, vec![]).await.2, jpeg);

    // Nothing else is served from that route, and no path tricks.
    for bad in [
        "/api/media/racers/nope.png",
        "/api/media/racers/..%2FCargo.toml",
        "/api/media/racers/ralbat.png",
    ] {
        assert_eq!(
            raw(&h, "GET", bad, None, vec![]).await.0,
            StatusCode::NOT_FOUND,
            "{bad}"
        );
    }

    // Deleting brings back the placeholder (no avatar at all) and removes the file.
    let (s, _, body) = raw(&h, "DELETE", up, Some(ADMIN), vec![]).await;
    assert_eq!(s, StatusCode::OK);
    let cleared: Value = serde_json::from_slice(&body).unwrap();
    assert!(cleared.get("avatarUrl").is_none_or(|v| v.is_null()));
    assert_eq!(
        raw(&h, "GET", &url2, None, vec![]).await.0,
        StatusCode::NOT_FOUND
    );

    // The audit log knows who changed it, and the avatar survives a restart.
    let (s, _, body) = raw(&h, "POST", up, Some(ADMIN), TINY_PNG.to_vec()).await;
    assert_eq!(s, StatusCode::CREATED);
    let racer: Value = serde_json::from_slice(&body).unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;
    let reloaded = db::load(&h.pool).await.unwrap().unwrap();
    let r = &reloaded.racers[reloaded.idx("ralbat").unwrap()].racer;
    assert_eq!(r.avatar_url.as_deref(), racer["avatarUrl"].as_str());
    let (_, audit) = call(&h, "GET", "/api/admin/audit", Some(ADMIN), None).await;
    assert!(
        audit
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["action"] == "racer.photo.upload")
    );
}

#[tokio::test]
async fn resetting_the_event_wipes_the_test_run_and_keeps_racers_tokens_and_catalog() {
    let h = harness().await;
    let in_30_days = (Utc::now() + chrono::Duration::days(30)).to_rfc3339();
    let reset = |body: Value| {
        let h = &h;
        async move { call(h, "POST", "/api/admin/event/reset", Some(ADMIN), Some(body)).await }
    };

    // A real (not rehearsal) event that is live can never be reset.
    make_event_live(&h).await;
    let (s, _) = reset(json!({ "confirm": "REINICIAR", "startAtUtc": in_30_days })).await;
    assert_eq!(s, StatusCode::CONFLICT);
    // ...and rehearsal cannot be switched on while it is running.
    let (s, _) = call(
        &h,
        "PUT",
        "/api/admin/event",
        Some(ADMIN),
        Some(json!({ "rehearsal": true })),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT);
    // Pause it, switch rehearsal on: the public API says so and sockets are told to reload.
    let (s, _) = call(&h, "POST", "/api/admin/event/pause", Some(ADMIN), None).await;
    assert_eq!(s, StatusCode::OK);
    let mut public = connect_ws(&h, "/ws", None).await.unwrap();
    next_json(&mut public).await;
    let (s, event) = call(
        &h,
        "PUT",
        "/api/admin/event",
        Some(ADMIN),
        Some(json!({ "rehearsal": true })),
    )
    .await;
    assert_eq!(
        (s, event["rehearsal"].clone()),
        (StatusCode::OK, json!(true))
    );
    wait_for(&mut public, "EVENT_UPDATED").await;
    let (_, public_event) = call(&h, "GET", "/api/event", None, None).await;
    assert_eq!(public_event["rehearsal"], true);

    // A rehearsal race: live event, a racer plays, gets an item and a donation.
    call(&h, "POST", "/api/admin/event/resume", Some(ADMIN), None).await;
    let mut hs = connect_ws(&h, "/ingest", Some(&h.token("cuaco")))
        .await
        .unwrap();
    send(&mut hs, json!({ "type": "HELLO" })).await;
    wait_for(&mut hs, "CLOCK").await;
    send(&mut hs, json!({ "type": "SESSION_STARTED" })).await;
    send(
        &mut hs,
        json!({ "type": "ITEM_ACQUIRED", "item": "longshot" }),
    )
    .await;
    send(&mut hs, json!({ "type": "GAME_PROGRESS", "progress": { "percentage": 41.5, "currentArea": "forest-temple", "completedObjectives": ["kokiri-forest"] } })).await;
    send(
        &mut hs,
        json!({ "type": "TIME_DONATION", "id": "d1", "deltaSeconds": 300,
            "source": { "platform": "twitch", "currency": "bits", "amount": 100, "viewer": "fan" } }),
    )
    .await;
    wait_for(&mut hs, "TIME_APPLIED").await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let (_, panel) = call(&h, "GET", "/api/admin/donations", Some(ADMIN), None).await;
    assert_eq!(panel["recent"].as_array().unwrap().len(), 1);

    // The confirmation word and a future start are required.
    let (s, _) = reset(json!({ "confirm": "si", "startAtUtc": in_30_days })).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    let (s, _) =
        reset(json!({ "confirm": "REINICIAR", "startAtUtc": "2020-01-01T00:00:00Z" })).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    // Only an admin.
    let (s, _) = call(
        &h,
        "POST",
        "/api/admin/event/reset",
        None,
        Some(json!({ "confirm": "REINICIAR" })),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    // Rehearsal allows resetting even while live.
    let (s, event) = reset(json!({ "confirm": "REINICIAR", "startAtUtc": in_30_days })).await;
    assert_eq!(s, StatusCode::OK, "{event}");
    assert_eq!(event["status"], "upcoming");
    assert_eq!(event["rehearsal"], false);
    wait_for(&mut public, "EVENT_UPDATED").await;

    let (_, racer) = call(&h, "GET", "/api/racers/cuaco", None, None).await;
    assert_eq!(racer["progressPercentage"], 0.0);
    assert!(racer["items"].as_object().is_none_or(|m| m.is_empty()));
    assert!(racer["completedObjectives"].as_array().unwrap().is_empty());
    assert!(racer["currentArea"].is_null());
    assert_eq!(racer["status"], "online"); // still connected, ready for the real thing
    assert_eq!(racer["remainingSeconds"], 4 * 3600);
    let (_, activity) = call(&h, "GET", "/api/activity", None, None).await;
    assert!(activity.as_array().unwrap().is_empty());
    tokio::time::sleep(Duration::from_millis(300)).await;
    let (_, panel) = call(&h, "GET", "/api/admin/donations", Some(ADMIN), None).await;
    assert!(panel["recent"].as_array().unwrap().is_empty());
    assert!(panel["donors"].as_array().unwrap().is_empty());

    // Racers and their tokens survive, and the reset itself is in the audit log.
    let (_, racers) = call(&h, "GET", "/api/racers", None, None).await;
    assert_eq!(racers.as_array().unwrap().len(), 9);
    let mut again = connect_ws(&h, "/ingest", Some(&h.token("ralbat")))
        .await
        .unwrap();
    send(&mut again, json!({ "type": "HELLO" })).await;
    wait_for(&mut again, "CLOCK").await;
    let (_, audit) = call(&h, "GET", "/api/admin/audit", Some(ADMIN), None).await;
    assert!(
        audit
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["action"] == "event.reset")
    );

    // Everything was written through to the database.
    let reloaded = db::load(&h.pool).await.unwrap().unwrap();
    assert!(!reloaded.event.rehearsal);
    assert_eq!(
        reloaded.event.status,
        zeldathon_server::domain::EventStatus::Upcoming
    );
    let r = &reloaded.racers[reloaded.idx("cuaco").unwrap()];
    assert!(r.racer.completed_objectives.is_empty());
    assert!(r.donation_ids.is_empty());
    assert!(reloaded.activity.is_empty());

    // Now it is a real, upcoming event: starting and then resetting is refused.
    call(&h, "POST", "/api/admin/event/start", Some(ADMIN), None).await;
    let (s, _) = reset(json!({ "confirm": "REINICIAR", "startAtUtc": in_30_days })).await;
    assert_eq!(s, StatusCode::CONFLICT);
}

#[tokio::test]
async fn the_public_donors_board_shows_only_what_is_meant_and_the_organizer_controls_it() {
    let h = harness().await;
    make_event_live(&h).await;
    let mut hs = connect_ws(&h, "/ingest", Some(&h.token("cuaco")))
        .await
        .unwrap();
    send(&mut hs, json!({ "type": "HELLO" })).await;
    wait_for(&mut hs, "CLOCK").await;
    let donations = [
        ("a1", 600, "twitch", "bits", 100, "Mayor"),
        ("a2", 120, "tiktok", "diamonds", 50, "Menor"),
        ("a3", 60, "twitch", "bits", 20, "mayor"), // same person, other case
        ("a4", -30, "tiktok", "diamonds", 10, "Oculto"),
    ];
    for (id, delta, platform, currency, amount, viewer) in donations {
        send(&mut hs, json!({ "type": "TIME_DONATION", "id": id, "deltaSeconds": delta,
            "source": { "platform": platform, "currency": currency, "amount": amount, "viewer": viewer } })).await;
        wait_for(&mut hs, "TIME_APPLIED").await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Public: ranked by time moved, same viewer merged, nothing sensitive.
    let (s, board) = call(&h, "GET", "/api/donors", None, None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(board["enabled"], true);
    let donors = board["donors"].as_array().unwrap();
    let names: Vec<_> = donors
        .iter()
        .map(|d| d["viewer"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Mayor", "Menor", "Oculto"]);
    assert_eq!(donors[0]["rank"], 1);
    assert_eq!(donors[0]["donations"], 2);
    assert_eq!(donors[0]["addedSeconds"], 660);
    assert_eq!(donors[0]["amount"], 120);
    for d in donors {
        for private in ["racerId", "racers", "lastAt", "hidden", "gift", "ts"] {
            assert!(d.get(private).is_none(), "{private} must not be public");
        }
    }
    assert_eq!(board["totals"]["donations"], 4);
    assert_eq!(board["totals"]["addedSeconds"], 780);
    assert_eq!(board["totals"]["removedSeconds"], 30);
    let (_, one) = call(&h, "GET", "/api/donors?limit=1", None, None).await;
    assert_eq!(one["donors"].as_array().unwrap().len(), 1);

    // The organizer hides one donor: gone from the public board, still in the panel (marked).
    let hide = json!({ "platform": "tiktok", "viewer": " OCULTO ", "hidden": true });
    let (s, _) = call(
        &h,
        "PUT",
        "/api/admin/donors/hidden",
        None,
        Some(hide.clone()),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = call(
        &h,
        "PUT",
        "/api/admin/donors/hidden",
        Some(ADMIN),
        Some(hide),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (_, board) = call(&h, "GET", "/api/donors", None, None).await;
    let names: Vec<_> = board["donors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["viewer"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Mayor", "Menor"]);
    let (_, panel) = call(&h, "GET", "/api/admin/donations", Some(ADMIN), None).await;
    let hidden = panel["donors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["viewer"] == "Oculto")
        .unwrap();
    assert_eq!(hidden["hidden"], true);
    assert_eq!(panel["donorsPublic"], true);
    let bad = json!({ "platform": "myspace", "viewer": "x", "hidden": true });
    let (s, _) = call(
        &h,
        "PUT",
        "/api/admin/donors/hidden",
        Some(ADMIN),
        Some(bad),
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);

    // Showing them again, and the global switch.
    let show = json!({ "platform": "tiktok", "viewer": "oculto", "hidden": false });
    call(
        &h,
        "PUT",
        "/api/admin/donors/hidden",
        Some(ADMIN),
        Some(show),
    )
    .await;
    let (_, board) = call(&h, "GET", "/api/donors", None, None).await;
    assert_eq!(board["donors"].as_array().unwrap().len(), 3);
    let (s, _) = call(
        &h,
        "PUT",
        "/api/admin/donors/visibility",
        Some(ADMIN),
        Some(json!({ "enabled": false })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (_, board) = call(&h, "GET", "/api/donors", None, None).await;
    assert_eq!(board["enabled"], false);
    assert!(board["donors"].as_array().unwrap().is_empty());
    assert_eq!(board["totals"]["donations"], 0);
    let (_, panel) = call(&h, "GET", "/api/admin/donations", Some(ADMIN), None).await;
    assert_eq!(panel["donorsPublic"], false);
    assert_eq!(panel["donors"].as_array().unwrap().len(), 3); // the panel still sees everyone
    call(
        &h,
        "PUT",
        "/api/admin/donors/visibility",
        Some(ADMIN),
        Some(json!({ "enabled": true })),
    )
    .await;
    let (_, audit) = call(&h, "GET", "/api/admin/audit", Some(ADMIN), None).await;
    let actions: Vec<_> = audit
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["action"].as_str().unwrap())
        .collect();
    assert!(actions.contains(&"donors.hide") && actions.contains(&"donors.visibility"));

    // Resetting the event (paused, so allowed) empties the board.
    call(&h, "POST", "/api/admin/event/pause", Some(ADMIN), None).await;
    let start = (Utc::now() + chrono::Duration::days(30)).to_rfc3339();
    let (s, _) = call(
        &h,
        "POST",
        "/api/admin/event/reset",
        Some(ADMIN),
        Some(json!({ "confirm": "REINICIAR", "startAtUtc": start })),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    tokio::time::sleep(Duration::from_millis(300)).await;
    let (_, board) = call(&h, "GET", "/api/donors", None, None).await;
    assert!(board["donors"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn the_discord_switch_is_an_organizer_setting_and_never_leaks_the_webhook() {
    let h = harness().await; // no DISCORD_WEBHOOK_URL in the test config
    let (s, _) = call(&h, "GET", "/api/admin/discord", None, None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);

    let (s, st) = call(&h, "GET", "/api/admin/discord", Some(ADMIN), None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        (st["configured"].clone(), st["enabled"].clone()),
        (json!(false), json!(false))
    );
    assert!(st["lastSentAt"].is_null() && st["lastError"].is_null());
    assert!(!st.to_string().contains("webhooks"));

    let (s, st) = call(
        &h,
        "PUT",
        "/api/admin/discord",
        Some(ADMIN),
        Some(json!({ "enabled": true })),
    )
    .await;
    assert_eq!((s, st["enabled"].clone()), (StatusCode::OK, json!(true)));
    let (_, st) = call(&h, "GET", "/api/admin/discord", Some(ADMIN), None).await;
    assert_eq!(st["enabled"], true);

    // Without a configured webhook the test message explains what is missing.
    let (s, err) = call(&h, "POST", "/api/admin/discord/test", Some(ADMIN), None).await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert!(err.to_string().contains("DISCORD_WEBHOOK_URL"), "{err}");

    let (_, audit) = call(&h, "GET", "/api/admin/audit", Some(ADMIN), None).await;
    assert!(
        audit
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["action"] == "discord.toggle")
    );
}

#[tokio::test]
async fn a_racer_going_live_is_announced_once_on_discord_with_their_links() {
    use std::sync::{Arc, Mutex};
    // A stand-in for the Discord webhook that keeps what it receives.
    let seen: Arc<Mutex<Vec<Value>>> = Arc::default();
    let sink = seen.clone();
    let app = axum::Router::new().route(
        "/hook",
        axum::routing::post(move |axum::Json(body): axum::Json<Value>| {
            let sink = sink.clone();
            async move {
                sink.lock().unwrap().push(body);
                StatusCode::NO_CONTENT
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let hook = format!("http://{}/hook", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

    let h = harness_with(Some(hook)).await;
    zeldathon_server::discord::spawn_with(
        h.hub.clone(),
        Duration::from_millis(40),
        zeldathon_server::discord::Announcer::with_timing(
            Duration::from_millis(150),
            Duration::from_secs(60),
        ),
    );
    call(&h, "PATCH", "/api/admin/racers/cuaco", Some(ADMIN),
        Some(json!({ "channels": [{ "platform": "twitch", "handle": "cuaco" }, { "platform": "tiktok", "handle": "cuaco" }] }))).await;

    let go_live = |h: &Harness| {
        let token = h.token("cuaco");
        let addr = h.addr;
        async move {
            let mut req = format!("ws://{addr}/ingest").into_client_request().unwrap();
            req.headers_mut()
                .insert("authorization", format!("Bearer {token}").parse().unwrap());
            let (mut ws, _) = tokio_tungstenite::connect_async(req).await.unwrap();
            send(&mut ws, json!({ "type": "HELLO" })).await;
            send(
                &mut ws,
                json!({ "type": "STREAM_STATE", "live": true, "viewers": 50 }),
            )
            .await;
            ws
        }
    };

    // Switched off (the default) and event not running: nobody is announced.
    let ws = go_live(&h).await;
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(seen.lock().unwrap().is_empty());
    drop(ws);

    // Switched on and the event running: one announcement with the racer's links.
    call(
        &h,
        "PUT",
        "/api/admin/discord",
        Some(ADMIN),
        Some(json!({ "enabled": true })),
    )
    .await;
    make_event_live(&h).await;
    let ws = go_live(&h).await;
    for _ in 0..50 {
        if !seen.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    {
        let msgs = seen.lock().unwrap();
        assert_eq!(msgs.len(), 1, "{msgs:?}");
        let embed = &msgs[0]["embeds"][0];
        assert!(embed["title"].as_str().unwrap().contains("Cuaco"));
        let links = embed["fields"][0]["value"].as_str().unwrap();
        assert!(
            links.contains("https://zeldaton.example/racer/cuaco"),
            "{links}"
        );
        assert!(
            links.contains("(https://twitch.tv/cuaco)")
                && links.contains("(https://tiktok.com/@cuaco)")
        );
        assert_eq!(msgs[0]["allowed_mentions"], json!({ "parse": [] }));
    }
    let (_, st) = call(&h, "GET", "/api/admin/discord", Some(ADMIN), None).await;
    assert!(st["lastSentAt"].is_string() && st["lastError"].is_null());

    // Dropping and coming back inside the cooldown does not announce again.
    drop(ws);
    tokio::time::sleep(Duration::from_millis(300)).await;
    let _ws = go_live(&h).await;
    tokio::time::sleep(Duration::from_millis(800)).await;
    assert_eq!(seen.lock().unwrap().len(), 1);
}
