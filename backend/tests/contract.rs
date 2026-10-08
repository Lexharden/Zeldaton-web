//! Wire-contract fixtures shared with the frontend (`../contract`).
//!
//! Rust is the producer, so these tests build every message the server can emit and compare it
//! with the checked-in JSON. Run with `UPDATE_CONTRACT=1 cargo test --test contract` to rewrite
//! the files after an intentional change; the frontend test (`src/contract.test.ts`) then
//! validates the same files against `normalizeMessage` and the TypeScript expectations.

use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use serde_json::{Value, json};
use zeldathon_server::domain::*;
use zeldathon_server::engine::*;
use zeldathon_server::seed;

const NOW_ISO: &str = "2026-10-07T13:00:00.000Z";

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../contract")
}

fn check(rel: &str, actual: Value) {
    let path = dir().join(rel);
    let pretty = serde_json::to_string_pretty(&actual).unwrap() + "\n";
    if std::env::var("UPDATE_CONTRACT").is_ok() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, pretty).unwrap();
        return;
    }
    let expected: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap_or_else(|_| {
            panic!("missing fixture {rel}; run `UPDATE_CONTRACT=1 cargo test --test contract`")
        }))
        .unwrap();
    assert_eq!(
        actual, expected,
        "fixture {rel} is out of date (UPDATE_CONTRACT=1 to refresh)"
    );
}

fn clock(status: RacerStatus, remaining_ms: i64) -> ClockState {
    ClockState {
        racer_id: "ralbat".into(),
        server_time_utc: NOW_ISO.into(),
        remaining_ms,
        status,
        reset_at_utc: "2026-10-08T12:00:00.000Z".into(),
        played_today_ms: 2_460_000,
        played_total_ms: 10_860_000,
    }
}

fn activity() -> ActivityItem {
    ActivityItem {
        id: "act-1".into(),
        timestamp_utc: NOW_ISO.into(),
        kind: ActivityKind::Item,
        racer_id: Some("ralbat".into()),
        racer_name: Some("Ralbat".into()),
        message: "Ralbat acquired longshot".into(),
        code: "ITEM_ACQUIRED".into(),
        detail: Some("LONGSHOT".into()),
        subject: Some("longshot".into()),
    }
}

fn ws_samples() -> Vec<(&'static str, WsMessage)> {
    let id = || "ralbat".to_string();
    vec![
        (
            "CLOCK_SNAPSHOT",
            WsMessage::ClockSnapshot {
                clocks: vec![clock(RacerStatus::Live, 10_800_000)],
            },
        ),
        (
            "CLOCK_SYNC",
            WsMessage::ClockSync {
                clock: clock(RacerStatus::Paused, 9_000_000),
            },
        ),
        (
            "DAILY_RESET",
            WsMessage::DailyReset {
                racer_id: id(),
                clock: clock(RacerStatus::Online, 14_400_000),
            },
        ),
        (
            "SESSION_STARTED",
            WsMessage::SessionStarted { racer_id: id() },
        ),
        (
            "SESSION_PAUSED",
            WsMessage::SessionPaused { racer_id: id() },
        ),
        (
            "SESSION_RESUMED",
            WsMessage::SessionResumed { racer_id: id() },
        ),
        (
            "SESSION_EXHAUSTED",
            WsMessage::SessionExhausted { racer_id: id() },
        ),
        (
            "GAME_FORCE_CLOSE",
            WsMessage::GameForceClose { racer_id: id() },
        ),
        (
            "GAME_PROGRESS",
            WsMessage::GameProgress {
                racer_id: id(),
                progress: GameProgressPatch {
                    percentage: Some(76.0),
                    current_area: Some("water-temple".into()),
                    current_objective: Some("shadow-temple".into()),
                    completed_objectives: Some(vec!["kokiri-forest".into(), "deku-tree".into()]),
                },
            },
        ),
        (
            "ITEM_ACQUIRED",
            WsMessage::ItemAcquired {
                racer_id: id(),
                item: "longshot".into(),
            },
        ),
        (
            "AREA_CHANGED",
            WsMessage::AreaChanged {
                racer_id: id(),
                area: "lake-hylia".into(),
            },
        ),
        (
            "BOSS_DEFEATED",
            WsMessage::BossDefeated {
                racer_id: id(),
                boss: "volvagia".into(),
                bosses_defeated: Some(5),
            },
        ),
        (
            "STATS_UPDATED",
            WsMessage::StatsUpdated {
                racer_id: id(),
                stats: RacerStats {
                    age: Some(LinkAge::Adult),
                    hearts: Some(17.0),
                    rupees: Some(210),
                    ..Default::default()
                },
            },
        ),
        (
            "GAME_FINISHED",
            WsMessage::GameFinished {
                racer_id: id(),
                final_time_seconds: Some(28_800),
                finished_at_utc: Some(NOW_ISO.into()),
            },
        ),
        (
            "RACER_STATUS_CHANGED",
            WsMessage::RacerStatusChanged {
                racer_id: id(),
                status: RacerStatus::Exhausted,
            },
        ),
        (
            "LIVE_ACTIVITY",
            WsMessage::LiveActivity {
                activity: activity(),
            },
        ),
        (
            "STREAM_UPDATED",
            WsMessage::StreamUpdated {
                racer_id: id(),
                stream: StreamState {
                    is_live: true,
                    thumbnail_url: None,
                    viewers: Some(482),
                },
            },
        ),
        ("EVENT_UPDATED", WsMessage::EventUpdated),
        (
            "CATALOG_UPDATED",
            WsMessage::CatalogUpdated {
                version: "a1b2c3d4e5f6".into(),
            },
        ),
        (
            "HIVESHOCK_STATS_UPDATED",
            WsMessage::HiveshockStatsUpdated {
                stats: HiveShockStats {
                    connected_racers: 8,
                    game_events: 12842,
                    item_events: 1294,
                    progress_events: 842,
                    chat_events: 319,
                },
            },
        ),
    ]
}

