//! Plays the role of HiveShock for every racer, so the whole stack (backend + website) can be
//! exercised end to end without the game.
//!
//!   cargo run -p zeldathon-server --bin simulate
//!
//! Env: `SERVER` (default 127.0.0.1:8080), `ADMIN_TOKEN` (to switch the event live),
//! `DEV_TOKENS_FILE` (default dev-tokens.json), `SIM_SPEED` (default 1; try 8 for a quick race).

use std::collections::BTreeMap;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use rand::RngExt;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

const OBJECTIVES: [&str; 10] = [
    "kokiri-forest",
    "deku-tree",
    "dodongos-cavern",
    "jabu-jabu",
    "forest-temple",
    "fire-temple",
    "water-temple",
    "shadow-temple",
    "spirit-temple",
    "ganons-castle",
];
const AREAS: [&str; 10] = [
    "kokiri-forest",
    "hyrule-field",
    "death-mountain",
    "zoras-river",
    "sacred-forest-meadow",
    "death-mountain-crater",
    "lake-hylia",
    "temple-of-time",
    "desert-colossus",
    "ganons-castle",
];
const ITEMS: [&str; 9] = [
    "master-sword",
    "hookshot",
    "longshot",
    "bow",
    "bombs",
    "boomerang",
    "megaton-hammer",
    "iron-boots",
    "mirror-shield",
];
const BOSSES: [&str; 6] = [
    "gohma",
    "king-dodongo",
    "barinade",
    "phantom-ganon",
    "volvagia",
    "morpha",
];

