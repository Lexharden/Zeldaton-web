use chrono::{DateTime, Duration, TimeZone, Utc};
use zeldathon_server::db::{DayDelta, PersistOp};
use zeldathon_server::domain::*;
use zeldathon_server::engine::*;
use zeldathon_server::notify::{Detail, Notice};
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

#[test]
fn rotating_the_token_clears_a_broadcast_reported_by_the_dropped_hiveshock() {
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

    let fx = s.rotate_token("ralbat", "new-hash".into()).unwrap();
    assert!(kinds(&fx).contains(&"STREAM_UPDATED".to_string()));
    let stream = s.view(s.idx("ralbat").unwrap(), t0()).stream.unwrap();
    assert_eq!(stream.viewers, None);
    assert!(!stream.is_live);

    // The old session closing afterwards changes nothing.
    s.detach_ingest("ralbat", 1);
    assert!(
        !s.view(s.idx("ralbat").unwrap(), t0())
            .stream
            .unwrap()
            .is_live
    );
}

#[test]
fn a_lost_heartbeat_and_a_restart_both_clear_a_reported_broadcast() {
    let mut s = state(t0());
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    s.attach_ingest(
        "ralbat",
        zeldathon_server::state::IngestHandle { conn_id: 1, tx },
    );
    ingest(
        &mut s,
        "ralbat",
        IngestMsg::Heartbeat { game_running: None },
        t0(),
    )
    .unwrap();
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
    let i = s.idx("ralbat").unwrap();

    // Heartbeat lost: the racer goes offline and the broadcast is no longer reported.
    let later = t0() + Duration::seconds(120);
    s.tick(later, 30);
    let stream = s.view(i, later).stream.unwrap();
    assert_eq!(stream.viewers, None);
    assert!(!stream.is_live);

    // A stale persisted broadcast on an offline racer does not survive a restart.
    let mut s = state(t0());
    let i = s.idx("ralbat").unwrap();
    {
        let stream = s.racers[i]
            .racer
            .stream
            .get_or_insert_with(Default::default);
        stream.is_live = true;
        stream.viewers = Some(5);
    }
    s.recover(t0());
    let stream = s.view(i, t0()).stream.unwrap();
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

// ---- time donations -------------------------------------------------------------------------------

fn bits(amount: i64) -> DonationSource {
    DonationSource {
        platform: Platform::Twitch,
        currency: DonationCurrency::Bits,
        amount,
        gift: None,
        gift_count: None,
        viewer: Some("viewer".into()),
    }
}

fn diamonds(gift: &str, count: i64, amount: i64) -> DonationSource {
    DonationSource {
        platform: Platform::Tiktok,
        currency: DonationCurrency::Diamonds,
        amount,
        gift: Some(gift.into()),
        gift_count: Some(count),
        viewer: None,
    }
}

fn donate(
    s: &mut RaceState,
    racer: &str,
    id: &str,
    delta_seconds: i64,
    source: DonationSource,
    now: DateTime<Utc>,
) -> Result<(Fx, Reply), IngestError> {
    s.apply_ingest(
        racer,
        IngestEnvelope {
            id: Some(id.into()),
            msg: IngestMsg::TimeDonation {
                delta_seconds,
                source,
            },
        },
        now,
    )
}

fn applied(reply: &Reply) -> &TimeApplied {
    match reply {
        Reply::TimeApplied(t) => t,
        other => panic!("expected TIME_APPLIED, got {other:?}"),
    }
}

fn ledger(fx: &Fx) -> Vec<&zeldathon_server::db::TimeDonationRow> {
    fx.ops
        .iter()
        .filter_map(|op| match op {
            zeldathon_server::db::PersistOp::TimeDonation(row) => Some(row.as_ref()),
            _ => None,
        })
        .collect()
}

#[test]
fn donations_add_and_remove_time_and_are_recorded() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    let i = s.idx("ralbat").unwrap();

    let (fx, reply) = donate(&mut s, "ralbat", "d1", 90, bits(150), t0()).unwrap();
    let t = applied(&reply);
    assert_eq!(
        (t.requested_seconds, t.applied_seconds, t.limited_by),
        (90, 90, None)
    );
    assert_eq!(t.clock.remaining_ms, 14_400_000 + 90_000);
    let row = ledger(&fx)[0];
    assert_eq!(
        (row.amount, row.applied_ms, row.client_id.as_str()),
        (150, 90_000, "d1")
    );
    let k = kinds(&fx);
    assert!(k.contains(&"CLOCK_SYNC".to_string()) && k.contains(&"LIVE_ACTIVITY".to_string()));
    let feed = &s.activity[0];
    assert_eq!(
        (feed.code.as_str(), feed.kind),
        ("TIME_ADDED", ActivityKind::Time)
    );
    assert_eq!(feed.detail.as_deref(), Some("+00:01:30 · 150 BITS"));

    // Diamonds use the organizer's rate (3 s each): 5 diamonds = 15 s, whatever HiveShock said.
    let (fx, reply) = donate(&mut s, "ralbat", "d2", -30, diamonds("Rose", 5, 5), t0()).unwrap();
    assert_eq!(applied(&reply).applied_seconds, -15);
    assert_eq!(applied(&reply).requested_seconds, -15);
    let row = ledger(&fx)[0];
    assert_eq!(
        (row.requested_ms, row.reported_ms),
        (-15_000, Some(-30_000))
    );
    assert_eq!(s.activity[0].code, "TIME_REMOVED");
    assert_eq!(
        s.activity[0].detail.as_deref(),
        Some("-00:00:15 · ROSE X5 · 5 DIAMONDS")
    );
    assert_eq!(s.clock(i, t0()).remaining_ms, 14_400_000 + 75_000);
    assert_eq!(
        (
            s.racers[i].donation_added_ms,
            s.racers[i].donation_removed_ms
        ),
        (90_000, 15_000)
    );
}