#[test]
fn every_websocket_message_matches_its_fixture() {
    let samples = ws_samples();
    assert_eq!(
        samples.len(),
        20,
        "one fixture per message type in src/types/websocket.ts"
    );
    for (name, msg) in samples {
        let frame: Value = serde_json::from_str(&encode_frame(&msg, NOW_ISO.into())).unwrap();
        assert_eq!(frame["type"], name, "tag of {name}");
        check(&format!("ws/{name}.json"), frame);
    }
}

#[test]
fn rest_payloads_match_their_fixtures() {
    let now = Utc.with_ymd_and_hms(2026, 10, 7, 13, 0, 0).unwrap();
    let mut state = seed::build(now).state;
    // Give the racer visible progress so the fixture covers optional fields.
    let ev = |msg| IngestEnvelope { id: None, msg };
    state
        .apply_ingest(
            "ralbat",
            ev(IngestMsg::Hello {
                client_version: None,
            }),
            now,
        )
        .unwrap();
    state
        .update_event(EventPatch {
            status: Some(EventStatus::Live),
            ..Default::default()
        })
        .unwrap();
    state
        .apply_ingest("ralbat", ev(IngestMsg::SessionStarted), now)
        .unwrap();
    state
        .apply_ingest(
            "ralbat",
            ev(IngestMsg::GameProgress {
                progress: GameProgressPatch {
                    percentage: Some(76.0),
                    current_area: Some("water-temple".into()),
                    current_objective: Some("shadow-temple".into()),
                    completed_objectives: Some(vec!["kokiri-forest".into(), "deku-tree".into()]),
                },
            }),
            now,
        )
        .unwrap();
    state
        .apply_ingest(
            "ralbat",
            ev(IngestMsg::ItemAcquired {
                item: "longshot".into(),
            }),
            now,
        )
        .unwrap();
    state
        .update_racer(
            "ralbat",
            RacerPatch {
                channels: Some(vec![
                    ChannelInput {
                        platform: Platform::Twitch,
                        handle: "ralbat".into(),
                    },
                    ChannelInput {
                        platform: Platform::Tiktok,
                        handle: "ralbat".into(),
                    },
                ]),
                ..Default::default()
            },
            now,
        )
        .unwrap();
    let later = now + chrono::Duration::seconds(60);

    let mut event = state.event.clone();
    event.status = state.event_status(later);
    check("rest/event.json", serde_json::to_value(&event).unwrap());
    let i = state.idx("ralbat").unwrap();
    check(
        "rest/racer.json",
        serde_json::to_value(state.view(i, later)).unwrap(),
    );
    // A racer that has not connected: minimal optional fields.
    let j = state.idx("insomnia").unwrap();
    check(
        "rest/racer-offline.json",
        serde_json::to_value(state.view(j, later)).unwrap(),
    );
    check(
        "rest/standings.json",
        serde_json::to_value(zeldathon_server::standings::compute(
            &state.views(later),
            &state.event.rules.required_objective_ids,
        ))
        .unwrap(),
    );
    check(
        "rest/clock.json",
        serde_json::to_value(state.clock(i, later)).unwrap(),
    );
    check(
        "rest/stream.json",
        json!({ "racerId": "ralbat", "isLive": true, "viewers": 1200 }),
    );
    // A trimmed public catalog: the shape is what matters (the real one has ~70 entries).
    let full = zeldathon_server::catalog::default_catalog();
    let mut sample = zeldathon_server::catalog::Catalog {
        items: vec![full.items[0].clone(), full.items[9].clone()],
        objectives: vec![full.objectives[0].clone(), full.objectives[4].clone()],
    };
    sample.items[1].icon = Some("/art/items/master-sword.png".into());
    check(
        "rest/catalog.json",
        json!({ "version": sample.version(), "items": sample.items, "objectives": sample.objectives }),
    );
    // The full factory catalog: the website ships it as its offline/mock fallback, so it can never
    // drift from what the server seeds.
    let factory = full.public();
    check(
        "rest/catalog-default.json",
        json!({ "version": factory.version(), "items": factory.items, "objectives": factory.objectives }),
    );
    check(
        "rest/activity.json",
        serde_json::to_value(vec![activity()]).unwrap(),
    );
    check(
        "rest/donors.json",
        zeldathon_server::api::donors_payload(
            true,
            vec![
                json!({ "viewer": "FanDeLink", "platform": "tiktok", "currency": "diamonds", "donations": 7,
                        "amount": 1200, "addedSeconds": 1500, "removedSeconds": 120, "racers": 3,
                        "lastAt": NOW_ISO, "hidden": false }),
                json!({ "viewer": "navi_fan", "platform": "twitch", "currency": "bits", "donations": 2,
                        "amount": 300, "addedSeconds": 600, "removedSeconds": 0, "racers": 1,
                        "lastAt": NOW_ISO, "hidden": false }),
            ],
            json!({ "donations": 9, "addedSeconds": 2100, "removedSeconds": 120 }),
        ),
    );
    check(
        "rest/hiveshock-stats.json",
        serde_json::to_value(&state.stats).unwrap_or_default(),
    );
}
