use chrono::{DateTime, Duration, TimeZone, Utc};
use zeldathon_server::domain::*;
use zeldathon_server::engine::*;
use zeldathon_server::seed;
use zeldathon_server::state::RaceState;

fn t0() -> DateTime<Utc> {
    // After the event start (7 Oct 2026 12:00Z), before the next 06:00 CDMX reset.
    Utc.with_ymd_and_hms(2026, 10, 7, 13, 0, 0).unwrap()
}

fn state(now: DateTime<Utc>) -> RaceState {
    seed::build(now).state
}

fn env(msg: IngestMsg) -> IngestEnvelope {
    IngestEnvelope { id: None, msg }
}

fn ingest(
    s: &mut RaceState,
    id: &str,
    msg: IngestMsg,
    now: DateTime<Utc>,
) -> Result<(Fx, Reply), IngestError> {
    s.apply_ingest(id, env(msg), now)
}

fn start(s: &mut RaceState, id: &str, now: DateTime<Utc>) {
    ingest(
        s,
        id,
        IngestMsg::Hello {
            client_version: None,
        },
        now,
    )
    .unwrap();
    ingest(s, id, IngestMsg::SessionStarted, now).unwrap();
}

fn kinds(fx: &Fx) -> Vec<String> {
    fx.msgs
        .iter()
        .map(|m| {
            serde_json::to_value(m).unwrap()["type"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect()
}

#[test]
fn seed_has_nine_racers_with_full_budget() {
    let seeded = seed::build(t0());
    assert_eq!(seeded.state.racers.len(), 9);
    assert_eq!(seeded.tokens.len(), 9);
    let r = seeded.state.view(0, t0());
    assert_eq!(r.remaining_seconds, 14_400);
    assert_eq!(r.status, RacerStatus::Offline);
}

#[test]
fn session_cannot_start_before_the_event() {
    let before = Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap();
    let mut s = state(before);
    assert_eq!(s.event_status(before), EventStatus::Upcoming);
    ingest(
        &mut s,
        "ralbat",
        IngestMsg::Hello {
            client_version: None,
        },
        before,
    )
    .unwrap();
    let err = ingest(&mut s, "ralbat", IngestMsg::SessionStarted, before).unwrap_err();
    assert_eq!(err, IngestError::EventNotLive);
}

#[test]
fn clock_counts_only_while_live() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    let i = s.idx("ralbat").unwrap();
    assert_eq!(
        s.clock(i, t0() + Duration::seconds(60)).remaining_ms,
        14_400_000 - 60_000
    );

    ingest(
        &mut s,
        "ralbat",
        IngestMsg::SessionPaused,
        t0() + Duration::seconds(60),
    )
    .unwrap();
    assert_eq!(
        s.clock(i, t0() + Duration::seconds(600)).remaining_ms,
        14_400_000 - 60_000
    );
    assert_eq!(
        s.view(i, t0() + Duration::seconds(600)).status,
        RacerStatus::Paused
    );
}

#[test]
fn exhaustion_forces_close_and_blocks_restart() {
    let mut s = state(t0());
    start(&mut s, "cuaco", t0());
    let end = t0() + Duration::hours(4);
    let fx = s.tick(end, 20);
    let k = kinds(&fx);
    assert!(k.contains(&"SESSION_EXHAUSTED".to_string()));
    assert!(k.contains(&"GAME_FORCE_CLOSE".to_string()));
    assert!(
        fx.down
            .iter()
            .any(|(id, d)| id == "cuaco" && *d == zeldathon_server::state::IngestDown::ForceClose)
    );
    let i = s.idx("cuaco").unwrap();
    assert_eq!(s.view(i, end).status, RacerStatus::Exhausted);
    assert_eq!(s.clock(i, end).remaining_ms, 0);

    let err = ingest(
        &mut s,
        "cuaco",
        IngestMsg::SessionStarted,
        end + Duration::seconds(5),
    )
    .unwrap_err();
    assert_eq!(err, IngestError::Exhausted);
}

#[test]
fn daily_reset_restores_the_budget_at_local_six() {
    let mut s = state(t0());
    start(&mut s, "cuaco", t0());
    s.tick(t0() + Duration::hours(4), 20); // exhausted
    // Next 06:00 in Mexico City = 2026-10-08T12:00Z.
    let reset = Utc.with_ymd_and_hms(2026, 10, 8, 12, 0, 0).unwrap();
    let fx = s.tick(reset + Duration::seconds(1), i64::MAX);
    assert!(kinds(&fx).contains(&"DAILY_RESET".to_string()));
    let i = s.idx("cuaco").unwrap();
    let v = s.view(i, reset + Duration::seconds(1));
    assert_eq!(v.remaining_seconds, 14_400);
    assert_ne!(v.status, RacerStatus::Exhausted);
    // The following reset is a day later.
    let c = s.clock(i, reset + Duration::seconds(1));
    assert_eq!(c.reset_at_utc, "2026-10-09T12:00:00.000Z");
}

#[test]
fn lost_heartbeat_takes_racer_offline_and_stops_the_clock() {
    let mut s = state(t0());
    start(&mut s, "xime", t0());
    let fx = s.tick(t0() + Duration::seconds(30), 20);
    assert!(kinds(&fx).contains(&"RACER_STATUS_CHANGED".to_string()));
    let i = s.idx("xime").unwrap();
    let rem = s.clock(i, t0() + Duration::seconds(30)).remaining_ms;
    assert_eq!(
        s.view(i, t0() + Duration::seconds(30)).status,
        RacerStatus::Offline
    );
    assert_eq!(s.clock(i, t0() + Duration::hours(1)).remaining_ms, rem);
}

#[test]
fn progress_is_validated_and_broadcast() {
    let mut s = state(t0());
    // No session yet.
    let p = GameProgressPatch {
        percentage: Some(10.0),
        ..Default::default()
    };
    assert!(matches!(
        ingest(
            &mut s,
            "ralbat",
            IngestMsg::GameProgress {
                progress: p.clone()
            },
            t0()
        ),
        Err(IngestError::OutOfSequence(_))
    ));
    start(&mut s, "ralbat", t0());
    let bad = GameProgressPatch {
        completed_objectives: Some(vec!["nope".into()]),
        ..Default::default()
    };
    assert!(matches!(
        ingest(
            &mut s,
            "ralbat",
            IngestMsg::GameProgress { progress: bad },
            t0()
        ),
        Err(IngestError::Invalid(_))
    ));
    let ok = GameProgressPatch {
        percentage: Some(250.0),
        current_area: Some("water-temple".into()),
        completed_objectives: Some(vec!["kokiri-forest".into(), "kokiri-forest".into()]),
        ..Default::default()
    };
    let (fx, _) = ingest(
        &mut s,
        "ralbat",
        IngestMsg::GameProgress { progress: ok },
        t0(),
    )
    .unwrap();
    let k = kinds(&fx);
    assert!(k.contains(&"GAME_PROGRESS".to_string()));
    assert!(k.contains(&"LIVE_ACTIVITY".to_string())); // area changed
    assert!(k.contains(&"HIVESHOCK_STATS_UPDATED".to_string()));
    let v = s.view(s.idx("ralbat").unwrap(), t0());
    assert_eq!(v.progress_percentage, 100.0); // clamped
    assert_eq!(v.completed_objectives, vec!["kokiri-forest"]); // deduped
    assert_eq!(s.stats.progress_events, 1);
}

#[test]
fn items_are_idempotent_and_catalog_checked() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    let item = |i: &str| IngestMsg::ItemAcquired { item: i.into() };
    assert!(ingest(&mut s, "ralbat", item("longshot"), t0()).is_ok());
    let (fx, _) = ingest(&mut s, "ralbat", item("longshot"), t0()).unwrap();
    assert!(fx.msgs.is_empty()); // already had it
    assert!(matches!(
        ingest(&mut s, "ralbat", item("triforce"), t0()),
        Err(IngestError::Invalid(_))
    ));
    assert_eq!(s.stats.item_events, 1);
}

#[test]
fn duplicate_event_ids_are_applied_once() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    let ev = || IngestEnvelope {
        id: Some("evt-1".into()),
        msg: IngestMsg::BossDefeated {
            boss: "volvagia".into(),
        },
    };
    s.apply_ingest("ralbat", ev(), t0()).unwrap();
    s.apply_ingest("ralbat", ev(), t0()).unwrap();
    let v = s.view(s.idx("ralbat").unwrap(), t0());
    assert_eq!(v.stats.unwrap().bosses_defeated, Some(1));
}

