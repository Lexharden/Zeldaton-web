//! SQLite persistence. The in-memory state is authoritative at runtime; every change is written
//! through by a single writer task so the process can restart without losing clocks or progress.

use std::collections::{HashMap, HashSet, VecDeque};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::catalog::{Age, Catalog, CatalogItem, CatalogObjective, default_catalog};
use crate::clock::Checkpoint;
use crate::domain::*;
use crate::state::{FinishInfo, RaceState, RacerRuntime};

pub async fn connect(url: &str) -> Result<SqlitePool, sqlx::Error> {
    let opts = SqliteConnectOptions::from_str(url)?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .foreign_keys(true);
    // In-memory databases are per connection: keep a single one so tests see their own data.
    let max = if url.contains(":memory:") { 1 } else { 5 };
    SqlitePoolOptions::new()
        .max_connections(max)
        .connect_with(opts)
        .await
}

pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

/// Persisted view of one racer's mutable state.
#[derive(Clone, Debug)]
pub struct RacerStateRow {
    pub racer_id: String,
    pub status: RacerStatus,
    pub remaining_ms: i64,
    pub checkpoint_at: DateTime<Utc>,
    pub reset_at: DateTime<Utc>,
    pub played_ms_total: i64,
    pub played_today_ms: i64,
    pub progress_pct: f64,
    pub current_area: Option<String>,
    pub current_objective: Option<String>,
    pub completed_objectives: Vec<String>,
    pub items: HashMap<String, bool>,
    pub stats: RacerStats,
    pub finished_at: Option<String>,
    pub final_time_seconds: Option<i64>,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub stream: StreamState,
    pub donation_added_ms: i64,
    pub donation_removed_ms: i64,
}

/// One row of the time-donation ledger.
#[derive(Clone, Debug)]
pub struct TimeDonationRow {
    pub ts: DateTime<Utc>,
    pub racer_id: String,
    pub client_id: String,
    pub platform: Platform,
    pub currency: DonationCurrency,
    pub amount: i64,
    pub gift: Option<String>,
    pub gift_count: Option<i64>,
    pub viewer: Option<String>,
    pub requested_ms: i64,
    /// What HiveShock asked for (differs from `requested_ms` when the server applies its own rate).
    pub reported_ms: Option<i64>,
    pub applied_ms: i64,
    pub limited_by: Option<String>,
}

#[derive(Clone, Debug)]
pub struct IdentityRow {
    pub id: String,
    pub slug: String,
    pub display_name: String,
    pub country: Option<String>,
    pub avatar_url: Option<String>,
    pub timezone: String,
    pub token_hash: String,
    pub sort_order: i64,
    pub channels: Vec<Channel>,
}

#[derive(Clone, Debug)]
pub struct AuditRow {
    pub ts: DateTime<Utc>,
    pub actor: String,
    pub action: String,
    pub racer_id: Option<String>,
    pub payload: serde_json::Value,
}

#[derive(Clone, Debug)]
pub enum PersistOp {
    Event(EventInfo),
    RacerState(Box<RacerStateRow>),
    Identity(Box<IdentityRow>),
    DeleteRacer(String),
    Activity(ActivityItem),
    Counters(HiveShockStats),
    Audit(AuditRow),
    TimeDonation(Box<TimeDonationRow>),
    CatalogItem(Box<CatalogItem>),
    DeleteCatalogItem(String),
    CatalogObjective(Box<CatalogObjective>),
    DeleteCatalogObjective(String),
    /// Event reset: forgets the race itself (activity feed and donation ledger). Racers, tokens,
    /// catalog, accounts and the audit log stay.
    ClearRaceData,
}

