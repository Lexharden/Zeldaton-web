//! Incidents: what the referees should look at, kept in the database.
//!
//! The scanner and the engine already detect the interesting things (a long disconnection, low
//! time, a suspicious progress jump, a donation cap, a racer out of time) to tell Discord. Here the
//! same notices are also stored, whether or not Discord is on, so a referee who joins late finds
//! them in the monitor, marks them reviewed or dismissed, and leaves a note. The condition and the
//! review are independent: `ended_at` says the problem went away (the racer reconnected), `status`
//! says a person looked at it.

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};

use crate::db::rfc;
use crate::notify::settings::Thresholds;
use crate::notify::text::hms;
use crate::notify::{Detail, Notice};

/// The same kind for the same racer is not opened again while one is still open, nor within this
/// many seconds of the last one (a flapping connection is one incident, not twenty).
const REOPEN_AFTER_SECS: i64 = 300;

/// An incident before it gets an id.
#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub kind: &'static str,
    pub severity: &'static str,
    pub message: String,
    pub payload: Value,
}

/// What a notice means for the monitor, or `None` when it is not an incident (a boss defeated, a
/// organizer action - those are in the audit log - and so on).
pub fn describe(n: &Notice, t: &Thresholds) -> Option<Draft> {
    let name = n
        .racer
        .as_ref()
        .map(|r| r.name.as_str())
        .unwrap_or("Un corredor");
    let d = match &n.detail {
        Detail::Disconnected {
            minutes,
            back: false,
        } => Draft {
            kind: "disconnected",
            severity: "error",
            message: format!("{name} lleva {minutes} min sin señal de HiveShock."),
            payload: json!({ "minutes": minutes }),
        },
        Detail::LowTime { minutes_left } => Draft {
            kind: "low_time",
            severity: "warn",
            message: format!("A {name} le quedan {minutes_left} min hoy."),
            payload: json!({ "minutesLeft": minutes_left }),
        },
        Detail::Jump { from, to, seconds } => {
            // The same thresholds the organizer set for Discord.
            if to - from < t.jump_percent || *seconds > t.jump_window_seconds {
                return None;
            }
            Draft {
                kind: "suspicious",
                severity: "error",
                message: format!(
                    "{name} pasó de {from:.0}% a {to:.0}% en {} (salto brusco).",
                    hms(*seconds)
                ),
                payload: json!({ "from": from, "to": to, "seconds": seconds }),
            }
        }
        Detail::DonationCap {
            adding,
            limit_seconds,
            viewer,
        } => Draft {
            kind: "donation_cap",
            severity: "info",
            message: format!(
                "{name} alcanzó el tope diario de {} por donaciones ({}).",
                if *adding {
                    "tiempo añadido"
                } else {
                    "tiempo quitado"
                },
                hms(*limit_seconds)
            ),
            payload: json!({ "adding": adding, "limitSeconds": limit_seconds, "viewer": viewer }),
        },
        Detail::Exhausted => Draft {
            kind: "exhausted",
            severity: "info",
            message: format!("{name} se quedó sin tiempo por hoy."),
            payload: json!({}),
        },
        _ => return None,
    };
    Some(d)
}

/// Stores the notice as an incident when it is one, and closes the open disconnection when the
/// racer is back. Errors are the caller's to log: a failed insert must never stop Discord.
pub async fn record(
    pool: &SqlitePool,
    n: &Notice,
    t: &Thresholds,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let racer_id = n.racer.as_ref().map(|r| r.id.as_str());
    if matches!(n.detail, Detail::Disconnected { back: true, .. }) {
        sqlx::query(
            "UPDATE incidents SET ended_at = ? WHERE kind = 'disconnected' AND racer_id IS ? AND ended_at IS NULL",
        )
        .bind(rfc(now))
        .bind(racer_id)
        .execute(pool)
        .await?;
        return Ok(());
    }
    let Some(draft) = describe(n, t) else {
        return Ok(());
    };
    let recent: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM incidents WHERE kind = ? AND racer_id IS ? AND (status = 'open' OR ts > ?) LIMIT 1",
    )
    .bind(draft.kind)
    .bind(racer_id)
    .bind(rfc(now - chrono::Duration::seconds(REOPEN_AFTER_SECS)))
    .fetch_optional(pool)
    .await?;
    if recent.is_some() {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO incidents (ts, kind, severity, racer_id, racer_name, message, payload) VALUES (?,?,?,?,?,?,?)",
    )
    .bind(rfc(n.at))
    .bind(draft.kind)
    .bind(draft.severity)
    .bind(racer_id)
    .bind(n.racer.as_ref().map(|r| r.name.as_str()))
    .bind(&draft.message)
    .bind(draft.payload.to_string())
    .execute(pool)
    .await?;
    Ok(())
}