#[test]
fn a_retried_donation_is_applied_once_even_after_the_recent_ids_are_gone() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    donate(&mut s, "ralbat", "same", 60, bits(100), t0()).unwrap();
    let i = s.idx("ralbat").unwrap();
    // A restart empties the short window of recent ids; the ledger ids stay.
    s.racers[i].seen_ids.clear();
    let (fx, reply) = donate(&mut s, "ralbat", "same", 60, bits(100), t0()).unwrap();
    assert_eq!(reply, Reply::Ack);
    assert!(ledger(&fx).is_empty());
    assert_eq!(s.clock(i, t0()).remaining_ms, 14_400_000 + 60_000);
}

#[test]
fn organizer_policy_turns_donations_off_or_limits_them() {
    let mut s = state(t0());
    start(&mut s, "cuaco", t0());

    s.event.donation_time.enabled = false;
    let err = donate(&mut s, "cuaco", "a", 60, bits(100), t0()).unwrap_err();
    assert_eq!(err.code(), "not_allowed");

    s.event.donation_time = DonationTimePolicy {
        allow_remove: false,
        max_seconds_per_donation: 120,
        max_added_seconds_per_day: 150,
        ..DonationTimePolicy::default()
    };
    let err = donate(&mut s, "cuaco", "b", -60, bits(100), t0()).unwrap_err();
    assert_eq!(err.code(), "not_allowed");

    let (_, r) = donate(&mut s, "cuaco", "c", 600, bits(1000), t0()).unwrap();
    assert_eq!(
        (applied(&r).applied_seconds, applied(&r).limited_by),
        (120, Some("per_donation"))
    );
    let (_, r) = donate(&mut s, "cuaco", "d", 100, bits(100), t0()).unwrap();
    assert_eq!(
        (applied(&r).applied_seconds, applied(&r).limited_by),
        (30, Some("daily_limit"))
    );
    let activity_before = s.activity.len();
    let (fx, r) = donate(&mut s, "cuaco", "e", 100, bits(100), t0()).unwrap();
    assert_eq!(
        (applied(&r).applied_seconds, applied(&r).limited_by),
        (0, Some("daily_limit"))
    );
    // Nothing changed on the clock: recorded in the ledger, nothing in the public feed.
    assert_eq!(ledger(&fx).len(), 1);
    assert_eq!(s.activity.len(), activity_before);
}

#[test]
fn donations_can_run_the_clock_out_and_bring_it_back() {
    let mut s = state(t0());
    start(&mut s, "cuaco", t0());
    let i = s.idx("cuaco").unwrap();
    s.event.donation_time.max_seconds_per_donation = DonationTimePolicy::MAX_SECONDS;
    s.event.donation_time.max_removed_seconds_per_day = DonationTimePolicy::MAX_SECONDS;

    let (fx, r) = donate(&mut s, "cuaco", "all", -20_000, bits(99_999), t0()).unwrap();
    assert_eq!(applied(&r).applied_seconds, -14_400);
    assert_eq!(applied(&r).limited_by, Some("clock_zero"));
    assert_eq!(s.view(i, t0()).status, RacerStatus::Exhausted);
    assert!(
        fx.down
            .iter()
            .any(|(id, d)| id == "cuaco" && *d == zeldathon_server::state::IngestDown::ForceClose)
    );

    donate(&mut s, "cuaco", "back", 300, bits(500), t0()).unwrap();
    let v = s.view(i, t0());
    assert_eq!(v.remaining_seconds, 300);
    assert_ne!(v.status, RacerStatus::Exhausted);
}