pub(crate) fn rfc(d: DateTime<Utc>) -> String {
    d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn parse_dt(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

pub async fn apply(pool: &SqlitePool, op: PersistOp) -> Result<(), sqlx::Error> {
    match op {
        PersistOp::Event(e) => {
            sqlx::query(
                "INSERT INTO event (id,name,game,edition,status,start_at_utc,end_at_utc,timezone,daily_budget_seconds,daily_reset_local_time,win_condition,required_objective_ids,donation_time,rehearsal)
                 VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?)
                 ON CONFLICT(id) DO UPDATE SET name=excluded.name, game=excluded.game, edition=excluded.edition, status=excluded.status,
                   start_at_utc=excluded.start_at_utc, end_at_utc=excluded.end_at_utc, timezone=excluded.timezone,
                   daily_budget_seconds=excluded.daily_budget_seconds, daily_reset_local_time=excluded.daily_reset_local_time,
                   win_condition=excluded.win_condition, required_objective_ids=excluded.required_objective_ids,
                   donation_time=excluded.donation_time, rehearsal=excluded.rehearsal",
            )
            .bind(&e.id)
            .bind(&e.name)
            .bind(&e.game)
            .bind(&e.edition)
            .bind(e.status.as_str())
            .bind(&e.start_at_utc)
            .bind(&e.end_at_utc)
            .bind(&e.timezone)
            .bind(e.daily_budget_seconds)
            .bind(&e.daily_reset_local_time)
            .bind(&e.rules.win_condition)
            .bind(serde_json::to_string(&e.rules.required_objective_ids).unwrap_or_default())
            .bind(serde_json::to_string(&e.donation_time).unwrap_or_else(|_| "{}".into()))
            .bind(e.rehearsal)
            .execute(pool)
            .await?;
        }
        PersistOp::RacerState(r) => {
            sqlx::query(
                "INSERT INTO racer_state (racer_id,status,remaining_ms,checkpoint_at,reset_at,played_ms_total,played_today_ms,progress_pct,current_area,current_objective,completed_objectives,items,stats,finished_at,final_time_seconds,last_heartbeat_at,stream_live,viewers,thumbnail_url,donation_added_ms,donation_removed_ms)
                 VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
                 ON CONFLICT(racer_id) DO UPDATE SET status=excluded.status, remaining_ms=excluded.remaining_ms, checkpoint_at=excluded.checkpoint_at,
                   reset_at=excluded.reset_at, played_ms_total=excluded.played_ms_total, played_today_ms=excluded.played_today_ms, progress_pct=excluded.progress_pct,
                   current_area=excluded.current_area, current_objective=excluded.current_objective,
                   completed_objectives=excluded.completed_objectives, items=excluded.items, stats=excluded.stats,
                   finished_at=excluded.finished_at, final_time_seconds=excluded.final_time_seconds,
                   last_heartbeat_at=excluded.last_heartbeat_at, stream_live=excluded.stream_live,
                   viewers=excluded.viewers, thumbnail_url=excluded.thumbnail_url,
                   donation_added_ms=excluded.donation_added_ms, donation_removed_ms=excluded.donation_removed_ms",
            )
            .bind(&r.racer_id)
            .bind(r.status.as_str())
            .bind(r.remaining_ms)
            .bind(rfc(r.checkpoint_at))
            .bind(rfc(r.reset_at))
            .bind(r.played_ms_total)
            .bind(r.played_today_ms)
            .bind(r.progress_pct)
            .bind(&r.current_area)
            .bind(&r.current_objective)
            .bind(serde_json::to_string(&r.completed_objectives).unwrap_or_default())
            .bind(serde_json::to_string(&r.items).unwrap_or_default())
            .bind(serde_json::to_string(&r.stats).unwrap_or_default())
            .bind(&r.finished_at)
            .bind(r.final_time_seconds)
            .bind(r.last_heartbeat_at.map(rfc))
            .bind(r.stream.is_live)
            .bind(r.stream.viewers)
            .bind(&r.stream.thumbnail_url)
            .bind(r.donation_added_ms)
            .bind(r.donation_removed_ms)
            .execute(pool)
            .await?;
        }
        PersistOp::Identity(i) => {
            let mut tx = pool.begin().await?;
            sqlx::query(
                "INSERT INTO racers (id,slug,display_name,country,avatar_url,timezone,token_hash,sort_order) VALUES (?,?,?,?,?,?,?,?)
                 ON CONFLICT(id) DO UPDATE SET slug=excluded.slug, display_name=excluded.display_name, country=excluded.country,
                   avatar_url=excluded.avatar_url, timezone=excluded.timezone, token_hash=excluded.token_hash, sort_order=excluded.sort_order",
            )
            .bind(&i.id)
            .bind(&i.slug)
            .bind(&i.display_name)
            .bind(&i.country)
            .bind(&i.avatar_url)
            .bind(&i.timezone)
            .bind(&i.token_hash)
            .bind(i.sort_order)
            .execute(&mut *tx)
            .await?;
            sqlx::query("DELETE FROM channels WHERE racer_id = ?")
                .bind(&i.id)
                .execute(&mut *tx)
                .await?;
            for c in &i.channels {
                sqlx::query("INSERT INTO channels (racer_id,platform,handle,url) VALUES (?,?,?,?)")
                    .bind(&i.id)
                    .bind(c.platform.as_str())
                    .bind(&c.handle)
                    .bind(&c.url)
                    .execute(&mut *tx)
                    .await?;
            }
            tx.commit().await?;
        }
        PersistOp::DeleteRacer(id) => {
            sqlx::query("DELETE FROM racers WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await?;
        }
        PersistOp::Activity(a) => {
            sqlx::query("INSERT OR IGNORE INTO activity (id,ts,kind,racer_id,racer_name,message,code,detail,subject) VALUES (?,?,?,?,?,?,?,?,?)")
                .bind(&a.id)
                .bind(&a.timestamp_utc)
                .bind(a.kind.as_str())
                .bind(&a.racer_id)
                .bind(&a.racer_name)
                .bind(&a.message)
                .bind(&a.code)
                .bind(&a.detail)
                .bind(&a.subject)
                .execute(pool)
                .await?;
        }
        PersistOp::ClearRaceData => {
            let mut tx = pool.begin().await?;
            sqlx::query("DELETE FROM activity")
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM time_donations")
                .execute(&mut *tx)
                .await?;
            // A reset race starts with a clean slate of "already announced" notices.
            sqlx::query("DELETE FROM notices_sent")
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM incidents")
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM referee_notes")
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
        }
        PersistOp::Counters(c) => {
            sqlx::query(
                "INSERT INTO hiveshock_counters (id,game_events,item_events,progress_events,chat_events) VALUES (1,?,?,?,?)
                 ON CONFLICT(id) DO UPDATE SET game_events=excluded.game_events, item_events=excluded.item_events,
                   progress_events=excluded.progress_events, chat_events=excluded.chat_events",
            )
            .bind(c.game_events)
            .bind(c.item_events)
            .bind(c.progress_events)
            .bind(c.chat_events)
            .execute(pool)
            .await?;
        }
        PersistOp::CatalogItem(i) => {
            sqlx::query(
                "INSERT INTO catalog_items (id,grp,age,name_es,name_en,short,icon,sort_order,enabled) VALUES (?,?,?,?,?,?,?,?,?)
                 ON CONFLICT(id) DO UPDATE SET grp=excluded.grp, age=excluded.age, name_es=excluded.name_es, name_en=excluded.name_en,
                   short=excluded.short, icon=excluded.icon, sort_order=excluded.sort_order, enabled=excluded.enabled",
            )
            .bind(&i.id)
            .bind(&i.group)
            .bind(i.age.as_str())
            .bind(&i.name_es)
            .bind(&i.name_en)
            .bind(&i.short)
            .bind(&i.icon)
            .bind(i.sort_order)
            .bind(i.enabled)
            .execute(pool)
            .await?;
        }
        PersistOp::DeleteCatalogItem(id) => {
            sqlx::query("DELETE FROM catalog_items WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await?;
        }
        PersistOp::CatalogObjective(o) => {
            sqlx::query(
                "INSERT INTO catalog_objectives (id,age,name_es,name_en,sort_order,required,enabled) VALUES (?,?,?,?,?,?,?)
                 ON CONFLICT(id) DO UPDATE SET age=excluded.age, name_es=excluded.name_es, name_en=excluded.name_en,
                   sort_order=excluded.sort_order, required=excluded.required, enabled=excluded.enabled",
            )
            .bind(&o.id)
            .bind(o.age.as_str())
            .bind(&o.name_es)
            .bind(&o.name_en)
            .bind(o.sort_order)
            .bind(o.required)
            .bind(o.enabled)
            .execute(pool)
            .await?;
        }
        PersistOp::DeleteCatalogObjective(id) => {
            sqlx::query("DELETE FROM catalog_objectives WHERE id = ?")
                .bind(id)
                .execute(pool)
                .await?;
        }
        PersistOp::TimeDonation(d) => {
            // OR IGNORE: the unique (racer_id, client_id) already guarantees one row per donation.
            sqlx::query(
                "INSERT OR IGNORE INTO time_donations (ts,racer_id,client_id,platform,currency,amount,gift,gift_count,viewer,requested_ms,reported_ms,applied_ms,limited_by)
                 VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",
            )
            .bind(rfc(d.ts))
            .bind(&d.racer_id)
            .bind(&d.client_id)
            .bind(d.platform.as_str())
            .bind(d.currency.as_str())
            .bind(d.amount)
            .bind(&d.gift)
            .bind(d.gift_count)
            .bind(&d.viewer)
            .bind(d.requested_ms)
            .bind(d.reported_ms)
            .bind(d.applied_ms)
            .bind(&d.limited_by)
            .execute(pool)
            .await?;
        }
        PersistOp::Audit(a) => {
            sqlx::query(
                "INSERT INTO audit_log (ts,actor,action,racer_id,payload) VALUES (?,?,?,?,?)",
            )
            .bind(rfc(a.ts))
            .bind(&a.actor)
            .bind(&a.action)
            .bind(&a.racer_id)
            .bind(a.payload.to_string())
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

/// Single writer: drains the queue in order so state changes land on disk in the order they happened.
pub async fn writer(pool: SqlitePool, mut rx: UnboundedReceiver<PersistOp>) {
    while let Some(op) = rx.recv().await {
        if let Err(e) = apply(&pool, op).await {
            tracing::error!(error = %e, "failed to persist state change");
        }
    }
}

/// Reads the catalog tables. Empty tables mean "not seeded yet": the factory catalog is used.
pub async fn load_catalog(pool: &SqlitePool) -> Result<Catalog, sqlx::Error> {
    let items: Vec<CatalogItem> =
        sqlx::query("SELECT * FROM catalog_items ORDER BY sort_order, id")
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|r| CatalogItem {
                id: r.get("id"),
                group: r.get("grp"),
                age: Age::parse(&r.get::<String, _>("age")),
                name_es: r.get("name_es"),
                name_en: r.get("name_en"),
                short: r.get("short"),
                icon: r.get("icon"),
                sort_order: r.get("sort_order"),
                enabled: r.get::<i64, _>("enabled") != 0,
            })
            .collect();
    let objectives: Vec<CatalogObjective> =
        sqlx::query("SELECT * FROM catalog_objectives ORDER BY sort_order, id")
            .fetch_all(pool)
            .await?
            .into_iter()
            .map(|r| CatalogObjective {
                id: r.get("id"),
                age: Age::parse(&r.get::<String, _>("age")),
                name_es: r.get("name_es"),
                name_en: r.get("name_en"),
                sort_order: r.get("sort_order"),
                required: r.get::<i64, _>("required") != 0,
                enabled: r.get::<i64, _>("enabled") != 0,
            })
            .collect();
    if items.is_empty() && objectives.is_empty() {
        return Ok(default_catalog());
    }
    Ok(Catalog { items, objectives })
}

/// Writes the factory catalog when the tables are empty (first start, or a database created
/// before the catalog existed). Never touches a catalog the organizer already edited.
pub async fn ensure_catalog(pool: &SqlitePool) -> Result<bool, sqlx::Error> {
    let items: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM catalog_items")
        .fetch_one(pool)
        .await?;
    let objectives: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM catalog_objectives")
        .fetch_one(pool)
        .await?;
    if items > 0 || objectives > 0 {
        return Ok(false);
    }
    let catalog = default_catalog();
    for item in catalog.items {
        apply(pool, PersistOp::CatalogItem(Box::new(item))).await?;
    }
    for objective in catalog.objectives {
        apply(pool, PersistOp::CatalogObjective(Box::new(objective))).await?;
    }
    Ok(true)
}

pub async fn has_event(pool: &SqlitePool) -> Result<bool, sqlx::Error> {
    Ok(sqlx::query("SELECT 1 FROM event LIMIT 1")
        .fetch_optional(pool)
        .await?
        .is_some())
}

pub async fn load(pool: &SqlitePool) -> Result<Option<RaceState>, sqlx::Error> {
    let Some(e) = sqlx::query("SELECT * FROM event LIMIT 1")
        .fetch_optional(pool)
        .await?
    else {
        return Ok(None);
    };
    let event = EventInfo {
        id: e.get("id"),
        name: e.get("name"),
        game: e.get("game"),
        edition: e.get("edition"),
        status: EventStatus::parse(&e.get::<String, _>("status")),
        start_at_utc: e.get("start_at_utc"),
        end_at_utc: e.get("end_at_utc"),
        timezone: e.get("timezone"),
        daily_budget_seconds: e.get("daily_budget_seconds"),
        daily_reset_local_time: e.get("daily_reset_local_time"),
        rules: EventRules {
            win_condition: e.get("win_condition"),
            required_objective_ids: serde_json::from_str(
                &e.get::<String, _>("required_objective_ids"),
            )
            .unwrap_or_default(),
        },
        donation_time: serde_json::from_str(&e.get::<String, _>("donation_time"))
            .unwrap_or_default(),
        rehearsal: e.get("rehearsal"),
    };

    let mut channels: HashMap<String, Vec<Channel>> = HashMap::new();
    for c in sqlx::query("SELECT * FROM channels ORDER BY platform")
        .fetch_all(pool)
        .await?
    {
        if let Some(platform) = Platform::parse(&c.get::<String, _>("platform")) {
            channels
                .entry(c.get("racer_id"))
                .or_default()
                .push(Channel {
                    platform,
                    handle: c.get("handle"),
                    url: c.get("url"),
                });
        }
    }

    let mut donation_ids: HashMap<String, HashSet<String>> = HashMap::new();
    for d in sqlx::query("SELECT racer_id, client_id FROM time_donations")
        .fetch_all(pool)
        .await?
    {
        donation_ids
            .entry(d.get("racer_id"))
            .or_default()
            .insert(d.get("client_id"));
    }

    let mut racers = Vec::new();
    let rows = sqlx::query(
        "SELECT r.*, s.status, s.remaining_ms, s.checkpoint_at, s.reset_at, s.played_ms_total, s.played_today_ms, s.progress_pct,
                s.current_area, s.current_objective, s.completed_objectives, s.items, s.stats, s.finished_at,
                s.final_time_seconds, s.last_heartbeat_at, s.stream_live, s.viewers, s.thumbnail_url,
                s.donation_added_ms, s.donation_removed_ms
         FROM racers r JOIN racer_state s ON s.racer_id = r.id ORDER BY r.sort_order, r.display_name",
    )
    .fetch_all(pool)
    .await?;
    for r in rows {
        let id: String = r.get("id");
        let timezone: String = r.get("timezone");
        let tz: Tz = timezone.parse().unwrap_or(chrono_tz::UTC);
        let stats: RacerStats =
            serde_json::from_str(&r.get::<String, _>("stats")).unwrap_or_default();
        let racer = Racer {
            id: id.clone(),
            display_name: r.get("display_name"),
            slug: r.get("slug"),
            avatar_url: r.get("avatar_url"),
            country: r.get("country"),
            timezone,
            status: RacerStatus::parse(&r.get::<String, _>("status")),
            elapsed_seconds: 0,
            played_today_seconds: 0,
            played_seconds: 0,
            remaining_seconds: 0,
            progress_percentage: r.get("progress_pct"),
            current_area: r.get("current_area"),
            current_objective: r.get("current_objective"),
            completed_objectives: serde_json::from_str(&r.get::<String, _>("completed_objectives"))
                .unwrap_or_default(),
            finished_at_utc: r.get("finished_at"),
            final_time_seconds: r.get("final_time_seconds"),
            channels: channels.remove(&id).unwrap_or_default(),
            stream: Some(StreamState {
                is_live: r.get::<i64, _>("stream_live") != 0,
                thumbnail_url: r.get("thumbnail_url"),
                viewers: r.get("viewers"),
            }),
            stats: Some(stats),
            items: serde_json::from_str(&r.get::<String, _>("items")).unwrap_or_default(),
        };
        racers.push(RacerRuntime {
            racer,
            token_hash: r.get("token_hash"),
            tz,
            checkpoint: Checkpoint {
                remaining_ms: r.get("remaining_ms"),
                at: parse_dt(&r.get::<String, _>("checkpoint_at")),
            },
            reset_at: parse_dt(&r.get::<String, _>("reset_at")),
            played_ms_total: r.get("played_ms_total"),
            played_today_ms: r.get("played_today_ms"),
            last_heartbeat: r
                .get::<Option<String>, _>("last_heartbeat_at")
                .map(|s| parse_dt(&s)),
            ingest: None,
            seen_ids: VecDeque::new(),
            donation_added_ms: r.get("donation_added_ms"),
            donation_removed_ms: r.get("donation_removed_ms"),
            donation_ids: donation_ids.remove(&id).unwrap_or_default(),
            progress_mark: None,
        });
    }

    let mut activity = VecDeque::new();
    for a in sqlx::query("SELECT * FROM activity ORDER BY ts DESC LIMIT 60")
        .fetch_all(pool)
        .await?
    {
        activity.push_back(ActivityItem {
            id: a.get("id"),
            timestamp_utc: a.get("ts"),
            kind: ActivityKind::parse(&a.get::<String, _>("kind")),
            racer_id: a.get("racer_id"),
            racer_name: a.get("racer_name"),
            message: a.get("message"),
            code: a.get("code"),
            detail: a.get("detail"),
            subject: a.get("subject"),
        });
    }

    let stats = match sqlx::query("SELECT * FROM hiveshock_counters WHERE id = 1")
        .fetch_optional(pool)
        .await?
    {
        Some(c) => HiveShockStats {
            connected_racers: 0,
            game_events: c.get("game_events"),
            item_events: c.get("item_events"),
            progress_events: c.get("progress_events"),
            chat_events: c.get("chat_events"),
        },
        None => HiveShockStats::default(),
    };

    let winner = racers
        .iter()
        .filter_map(|r| {
            r.racer.finished_at_utc.as_ref().map(|at| FinishInfo {
                racer_id: r.racer.id.clone(),
                final_time_seconds: r.racer.final_time_seconds,
                finished_at_utc: at.clone(),
            })
        })
        .min_by(|a, b| a.finished_at_utc.cmp(&b.finished_at_utc));

    let catalog = load_catalog(pool).await?;

    Ok(Some(RaceState {
        event,
        racers,
        activity,
        stats,
        winner,
        catalog,
    }))
}

pub async fn audit_tail(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows = sqlx::query("SELECT * FROM audit_log ORDER BY id DESC LIMIT ?")
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<i64, _>("id"),
                "ts": r.get::<String, _>("ts"),
                "actor": r.get::<String, _>("actor"),
                "action": r.get::<String, _>("action"),
                "racerId": r.get::<Option<String>, _>("racer_id"),
                "payload": serde_json::from_str::<serde_json::Value>(&r.get::<String, _>("payload")).unwrap_or_default(),
            })
        })
        .collect())
}

