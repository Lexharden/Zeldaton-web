//! "Who will win?" public poll. One vote per connection: the voter is identified by a salted hash
//! of their address (never the address itself) and the table's primary key rejects a second vote.

use std::collections::HashMap;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use chrono::Utc;
use rand::RngExt;
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use crate::domain::EventStatus;
use crate::error::ApiError;
use crate::hub::AppState;

const MODE_KEY: &str = "poll_mode";
const SALT_KEY: &str = "poll_salt";

/// `auto`: open until the event starts. `open` / `closed`: the organizer decides.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Auto,
    Open,
    Closed,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Auto => "auto",
            Mode::Open => "open",
            Mode::Closed => "closed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(Mode::Auto),
            "open" => Some(Mode::Open),
            "closed" => Some(Mode::Closed),
            _ => None,
        }
    }

    pub fn is_open(self, status: EventStatus) -> bool {
        match self {
            Mode::Auto => status == EventStatus::Upcoming,
            Mode::Open => true,
            Mode::Closed => false,
        }
    }
}

pub async fn mode(pool: &SqlitePool) -> Mode {
    match crate::db::setting(pool, MODE_KEY).await {
        Ok(Some(v)) => Mode::parse(&v).unwrap_or(Mode::Auto),
        _ => Mode::Auto,
    }
}

/// The secret that keys the address hashes. Created on first use; `INSERT OR IGNORE` + read back
/// keeps two simultaneous first votes from ending up with different salts.
async fn salt(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let mut bytes = [0u8; 16];
    rand::rng().fill(&mut bytes);
    sqlx::query("INSERT OR IGNORE INTO app_settings (key, value) VALUES (?, ?)")
        .bind(SALT_KEY)
        .bind(hex::encode(bytes))
        .execute(pool)
        .await?;
    crate::db::setting(pool, SALT_KEY)
        .await
        .map(|v| v.unwrap_or_default())
}

pub fn ip_hash(salt: &str, ip: &str) -> String {
    hex::encode(Sha256::digest(format!("{salt}:{ip}").as_bytes()))
}

/// A racer the public can vote for.
#[derive(Clone, Debug)]
pub struct PollOption {
    pub racer_id: String,
    pub name: String,
    pub avatar_url: Option<String>,
}

/// Votes per racer in `options`, most voted first (ties by roster order). Votes for racers that no
/// longer exist are ignored. Returns `(racerId, votes, percent)` with the percent to one decimal.
pub fn tally(
    options: &[PollOption],
    counts: &HashMap<String, i64>,
) -> (i64, Vec<(String, i64, f64)>) {
    let total: i64 = options
        .iter()
        .map(|o| counts.get(&o.racer_id).copied().unwrap_or(0))
        .sum();
    let mut rows: Vec<(usize, String, i64)> = options
        .iter()
        .enumerate()
        .map(|(i, o)| {
            (
                i,
                o.racer_id.clone(),
                counts.get(&o.racer_id).copied().unwrap_or(0),
            )
        })
        .collect();
    rows.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    let rows = rows
        .into_iter()
        .map(|(_, id, votes)| {
            let percent = if total == 0 {
                0.0
            } else {
                (votes as f64 * 1000.0 / total as f64).round() / 10.0
            };
            (id, votes, percent)
        })
        .collect();
    (total, rows)
}

/// The JSON the page gets. `results` is only included when `reveal` is set (the voter already
/// voted, the poll is closed, or it is the organizer asking), so nobody votes with the crowd.
pub fn poll_payload(
    open: bool,
    mode: Mode,
    options: &[PollOption],
    counts: &HashMap<String, i64>,
    voted: Option<&str>,
    reveal: bool,
) -> Value {
    let (total, rows) = tally(options, counts);
    let mut out = json!({
        "open": open,
        "mode": mode.as_str(),
        "options": options.iter().map(|o| {
            let mut v = json!({ "racerId": o.racer_id, "name": o.name });
            if let Some(a) = &o.avatar_url {
                v["avatarUrl"] = json!(a);
            }
            v
        }).collect::<Vec<_>>(),
        "voted": voted,
    });
    if reveal {
        out["totalVotes"] = json!(total);
        out["results"] = rows
            .into_iter()
            .map(|(racer_id, votes, percent)| json!({ "racerId": racer_id, "votes": votes, "percent": percent }))
            .collect();
    }
    out
}