#[test]
fn donations_are_validated() {
    let mut s = state(t0());
    start(&mut s, "cuaco", t0());
    let no_id = s.apply_ingest(
        "cuaco",
        env(IngestMsg::TimeDonation {
            delta_seconds: 60,
            source: bits(100),
        }),
        t0(),
    );
    assert_eq!(no_id.unwrap_err().code(), "invalid");
    let mut wrong = bits(100);
    wrong.currency = DonationCurrency::Diamonds;
    assert_eq!(
        donate(&mut s, "cuaco", "w", 60, wrong, t0())
            .unwrap_err()
            .code(),
        "invalid"
    );
    assert_eq!(
        donate(&mut s, "cuaco", "z", 0, bits(100), t0())
            .unwrap_err()
            .code(),
        "invalid"
    );
    assert_eq!(
        donate(&mut s, "cuaco", "n", 60, bits(0), t0())
            .unwrap_err()
            .code(),
        "invalid"
    );

    let before = Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap();
    let mut early = state(before);
    assert_eq!(
        donate(&mut early, "cuaco", "x", 60, bits(100), before).unwrap_err(),
        IngestError::EventNotLive
    );
}

#[test]
fn the_daily_reset_restores_the_donation_allowance() {
    let mut s = state(t0());
    start(&mut s, "cuaco", t0());
    s.event.donation_time.max_added_seconds_per_day = 60;
    donate(&mut s, "cuaco", "a", 60, bits(100), t0()).unwrap();
    let reset = Utc.with_ymd_and_hms(2026, 10, 8, 12, 0, 1).unwrap();
    s.tick(reset, i64::MAX);
    let (_, r) = donate(&mut s, "cuaco", "b", 60, bits(100), reset).unwrap();
    assert_eq!(applied(&r).applied_seconds, 60);
}

#[test]
fn the_donation_policy_is_edited_with_the_event_and_validated() {
    let mut s = state(t0());
    let policy = DonationTimePolicy {
        allow_add: false,
        max_seconds_per_donation: 300,
        ..DonationTimePolicy::default()
    };
    s.update_event(EventPatch {
        donation_time: Some(policy.clone()),
        ..EventPatch::default()
    })
    .unwrap();
    assert_eq!(s.event.donation_time, policy);
    let bad = DonationTimePolicy {
        max_seconds_per_donation: 0,
        ..DonationTimePolicy::default()
    };
    assert!(
        s.update_event(EventPatch {
            donation_time: Some(bad),
            ..EventPatch::default()
        })
        .is_err()
    );
}

// ---- notices for Discord -------------------------------------------------------------------------

fn notices(fx: &Fx) -> Vec<&Notice> {
    fx.notices.iter().collect()
}

#[test]
fn a_boss_makes_one_keyed_notice_with_the_running_count() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    let boss = |name: &str| IngestMsg::BossDefeated { boss: name.into() };
    let (fx, _) = ingest(&mut s, "ralbat", boss("gohma"), t0()).unwrap();
    let n = notices(&fx);
    assert_eq!(n.len(), 1);
    assert_eq!(
        n[0].detail,
        Detail::Boss {
            boss: "gohma".into(),
            count: Some(1)
        }
    );
    assert_eq!(n[0].dedupe.as_deref(), Some("boss:ralbat:gohma"));
    assert_eq!(n[0].racer.as_ref().unwrap().name, "Ralbat");
    let (fx, _) = ingest(&mut s, "ralbat", boss("morpha"), t0()).unwrap();
    assert_eq!(
        notices(&fx)[0].detail,
        Detail::Boss {
            boss: "morpha".into(),
            count: Some(2)
        }
    );
}

