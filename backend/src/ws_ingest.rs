//! HiveShock ingestion: `WS /ingest` (main channel) and `POST /ingest/events` (batch fallback).
//! Authenticated with the racer's own token (`Authorization: Bearer <token>`).

use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::header::AUTHORIZATION;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::mpsc::unbounded_channel;

use crate::auth::bearer;
use crate::engine::{IngestEnvelope, Reply};
use crate::hub::AppState;
use crate::state::IngestDown;

fn authenticate(hub: &AppState, headers: &HeaderMap) -> Option<String> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok();
    hub.racer_id_for_token(bearer(value)?)
}

/// One inbound message -> the JSON reply for HiveShock.
fn handle_text(hub: &AppState, racer_id: &str, text: &str) -> Value {
    let env: IngestEnvelope = match serde_json::from_str(text) {
        Ok(e) => e,
        Err(e) => {
            return json!({ "type": "ERROR", "code": "invalid", "message": format!("unreadable message: {e}") });
        }
    };
    let id = env.id.clone();
    match hub.ingest(racer_id, env, Utc::now()) {
        Ok(Reply::Ack) => json!({ "type": "ACK", "id": id }),
        Ok(Reply::Clock(clock)) => json!({ "type": "CLOCK", "id": id, "clock": clock }),
        Ok(Reply::TimeApplied(t)) => {
            let mut out = json!({ "type": "TIME_APPLIED", "id": id });
            if let (Some(o), Ok(Value::Object(fields))) =
                (out.as_object_mut(), serde_json::to_value(&*t))
            {
                o.extend(fields);
            }
            out
        }
        Err(e) => json!({ "type": "ERROR", "id": id, "code": e.code(), "message": e.to_string() }),
    }
}

pub async fn ws_handler(
    State(hub): State<AppState>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let Some(racer_id) = authenticate(&hub, &headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    ws.max_message_size(16 * 1024)
        .on_upgrade(move |socket| session(hub, racer_id, socket))
}

async fn session(hub: AppState, racer_id: String, socket: WebSocket) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = unbounded_channel::<IngestDown>();
    let conn_id = hub.next_conn_id();
    hub.attach_ingest(&racer_id, conn_id, tx, Utc::now());
    tracing::info!(racer = %racer_id, conn_id, "HiveShock connected");

    let mut ping = tokio::time::interval(Duration::from_secs(20));
    ping.tick().await;
    loop {
        tokio::select! {
            incoming = stream.next() => match incoming {
                Some(Ok(Message::Text(text))) => {
                    let reply = handle_text(&hub, &racer_id, &text);
                    if sink.send(Message::Text(reply.to_string().into())).await.is_err() { break; }
                }
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                Some(Ok(_)) => {}
            },
            down = rx.recv() => match down {
                Some(IngestDown::ForceClose) => {
                    if sink.send(Message::Text(json!({ "type": "GAME_FORCE_CLOSE" }).to_string().into())).await.is_err() { break; }
                }
                Some(IngestDown::Clock(clock)) => {
                    if sink.send(Message::Text(json!({ "type": "CLOCK", "clock": clock }).to_string().into())).await.is_err() { break; }
                }
                Some(IngestDown::Replaced) => {
                    let _ = sink.send(Message::Text(json!({ "type": "ERROR", "code": "replaced", "message": "another connection took over" }).to_string().into())).await;
                    break;
                }
                None => break,
            },
            _ = ping.tick() => {
                if sink.send(Message::Ping(Vec::new().into())).await.is_err() { break; }
            }
        }
    }
    hub.detach_ingest(&racer_id, conn_id, Utc::now());
    tracing::info!(racer = %racer_id, conn_id, "HiveShock disconnected");
}

#[derive(Deserialize)]
pub struct Batch {
    events: Vec<Value>,
}

/// Fallback for clients that cannot keep a socket open. Events are applied in order; every
/// event gets its own result so one bad event does not lose the rest.
pub async fn post_events(
    State(hub): State<AppState>,
    headers: HeaderMap,
    Json(batch): Json<Batch>,
) -> Response {
    let Some(racer_id) = authenticate(&hub, &headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if batch.events.len() > 100 {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(json!({ "error": "too_many_events", "max": 100 })),
        )
            .into_response();
    }
    let results: Vec<Value> = batch
        .events
        .iter()
        .map(|e| handle_text(&hub, &racer_id, &e.to_string()))
        .collect();
    Json(json!({ "results": results })).into_response()
}