/// Latest time donations, newest first; `racer` narrows it to one racer.
pub async fn donations_tail(
    pool: &SqlitePool,
    racer: Option<&str>,
    limit: i64,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT * FROM time_donations WHERE (?1 IS NULL OR racer_id = ?1) ORDER BY id DESC LIMIT ?2",
    )
    .bind(racer)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<i64, _>("id"),
                "ts": r.get::<String, _>("ts"),
                "racerId": r.get::<String, _>("racer_id"),
                "platform": r.get::<String, _>("platform"),
                "currency": r.get::<String, _>("currency"),
                "amount": r.get::<i64, _>("amount"),
                "gift": r.get::<Option<String>, _>("gift"),
                "giftCount": r.get::<Option<i64>, _>("gift_count"),
                "viewer": r.get::<Option<String>, _>("viewer"),
                "requestedSeconds": r.get::<i64, _>("requested_ms") / 1000,
                "reportedSeconds": r.get::<Option<i64>, _>("reported_ms").map(|ms| ms / 1000),
                "appliedSeconds": r.get::<i64, _>("applied_ms") / 1000,
                "limitedBy": r.get::<Option<String>, _>("limited_by"),
            })
        })
        .collect())
}

/// Whole-event totals per racer: donations, seconds added and seconds removed.
pub async fn donation_totals(pool: &SqlitePool) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT racer_id, COUNT(*) AS n,
                COALESCE(SUM(CASE WHEN applied_ms > 0 THEN applied_ms ELSE 0 END), 0) AS added,
                COALESCE(SUM(CASE WHEN applied_ms < 0 THEN -applied_ms ELSE 0 END), 0) AS removed
         FROM time_donations GROUP BY racer_id",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "racerId": r.get::<String, _>("racer_id"),
                "donations": r.get::<i64, _>("n"),
                "addedSeconds": r.get::<i64, _>("added") / 1000,
                "removedSeconds": r.get::<i64, _>("removed") / 1000,
            })
        })
        .collect())
}