async fn counts(pool: &SqlitePool) -> Result<HashMap<String, i64>, sqlx::Error> {
    let rows: Vec<(String, i64)> =
        sqlx::query_as("SELECT racer_id, COUNT(*) FROM poll_votes GROUP BY racer_id")
            .fetch_all(pool)
            .await?;
    Ok(rows.into_iter().collect())
}

async fn voted_for(pool: &SqlitePool, hash: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT racer_id FROM poll_votes WHERE ip_hash = ?")
        .bind(hash)
        .fetch_optional(pool)
        .await
}

fn options_of(hub: &AppState) -> Vec<PollOption> {
    hub.racers(Utc::now())
        .into_iter()
        .map(|r| PollOption {
            racer_id: r.id,
            name: r.display_name,
            avatar_url: r.avatar_url,
        })
        .collect()
}

/// Builds the payload for one viewer (`hash` = who is asking, `None` for the organizer's view).
async fn build(hub: &AppState, hash: Option<&str>, organizer: bool) -> Result<Value, ApiError> {
    let m = mode(&hub.pool).await;
    let open = m.is_open(hub.event(Utc::now()).status);
    let options = options_of(hub);
    let counts = counts(&hub.pool).await.map_err(ApiError::internal)?;
    let voted = match hash {
        Some(h) => voted_for(&hub.pool, h).await.map_err(ApiError::internal)?,
        None => None,
    };
    let reveal = organizer || voted.is_some() || !open;
    Ok(poll_payload(
        open,
        m,
        &options,
        &counts,
        voted.as_deref(),
        reveal,
    ))
}

async fn viewer_hash(hub: &AppState, headers: &HeaderMap) -> Result<(String, String), ApiError> {
    let ip = crate::auth::client_ip(headers)
        .ok_or_else(|| ApiError::BadRequest("could not identify the connection".into()))?;
    let salt = salt(&hub.pool).await.map_err(ApiError::internal)?;
    Ok((ip_hash(&salt, &ip), ip))
}

pub async fn get_poll(
    State(hub): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    // A request we cannot attribute still sees the options (just never as "already voted").
    let hash = viewer_hash(&hub, &headers).await.ok().map(|(h, _)| h);
    build(&hub, hash.as_deref(), false).await.map(Json)
}

#[derive(Deserialize)]
pub struct VoteBody {
    #[serde(rename = "racerId")]
    racer_id: String,
    /// Honeypot: hidden in the page, so only a script fills it in.
    #[serde(default)]
    website: String,
}

pub async fn vote(
    State(hub): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<VoteBody>,
) -> Result<Json<Value>, ApiError> {
    let (hash, ip) = viewer_hash(&hub, &headers).await?;
    let key = format!("v:{ip}");
    hub.vote_limiter
        .check(&key)
        .map_err(ApiError::TooManyRequests)?;
    hub.vote_limiter.fail(&key);
    if !body.website.is_empty() {
        return Err(ApiError::BadRequest("invalid vote".into()));
    }
    let open = mode(&hub.pool).await.is_open(hub.event(Utc::now()).status);
    if !open {
        return Err(ApiError::Forbidden("the poll is closed".into()));
    }
    if !options_of(&hub).iter().any(|o| o.racer_id == body.racer_id) {
        return Err(ApiError::NotFound);
    }
    let done = sqlx::query(
        "INSERT INTO poll_votes (ip_hash, racer_id, created_at) VALUES (?, ?, ?)
         ON CONFLICT(ip_hash) DO NOTHING",
    )
    .bind(&hash)
    .bind(&body.racer_id)
    .bind(crate::engine::iso(Utc::now()))
    .execute(&hub.pool)
    .await
    .map_err(ApiError::internal)?;
    if done.rows_affected() == 0 {
        return Err(ApiError::Conflict("you already voted".into()));
    }
    build(&hub, Some(&hash), false).await.map(Json)
}