#[test]
fn finishing_places_each_racer_and_keys_the_notice_per_racer() {
    let mut s = state(t0());
    let all = zeldathon_server::catalog::default_catalog().default_required();
    let mut places = vec![];
    for (k, id) in ["pinchiviejo", "xime"].into_iter().enumerate() {
        start(&mut s, id, t0());
        let patch = GameProgressPatch {
            completed_objectives: Some(all.clone()),
            ..Default::default()
        };
        ingest(
            &mut s,
            id,
            IngestMsg::GameProgress { progress: patch },
            t0(),
        )
        .unwrap();
        let at = t0() + Duration::seconds(60 * (k as i64 + 1));
        let (fx, _) = ingest(&mut s, id, IngestMsg::GameFinished, at).unwrap();
        let n = notices(&fx);
        assert_eq!(n.len(), 1);
        assert_eq!(
            n[0].dedupe.as_deref(),
            Some(format!("finish:{id}").as_str())
        );
        if let Detail::Winner {
            place,
            final_seconds,
        } = &n[0].detail
        {
            places.push((*place, *final_seconds));
        }
    }
    assert_eq!(places, [(1, Some(60)), (2, Some(120))]);
}

#[test]
fn running_out_of_time_notifies_once_per_day_of_that_racer() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    let end = t0() + Duration::hours(5); // past the 4 h budget
    let fx = s.tick(end, 20);
    let n = notices(&fx);
    let exhausted: Vec<_> = n
        .iter()
        .filter(|n| matches!(n.detail, Detail::Exhausted { .. }))
        .collect();
    assert_eq!(exhausted.len(), 1);
    let key = exhausted[0].dedupe.clone().unwrap();
    assert!(key.starts_with("exhausted:ralbat:"), "{key}");
}

#[test]
fn a_big_quick_rise_in_progress_is_reported_as_a_possible_jump() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    let pct = |p: f64| IngestMsg::GameProgress {
        progress: GameProgressPatch {
            percentage: Some(p),
            ..Default::default()
        },
    };
    let jumps = |fx: &Fx| -> Vec<(f64, f64, i64)> {
        fx.notices
            .iter()
            .filter_map(|n| match n.detail {
                Detail::Jump { from, to, seconds } => Some((from, to, seconds)),
                _ => None,
            })
            .collect()
    };
    let (fx, _) = ingest(&mut s, "ralbat", pct(10.0), t0()).unwrap();
    assert!(jumps(&fx).is_empty(), "the first value is only a reference");
    let (fx, _) = ingest(&mut s, "ralbat", pct(13.0), t0() + Duration::seconds(20)).unwrap();
    assert!(jumps(&fx).is_empty(), "a normal step");
    let (fx, _) = ingest(&mut s, "ralbat", pct(60.0), t0() + Duration::seconds(50)).unwrap();
    assert_eq!(
        jumps(&fx),
        [(10.0, 60.0, 50)],
        "measured from the lowest recent value"
    );
}

#[test]
fn hitting_the_daily_donation_cap_notifies_the_referees_once_per_kind_and_day() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    s.update_event(EventPatch {
        donation_time: Some(DonationTimePolicy {
            max_removed_seconds_per_day: 100,
            ..DonationTimePolicy::default()
        }),
        ..Default::default()
    })
    .unwrap();
    let donate = |id: &str, secs: i64| IngestEnvelope {
        id: Some(id.into()),
        msg: IngestMsg::TimeDonation {
            delta_seconds: secs,
            source: DonationSource {
                platform: Platform::Twitch,
                currency: DonationCurrency::Bits,
                amount: 10,
                gift: None,
                gift_count: None,
                viewer: Some("fan".into()),
            },
        },
    };
    let (fx, _) = s.apply_ingest("ralbat", donate("d1", -60), t0()).unwrap();
    assert!(
        notices(&fx)
            .iter()
            .all(|n| !matches!(n.detail, Detail::DonationCap { .. }))
    );
    let (fx, _) = s.apply_ingest("ralbat", donate("d2", -60), t0()).unwrap(); // only 40 s left
    let cap: Vec<_> = notices(&fx)
        .into_iter()
        .filter(|n| matches!(n.detail, Detail::DonationCap { .. }))
        .collect();
    assert_eq!(cap.len(), 1);
    assert_eq!(
        cap[0].detail,
        Detail::DonationCap {
            adding: false,
            limit_seconds: 100,
            viewer: Some("fan".into())
        }
    );
    assert!(
        cap[0]
            .dedupe
            .as_deref()
            .unwrap()
            .starts_with("cap:ralbat:remove:")
    );
}

// ---- the organizer's rate per diamond / bit ------------------------------------------------------

fn with_rates(s: &mut RaceState, diamond: Option<i64>, bit: Option<i64>) {
    s.update_event(EventPatch {
        donation_time: Some(DonationTimePolicy {
            seconds_per_diamond: diamond,
            seconds_per_bit: bit,
            ..DonationTimePolicy::default()
        }),
        ..Default::default()
    })
    .unwrap();
}