/// Who donated the most. A donor is one viewer name on one platform (the same handle can exist
/// on both). Ranked by the time they moved, the only measure that is comparable between TikTok
/// diamonds and Twitch bits; the amount paid is kept in its own currency. Donations without a
/// viewer name are left out. Donors the organizer hid are skipped unless `include_hidden` (the
/// panel shows everyone and marks them).
pub async fn top_donors(
    pool: &SqlitePool,
    limit: i64,
    include_hidden: bool,
) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT MIN(d.viewer) AS viewer, d.platform, d.currency, COUNT(*) AS n, SUM(d.amount) AS amount,
                COALESCE(SUM(CASE WHEN d.applied_ms > 0 THEN d.applied_ms ELSE 0 END), 0) AS added,
                COALESCE(SUM(CASE WHEN d.applied_ms < 0 THEN -d.applied_ms ELSE 0 END), 0) AS removed,
                COUNT(DISTINCT d.racer_id) AS racers,
                MAX(d.ts) AS last_ts,
                MAX(CASE WHEN h.platform IS NULL THEN 0 ELSE 1 END) AS hidden
         FROM time_donations d
         LEFT JOIN hidden_donors h
                ON h.platform = d.platform AND h.viewer_key = LOWER(TRIM(d.viewer))
         WHERE d.viewer IS NOT NULL AND TRIM(d.viewer) <> ''
           AND (?2 = 1 OR h.platform IS NULL)
         GROUP BY LOWER(TRIM(d.viewer)), d.platform, d.currency
         ORDER BY SUM(ABS(d.applied_ms)) DESC, SUM(d.amount) DESC
         LIMIT ?1",
    )
    .bind(limit)
    .bind(include_hidden)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "viewer": r.get::<String, _>("viewer"),
                "platform": r.get::<String, _>("platform"),
                "currency": r.get::<String, _>("currency"),
                "donations": r.get::<i64, _>("n"),
                "amount": r.get::<i64, _>("amount"),
                "addedSeconds": r.get::<i64, _>("added") / 1000,
                "removedSeconds": r.get::<i64, _>("removed") / 1000,
                "racers": r.get::<i64, _>("racers"),
                "lastAt": r.get::<String, _>("last_ts"),
                "hidden": r.get::<i64, _>("hidden") != 0,
            })
        })
        .collect())
}

