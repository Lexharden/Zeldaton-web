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
    let (_, standings) = call(&h, "GET", "/api/standings", None, None).await;
    assert_eq!(standings[0], json!({ "racerId": "ralbat", "rank": 1 }));
    let (_, stats) = call(&h, "GET", "/api/hiveshock/stats", None, None).await;
    assert_eq!(stats["connectedRacers"], 1);
    assert!(stats["progressEvents"].as_i64().unwrap() >= 1);

    // State survives a restart: everything was written through to SQLite.
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
    assert_eq!(loaded, zeldathon_server::catalog::default_catalog().clone_sorted());
}
