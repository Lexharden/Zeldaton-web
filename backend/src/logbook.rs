//! The referees' log book: free notes to hand a shift over ("xime came back after 5 min", "the
//! emulator lagged, time adjusted"). Distinct from the audit log, which records actions, not what
//! someone saw. Emptied together with the rest of the race data when the event is reset.

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use sqlx::{Row, SqlitePool};

use crate::db::rfc;

pub const MAX_NOTE_CHARS: usize = 500;

#[derive(Debug)]
pub enum NoteError {
    Invalid(String),
    /// The racer does not exist.
    NotFound,
    Db(String),
}

impl From<sqlx::Error> for NoteError {
    fn from(e: sqlx::Error) -> Self {
        match &e {
            sqlx::Error::Database(d) if d.is_foreign_key_violation() => NoteError::NotFound,
            _ => NoteError::Db(e.to_string()),
        }
    }
}

pub async fn add_note(
    pool: &SqlitePool,
    racer: Option<&str>,
    author: &str,
    text: &str,
    now: DateTime<Utc>,
) -> Result<Value, NoteError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(NoteError::Invalid("the note is empty".into()));
    }
    if text.chars().count() > MAX_NOTE_CHARS {
        return Err(NoteError::Invalid(format!(
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