#[test]
fn by_default_a_diamond_is_worth_three_seconds_and_hiveshock_only_picks_the_direction() {
    assert_eq!(DonationTimePolicy::default().seconds_per_diamond, Some(3));
    assert_eq!(DonationTimePolicy::default().seconds_per_bit, None);
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    // The 1-diamond Rose that HiveShock priced at 1 h 28 min is 3 seconds.
    let (fx, reply) = donate(
        &mut s,
        "ralbat",
        "rose",
        -5280,
        diamonds("Rose", 1, 1),
        t0(),
    )
    .unwrap();
    assert_eq!(applied(&reply).requested_seconds, -3);
    assert_eq!(applied(&reply).applied_seconds, -3);
    assert_eq!(applied(&reply).limited_by, None);
    let row = ledger(&fx)[0];
    assert_eq!(
        (row.requested_ms, row.applied_ms, row.reported_ms),
        (-3_000, -3_000, Some(-5_280_000))
    );
    // A big gift scales with its diamonds (then the organizer's per-donation cap applies).
    let (_, reply) = donate(
        &mut s,
        "ralbat",
        "whale",
        1,
        diamonds("Whale diving", 1, 2150),
        t0(),
    )
    .unwrap();
    let t = applied(&reply);
    assert_eq!(
        (t.requested_seconds, t.applied_seconds, t.limited_by),
        (6450, 3600, Some("per_donation"))
    );
    // The direction is the only thing taken from HiveShock.
    let (_, reply) = donate(
        &mut s,
        "ralbat",
        "up",
        99_999,
        diamonds("Heart Me", 1, 10),
        t0(),
    )
    .unwrap();
    assert_eq!(applied(&reply).requested_seconds, 30);
    assert_eq!(
        donate(
            &mut s,
            "ralbat",
            "zero",
            0,
            diamonds("Heart Me", 1, 10),
            t0()
        )
        .unwrap_err()
        .code(),
        "invalid"
    );
}

#[test]
fn rates_can_be_set_per_currency_or_left_to_hiveshock() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    // Bits at 2 s each, diamonds back to HiveShock's own number.
    with_rates(&mut s, None, Some(2));
    let (_, r) = donate(&mut s, "ralbat", "b", 10, bits(50), t0()).unwrap();
    assert_eq!(applied(&r).requested_seconds, 100);
    let (_, r) = donate(&mut s, "ralbat", "d", -45, diamonds("Rose", 1, 99), t0()).unwrap();
    assert_eq!(applied(&r).requested_seconds, -45);
    // Both off: HiveShock decides everything, as before this feature existed.
    with_rates(&mut s, None, None);
    let (_, r) = donate(&mut s, "ralbat", "b2", 77, bits(50), t0()).unwrap();
    assert_eq!(applied(&r).requested_seconds, 77);
    assert_eq!(
        donate(&mut s, "ralbat", "huge", 999_999_999, bits(1), t0())
            .unwrap_err()
            .code(),
        "invalid"
    );
}