#[test]
fn finishing_requires_every_required_objective() {
    let mut s = state(t0());
    start(&mut s, "pinchiviejo", t0());
    let half = GameProgressPatch {
        completed_objectives: Some(vec!["kokiri-forest".into(), "deku-tree".into()]),
        ..Default::default()
    };
    ingest(
        &mut s,
        "pinchiviejo",
        IngestMsg::GameProgress { progress: half },
        t0(),
    )
    .unwrap();
    assert_eq!(
        ingest(&mut s, "pinchiviejo", IngestMsg::GameFinished, t0()).unwrap_err(),
        IngestError::RequirementsNotMet
    );

    let all = GameProgressPatch {
        completed_objectives: Some(zeldathon_server::catalog::default_catalog().default_required()),
        ..Default::default()
    };
    ingest(
        &mut s,
        "pinchiviejo",
        IngestMsg::GameProgress { progress: all },
        t0() + Duration::seconds(30),
    )
    .unwrap();
    let done_at = t0() + Duration::seconds(90);
    let (fx, _) = ingest(&mut s, "pinchiviejo", IngestMsg::GameFinished, done_at).unwrap();
    assert!(kinds(&fx).contains(&"GAME_FINISHED".to_string()));
    let v = s.view(s.idx("pinchiviejo").unwrap(), done_at);
    assert_eq!(v.status, RacerStatus::Finished);
    assert_eq!(v.final_time_seconds, Some(90)); // only played time counts
    assert_eq!(s.winner.as_ref().unwrap().racer_id, "pinchiviejo");
}

