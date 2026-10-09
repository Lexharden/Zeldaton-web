//! The day-by-day statistics: rebuilt once from the logs of a race that already has history, and the
//! endpoints that serve them.

use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;
use zeldathon_server::db::{self, PersistOp, TimeDonationRow};
use zeldathon_server::domain::*;
use zeldathon_server::seed;

async fn db_with_history() -> sqlx::SqlitePool {
    let pool = db::connect("sqlite::memory:").await.unwrap();
    db::migrate(&pool).await.unwrap();
    let seeded = seed::build(Utc.with_ymd_and_hms(2026, 10, 7, 12, 0, 0).unwrap());
    for op in seeded.ops {
        db::apply(&pool, op).await.unwrap();
    }
    pool
}

fn activity(id: &str, ts: &str, racer: &str, code: &str) -> PersistOp {
    PersistOp::Activity(ActivityItem {
        id: id.into(),
        timestamp_utc: ts.into(),
        kind: ActivityKind::Item,
        racer_id: Some(racer.into()),
        racer_name: Some(racer.into()),
        message: code.into(),
        code: code.into(),
        detail: None,
        subject: None,
    })
}

#[tokio::test]
async fn days_before_the_feature_are_rebuilt_once_from_the_logs() {
    let pool = db_with_history().await;
    // Mexico City resets at 06:00 (UTC-6): 12:00Z. 14:00Z on the 7th and 03:00Z on the 8th (21:00
    // local) are both the day that began on the 7th; 13:00Z on the 8th starts the day of the 8th.
    for (id, ts, code) in [
        ("a1", "2026-10-07T14:00:00.000Z", "ITEM_ACQUIRED"),
        ("a2", "2026-10-08T03:00:00.000Z", "BOSS_DEFEATED"),
        ("a3", "2026-10-08T03:30:00.000Z", "AREA_CHANGED"),
        ("a4", "2026-10-08T13:00:00.000Z", "ITEM_ACQUIRED"),
        ("a5", "2026-10-08T13:10:00.000Z", "SESSION_EXHAUSTED"),
    ] {
        db::apply(&pool, activity(id, ts, "ralbat", code))
            .await
            .unwrap();
    }
    for (id, ts, applied, cur, amount, limited) in [
        (
            "d1",
            "2026-10-07T15:00:00Z",
            -9_000,
            DonationCurrency::Diamonds,
            3,
            None,
        ),
        (
            "d2",
            "2026-10-07T16:00:00Z",
            60_000,
            DonationCurrency::Bits,
            100,
            None,
        ),
        (
            "d3",
            "2026-10-08T14:00:00Z",
            -255_000,
            DonationCurrency::Diamonds,
            2,
            Some("daily_limit".to_string()),
        ),
    ] {
        db::apply(
            &pool,
            PersistOp::TimeDonation(Box::new(TimeDonationRow {
                ts: DateTime::parse_from_rfc3339(ts)
                    .unwrap()
                    .with_timezone(&Utc),
                racer_id: "ralbat".into(),
                client_id: id.into(),
                platform: if cur == DonationCurrency::Bits {
                    Platform::Twitch
                } else {
                    Platform::Tiktok
                },
                currency: cur,
                amount,
                gift: None,
                gift_count: None,
                viewer: None,
                requested_ms: applied,
                reported_ms: Some(applied),
                applied_ms: applied,
                limited_by: limited,
            })),
        )
        .await
        .unwrap();
    }
    db::apply(
        &pool,
        PersistOp::Audit(db::AuditRow {
            ts: Utc.with_ymd_and_hms(2026, 10, 8, 15, 0, 0).unwrap(),
            actor: "ana".into(),
            action: "racer.adjust-time".into(),
            racer_id: Some("ralbat".into()),
            payload: json!({ "deltaSeconds": 300, "reason": "lag" }),
        }),
    )
    .await
    .unwrap();

    let written = db::backfill_days(&pool).await.unwrap();
    assert_eq!(written, 2, "two days with something to say");
    let rows = db::racer_days(&pool, Some("ralbat")).await.unwrap();
    let (d7, d8) = (&rows[0], &rows[1]);
    assert_eq!(
        (d7["day"].as_str(), d8["day"].as_str()),
        (Some("2026-10-07"), Some("2026-10-08"))
    );
    assert_eq!(
        (
            d7["items"].clone(),
            d7["bosses"].clone(),
            d7["areas"].clone()
        ),
        (json!(1), json!(1), json!(1))
    );
    assert_eq!(
        (
            d7["donations"].clone(),
            d7["diamonds"].clone(),
            d7["bits"].clone()
        ),
        (json!(2), json!(3), json!(100))
    );
    assert_eq!(
        (
            d7["donationRemovedSeconds"].clone(),
            d7["donationAddedSeconds"].clone()
        ),
        (json!(9), json!(60))
    );
    assert_eq!(
        (
            d8["items"].clone(),
            d8["exhausted"].clone(),
            d8["donationCapped"].clone()
        ),
        (json!(1), json!(1), json!(1))
    );
    assert_eq!(d8["adjustSeconds"], 300);
    // Honest about what could not be recovered.
    assert_eq!(
        (d7["partial"].clone(), d8["partial"].clone()),
        (json!(true), json!(true))
    );
    assert_eq!(d7["playedSeconds"], 0);
    // Other racers have nothing to show.
    assert!(
        db::racer_days(&pool, Some("xime"))
            .await
            .unwrap()
            .is_empty()
    );

    // Once the table has rows the rebuild never runs again (it would double count).
    assert_eq!(db::backfill_days(&pool).await.unwrap(), 0);
    assert_eq!(
        db::racer_days(&pool, Some("ralbat")).await.unwrap().len(),
        2
    );
}

#[tokio::test]
async fn day_figures_add_up_in_the_database() {
    let pool = db_with_history().await;
    let mut a = db::DayDelta::new("ralbat", "2026-10-07");
    a.played_ms = 60_000;
    a.items = 1;
    a.progress_start = Some(10.0);
    a.progress_end = Some(20.0);
    a.peak_viewers = Some(40);
    let mut b = db::DayDelta::new("ralbat", "2026-10-07");
    b.played_ms = 30_000;
    b.items = 2;
    b.progress_start = Some(99.0); // the first start of the day wins
    b.progress_end = Some(35.0); // the last end wins
    b.peak_viewers = Some(25); // the highest wins
    for d in [a, b] {
        db::apply(&pool, PersistOp::DayStat(Box::new(d)))
            .await
            .unwrap();
    }
    let rows = db::racer_days(&pool, Some("ralbat")).await.unwrap();
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!(
        (r["playedSeconds"].clone(), r["items"].clone()),
        (json!(90), json!(3))
    );
    assert_eq!(
        (
            r["progressStart"].clone(),
            r["progressEnd"].clone(),
            r["peakViewers"].clone()
        ),
        (json!(10.0), json!(35.0), json!(40))
    );
    assert_eq!(r["partial"], false);
}
