//! Shared application state: the authoritative `RaceState`, the broadcast channel to public
//! WebSocket clients, and the persistence queue. Every mutation runs under one lock and its
//! effects are published before the lock is released, so clients see changes in state order.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};

use chrono::{DateTime, Utc};
use tokio::sync::broadcast;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::auth::{LoginLimiter, constant_eq, hash_token};
use crate::config::Config;
use crate::db::{AuditRow, PersistOp};
use crate::domain::*;
use crate::engine::{
    AdminAction, EventPatch, Fx, IngestEnvelope, IngestError, NewRacer, RacerPatch, Reply, iso,
};
use crate::notify::settings::NotifySettings;
use crate::notify::{Channel, ChannelStatus, LiveInfo, Notice};
use crate::state::{IngestDown, IngestHandle, RaceState};

pub struct Hub {
    state: Mutex<RaceState>,
    tx: broadcast::Sender<Arc<str>>,
    persist: UnboundedSender<PersistOp>,
    pub cfg: Config,
    pub pool: sqlx::SqlitePool,
    conns: AtomicU64,
    /// Failed-login throttles: per username and per client address.
    pub user_limiter: LoginLimiter,
    pub ip_limiter: LoginLimiter,
    /// Where the race engine and the organizer actions drop what is worth telling Discord about.
    /// The dispatcher takes the receiving end once (`take_notice_receiver`).
    notices_tx: UnboundedSender<Notice>,
    notices_rx: Mutex<Option<UnboundedReceiver<Notice>>>,
    /// The organizer's Discord settings, cached (the dispatcher reads them for every notice).
    notify: RwLock<NotifySettings>,
    /// Outcome of the latest posts per Discord channel (shown in the panel).
    notify_status: Mutex<HashMap<Channel, ChannelStatus>>,
}

pub type AppState = Arc<Hub>;

