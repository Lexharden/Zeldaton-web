//! Public WebSocket (`/ws`): live race updates for the website.
//!
//! Protocol notes (must match `src/services/websocket/*` in the frontend):
//! - every frame carries a top-level `serverTimeUtc`;
//! - the client's JSON `PING` is tolerated but never answered with a custom message
//!   (`PONG` is not a known event and would only log warnings), liveness uses protocol pings;
//! - a client that falls behind receives a fresh `CLOCK_SNAPSHOT` instead of being dropped.

use std::time::Duration;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast::error::RecvError;

use crate::domain::PublicClientMessage;
use crate::hub::AppState;

pub async fn handler(State(hub): State<AppState>, ws: WebSocketUpgrade) -> Response {
    ws.max_message_size(4 * 1024)
        .on_upgrade(move |socket| session(hub, socket))
}

async fn session(hub: AppState, socket: WebSocket) {
    let (mut sink, mut stream) = socket.split();
    // Subscribe before taking the snapshot so no update can slip between the two.
    let mut rx = hub.subscribe();
    if sink
        .send(Message::Text(hub.snapshot_frame(Utc::now()).into()))
        .await
        .is_err()
    {
        return;
    }
    let mut ping = tokio::time::interval(Duration::from_secs(25));
    ping.tick().await;

    loop {
        tokio::select! {
            update = rx.recv() => match update {
                Ok(frame) => {
                    if sink.send(Message::Text(frame.to_string().into())).await.is_err() { break; }
                }
                Err(RecvError::Lagged(skipped)) => {
                    tracing::warn!(skipped, "public ws client lagged; resending snapshot");
                    if sink.send(Message::Text(hub.snapshot_frame(Utc::now()).into())).await.is_err() { break; }
                }
                Err(RecvError::Closed) => break,
            },
            incoming = stream.next() => match incoming {
                Some(Ok(Message::Text(text))) => {
                    if let Ok(PublicClientMessage::ClockSyncRequest) = serde_json::from_str(&text)
                        && sink.send(Message::Text(hub.snapshot_frame(Utc::now()).into())).await.is_err()
                    {
                        break;
                    }
                }
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                Some(Ok(_)) => {}
            },
            _ = ping.tick() => {
                if sink.send(Message::Ping(Vec::new().into())).await.is_err() { break; }
            }
        }
    }
}