#[test]
fn the_rate_is_validated() {
    for bad in [0, -1, 3601] {
        let p = DonationTimePolicy {
            seconds_per_diamond: Some(bad),
            ..DonationTimePolicy::default()
        };
        assert!(p.validate().is_err(), "{bad}");
        let p = DonationTimePolicy {
            seconds_per_bit: Some(bad),
            ..DonationTimePolicy::default()
        };
        assert!(p.validate().is_err(), "{bad}");
    }
    assert!(
        DonationTimePolicy {
            seconds_per_diamond: Some(3600),
            ..DonationTimePolicy::default()
        }
        .validate()
        .is_ok()
    );
    // An older stored policy (without the fields) picks up the defaults.
    let old: DonationTimePolicy =
        serde_json::from_str(r#"{"enabled":true,"maxSecondsPerDonation":600}"#).unwrap();
    assert_eq!(
        (old.seconds_per_diamond, old.max_seconds_per_donation),
        (Some(3), 600)
    );
    let off: DonationTimePolicy = serde_json::from_str(r#"{"secondsPerDiamond":null}"#).unwrap();
    assert_eq!(off.seconds_per_diamond, None);
}

// ---- time really played --------------------------------------------------------------------------

#[test]
fn played_time_counts_only_the_running_game_and_ignores_donations_and_adjustments() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    let i = s.idx("ralbat").unwrap();
    let at = |secs: i64| t0() + Duration::seconds(secs);
    // 10 minutes of play; the live view already counts the stretch that is still running.
    let v = s.view(i, at(600));
    assert_eq!((v.played_today_seconds, v.played_seconds), (600, 600));
    let c = s.clock(i, at(600));
    assert_eq!((c.played_today_ms, c.played_total_ms), (600_000, 600_000));
    // Donations and organizer adjustments move the clock, never the played time.
    donate(&mut s, "ralbat", "d", -1, diamonds("Rose", 1, 100), at(600)).unwrap(); // -300 s
    s.admin_action(
        "ralbat",
        AdminAction::AdjustTime {
            delta_seconds: -900,
        },
        at(600),
    )
    .unwrap();
    let v = s.view(i, at(600));
    assert_eq!(v.played_today_seconds, 600);
    assert!(
        v.elapsed_seconds > 600,
        "elapsed (budget - left) is what they distort"
    );
    // Paused time is not played time.
    ingest(&mut s, "ralbat", IngestMsg::SessionPaused, at(600)).unwrap();
    assert_eq!(s.view(i, at(5000)).played_today_seconds, 600);
    ingest(&mut s, "ralbat", IngestMsg::SessionResumed, at(5000)).unwrap();
    assert_eq!(s.view(i, at(5060)).played_today_seconds, 660);
}

#[test]
fn a_daily_reset_starts_a_new_day_of_play_but_keeps_the_total() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    let i = s.idx("ralbat").unwrap();
    let noon = t0() + Duration::seconds(1800);
    s.admin_action("ralbat", AdminAction::ResetDay, noon)
        .unwrap();
    let v = s.view(i, noon);
    assert_eq!((v.played_today_seconds, v.played_seconds), (0, 1800));
    let later = noon + Duration::seconds(60);
    let v = s.view(i, later);
    assert_eq!((v.played_today_seconds, v.played_seconds), (60, 1860));
}

#[test]
fn closing_the_game_or_running_out_of_time_says_how_long_they_played() {
    // Running out of time: the feed and the notice carry the day's played time.
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    let end = t0() + Duration::hours(5);
    let fx = s.tick(end, 20);
    let note = s
        .activity
        .iter()
        .find(|a| a.code == "SESSION_EXHAUSTED")
        .unwrap();
    assert_eq!(note.subject.as_deref(), Some("04:00:00"));
    let n = fx
        .notices
        .iter()
        .find(|n| matches!(n.detail, Detail::Exhausted { .. }))
        .unwrap();
    assert_eq!(
        n.detail,
        Detail::Exhausted {
            played_seconds: 14_400
        }
    );

    // An organizer closing a running game: same, as its own feed entry.
    let mut s = state(t0());
    start(&mut s, "xime", t0());
    let at = t0() + Duration::seconds(95);
    s.admin_action("xime", AdminAction::ForceClose, at).unwrap();
    let note = s.activity.iter().find(|a| a.code == "GAME_CLOSED").unwrap();
    assert_eq!(note.subject.as_deref(), Some("00:01:35"));
    // Closing a game that is not running says nothing.
    s.admin_action("xime", AdminAction::ForceClose, at).unwrap();
    assert_eq!(
        s.activity
            .iter()
            .filter(|a| a.code == "GAME_CLOSED")
            .count(),
        1
    );
}

// ---- ranking milestone ---------------------------------------------------------------------------

#[test]
fn the_milestone_moves_only_when_the_count_of_required_objectives_changes() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    let i = s.idx("ralbat").unwrap();
    let done = |ids: &[&str]| IngestMsg::GameProgress {
        progress: GameProgressPatch {
            completed_objectives: Some(ids.iter().map(|s| s.to_string()).collect()),
            ..Default::default()
        },
    };
    let at = |secs: i64| t0() + Duration::seconds(secs);
    assert!(s.view(i, at(0)).milestone_at_utc.is_none());
    // Reaching a first required objective sets it.
    ingest(&mut s, "ralbat", done(&["kokiri-forest"]), at(10)).unwrap();
    let first = s.view(i, at(10)).milestone_at_utc.unwrap();
    // Re-reporting the same list, or only progress, does not move it...
    ingest(&mut s, "ralbat", done(&["kokiri-forest"]), at(20)).unwrap();
    ingest(
        &mut s,
        "ralbat",
        IngestMsg::GameProgress {
            progress: GameProgressPatch {
                percentage: Some(33.0),
                ..Default::default()
            },
        },
        at(25),
    )
    .unwrap();
    assert_eq!(s.view(i, at(30)).milestone_at_utc.unwrap(), first);
    // ...and neither does an objective the event does not require.
    let extra = zeldathon_server::catalog::default_catalog()
        .objectives
        .iter()
        .map(|o| o.id.clone())
        .find(|id| !s.event.rules.required_objective_ids.contains(id));
    if let Some(extra) = extra {
        ingest(&mut s, "ralbat", done(&["kokiri-forest", &extra]), at(40)).unwrap();
        assert_eq!(s.view(i, at(40)).milestone_at_utc.unwrap(), first);
    }
    // A second required objective moves it.
    ingest(
        &mut s,
        "ralbat",
        done(&["kokiri-forest", "deku-tree"]),
        at(50),
    )
    .unwrap();
    assert_ne!(s.view(i, at(50)).milestone_at_utc.unwrap(), first);
}