impl Hub {
    pub fn new(
        state: RaceState,
        persist: UnboundedSender<PersistOp>,
        cfg: Config,
        pool: sqlx::SqlitePool,
    ) -> Arc<Self> {
        let (tx, _) = broadcast::channel(512);
        let (notices_tx, notices_rx) = unbounded_channel();
        Arc::new(Self {
            state: Mutex::new(state),
            tx,
            persist,
            cfg,
            pool,
            conns: AtomicU64::new(1),
            user_limiter: LoginLimiter::new(5, std::time::Duration::from_secs(15 * 60)),
            ip_limiter: LoginLimiter::new(30, std::time::Duration::from_secs(15 * 60)),
            notices_tx,
            notices_rx: Mutex::new(Some(notices_rx)),
            notify: RwLock::new(NotifySettings::default()),
            notify_status: Mutex::new(HashMap::new()),
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<str>> {
        self.tx.subscribe()
    }

    pub fn next_conn_id(&self) -> u64 {
        self.conns.fetch_add(1, Ordering::Relaxed)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, RaceState> {
        // A panic while holding the lock must not take the whole race down.
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn read<R>(&self, f: impl FnOnce(&RaceState) -> R) -> R {
        f(&self.lock())
    }

    /// Runs a transition and publishes its effects while still holding the lock.
    fn mutate<R>(&self, now: DateTime<Utc>, f: impl FnOnce(&mut RaceState) -> (Fx, R)) -> R {
        let mut guard = self.lock();
        let (fx, out) = f(&mut guard);
        self.publish(&guard, fx, now);
        out
    }

    fn publish(&self, state: &RaceState, fx: Fx, now: DateTime<Utc>) {
        self.publish_with(state, fx, now, true);
    }

    /// `push_clock`: also send a fresh `CLOCK` to the racer's HiveShock for clock-changing messages.
    /// Off for the racer's own ingest calls, whose replies already carry what they need.
    fn publish_with(&self, state: &RaceState, fx: Fx, now: DateTime<Utc>, push_clock: bool) {
        let stamp = iso(now);
        for msg in &fx.msgs {
            // No receivers is fine (nobody is watching yet).
            let _ = self.tx.send(Arc::from(encode_frame(msg, stamp.clone())));
        }
        for op in fx.ops {
            let _ = self.persist.send(op);
        }
        for notice in fx.notices {
            let _ = self.notices_tx.send(notice);
        }
        for (racer_id, down) in fx.down {
            if let Some(i) = state.idx(&racer_id)
                && let Some(h) = &state.racers[i].ingest
            {
                let _ = h.tx.send(down);
            }
        }
        // HiveShock's overlay must not wait for the next heartbeat after an organizer action or a
        // status change: push the official clock right away to the racer it concerns.
        let mut pushed: Vec<&str> = Vec::new();
        for msg in fx.msgs.iter().filter(|_| push_clock) {
            let Some(racer_id) = clock_affected_racer(msg) else {
                continue;
            };
            if pushed.contains(&racer_id) {
                continue;
            }
            pushed.push(racer_id);
            if let Some(i) = state.idx(racer_id)
                && let Some(h) = &state.racers[i].ingest
            {
                let _ = h.tx.send(IngestDown::Clock(state.clock(i, now)));
            }
        }
    }

    // ---- public reads -------------------------------------------------------------------------

    pub fn event(&self, now: DateTime<Utc>) -> EventInfo {
        self.read(|s| {
            let mut e = s.event.clone();
            e.status = s.event_status(now);
            e
        })
    }

    pub fn racers(&self, now: DateTime<Utc>) -> Vec<Racer> {
        self.read(|s| s.views(now))
    }

    pub fn racer(&self, id: &str, now: DateTime<Utc>) -> Option<Racer> {
        self.read(|s| s.idx(id).map(|i| s.view(i, now)))
    }

    pub fn clocks(&self, now: DateTime<Utc>) -> Vec<ClockState> {
        self.read(|s| s.clocks(now))
    }

    pub fn snapshot_frame(&self, now: DateTime<Utc>) -> String {
        encode_frame(
            &WsMessage::ClockSnapshot {
                clocks: self.clocks(now),
            },
            iso(now),
        )
    }

    pub fn activity(&self) -> Vec<ActivityItem> {
        self.read(|s| s.activity.iter().cloned().collect())
    }

    /// The whole catalog, disabled entries included (organizer view).
    pub fn catalog_all(&self) -> crate::catalog::Catalog {
        self.read(|s| s.catalog.clone())
    }

    /// What the public site and HiveShock see: enabled entries in display order.
    pub fn catalog_public(&self) -> crate::catalog::Catalog {
        self.read(|s| s.catalog.public())
    }

    pub fn catalog_upsert_item(
        &self,
        item: crate::catalog::CatalogItem,
        now: DateTime<Utc>,
    ) -> Result<(), String> {
        self.mutate(now, |s| match s.catalog_upsert_item(item) {
            Ok(fx) => (fx, Ok(())),
            Err(e) => (Fx::default(), Err(e)),
        })
    }

    pub fn catalog_delete_item(&self, id: &str, now: DateTime<Utc>) -> Result<(), String> {
        self.mutate(now, |s| match s.catalog_delete_item(id) {
            Ok(fx) => (fx, Ok(())),
            Err(e) => (Fx::default(), Err(e)),
        })
    }

    pub fn catalog_upsert_objective(
        &self,
        objective: crate::catalog::CatalogObjective,
        now: DateTime<Utc>,
    ) -> Result<(), String> {
        self.mutate(now, |s| match s.catalog_upsert_objective(objective) {
            Ok(fx) => (fx, Ok(())),
            Err(e) => (Fx::default(), Err(e)),
        })
    }

    pub fn catalog_delete_objective(&self, id: &str, now: DateTime<Utc>) -> Result<(), String> {
        self.mutate(now, |s| match s.catalog_delete_objective(id) {
            Ok(fx) => (fx, Ok(())),
            Err(e) => (Fx::default(), Err(e)),
        })
    }

    pub fn stats(&self) -> HiveShockStats {
        self.read(|s| {
            let mut st = s.stats.clone();
            st.connected_racers = s.racers.iter().filter(|r| r.ingest.is_some()).count() as i64;
            st
        })
    }

    pub fn racer_id_for_token(&self, token: &str) -> Option<String> {
        let hash = hash_token(token);
        self.read(|s| {
            s.racers
                .iter()
                .find(|r| constant_eq(&r.token_hash, &hash))
                .map(|r| r.racer.id.clone())
        })
    }

    // ---- ingest -------------------------------------------------------------------------------

    pub fn attach_ingest(
        &self,
        id: &str,
        conn_id: u64,
        tx: tokio::sync::mpsc::UnboundedSender<IngestDown>,
        now: DateTime<Utc>,
    ) {
        self.mutate(now, |s| {
            (
                s.attach_ingest(id, IngestHandle { conn_id, tx })
                    .unwrap_or_default(),
                (),
            )
        });
    }

    pub fn detach_ingest(&self, id: &str, conn_id: u64, now: DateTime<Utc>) {
        self.mutate(now, |s| (s.detach_ingest(id, conn_id), ()));
    }

    pub fn ingest(
        &self,
        id: &str,
        env: IngestEnvelope,
        now: DateTime<Utc>,
    ) -> Result<Reply, IngestError> {
        let mut guard = self.lock();
        match guard.apply_ingest(id, env, now) {
            Ok((fx, reply)) => {
                self.publish_with(&guard, fx, now, false);
                Ok(reply)
            }
            Err(e) => {
                if e == IngestError::Exhausted {
                    // The game is running without time: tell HiveShock to close it.
                    let mut fx = Fx::default();
                    fx.down.push((id.to_string(), IngestDown::ForceClose));
                    self.publish(&guard, fx, now);
                }
                Err(e)
            }
        }
    }

    pub fn tick(&self, now: DateTime<Utc>) {
        let stale = self.cfg.stale_heartbeat_secs;
        self.mutate(now, |s| (s.tick(now, stale), ()));
    }

    // ---- admin --------------------------------------------------------------------------------

    pub fn audit(&self, action: &str, racer_id: Option<&str>, payload: serde_json::Value) {
        self.audit_as("admin", action, racer_id, payload);
    }

    /// The official session log: who (`actor`: a username or `admin-token`) did what.
    pub fn audit_as(
        &self,
        actor: &str,
        action: &str,
        racer_id: Option<&str>,
        payload: serde_json::Value,
    ) {
        let now = Utc::now();
        // Referees hear about what organizers do (who, what and why).
        let racer =
            racer_id.and_then(|id| self.read(|s| s.idx(id).map(|i| LiveInfo::from_state(s, i))));
        if let Some(notice) = crate::notify::from_audit(actor, action, racer, &payload, now) {
            let _ = self.notices_tx.send(notice);
        }
        let _ = self.persist.send(PersistOp::Audit(AuditRow {
            ts: now,
            actor: actor.into(),
            action: action.into(),
            racer_id: racer_id.map(str::to_string),
            payload,
        }));
    }

    // ---- Discord notifications ------------------------------------------------------------------

    /// The dispatcher takes the notices' receiving end once; later calls get `None`.
    pub fn take_notice_receiver(&self) -> Option<UnboundedReceiver<Notice>> {
        self.notices_rx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    pub fn notify_settings(&self) -> NotifySettings {
        self.notify
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn set_notify_settings(&self, settings: NotifySettings) {
        *self.notify.write().unwrap_or_else(|e| e.into_inner()) = settings;
    }

    /// Loads the saved settings (call once at start-up).
    pub async fn load_notify_settings(&self) {
        let settings = crate::notify::settings::load(&self.pool).await;
        self.set_notify_settings(settings);
    }

    pub fn channel_status(&self, channel: Channel) -> ChannelStatus {
        self.notify_status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&channel)
            .cloned()
            .unwrap_or_default()
    }

    pub fn update_channel(&self, channel: Channel, f: impl FnOnce(&mut ChannelStatus)) {
        let mut all = self.notify_status.lock().unwrap_or_else(|e| e.into_inner());
        f(all.entry(channel).or_default());
    }

    /// Remembers how the latest post to a channel went.
    pub fn record_send(&self, channel: Channel, result: &Result<(), String>) {
        self.update_channel(channel, |s| match result {
            Ok(()) => {
                s.last_sent_at = Some(Utc::now());
                s.last_error = None;
            }
            Err(e) => s.last_error = Some(e.clone()),
        });
    }

    pub fn admin_action(
        &self,
        id: &str,
        action: AdminAction,
        now: DateTime<Utc>,
    ) -> Result<(), String> {
        self.mutate(now, |s| match s.admin_action(id, action, now) {
            Ok(fx) => (fx, Ok(())),
            Err(e) => (Fx::default(), Err(e)),
        })
    }

    pub fn update_event(&self, patch: EventPatch, now: DateTime<Utc>) -> Result<EventInfo, String> {
        self.mutate(now, |s| {
            if patch.rehearsal == Some(true)
                && !s.event.rehearsal
                && s.event_status(now) == EventStatus::Live
            {
                return (
                    Fx::default(),
                    Err("the event is live: pause it before turning on rehearsal mode".into()),
                );
            }
            match s.update_event(patch) {
                Ok(fx) => {
                    let mut e = s.event.clone();
                    e.status = s.event_status(now);
                    (fx, Ok(e))
                }
                Err(e) => (Fx::default(), Err(e)),
            }
        })
    }

    /// See `RaceState::reset_event`.
    pub fn reset_event(
        &self,
        start_at_utc: Option<String>,
        leave_rehearsal: bool,
        now: DateTime<Utc>,
    ) -> Result<EventInfo, String> {
        self.mutate(now, |s| {
            match s.reset_event(start_at_utc, leave_rehearsal, now) {
                Ok(fx) => {
                    let mut e = s.event.clone();
                    e.status = s.event_status(now);
                    (fx, Ok(e))
                }
                Err(e) => (Fx::default(), Err(e)),
            }
        })
    }

    pub fn add_racer(&self, n: NewRacer, token: &str, now: DateTime<Utc>) -> Result<Racer, String> {
        let hash = hash_token(token);
        self.mutate(now, |s| {
            let id = n.id.clone();
            match s.add_racer(n, hash, now) {
                Ok(fx) => {
                    let i = s.idx(&id).expect("just added");
                    (fx, Ok(s.view(i, now)))
                }
                Err(e) => (Fx::default(), Err(e)),
            }
        })
    }

    pub fn update_racer(
        &self,
        id: &str,
        p: RacerPatch,
        now: DateTime<Utc>,
    ) -> Result<Racer, String> {
        self.mutate(now, |s| match s.update_racer(id, p, now) {
            Ok(fx) => {
                let i = s.idx(id).expect("exists");
                (fx, Ok(s.view(i, now)))
            }
            Err(e) => (Fx::default(), Err(e)),
        })
    }

    pub fn delete_racer(&self, id: &str, now: DateTime<Utc>) -> Result<(), String> {
        self.mutate(now, |s| match s.delete_racer(id, now) {
            Ok(fx) => (fx, Ok(())),
            Err(e) => (Fx::default(), Err(e)),
        })
    }

    pub fn rotate_token(&self, id: &str, token: &str, now: DateTime<Utc>) -> Result<(), String> {
        let hash = hash_token(token);
        self.mutate(now, |s| match s.rotate_token(id, hash) {
            Ok(fx) => (fx, Ok(())),
            Err(e) => (Fx::default(), Err(e)),
        })
    }
}

/// The racer whose official clock a public message changes (so ingest gets a fresh `CLOCK`).
fn clock_affected_racer(msg: &WsMessage) -> Option<&str> {
    match msg {
        WsMessage::ClockSync { clock } | WsMessage::DailyReset { clock, .. } => {
            Some(clock.racer_id.as_str())
        }
        WsMessage::SessionStarted { racer_id }
        | WsMessage::SessionPaused { racer_id }
        | WsMessage::SessionResumed { racer_id }
        | WsMessage::SessionExhausted { racer_id }
        | WsMessage::RacerStatusChanged { racer_id, .. } => Some(racer_id.as_str()),
        _ => None,
    }
}