fn to_json(r: &sqlx::sqlite::SqliteRow) -> Value {
    json!({
        "id": r.get::<i64, _>("id"),
        "ts": r.get::<String, _>("ts"),
        "kind": r.get::<String, _>("kind"),
        "severity": r.get::<String, _>("severity"),
        "racerId": r.get::<Option<String>, _>("racer_id"),
        "racerName": r.get::<Option<String>, _>("racer_name"),
        "message": r.get::<String, _>("message"),
        "payload": serde_json::from_str::<Value>(&r.get::<String, _>("payload")).unwrap_or_default(),
        "status": r.get::<String, _>("status"),
        "reviewedBy": r.get::<Option<String>, _>("reviewed_by"),
        "reviewedAt": r.get::<Option<String>, _>("reviewed_at"),
        "note": r.get::<Option<String>, _>("note"),
        "endedAt": r.get::<Option<String>, _>("ended_at"),
    })
}

/// Open incidents first-come (oldest first: what has waited longest), then the latest reviewed.
pub async fn list(
    pool: &SqlitePool,
    status: Option<&str>,
    racer: Option<&str>,
    limit: i64,
) -> Result<Vec<Value>, sqlx::Error> {
    // Open ones oldest first (what has waited longest); the rest newest first.
    let rows = if status == Some("open") {
        sqlx::query(
            "SELECT * FROM incidents WHERE status = 'open' AND (?1 IS NULL OR racer_id = ?1) ORDER BY id ASC LIMIT ?2",
        )
        .bind(racer)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT * FROM incidents WHERE (?1 IS NULL OR status = ?1) AND (?2 IS NULL OR racer_id = ?2) ORDER BY id DESC LIMIT ?3",
        )
        .bind(status)
        .bind(racer)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };
    Ok(rows.iter().map(to_json).collect())
}

pub async fn open_count(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT COUNT(*) FROM incidents WHERE status = 'open'")
        .fetch_one(pool)
        .await
}

