//! The streams' schedule and the referees' log book.
//!
//! Each streamer plays at their own hours. A *slot* says when a racer is expected to be live; any
//! referee can take a slot (so the others know it is covered) and the scanner warns the referees'
//! Discord channel when a slot starts without anyone assigned or the racer does not show up. Times
//! are stored in UTC; the panel shows them in the viewer's zone and the racer's.

use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};

use crate::db::rfc;

pub const MAX_SLOT_HOURS: i64 = 24;
pub const MAX_NOTE_CHARS: usize = 500;

#[derive(Debug)]
pub enum ScheduleError {
    Invalid(String),
    Overlap,
    NotFound,
    Db(String),
}

impl From<sqlx::Error> for ScheduleError {
    fn from(e: sqlx::Error) -> Self {
        match &e {
            // A racer or a user that does not exist.
            sqlx::Error::Database(d) if d.is_foreign_key_violation() => ScheduleError::NotFound,
            _ => ScheduleError::Db(e.to_string()),
        }
    }
}

/// A slot as the scanner needs it.
#[derive(Clone, Debug)]
pub struct SlotSnap {
    pub id: i64,
    pub racer_id: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub assignees: usize,
}

fn parse_time(s: &str) -> Result<DateTime<Utc>, ScheduleError> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| ScheduleError::Invalid(format!("`{s}` is not an RFC 3339 date")))
}

/// Start and end as instants, checked: the end after the start, at most a day long.
pub fn parse_window(
    start: &str,
    end: &str,
) -> Result<(DateTime<Utc>, DateTime<Utc>), ScheduleError> {
    let (s, e) = (parse_time(start)?, parse_time(end)?);
    if e <= s {
        return Err(ScheduleError::Invalid(
            "the end must be after the start".into(),
        ));
    }
    if e - s > Duration::hours(MAX_SLOT_HOURS) {
        return Err(ScheduleError::Invalid(format!(
            "a slot lasts at most {MAX_SLOT_HOURS} hours"
        )));
    }
    Ok((s, e))
}

/// Slots overlapping `from..to`, with who referees each, soonest first.
pub async fn list(
    pool: &SqlitePool,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Result<Vec<Value>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT s.id, s.racer_id, r.display_name, r.timezone, s.start_utc, s.end_utc, s.note
         FROM schedule_slots s JOIN racers r ON r.id = s.racer_id
         WHERE s.end_utc > ? AND s.start_utc < ?
         ORDER BY s.start_utc, s.id",
    )
    .bind(rfc(from))
    .bind(rfc(to))
    .fetch_all(pool)
    .await?;
    let who = sqlx::query(
        "SELECT a.slot_id, u.id AS user_id, u.username FROM slot_assignments a
         JOIN users u ON u.id = a.user_id ORDER BY u.username",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|r| {
            let id: i64 = r.get("id");
            let assignees: Vec<Value> = who
                .iter()
                .filter(|w| w.get::<i64, _>("slot_id") == id)
                .map(|w| json!({ "userId": w.get::<i64, _>("user_id"), "username": w.get::<String, _>("username") }))
                .collect();
            json!({
                "id": id,
                "racerId": r.get::<String, _>("racer_id"),
                "racerName": r.get::<String, _>("display_name"),
                "racerTimezone": r.get::<String, _>("timezone"),
                "startUtc": r.get::<String, _>("start_utc"),
                "endUtc": r.get::<String, _>("end_utc"),
                "note": r.get::<Option<String>, _>("note"),
                "assignees": assignees,
            })
        })
        .collect())
}

/// Slots around `now` for the scanner (started up to 6 hours ago or starting within 12 hours).
pub async fn around(pool: &SqlitePool, now: DateTime<Utc>) -> Result<Vec<SlotSnap>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT s.id, s.racer_id, s.start_utc, s.end_utc,
                (SELECT COUNT(*) FROM slot_assignments a WHERE a.slot_id = s.id) AS n
         FROM schedule_slots s WHERE s.end_utc > ? AND s.start_utc < ?",
    )
    .bind(rfc(now - Duration::hours(6)))
    .bind(rfc(now + Duration::hours(12)))
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .filter_map(|r| {
            Some(SlotSnap {
                id: r.get("id"),
                racer_id: r.get("racer_id"),
                start: parse_time(&r.get::<String, _>("start_utc")).ok()?,
                end: parse_time(&r.get::<String, _>("end_utc")).ok()?,
                assignees: r.get::<i64, _>("n") as usize,
            })
        })
        .collect())
}

async fn overlaps(
    pool: &SqlitePool,
    racer: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    except: Option<i64>,
) -> Result<bool, sqlx::Error> {
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM schedule_slots
         WHERE racer_id = ? AND start_utc < ? AND end_utc > ? AND (? IS NULL OR id <> ?)",
    )
    .bind(racer)
    .bind(rfc(end))
    .bind(rfc(start))
    .bind(except)
    .bind(except)
    .fetch_one(pool)
    .await?;
    Ok(n > 0)
}