// ---- organizer ----------------------------------------------------------------------------------

/// Full results for the panel, whatever the mode.
pub async fn admin_view(hub: &AppState) -> Result<Value, ApiError> {
    build(hub, None, true).await
}

pub async fn set_mode(hub: &AppState, m: Mode) -> Result<(), ApiError> {
    crate::db::set_setting(&hub.pool, MODE_KEY, m.as_str())
        .await
        .map_err(ApiError::internal)
}

/// Deletes every vote; returns how many there were.
pub async fn clear_votes(hub: &AppState) -> Result<u64, ApiError> {
    sqlx::query("DELETE FROM poll_votes")
        .execute(&hub.pool)
        .await
        .map(|r| r.rows_affected())
        .map_err(ApiError::internal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::normalize_ip;

    fn opts() -> Vec<PollOption> {
        ["a", "b", "c"]
            .iter()
            .map(|id| PollOption {
                racer_id: id.to_string(),
                name: id.to_uppercase(),
                avatar_url: None,
            })
            .collect()
    }

    #[test]
    fn ipv4_is_kept_and_ipv6_collapses_to_its_64() {
        assert_eq!(normalize_ip("203.0.113.9").as_deref(), Some("203.0.113.9"));
        assert_eq!(
            normalize_ip(" 203.0.113.9 ").as_deref(),
            Some("203.0.113.9")
        );
        assert_eq!(
            normalize_ip("2001:db8:1:2:aaaa:bbbb:cccc:dddd").as_deref(),
            Some("2001:db8:1:2::/64")
        );
        // Another address of the same /64 is the same voter.
        assert_eq!(
            normalize_ip("2001:db8:1:2::1"),
            normalize_ip("2001:db8:1:2:ffff::9")
        );
        assert_eq!(
            normalize_ip("::ffff:203.0.113.9").as_deref(),
            Some("203.0.113.9")
        );
        assert_eq!(normalize_ip("not an ip"), None);
        assert_eq!(normalize_ip(""), None);
    }

    #[test]
    fn the_hash_is_stable_and_depends_on_the_salt() {
        assert_eq!(ip_hash("s", "1.2.3.4"), ip_hash("s", "1.2.3.4"));
        assert_ne!(ip_hash("s", "1.2.3.4"), ip_hash("t", "1.2.3.4"));
        assert_ne!(ip_hash("s", "1.2.3.4"), ip_hash("s", "1.2.3.5"));
        assert!(!ip_hash("s", "1.2.3.4").contains("1.2.3.4"));
    }

    #[test]
    fn tally_orders_by_votes_then_roster_and_ignores_removed_racers() {
        let counts: HashMap<String, i64> = [("a", 1), ("b", 3), ("c", 1), ("gone", 50)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        let (total, rows) = tally(&opts(), &counts);
        assert_eq!(total, 5);
        let ids: Vec<&str> = rows.iter().map(|r| r.0.as_str()).collect();
        assert_eq!(ids, ["b", "a", "c"]);
        assert_eq!(rows[0].2, 60.0);
        assert_eq!(rows[1].2, 20.0);
    }

    #[test]
    fn tally_with_no_votes_is_all_zero() {
        let (total, rows) = tally(&opts(), &HashMap::new());
        assert_eq!(total, 0);
        assert!(rows.iter().all(|r| r.1 == 0 && r.2 == 0.0));
    }

    #[test]
    fn results_stay_hidden_until_revealed() {
        let hidden = poll_payload(true, Mode::Auto, &opts(), &HashMap::new(), None, false);
        assert!(hidden.get("results").is_none() && hidden.get("totalVotes").is_none());
        let shown = poll_payload(true, Mode::Auto, &opts(), &HashMap::new(), Some("a"), true);
        assert_eq!(shown["voted"], "a");
        assert_eq!(shown["results"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn auto_closes_when_the_event_starts() {
        assert!(Mode::Auto.is_open(EventStatus::Upcoming));
        assert!(!Mode::Auto.is_open(EventStatus::Live));
        assert!(Mode::Open.is_open(EventStatus::Live));
        assert!(!Mode::Closed.is_open(EventStatus::Upcoming));
    }
}