/// Event-wide totals of every time donation (hidden donors included: it is an aggregate).
pub async fn donation_summary(pool: &SqlitePool) -> Result<serde_json::Value, sqlx::Error> {
    let r = sqlx::query(
        "SELECT COUNT(*) AS n,
                COALESCE(SUM(CASE WHEN applied_ms > 0 THEN applied_ms ELSE 0 END), 0) AS added,
                COALESCE(SUM(CASE WHEN applied_ms < 0 THEN -applied_ms ELSE 0 END), 0) AS removed
         FROM time_donations",
    )
    .fetch_one(pool)
    .await?;
    Ok(serde_json::json!({
        "donations": r.get::<i64, _>("n"),
        "addedSeconds": r.get::<i64, _>("added") / 1000,
        "removedSeconds": r.get::<i64, _>("removed") / 1000,
    }))
}

/// A small organizer setting, or `None` when it was never set.
pub async fn setting(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
}

pub async fn set_setting(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO app_settings (key, value) VALUES (?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(key)
    .bind(value)
    .execute(pool)
    .await?;
    Ok(())
}

/// Hides (or shows again) one donor on the public board.
pub async fn set_donor_hidden(
    pool: &SqlitePool,
    platform: &str,
    viewer: &str,
    hidden: bool,
) -> Result<(), sqlx::Error> {
    let key = viewer.trim().to_lowercase();
    if hidden {
        sqlx::query("INSERT OR IGNORE INTO hidden_donors (platform, viewer_key) VALUES (?, ?)")
    } else {
        sqlx::query("DELETE FROM hidden_donors WHERE platform = ? AND viewer_key = ?")
    }
    .bind(platform)
    .bind(key)
    .execute(pool)
    .await?;
    Ok(())
}

/// Reserves a one-off notice key. `true` the first time (the notice may be sent), `false` when it
/// was already sent, even before a restart.
pub async fn notice_reserve(
    pool: &SqlitePool,
    key: &str,
    at: DateTime<Utc>,
) -> Result<bool, sqlx::Error> {
    let done = sqlx::query("INSERT OR IGNORE INTO notices_sent (key, sent_at) VALUES (?, ?)")
        .bind(key)
        .bind(rfc(at))
        .execute(pool)
        .await?;
    Ok(done.rows_affected() == 1)
}
