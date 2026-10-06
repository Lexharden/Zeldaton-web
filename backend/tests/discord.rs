//! The Discord client against a local stand-in for the webhook (no real network).

use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};
use zeldathon_server::notify::client::DiscordClient;

#[derive(Clone, Default)]
struct Hook {
    /// Statuses to answer with, in order (then 204).
    script: Arc<Mutex<Vec<u16>>>,
    bodies: Arc<Mutex<Vec<Value>>>,
}

async fn serve(hook: Hook) -> String {
    async fn handle(State(h): State<Hook>, Json(body): Json<Value>) -> (StatusCode, Json<Value>) {
        h.bodies.lock().unwrap().push(body);
        let next = {
            let mut s = h.script.lock().unwrap();
            if s.is_empty() { 204 } else { s.remove(0) }
        };
        let status = StatusCode::from_u16(next).unwrap();
        let payload = if next == 429 {
            json!({ "retry_after": 0.05 })
        } else {
            json!({})
        };
        (status, Json(payload))
    }
    let app = Router::new().route("/hook", post(handle)).with_state(hook);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{addr}/hook")
}

#[tokio::test]
async fn it_posts_the_payload_and_waits_out_a_rate_limit() {
    let hook = Hook::default();
    *hook.script.lock().unwrap() = vec![429];
    let url = serve(hook.clone()).await;
    let body = json!({ "username": "Zeldatón", "embeds": [{ "title": "hola" }] });
    DiscordClient::new(url).send(&body).await.unwrap();
    let seen = hook.bodies.lock().unwrap();
    assert_eq!(seen.len(), 2, "one 429, then the retry that worked");
    assert_eq!(seen[1], body);
}

#[tokio::test]
async fn a_bad_webhook_fails_at_once_without_leaking_the_url() {
    let hook = Hook::default();
    *hook.script.lock().unwrap() = vec![404, 404, 404];
    let url = serve(hook.clone()).await;
    let err = DiscordClient::new(url.clone())
        .send(&json!({}))
        .await
        .unwrap_err();
    assert!(err.contains("404") && !err.contains(&url), "{err}");
    assert_eq!(
        hook.bodies.lock().unwrap().len(),
        1,
        "no point retrying a 404"
    );
}