#[test]
fn recovery_marks_everyone_offline_without_counting_downtime() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    let restart = t0() + Duration::minutes(10);
    // Persisted state has the racer live from t0; the server was down until `restart`.
    let ri = s.idx("ralbat").unwrap();
    s.racers[ri].checkpoint.at = t0();
    s.recover(restart);
    let i = s.idx("ralbat").unwrap();
    assert_eq!(s.view(i, restart).status, RacerStatus::Offline);
    let held = s.racers[i].checkpoint.remaining_ms;
    assert_eq!(s.clock(i, restart + Duration::hours(1)).remaining_ms, held);
}

#[test]
fn admin_can_adjust_time_and_revive_an_exhausted_racer() {
    let mut s = state(t0());
    start(&mut s, "cuaco", t0());
    s.tick(t0() + Duration::hours(4), 20);
    s.admin_action(
        "cuaco",
        AdminAction::AdjustTime { delta_seconds: 600 },
        t0() + Duration::hours(4),
    )
    .unwrap();
    let i = s.idx("cuaco").unwrap();
    let v = s.view(i, t0() + Duration::hours(4));
    assert_eq!(v.remaining_seconds, 600);
    assert_ne!(v.status, RacerStatus::Exhausted);
}

#[test]
fn stream_state_updates_viewers_publishes_once_and_clamps() {
    let mut s = state(t0());
    ingest(
        &mut s,
        "ralbat",
        IngestMsg::Hello {
            client_version: None,
        },
        t0(),
    )
    .unwrap();

    // Works without a game session: the racer can be live on Twitch before opening the game.
    let (fx, _) = ingest(
        &mut s,
        "ralbat",
        IngestMsg::StreamState {
            live: true,
            viewers: Some(482),
        },
        t0(),
    )
    .unwrap();
    assert_eq!(kinds(&fx), vec!["STREAM_UPDATED"]);
    let stream = s.view(s.idx("ralbat").unwrap(), t0()).stream.unwrap();
    assert!(stream.is_live);
    assert_eq!(stream.viewers, Some(482));

    // Same values again: nothing to publish.
    let (fx, _) = ingest(
        &mut s,
        "ralbat",
        IngestMsg::StreamState {
            live: true,
            viewers: Some(482),
        },
        t0(),
    )
    .unwrap();
    assert!(fx.msgs.is_empty());

    // Nonsense is clamped, a missing count is zero.
    ingest(
        &mut s,
        "ralbat",
        IngestMsg::StreamState {
            live: true,
            viewers: Some(-5),
        },
        t0(),
    )
    .unwrap();
    let stream = s.view(s.idx("ralbat").unwrap(), t0()).stream.unwrap();
    assert_eq!(stream.viewers, Some(0));

    // Off the air: no viewers, and not live because nobody is playing either.
    ingest(
        &mut s,
        "ralbat",
        IngestMsg::StreamState {
            live: false,
            viewers: Some(99),
        },
        t0(),
    )
    .unwrap();
    let stream = s.view(s.idx("ralbat").unwrap(), t0()).stream.unwrap();
    assert_eq!(stream.viewers, None);
    assert!(!stream.is_live);
}