pub async fn create(
    pool: &SqlitePool,
    racer: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    note: Option<&str>,
    actor: &str,
) -> Result<i64, ScheduleError> {
    if overlaps(pool, racer, start, end, None).await? {
        return Err(ScheduleError::Overlap);
    }
    let done = sqlx::query(
        "INSERT INTO schedule_slots (racer_id, start_utc, end_utc, note, created_by) VALUES (?,?,?,?,?)",
    )
    .bind(racer)
    .bind(rfc(start))
    .bind(rfc(end))
    .bind(note)
    .bind(actor)
    .execute(pool)
    .await?;
    Ok(done.last_insert_rowid())
}

/// The racer of a slot, or `None` when it does not exist.
pub async fn racer_of(pool: &SqlitePool, id: i64) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT racer_id FROM schedule_slots WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
}

pub async fn update(
    pool: &SqlitePool,
    id: i64,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    note: Option<&str>,
) -> Result<(), ScheduleError> {
    let racer = racer_of(pool, id).await?.ok_or(ScheduleError::NotFound)?;
    if overlaps(pool, &racer, start, end, Some(id)).await? {
        return Err(ScheduleError::Overlap);
    }
    sqlx::query("UPDATE schedule_slots SET start_utc = ?, end_utc = ?, note = ? WHERE id = ?")
        .bind(rfc(start))
        .bind(rfc(end))
        .bind(note)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
    Ok(sqlx::query("DELETE FROM schedule_slots WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?
        .rows_affected()
        > 0)
}

pub async fn assign(pool: &SqlitePool, slot: i64, user: i64) -> Result<(), ScheduleError> {
    // The foreign keys reject a slot or a user that does not exist.
    sqlx::query("INSERT OR IGNORE INTO slot_assignments (slot_id, user_id) VALUES (?, ?)")
        .bind(slot)
        .bind(user)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn unassign(pool: &SqlitePool, slot: i64, user: i64) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM slot_assignments WHERE slot_id = ? AND user_id = ?")
        .bind(slot)
        .bind(user)
        .execute(pool)
        .await?;
    Ok(())
}

// ---- log book -----------------------------------------------------------------------------------

pub async fn add_note(
    pool: &SqlitePool,
    racer: Option<&str>,
    author: &str,
    text: &str,
    now: DateTime<Utc>,
) -> Result<Value, ScheduleError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(ScheduleError::Invalid("the note is empty".into()));
    }
    if text.chars().count() > MAX_NOTE_CHARS {
        return Err(ScheduleError::Invalid(format!(
            "the note is too long ({MAX_NOTE_CHARS} max)"
        )));
    }
    let done =
        sqlx::query("INSERT INTO referee_notes (ts, racer_id, author, text) VALUES (?,?,?,?)")
            .bind(rfc(now))
            .bind(racer)
            .bind(author)
            .bind(text)
            .execute(pool)
            .await?;
    Ok(json!({
        "id": done.last_insert_rowid(), "ts": rfc(now), "racerId": racer, "author": author, "text": text,
    }))
}

pub async fn notes(
    pool: &SqlitePool,
    racer: Option<&str>,
    limit: i64,
) -> Result<Vec<Value>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT * FROM referee_notes WHERE (?1 IS NULL OR racer_id = ?1) ORDER BY id DESC LIMIT ?2",
    )
    .bind(racer)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|r| {
            json!({
                "id": r.get::<i64, _>("id"),
                "ts": r.get::<String, _>("ts"),
                "racerId": r.get::<Option<String>, _>("racer_id"),
                "author": r.get::<String, _>("author"),
                "text": r.get::<String, _>("text"),
            })
        })
        .collect())
}

/// Deletes a note. `author` limits it to the notes of that person (admins pass `None`).
pub async fn delete_note(
    pool: &SqlitePool,
    id: i64,
    author: Option<&str>,
) -> Result<bool, sqlx::Error> {
    Ok(
        sqlx::query("DELETE FROM referee_notes WHERE id = ? AND (? IS NULL OR author = ?)")
            .bind(id)
            .bind(author)
            .bind(author)
            .execute(pool)
            .await?
            .rows_affected()
            > 0,
    )
}

pub async fn clear_notes(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM referee_notes")
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slot_has_a_sane_window() {
        assert!(parse_window("2026-10-07T18:00:00Z", "2026-10-07T21:00:00Z").is_ok());
        for (s, e) in [
            ("2026-10-07T21:00:00Z", "2026-10-07T18:00:00Z"),
            ("2026-10-07T18:00:00Z", "2026-10-07T18:00:00Z"),
            ("2026-10-07T18:00:00Z", "2026-10-09T18:00:00Z"),
            ("mañana", "2026-10-09T18:00:00Z"),
        ] {
            assert!(parse_window(s, e).is_err(), "{s} {e}");
        }
    }
}
