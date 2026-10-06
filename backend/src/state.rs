//! In-memory authoritative state. All mutation goes through `engine.rs`.

use std::collections::{HashSet, VecDeque};

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use tokio::sync::mpsc::UnboundedSender;

use crate::catalog::Catalog;
use crate::clock::Checkpoint;
use crate::domain::{ActivityItem, ClockState, EventInfo, HiveShockStats, Racer};

/// Messages pushed down an open HiveShock ingest connection.
#[derive(Clone, Debug, PartialEq)]
pub enum IngestDown {
    Clock(ClockState),
    /// The daily time is gone: the game must close.
    ForceClose,
    /// Another connection took over for this racer.
    Replaced,
}

#[derive(Clone)]
pub struct IngestHandle {
    pub conn_id: u64,
    pub tx: UnboundedSender<IngestDown>,
}

pub struct RacerRuntime {
    /// Identity + telemetry. `status` is authoritative; times are derived on read.
    pub racer: Racer,
    pub token_hash: String,
    pub tz: Tz,
    pub checkpoint: Checkpoint,
    pub reset_at: DateTime<Utc>,
    /// Total time actually played across all days; becomes `finalTimeSeconds`.
    pub played_ms_total: i64,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub ingest: Option<IngestHandle>,
    /// Recent client event ids, for idempotent retries.
    pub seen_ids: VecDeque<String>,
    /// Time donations already added / removed today (daily caps). Reset with the budget.
    pub donation_added_ms: i64,
    pub donation_removed_ms: i64,
    /// Every donation id ever applied (from the ledger): a retry is never applied twice, even
    /// after a server restart, which the short `seen_ids` window cannot guarantee.
    pub donation_ids: HashSet<String>,
    /// Reference point to spot a sudden jump in progress: (when, percentage).
    pub progress_mark: Option<(DateTime<Utc>, f64)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FinishInfo {
    pub racer_id: String,
    pub final_time_seconds: Option<i64>,
    pub finished_at_utc: String,
}

pub struct RaceState {
    pub event: EventInfo,
    pub racers: Vec<RacerRuntime>,
    pub activity: VecDeque<ActivityItem>,
    pub stats: HiveShockStats,
    pub winner: Option<FinishInfo>,
    /// Items and objectives racers may report (edited from the organizer panel).
    pub catalog: Catalog,
}

pub const MAX_ACTIVITY: usize = 60;