#[test]
fn stream_stays_live_while_playing_even_if_the_broadcast_is_reported_off() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    ingest(
        &mut s,
        "ralbat",
        IngestMsg::StreamState {
            live: false,
            viewers: None,
        },
        t0(),
    )
    .unwrap();
    let stream = s.view(s.idx("ralbat").unwrap(), t0()).stream.unwrap();
    assert!(stream.is_live, "playing counts as live");
}

#[test]
fn losing_hiveshock_clears_the_reported_viewers() {
    let mut s = state(t0());
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    s.attach_ingest(
        "ralbat",
        zeldathon_server::state::IngestHandle { conn_id: 1, tx },
    );
    ingest(
        &mut s,
        "ralbat",
        IngestMsg::StreamState {
            live: true,
            viewers: Some(10),
        },
        t0(),
    )
    .unwrap();

    let fx = s.detach_ingest("ralbat", 1);
    assert!(kinds(&fx).contains(&"STREAM_UPDATED".to_string()));
    let stream = s.view(s.idx("ralbat").unwrap(), t0()).stream.unwrap();
    assert_eq!(stream.viewers, None);
    assert!(!stream.is_live);
}

// ---- catalog ------------------------------------------------------------------------------------

use zeldathon_server::catalog::{Age, CatalogItem, CatalogObjective};

fn new_item(id: &str) -> CatalogItem {
    CatalogItem {
        id: id.into(),
        group: "tool".into(),
        age: Age::Both,
        name_es: "Nuevo".into(),
        name_en: "New".into(),
        short: "NW".into(),
        icon: None,
        sort_order: 999,
        enabled: true,
    }
}

#[test]
fn the_factory_catalog_is_loaded_and_ingest_accepts_the_new_items() {
    let mut s = state(t0());
    assert!(s.catalog.items.len() >= 60);
    start(&mut s, "ralbat", t0());
    // Items that did not exist before the catalog grew.
    for item in [
        "kokiri-sword",
        "hover-boots",
        "nayrus-love",
        "forest-medallion",
        "zeldas-lullaby",
    ] {
        ingest(
            &mut s,
            "ralbat",
            IngestMsg::ItemAcquired { item: item.into() },
            t0(),
        )
        .unwrap_or_else(|e| panic!("{item}: {e}"));
    }
    let racer = s.view(s.idx("ralbat").unwrap(), t0());
    assert_eq!(racer.items.get("hover-boots"), Some(&true));
    // Still unknown.
    let err = ingest(
        &mut s,
        "ralbat",
        IngestMsg::ItemAcquired {
            item: "banana".into(),
        },
        t0(),
    )
    .unwrap_err();
    assert_eq!(err.code(), "invalid");
}

