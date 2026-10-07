use chrono::{DateTime, Duration, TimeZone, Utc};
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
    assert!(!s.view(s.idx("ralbat").unwrap(), t0()).stream.unwrap().is_live);
}

#[test]
fn a_lost_heartbeat_and_a_restart_both_clear_a_reported_broadcast() {
    let mut s = state(t0());
    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    s.attach_ingest(
        "ralbat",
        zeldathon_server::state::IngestHandle { conn_id: 1, tx },
    );
    ingest(&mut s, "ralbat", IngestMsg::Heartbeat { game_running: None }, t0()).unwrap();
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
        let stream = s.racers[i].racer.stream.get_or_insert_with(Default::default);
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

    let (_, reply) = donate(&mut s, "ralbat", "d2", -30, diamonds("Rose", 5, 5), t0()).unwrap();
    assert_eq!(applied(&reply).applied_seconds, -30);
    assert_eq!(s.activity[0].code, "TIME_REMOVED");
    assert_eq!(
        s.activity[0].detail.as_deref(),
        Some("-00:00:30 · ROSE X5 · 5 DIAMONDS")
    );
    assert_eq!(s.clock(i, t0()).remaining_ms, 14_400_000 + 60_000);
    assert_eq!(
        (
            s.racers[i].donation_added_ms,
            s.racers[i].donation_removed_ms
        ),
        (90_000, 30_000)
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
    let exhausted: Vec<_> = n.iter().filter(|n| n.detail == Detail::Exhausted).collect();
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