/// Marks an incident `reviewed` or `dismissed` (or reopens it). `None` when it does not exist.
pub async fn review(
    pool: &SqlitePool,
    id: i64,
    status: &str,
    note: Option<&str>,
    actor: &str,
    now: DateTime<Utc>,
) -> Result<Option<Value>, sqlx::Error> {
    let reopen = status == "open";
    let done = sqlx::query(
        "UPDATE incidents SET status = ?, reviewed_by = ?, reviewed_at = ?, note = COALESCE(?, note) WHERE id = ?",
    )
    .bind(status)
    .bind(if reopen { None } else { Some(actor) })
    .bind(if reopen { None } else { Some(rfc(now)) })
    .bind(note)
    .bind(id)
    .execute(pool)
    .await?;
    if done.rows_affected() == 0 {
        return Ok(None);
    }
    let row = sqlx::query("SELECT * FROM incidents WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(Some(to_json(&row)))
}

pub async fn clear(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM incidents").execute(pool).await?;
    Ok(())
}

/// When this referee last opened the monitor.
pub async fn last_seen(pool: &SqlitePool, user_id: i64) -> Result<Option<String>, sqlx::Error> {
    Ok(
        sqlx::query_scalar::<_, Option<String>>("SELECT monitor_seen_at FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .flatten(),
    )
}

pub async fn mark_seen(
    pool: &SqlitePool,
    user_id: i64,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE users SET monitor_seen_at = ? WHERE id = ?")
        .bind(rfc(now))
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// What happened since `since` (an RFC 3339 instant): new incidents by kind, and the organizer
/// actions in the audit log. Everything a referee who just arrived should read first.
pub async fn catch_up(pool: &SqlitePool, since: &str) -> Result<Value, sqlx::Error> {
    let kinds = sqlx::query("SELECT kind, COUNT(*) AS n FROM incidents WHERE ts > ? GROUP BY kind")
        .bind(since)
        .fetch_all(pool)
        .await?;
    let mut by_kind = serde_json::Map::new();
    let mut total = 0;
    for r in &kinds {
        let n = r.get::<i64, _>("n");
        total += n;
        by_kind.insert(r.get::<String, _>("kind"), json!(n));
    }
    let actions = sqlx::query(
        "SELECT ts, actor, action, racer_id, payload FROM audit_log
         WHERE ts > ? AND (action LIKE 'racer.%' OR action LIKE 'event.%' OR action LIKE 'incident.%')
         ORDER BY id DESC LIMIT 30",
    )
    .bind(since)
    .fetch_all(pool)
    .await?;
    let actions: Vec<Value> = actions
        .iter()
        .map(|r| {
            json!({
                "ts": r.get::<String, _>("ts"),
                "actor": r.get::<String, _>("actor"),
                "action": r.get::<String, _>("action"),
                "racerId": r.get::<Option<String>, _>("racer_id"),
                "payload": serde_json::from_str::<Value>(&r.get::<String, _>("payload")).unwrap_or_default(),
            })
        })
        .collect();
    let donations: (i64, i64) = sqlx::query_as(
        "SELECT COUNT(*), COALESCE(SUM(CASE WHEN limited_by IS NOT NULL THEN 1 ELSE 0 END), 0)
         FROM time_donations WHERE ts > ?",
    )
    .bind(since)
    .fetch_one(pool)
    .await?;
    Ok(json!({
        "since": since,
        "newIncidents": total,
        "byKind": by_kind,
        "actions": actions,
        "donations": donations.0,
        "donationsLimited": donations.1,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notify::LiveInfo;

    fn racer(id: &str) -> LiveInfo {
        LiveInfo {
            id: id.into(),
            name: id.into(),
            avatar_url: None,
            progress_pct: 10.0,
            area: None,
            viewers: None,
            channels: vec![],
        }
    }
    fn notice(d: Detail) -> Notice {
        Notice::new(Some(racer("ana")), d, Utc::now())
    }

    #[test]
    fn only_referee_relevant_notices_are_incidents() {
        let t = Thresholds::default();
        let cut = describe(
            &notice(Detail::Disconnected {
                minutes: 4,
                back: false,
            }),
            &t,
        )
        .unwrap();
        assert_eq!(cut.kind, "disconnected");
        assert_eq!(cut.severity, "error");
        // Coming back is not an incident (it closes the open one).
        assert!(
            describe(
                &notice(Detail::Disconnected {
                    minutes: 4,
                    back: true
                }),
                &t
            )
            .is_none()
        );
        assert!(describe(&notice(Detail::Live), &t).is_none());
        assert!(
            describe(
                &notice(Detail::Boss {
                    boss: "gohma".into(),
                    count: None
                }),
                &t
            )
            .is_none()
        );
    }

    #[test]
    fn a_jump_follows_the_organizers_thresholds() {
        let t = Thresholds {
            jump_percent: 20.0,
            jump_window_seconds: 120,
            ..Thresholds::default()
        };
        let jump = |from, to, seconds| notice(Detail::Jump { from, to, seconds });
        assert!(describe(&jump(10.0, 35.0, 60), &t).is_some());
        assert!(describe(&jump(10.0, 25.0, 60), &t).is_none());
        assert!(describe(&jump(10.0, 35.0, 500), &t).is_none());
    }
}