#[test]
fn a_new_catalog_item_is_reportable_and_a_disabled_one_is_not() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());

    let fx = s.catalog_upsert_item(new_item("magic-beans")).unwrap();
    assert_eq!(kinds(&fx), vec!["CATALOG_UPDATED"]);
    ingest(
        &mut s,
        "ralbat",
        IngestMsg::ItemAcquired {
            item: "magic-beans".into(),
        },
        t0(),
    )
    .unwrap();

    let mut off = new_item("magic-beans");
    off.enabled = false;
    s.catalog_upsert_item(off).unwrap();
    let err = ingest(
        &mut s,
        "ralbat",
        IngestMsg::ItemAcquired {
            item: "magic-beans".into(),
        },
        t0(),
    );
    // Already owned counts as a repeat first; a different racer proves the rejection.
    assert!(err.is_ok() || err.unwrap_err().code() == "invalid");
    start(&mut s, "xime", t0());
    assert_eq!(
        ingest(
            &mut s,
            "xime",
            IngestMsg::ItemAcquired {
                item: "magic-beans".into()
            },
            t0()
        )
        .unwrap_err()
        .code(),
        "invalid"
    );
    assert!(
        !s.catalog
            .public()
            .items
            .iter()
            .any(|i| i.id == "magic-beans")
    );
}

#[test]
fn catalog_edits_are_validated_and_deletes_report_missing_entries() {
    let mut s = state(t0());
    let mut bad = new_item("Not Valid");
    assert!(s.catalog_upsert_item(bad.clone()).is_err());
    bad.id = "fine-id".into();
    bad.group = "nope".into();
    assert!(s.catalog_upsert_item(bad).is_err());

    s.catalog_upsert_item(new_item("temp-item")).unwrap();
    let fx = s.catalog_delete_item("temp-item").unwrap();
    assert_eq!(kinds(&fx), vec!["CATALOG_UPDATED"]);
    assert!(s.catalog_delete_item("temp-item").is_err());
}

#[test]
fn required_objectives_cannot_be_disabled_or_deleted_but_others_can() {
    let mut s = state(t0());
    let mut ganon = s
        .catalog
        .objectives
        .iter()
        .find(|o| o.id == "ganons-castle")
        .unwrap()
        .clone();
    ganon.enabled = false;
    assert!(s.catalog_upsert_objective(ganon).is_err());
    assert!(s.catalog_delete_objective("ganons-castle").is_err());

    // Once it is no longer required by the event, it can go.
    s.update_event(EventPatch {
        required_objective_ids: Some(vec!["deku-tree".into()]),
        ..Default::default()
    })
    .unwrap();
    s.catalog_delete_objective("ganons-castle").unwrap();
    assert!(!s.catalog.objective_exists("ganons-castle"));

    let extra = CatalogObjective {
        id: "bonus-goal".into(),
        age: Age::Child,
        name_es: "Bono".into(),
        name_en: "Bonus".into(),
        sort_order: 5,
        required: false,
        enabled: true,
    };
    s.catalog_upsert_objective(extra).unwrap();
    assert!(s.catalog.is_objective("bonus-goal"));
}

#[test]
fn stats_carry_the_current_link_age() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    ingest(
        &mut s,
        "ralbat",
        IngestMsg::StatsUpdated {
            stats: RacerStats {
                age: Some(LinkAge::Child),
                ..Default::default()
            },
        },
        t0(),
    )
    .unwrap();
    ingest(
        &mut s,
        "ralbat",
        IngestMsg::StatsUpdated {
            stats: RacerStats {
                hearts: Some(3.0),
                ..Default::default()
            },
        },
        t0(),
    )
    .unwrap();
    let stats = s.view(s.idx("ralbat").unwrap(), t0()).stats.unwrap();
    assert_eq!(
        stats.age,
        Some(LinkAge::Child),
        "a partial update keeps the age"
    );
    assert_eq!(stats.hearts, Some(3.0));
}