#[test]
fn the_milestone_is_saved_and_a_reset_clears_it() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    let patch = GameProgressPatch {
        completed_objectives: Some(vec!["kokiri-forest".into()]),
        ..Default::default()
    };
    let (fx, _) = ingest(
        &mut s,
        "ralbat",
        IngestMsg::GameProgress { progress: patch },
        t0(),
    )
    .unwrap();
    let saved = fx
        .ops
        .iter()
        .find_map(|op| match op {
            zeldathon_server::db::PersistOp::RacerState(row) => row.milestone_at.clone(),
            _ => None,
        })
        .expect("the racer row carries the milestone");
    assert!(saved.starts_with("2026-10-07T13:00:00"));
    s.event.rehearsal = true;
    s.reset_event(Some("2026-12-01T12:00:00Z".into()), true, t0())
        .unwrap();
    let i = s.idx("ralbat").unwrap();
    assert!(s.view(i, t0()).milestone_at_utc.is_none());
}

// ---- statistics by day ---------------------------------------------------------------------------

/// The day figures an `Fx` hands to the queue, merged per day.
fn days(fx: &[&Fx]) -> std::collections::BTreeMap<String, DayDelta> {
    // The race has nine racers and a tick touches all of them: these tests follow ralbat.
    let mut out: std::collections::BTreeMap<String, DayDelta> = Default::default();
    for fx in fx {
        for op in &fx.ops {
            if let PersistOp::DayStat(d) = op
                && d.racer_id == "ralbat"
            {
                let e = out
                    .entry(d.day.clone())
                    .or_insert_with(|| DayDelta::new(&d.racer_id, &d.day));
                e.played_ms += d.played_ms;
                e.sessions += d.sessions;
                e.objectives += d.objectives;
                e.items += d.items;
                e.bosses += d.bosses;
                e.areas += d.areas;
                e.donations += d.donations;
                e.donation_added_ms += d.donation_added_ms;
                e.donation_removed_ms += d.donation_removed_ms;
                e.donation_capped += d.donation_capped;
                e.diamonds += d.diamonds;
                e.bits += d.bits;
                e.adjust_ms += d.adjust_ms;
                e.exhausted += d.exhausted;
                e.force_closed += d.force_closed;
                e.progress_start = e.progress_start.or(d.progress_start);
                e.progress_end = d.progress_end.or(e.progress_end);
                e.peak_viewers = e.peak_viewers.max(d.peak_viewers);
            }
        }
    }
    out
}