/// Minimal HTTP/1.1 PUT (no client dependency) to flip the event live.
async fn set_event_live(server: &str, admin: &str) -> std::io::Result<String> {
    let body = r#"{"status":"live"}"#;
    let req = format!(
        "PUT /api/admin/event HTTP/1.1\r\nHost: {server}\r\nAuthorization: Bearer {admin}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let mut s = tokio::net::TcpStream::connect(server).await?;
    s.write_all(req.as_bytes()).await?;
    let mut out = String::new();
    s.read_to_string(&mut out).await?;
    Ok(out.lines().next().unwrap_or_default().to_string())
}

async fn send(
    ws: &mut (impl SinkExt<Message, Error = tokio_tungstenite::tungstenite::Error> + Unpin),
    v: Value,
) {
    let _ = ws.send(Message::Text(v.to_string().into())).await;
}

async fn racer(server: String, id: String, token: String, speed: f64, index: usize) {
    let mut req = format!("ws://{server}/ingest")
        .into_client_request()
        .expect("valid url");
    req.headers_mut().insert(
        "authorization",
        format!("Bearer {token}").parse().expect("header"),
    );
    let (ws, _) = match tokio_tungstenite::connect_async(req).await {
        Ok(ok) => ok,
        Err(e) => return eprintln!("[{id}] cannot connect: {e}"),
    };
    let (mut tx, mut rx) = ws.split();
    println!("[{id}] connected");
    tokio::time::sleep(Duration::from_millis(400 * index as u64)).await;

    send(
        &mut tx,
        json!({ "type": "HELLO", "clientVersion": "simulate/0.1" }),
    )
    .await;
    send(&mut tx, json!({ "type": "SESSION_STARTED" })).await;

    let mut pct: f64 = 0.0;
    let mut items: Vec<&str> = vec![];
    // StdRng (unlike the thread-local rng) can be held across awaits in a spawned task.
    let mut rng: rand::rngs::StdRng = rand::make_rng();
    let mut hearts = 3.0f64;
    let mut rupees = 0i64;
    let mut paused = false;
    let mut closed = false;
    let mut heartbeat = tokio::time::interval(Duration::from_secs(5));
    let mut step = tokio::time::interval(Duration::from_millis(
        (2500.0 / speed) as u64 + (index as u64 * 137) % 900,
    ));

    loop {
        tokio::select! {
            incoming = rx.next() => match incoming {
                Some(Ok(Message::Text(t))) => {
                    let v: Value = serde_json::from_str(&t).unwrap_or_default();
                    match v["type"].as_str() {
                        Some("GAME_FORCE_CLOSE") => { println!("[{id}] time is up: closing the game"); closed = true; }
                        Some("ERROR") => println!("[{id}] server: {} ({})", v["message"], v["code"]),
                        _ => {}
                    }
                }
                Some(Ok(_)) => {}
                _ => { println!("[{id}] disconnected"); return; }
            },
            _ = heartbeat.tick() => send(&mut tx, json!({ "type": "HEARTBEAT", "gameRunning": !closed && !paused })).await,
            _ = step.tick() => {
                if closed { continue; }
                match rng.random_range(0..12) {
                    0..=5 if !paused => {
                        pct = (pct + rng.random_range(0.6..2.4) * speed.min(4.0)).min(100.0);
                        let done = ((pct / 10.0).floor() as usize).min(10);
                        let next = OBJECTIVES.get(done).copied().unwrap_or("ganons-castle");
                        send(&mut tx, json!({ "type": "GAME_PROGRESS", "progress": {
                            "percentage": (pct * 10.0).round() / 10.0,
                            "currentArea": AREAS[done.min(9)],
                            "currentObjective": next,
                            "completedObjectives": &OBJECTIVES[..done],
                        }})).await;
                        if pct >= 100.0 {
                            send(&mut tx, json!({ "type": "GAME_FINISHED" })).await;
                            println!("[{id}] FINISHED the game");
                            return;
                        }
                    }
                    6 if !paused => {
                        let missing: Vec<_> = ITEMS.iter().filter(|i| !items.contains(i)).collect();
                        if let Some(item) = missing.get(rng.random_range(0..missing.len().max(1))) {
                            items.push(item);
                            send(&mut tx, json!({ "type": "ITEM_ACQUIRED", "item": item })).await;
                        }
                    }
                    7 if !paused => send(&mut tx, json!({ "type": "BOSS_DEFEATED", "boss": BOSSES[rng.random_range(0..BOSSES.len())] })).await,
                    8 if !paused => {
                        hearts = (hearts + 1.0).min(20.0);
                        rupees += rng.random_range(5..60);
                        send(&mut tx, json!({ "type": "STATS_UPDATED", "stats": { "hearts": hearts, "maxHearts": 20, "rupees": rupees, "skulltulas": (pct / 2.5) as i64 } })).await;
                    }
                    9 => {
                        paused = !paused;
                        send(&mut tx, json!({ "type": if paused { "SESSION_PAUSED" } else { "SESSION_RESUMED" } })).await;
                    }
                    10 => send(&mut tx, json!({ "type": "CHAT_EVENT", "count": rng.random_range(1..8) })).await,
                    _ => {}
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let server = std::env::var("SERVER").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let speed: f64 = std::env::var("SIM_SPEED")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1.0);
    let path = std::env::var("DEV_TOKENS_FILE").unwrap_or_else(|_| "dev-tokens.json".into());
    let tokens: BTreeMap<String, String> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| {
            panic!("no tokens: start the server with DEV_TOKENS_FILE={path} on a fresh database")
        });

    if let Ok(admin) = std::env::var("ADMIN_TOKEN") {
        match set_event_live(&server, &admin).await {
            Ok(status) => println!("event -> live: {status}"),
            Err(e) => eprintln!("could not switch the event live: {e}"),
        }
    } else {
        println!("ADMIN_TOKEN not set: the event must already be live (or past its start date).");
    }

    let handles: Vec<_> = tokens
        .into_iter()
        .enumerate()
        .map(|(i, (id, token))| tokio::spawn(racer(server.clone(), id, token, speed, i)))
        .collect();
    println!(
        "simulating {} racers at {speed}x — Ctrl+C to stop",
        handles.len()
    );
    for h in handles {
        let _ = h.await;
    }
}