#[test]
fn everything_a_racer_does_lands_on_the_day_it_happened() {
    let mut s = state(t0());
    let at = |secs: i64| t0() + Duration::seconds(secs);
    let (a, _) = ingest(
        &mut s,
        "ralbat",
        IngestMsg::Hello {
            client_version: None,
        },
        t0(),
    )
    .unwrap();
    let (b, _) = ingest(&mut s, "ralbat", IngestMsg::SessionStarted, t0()).unwrap();
    let (c, _) = ingest(
        &mut s,
        "ralbat",
        IngestMsg::ItemAcquired {
            item: "longshot".into(),
        },
        at(10),
    )
    .unwrap();
    let (d, _) = ingest(
        &mut s,
        "ralbat",
        IngestMsg::BossDefeated {
            boss: "gohma".into(),
        },
        at(20),
    )
    .unwrap();
    let progress = IngestMsg::GameProgress {
        progress: GameProgressPatch {
            percentage: Some(12.0),
            current_area: Some("forest-temple".into()),
            completed_objectives: Some(vec!["kokiri-forest".into(), "deku-tree".into()]),
            ..Default::default()
        },
    };
    let (e, _) = ingest(&mut s, "ralbat", progress, at(30)).unwrap();
    let (f, _) = ingest(
        &mut s,
        "ralbat",
        IngestMsg::StreamState {
            live: true,
            viewers: Some(80),
        },
        at(35),
    )
    .unwrap();
    let (g, _) = ingest(
        &mut s,
        "ralbat",
        IngestMsg::StreamState {
            live: true,
            viewers: Some(50),
        },
        at(36),
    )
    .unwrap();
    // 3 diamonds at the default 3 s each remove 9 s; a bits donation adds 60 s.
    let (h, _) = donate(&mut s, "ralbat", "d1", -1, diamonds("Rose", 1, 3), at(40)).unwrap();
    let (i, _) = donate(&mut s, "ralbat", "d2", 60, bits(100), at(41)).unwrap();
    let j = s
        .admin_action(
            "ralbat",
            AdminAction::AdjustTime {
                delta_seconds: -120,
            },
            at(50),
        )
        .unwrap();
    let k = s
        .admin_action("ralbat", AdminAction::ForceClose, at(60))
        .unwrap();

    let all = days(&[&a, &b, &c, &d, &e, &f, &g, &h, &i, &j, &k]);
    assert_eq!(all.len(), 1, "one game day so far");
    let day = all.values().next().unwrap();
    assert_eq!(day.racer_id, "ralbat");
    assert_eq!(
        day.played_ms, 60_000,
        "played from the session start to the close"
    );
    assert_eq!(
        (
            day.sessions,
            day.items,
            day.bosses,
            day.areas,
            day.objectives
        ),
        (1, 1, 1, 1, 2)
    );
    assert_eq!((day.donations, day.diamonds, day.bits), (2, 3, 100));
    assert_eq!(
        (day.donation_removed_ms, day.donation_added_ms),
        (9_000, 60_000)
    );
    assert_eq!(day.adjust_ms, -120_000);
    assert_eq!((day.force_closed, day.exhausted), (1, 0));
    assert_eq!(
        (day.progress_start, day.progress_end),
        (Some(0.0), Some(12.0))
    );
    assert_eq!(day.peak_viewers, Some(80), "the peak, not the last value");
    // The day is named by the local date it began on (Mexico City, reset 06:00).
    assert_eq!(all.keys().next().unwrap(), "2026-10-07");
}

#[test]
fn a_daily_reset_closes_the_day_and_the_next_one_starts_where_it_ended() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    let patch = IngestMsg::GameProgress {
        progress: GameProgressPatch {
            percentage: Some(40.0),
            ..Default::default()
        },
    };
    let (p, _) = ingest(&mut s, "ralbat", patch, t0() + Duration::seconds(100)).unwrap();
    // Past 06:00 Mexico City the next morning (12:00 UTC): the tick resets the day.
    let next_morning = Utc.with_ymd_and_hms(2026, 10, 8, 12, 5, 0).unwrap();
    let reset = s.tick(next_morning, 1_000_000);
    let later = ingest(
        &mut s,
        "ralbat",
        IngestMsg::BossDefeated {
            boss: "morpha".into(),
        },
        next_morning + Duration::seconds(5),
    )
    .map(|(fx, _)| fx);
    // After the reset the racer's game is no longer running (no heartbeat): the boss is refused or
    // lands on the new day; either way nothing leaks into the old one.
    let all = days(&[&p, &reset]);
    let first = &all["2026-10-07"];
    assert!(first.played_ms > 0);
    assert_eq!(first.progress_end, Some(40.0));
    let second = &all["2026-10-08"];
    assert_eq!(
        (second.progress_start, second.progress_end),
        (Some(40.0), Some(40.0))
    );
    assert_eq!(second.played_ms, 0);
    let _ = later;
}

#[test]
fn running_out_of_time_and_the_event_reset_are_counted_and_cleared() {
    let mut s = state(t0());
    start(&mut s, "ralbat", t0());
    let fx = s.tick(t0() + Duration::hours(5), 1_000_000);
    let day = &days(&[&fx])["2026-10-07"];
    assert_eq!(day.exhausted, 1);
    assert_eq!(
        day.played_ms,
        4 * 3_600_000,
        "played exactly the daily budget"
    );
    // A rehearsal reset leaves nothing pending for the old run.
    s.event.rehearsal = true;
    let fx = s
        .reset_event(Some("2026-12-01T12:00:00Z".into()), true, t0())
        .unwrap();
    assert!(days(&[&fx]).is_empty(), "the reset forgets the day figures");
    assert!(
        fx.ops
            .iter()
            .any(|op| matches!(op, PersistOp::ClearRaceData))
    );
}
