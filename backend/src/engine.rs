//! Game rules. Pure state transitions: `&mut RaceState` in, effects (WebSocket messages,
//! persistence ops, downstream ingest commands) out. No I/O and no clock reads, so every rule
//! is unit-testable with a fixed `now`.

use std::collections::{HashMap, VecDeque};

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use crate::catalog::{CatalogItem, CatalogObjective};
use crate::clock::{Checkpoint, format_hms, next_reset_utc, parse_local_time};
use crate::db::{IdentityRow, PersistOp, RacerStateRow};
use crate::domain::*;
use crate::state::*;

pub fn iso(d: DateTime<Utc>) -> String {
    d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Side effects produced by a transition.
#[derive(Default, Debug)]
pub struct Fx {
    pub msgs: Vec<WsMessage>,
    pub ops: Vec<PersistOp>,
    pub down: Vec<(String, IngestDown)>,
    touched: Vec<usize>,
    stats_dirty: bool,
}

impl Fx {
    fn touch(&mut self, i: usize) {
        if !self.touched.contains(&i) {
            self.touched.push(i);
        }
    }
}

// ---- Ingest protocol (HiveShock -> backend) ---------------------------------------------------

#[derive(Deserialize, Debug, Clone)]
#[serde(
    tag = "type",
    rename_all = "SCREAMING_SNAKE_CASE",
    rename_all_fields = "camelCase"
)]
pub enum IngestMsg {
    Hello {
        client_version: Option<String>,
    },
    Heartbeat {
        game_running: Option<bool>,
    },
    SessionStarted,
    SessionPaused,
    SessionResumed,
    SessionEnded,
    GameProgress {
        progress: GameProgressPatch,
    },
    ItemAcquired {
        item: String,
    },
    AreaChanged {
        area: String,
    },
    BossDefeated {
        boss: String,
    },
    StatsUpdated {
        stats: RacerStats,
    },
    GameFinished,
    ChatEvent {
        count: Option<u32>,
    },
    /// Whether the racer is broadcasting and how many people watch (HiveShock knows: it is
    /// connected to TikTok/Twitch). Valid at any time while connected.
    StreamState {
        live: bool,
        viewers: Option<i64>,
    },
}

#[derive(Deserialize, Debug, Clone)]
pub struct IngestEnvelope {
    /// Optional client event id; a repeated id is acknowledged but not applied twice.
    pub id: Option<String>,
    #[serde(flatten)]
    pub msg: IngestMsg,
}

#[derive(Debug, PartialEq)]
pub enum Reply {
    Ack,
    Clock(ClockState),
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum IngestError {
    #[error("unknown racer")]
    UnknownRacer,
    #[error("the event is not live")]
    EventNotLive,
    #[error("out of sequence: {0}")]
    OutOfSequence(&'static str),
    #[error("invalid payload: {0}")]
    Invalid(String),
    #[error("required objectives are not completed")]
    RequirementsNotMet,
    #[error("daily time is exhausted")]
    Exhausted,
}

impl IngestError {
    pub fn code(&self) -> &'static str {
        match self {
            IngestError::UnknownRacer => "unknown_racer",
            IngestError::EventNotLive => "event_not_live",
            IngestError::OutOfSequence(_) => "out_of_sequence",
            IngestError::Invalid(_) => "invalid",
            IngestError::RequirementsNotMet => "requirements_not_met",
            IngestError::Exhausted => "exhausted",
        }
    }
}

// ---- Admin ------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum AdminAction {
    Pause,
    Resume,
    ForceClose,
    ResetDay,
    AdjustTime { delta_seconds: i64 },
    Finish { final_time_seconds: Option<i64> },
}

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EventPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_at_utc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_at_utc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<EventStatus>,
    pub daily_budget_seconds: Option<i64>,
    pub daily_reset_local_time: Option<String>,
    pub win_condition: Option<String>,
    pub required_objective_ids: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChannelInput {
    pub platform: Platform,
    pub handle: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NewRacer {
    pub id: String,
    pub display_name: String,
    pub timezone: String,
    pub country: Option<String>,
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub channels: Vec<ChannelInput>,
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RacerPatch {
    pub display_name: Option<String>,
    pub timezone: Option<String>,
    pub country: Option<String>,
    pub avatar_url: Option<String>,
    pub channels: Option<Vec<ChannelInput>>,
}

pub fn build_channels(input: &[ChannelInput]) -> Vec<Channel> {
    let mut seen = Vec::new();
    input
        .iter()
        .filter_map(|c| {
            let handle = c.handle.trim().trim_start_matches('@').to_string();
            if handle.is_empty() || seen.contains(&c.platform) {
                return None;
            }
            seen.push(c.platform);
            Some(Channel {
                platform: c.platform,
                url: c.platform.url_for(&handle),
                handle,
            })
        })
        .collect()
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 40
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

// ---- State transitions ------------------------------------------------------------------------

impl RaceState {
    pub fn idx(&self, id: &str) -> Option<usize> {
        self.racers.iter().position(|r| r.racer.id == id)
    }

    fn reset_time(&self) -> chrono::NaiveTime {
        parse_local_time(&self.event.daily_reset_local_time)
            .unwrap_or_else(|| chrono::NaiveTime::from_hms_opt(6, 0, 0).expect("valid time"))
    }

    fn budget_ms(&self) -> i64 {
        self.event.daily_budget_seconds * 1000
    }

    /// Stored `upcoming` follows the schedule; `live`, `paused` and `finished` are explicit.
    pub fn event_status(&self, now: DateTime<Utc>) -> EventStatus {
        match self.event.status {
            EventStatus::Upcoming => match DateTime::parse_from_rfc3339(&self.event.start_at_utc) {
                Ok(start) if now >= start.with_timezone(&Utc) => EventStatus::Live,
                _ => EventStatus::Upcoming,
            },
            other => other,
        }
    }

    pub fn view(&self, i: usize, now: DateTime<Utc>) -> Racer {
        let r = &self.racers[i];
        let mut out = r.racer.clone();
        let rem_ms = r.checkpoint.remaining(r.racer.status.is_running(), now);
        out.remaining_seconds = (rem_ms + 999) / 1000;
        out.elapsed_seconds = (self.event.daily_budget_seconds - out.remaining_seconds).max(0);
        out
    }

    pub fn views(&self, now: DateTime<Utc>) -> Vec<Racer> {
        (0..self.racers.len()).map(|i| self.view(i, now)).collect()
    }

    pub fn clock(&self, i: usize, now: DateTime<Utc>) -> ClockState {
        let r = &self.racers[i];
        ClockState {
            racer_id: r.racer.id.clone(),
            server_time_utc: iso(now),
            remaining_ms: r.checkpoint.remaining(r.racer.status.is_running(), now),
            status: r.racer.status,
            reset_at_utc: iso(r.reset_at),
        }
    }

    pub fn clocks(&self, now: DateTime<Utc>) -> Vec<ClockState> {
        (0..self.racers.len()).map(|i| self.clock(i, now)).collect()
    }

    fn racer_row(&self, i: usize) -> RacerStateRow {
        let r = &self.racers[i];
        RacerStateRow {
            racer_id: r.racer.id.clone(),
            status: r.racer.status,
            remaining_ms: r.checkpoint.remaining_ms,
            checkpoint_at: r.checkpoint.at,
            reset_at: r.reset_at,
            played_ms_total: r.played_ms_total,
            progress_pct: r.racer.progress_percentage,
            current_area: r.racer.current_area.clone(),
            current_objective: r.racer.current_objective.clone(),
            completed_objectives: r.racer.completed_objectives.clone(),
            items: r.racer.items.clone(),
            stats: r.racer.stats.clone().unwrap_or_default(),
            finished_at: r.racer.finished_at_utc.clone(),
            final_time_seconds: r.racer.final_time_seconds,
            last_heartbeat_at: r.last_heartbeat,
            stream: r.racer.stream.clone().unwrap_or_default(),
        }
    }

    /// Adds persistence ops and the stats broadcast for everything touched by a transition.
    fn flush(&mut self, fx: &mut Fx) {
        for i in std::mem::take(&mut fx.touched) {
            if i < self.racers.len() {
                fx.ops
                    .push(PersistOp::RacerState(Box::new(self.racer_row(i))));
            }
        }
        if fx.stats_dirty {
            fx.stats_dirty = false;
            self.stats.connected_racers =
                self.racers.iter().filter(|r| r.ingest.is_some()).count() as i64;
            fx.msgs.push(WsMessage::HiveshockStatsUpdated {
                stats: self.stats.clone(),
            });
            fx.ops.push(PersistOp::Counters(self.stats.clone()));
        }
    }

    /// Moves used time into the running totals and restarts the interval at `now`.
    fn freeze(&mut self, i: usize, now: DateTime<Utc>) {
        let r = &mut self.racers[i];
        if r.racer.status.is_running() {
            let elapsed = (now - r.checkpoint.at).num_milliseconds().max(0);
            let used = elapsed.min(r.checkpoint.remaining_ms.max(0));
            r.played_ms_total += used;
            r.checkpoint.remaining_ms = (r.checkpoint.remaining_ms - used).max(0);
        }
        r.checkpoint.at = now;
    }

    fn set_status_quiet(&mut self, i: usize, status: RacerStatus, now: DateTime<Utc>, fx: &mut Fx) {
        self.freeze(i, now);
        let r = &mut self.racers[i];
        r.racer.status = status;
        let stream = r.racer.stream.get_or_insert_with(StreamState::default);
        // Live = playing, or HiveShock reports a broadcast (`viewers` is `Some` only then).
        stream.is_live =
            matches!(status, RacerStatus::Live | RacerStatus::Paused) || stream.viewers.is_some();
        fx.touch(i);
    }

    fn set_status(&mut self, i: usize, status: RacerStatus, now: DateTime<Utc>, fx: &mut Fx) {
        if self.racers[i].racer.status == status {
            return;
        }
        self.set_status_quiet(i, status, now, fx);
        fx.msgs.push(WsMessage::RacerStatusChanged {
            racer_id: self.racers[i].racer.id.clone(),
            status,
        });
        fx.msgs.push(WsMessage::ClockSync {
            clock: self.clock(i, now),
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn note(
        &mut self,
        fx: &mut Fx,
        now: DateTime<Utc>,
        kind: ActivityKind,
        i: Option<usize>,
        code: &str,
        message: String,
        detail: Option<String>,
        subject: Option<String>,
    ) {
        let item = ActivityItem {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp_utc: iso(now),
            kind,
            racer_id: i.map(|i| self.racers[i].racer.id.clone()),
            racer_name: i.map(|i| self.racers[i].racer.display_name.clone()),
            message,
            code: code.to_string(),
            detail,
            subject,
        };
        self.activity.push_front(item.clone());
        self.activity.truncate(MAX_ACTIVITY);
        fx.msgs.push(WsMessage::LiveActivity {
            activity: item.clone(),
        });
        fx.ops.push(PersistOp::Activity(item));
    }

    fn bump(&mut self, fx: &mut Fx, f: impl FnOnce(&mut HiveShockStats)) {
        f(&mut self.stats);
        fx.stats_dirty = true;
    }

    // ---- ingest -------------------------------------------------------------------------------

    pub fn apply_ingest(
        &mut self,
        racer_id: &str,
        env: IngestEnvelope,
        now: DateTime<Utc>,
    ) -> Result<(Fx, Reply), IngestError> {
        let i = self.idx(racer_id).ok_or(IngestError::UnknownRacer)?;
        let mut fx = Fx::default();

        if let Some(id) = &env.id {
            let seen = &mut self.racers[i].seen_ids;
            if seen.contains(id) {
                return Ok((fx, Reply::Ack));
            }
            seen.push_back(id.clone());
            if seen.len() > 64 {
                seen.pop_front();
            }
        }

        let live_event = self.event_status(now) == EventStatus::Live;
        let status = self.racers[i].racer.status;
        let name = self.racers[i].racer.display_name.clone();
        let playing = matches!(status, RacerStatus::Live | RacerStatus::Paused);
        let mut reply = Reply::Ack;

        match env.msg {
            IngestMsg::Hello { .. } | IngestMsg::Heartbeat { .. } => {
                self.racers[i].last_heartbeat = Some(now);
                fx.touch(i);
                if status == RacerStatus::Offline {
                    self.set_status(i, RacerStatus::Online, now, &mut fx);
                }
                reply = Reply::Clock(self.clock(i, now));
            }
            IngestMsg::SessionStarted | IngestMsg::SessionResumed => {
                if !live_event {
                    return Err(IngestError::EventNotLive);
                }
                match status {
                    RacerStatus::Finished => {
                        return Err(IngestError::OutOfSequence("racer already finished"));
                    }
                    RacerStatus::Exhausted => return Err(IngestError::Exhausted),
                    RacerStatus::Live => return Ok((fx, Reply::Ack)),
                    _ => {}
                }
                if self.racers[i].checkpoint.remaining_ms <= 0 {
                    return Err(IngestError::Exhausted);
                }
                let resumed = status == RacerStatus::Paused;
                self.racers[i].last_heartbeat = Some(now);
                self.set_status(i, RacerStatus::Live, now, &mut fx);
                let id = racer_id.to_string();
                fx.msgs.push(if resumed {
                    WsMessage::SessionResumed { racer_id: id }
                } else {
                    WsMessage::SessionStarted { racer_id: id }
                });
                if resumed {
                    self.note(
                        &mut fx,
                        now,
                        ActivityKind::Status,
                        Some(i),
                        "SESSION_RESUMED",
                        format!("{name} resumed the game"),
                        None,
                        None,
                    );
                }
                self.bump(&mut fx, |s| s.game_events += 1);
            }
            IngestMsg::SessionPaused => {
                if status != RacerStatus::Live {
                    return Err(IngestError::OutOfSequence("session is not running"));
                }
                self.set_status(i, RacerStatus::Paused, now, &mut fx);
                fx.msgs.push(WsMessage::SessionPaused {
                    racer_id: racer_id.to_string(),
                });
                self.note(
                    &mut fx,
                    now,
                    ActivityKind::Status,
                    Some(i),
                    "SESSION_PAUSED",
                    format!("{name} paused the game"),
                    None,
                    None,
                );
                self.bump(&mut fx, |s| s.game_events += 1);
            }
            IngestMsg::SessionEnded => {
                if playing {
                    self.set_status(i, RacerStatus::Online, now, &mut fx);
                }
            }
            IngestMsg::GameProgress { progress } => {
                if !playing {
                    return Err(IngestError::OutOfSequence("no active session"));
                }
                let patch = self.sanitize_progress(progress)?;
                let before_area = self.racers[i].racer.current_area.clone();
                let r = &mut self.racers[i].racer;
                if let Some(p) = patch.percentage {
                    r.progress_percentage = p;
                }
                if let Some(a) = &patch.current_area {
                    r.current_area = Some(a.clone());
                }
                if let Some(o) = &patch.current_objective {
                    r.current_objective = Some(o.clone());
                }
                if let Some(c) = &patch.completed_objectives {
                    r.completed_objectives = c.clone();
                }
                fx.touch(i);
                fx.msgs.push(WsMessage::GameProgress {
                    racer_id: racer_id.to_string(),
                    progress: patch.clone(),
                });
                if let Some(area) = patch
                    .current_area
                    .filter(|a| Some(a) != before_area.as_ref())
                {
                    self.note(
                        &mut fx,
                        now,
                        ActivityKind::Area,
                        Some(i),
                        "AREA_CHANGED",
                        format!("{name} entered {area}"),
                        Some(area.to_uppercase().replace('-', "_")),
                        Some(area),
                    );
                }
                self.bump(&mut fx, |s| {
                    s.game_events += 1;
                    s.progress_events += 1;
                });
            }
            IngestMsg::ItemAcquired { item } => {
                if !playing {
                    return Err(IngestError::OutOfSequence("no active session"));
                }
                if !self.catalog.is_item(&item) {
                    return Err(IngestError::Invalid(format!("unknown item `{item}`")));
                }
                if self.racers[i]
                    .racer
                    .items
                    .get(&item)
                    .copied()
                    .unwrap_or(false)
                {
                    return Ok((fx, Reply::Ack));
                }
                self.racers[i].racer.items.insert(item.clone(), true);
                fx.touch(i);
                fx.msgs.push(WsMessage::ItemAcquired {
                    racer_id: racer_id.to_string(),
                    item: item.clone(),
                });
                self.note(
                    &mut fx,
                    now,
                    ActivityKind::Item,
                    Some(i),
                    "ITEM_ACQUIRED",
                    format!("{name} acquired {item}"),
                    Some(item.to_uppercase().replace('-', "_")),
                    Some(item),
                );
                self.bump(&mut fx, |s| {
                    s.game_events += 1;
                    s.item_events += 1;
                });
            }
            IngestMsg::AreaChanged { area } => {
                if !playing {
                    return Err(IngestError::OutOfSequence("no active session"));
                }
                let area = area.trim().to_string();
                if area.is_empty() || area.len() > 64 {
                    return Err(IngestError::Invalid("area must be 1-64 characters".into()));
                }
                if self.racers[i].racer.current_area.as_deref() == Some(area.as_str()) {
                    return Ok((fx, Reply::Ack));
                }
                self.racers[i].racer.current_area = Some(area.clone());
                fx.touch(i);
                fx.msgs.push(WsMessage::AreaChanged {
                    racer_id: racer_id.to_string(),
                    area: area.clone(),
                });
                self.note(
                    &mut fx,
                    now,
                    ActivityKind::Area,
                    Some(i),
                    "AREA_CHANGED",
                    format!("{name} entered {area}"),
                    Some(area.to_uppercase().replace('-', "_")),
                    Some(area),
                );
                self.bump(&mut fx, |s| s.game_events += 1);
            }
            IngestMsg::BossDefeated { boss } => {
                if !playing {
                    return Err(IngestError::OutOfSequence("no active session"));
                }
                let boss = boss.trim().to_string();
                if boss.is_empty() || boss.len() > 64 {
                    return Err(IngestError::Invalid("boss must be 1-64 characters".into()));
                }
                let stats = self.racers[i]
                    .racer
                    .stats
                    .get_or_insert_with(RacerStats::default);
                let count = stats.bosses_defeated.unwrap_or(0) + 1;
                stats.bosses_defeated = Some(count);
                fx.touch(i);
                fx.msgs.push(WsMessage::BossDefeated {
                    racer_id: racer_id.to_string(),
                    boss: boss.clone(),
                    bosses_defeated: Some(count),
                });
                self.note(
                    &mut fx,
                    now,
                    ActivityKind::Boss,
                    Some(i),
                    "BOSS_DEFEATED",
                    format!("{name} defeated {boss}"),
                    Some(boss.to_uppercase().replace('-', "_")),
                    Some(boss),
                );
                self.bump(&mut fx, |s| s.game_events += 1);
            }
            IngestMsg::StatsUpdated { stats } => {
                if !playing {
                    return Err(IngestError::OutOfSequence("no active session"));
                }
                self.racers[i]
                    .racer
                    .stats
                    .get_or_insert_with(RacerStats::default)
                    .merge(&stats);
                fx.touch(i);
                fx.msgs.push(WsMessage::StatsUpdated {
                    racer_id: racer_id.to_string(),
                    stats,
                });
                self.bump(&mut fx, |s| s.game_events += 1);
            }
            IngestMsg::GameFinished => {
                if !playing {
                    return Err(IngestError::OutOfSequence("no active session"));
                }
                let done = &self.racers[i].racer.completed_objectives;
                if !self
                    .event
                    .rules
                    .required_objective_ids
                    .iter()
                    .all(|id| done.contains(id))
                {
                    return Err(IngestError::RequirementsNotMet);
                }
                self.finish_racer(i, None, now, &mut fx);
                self.bump(&mut fx, |s| s.game_events += 1);
            }
            IngestMsg::StreamState { live, viewers } => {
                let viewers = viewers.map(|v| v.clamp(0, 10_000_000));
                let racer = &mut self.racers[i].racer;
                let stream = racer.stream.get_or_insert_with(StreamState::default);
                let next_viewers = live.then(|| viewers.unwrap_or(0));
                let next_live = live || playing;
                if stream.viewers != next_viewers || stream.is_live != next_live {
                    stream.viewers = next_viewers;
                    stream.is_live = next_live;
                    let update = stream.clone();
                    fx.touch(i);
                    fx.msgs.push(WsMessage::StreamUpdated {
                        racer_id: racer_id.to_string(),
                        stream: update,
                    });
                }
            }
            IngestMsg::ChatEvent { count } => {
                let n = i64::from(count.unwrap_or(1).min(1000));
                self.bump(&mut fx, |s| s.chat_events += n);
            }
        }

        self.flush(&mut fx);
        Ok((fx, reply))
    }

    fn sanitize_progress(
        &self,
        mut p: GameProgressPatch,
    ) -> Result<GameProgressPatch, IngestError> {
        if let Some(pct) = p.percentage {
            if !pct.is_finite() {
                return Err(IngestError::Invalid(
                    "percentage must be a finite number".into(),
                ));
            }
            p.percentage = Some(pct.clamp(0.0, 100.0));
        }
        if let Some(list) = p.completed_objectives.take() {
            let mut out: Vec<String> = Vec::new();
            for id in list {
                if !self.catalog.is_objective(&id) {
                    return Err(IngestError::Invalid(format!("unknown objective `{id}`")));
                }
                if !out.contains(&id) {
                    out.push(id);
                }
            }
            p.completed_objectives = Some(out);
        }
        for text in [&p.current_area, &p.current_objective]
            .into_iter()
            .flatten()
        {
            if text.len() > 64 {
                return Err(IngestError::Invalid(
                    "text fields are limited to 64 characters".into(),
                ));
            }
        }
        Ok(p)
    }

    /// Marks a racer as finished. `final_time_override` is for organizer corrections.
    fn finish_racer(
        &mut self,
        i: usize,
        final_time_override: Option<i64>,
        now: DateTime<Utc>,
        fx: &mut Fx,
    ) {
        self.freeze(i, now);
        let final_time = final_time_override.unwrap_or(self.racers[i].played_ms_total / 1000);
        let finished_at = iso(now);
        {
            let r = &mut self.racers[i].racer;
            r.progress_percentage = 100.0;
            r.finished_at_utc = Some(finished_at.clone());
            r.final_time_seconds = Some(final_time);
        }
        let id = self.racers[i].racer.id.clone();
        fx.msgs.push(WsMessage::GameFinished {
            racer_id: id.clone(),
            final_time_seconds: Some(final_time),
            finished_at_utc: Some(finished_at.clone()),
        });
        self.set_status(i, RacerStatus::Finished, now, fx);
        if self.winner.is_none() {
            self.winner = Some(FinishInfo {
                racer_id: id,
                final_time_seconds: Some(final_time),
                finished_at_utc: finished_at,
            });
        }
        let name = self.racers[i].racer.display_name.clone();
        self.note(
            fx,
            now,
            ActivityKind::Finish,
            Some(i),
            "GAME_FINISHED",
            format!("{name} completed Ocarina of Time"),
            None,
            None,
        );
        fx.touch(i);
    }

    // ---- scheduler ----------------------------------------------------------------------------

    /// Called every second: daily resets, time exhaustion and lost connections.
    pub fn tick(&mut self, now: DateTime<Utc>, stale_secs: i64) -> Fx {
        let mut fx = Fx::default();
        for i in 0..self.racers.len() {
            if self.racers[i].racer.status == RacerStatus::Finished {
                continue;
            }
            if now >= self.racers[i].reset_at {
                self.daily_reset(i, now, &mut fx);
            }
            let r = &self.racers[i];
            if r.racer.status == RacerStatus::Live && r.checkpoint.remaining(true, now) <= 0 {
                self.exhaust(i, now, &mut fx);
            }
            let r = &self.racers[i];
            if let Some(hb) = r.last_heartbeat
                && matches!(
                    r.racer.status,
                    RacerStatus::Online | RacerStatus::Live | RacerStatus::Paused
                )
                && (now - hb).num_seconds() > stale_secs
            {
                // Lost connection: the backend keeps the clock, but it stops counting.
                self.set_status(i, RacerStatus::Offline, now, &mut fx);
            }
        }
        self.flush(&mut fx);
        fx
    }

    fn daily_reset(&mut self, i: usize, now: DateTime<Utc>, fx: &mut Fx) {
        self.freeze(i, now);
        let budget = self.budget_ms();
        let next = next_reset_utc(now, self.racers[i].tz, self.reset_time());
        {
            let r = &mut self.racers[i];
            r.checkpoint = Checkpoint {
                remaining_ms: budget,
                at: now,
            };
            r.reset_at = next;
        }
        if self.racers[i].racer.status == RacerStatus::Exhausted {
            let next_status = if self.racers[i].ingest.is_some() {
                RacerStatus::Online
            } else {
                RacerStatus::Offline
            };
            self.set_status_quiet(i, next_status, now, fx);
        }
        fx.touch(i);
        let id = self.racers[i].racer.id.clone();
        let name = self.racers[i].racer.display_name.clone();
        fx.msgs.push(WsMessage::DailyReset {
            racer_id: id,
            clock: self.clock(i, now),
        });
        self.note(
            fx,
            now,
            ActivityKind::Reset,
            Some(i),
            "DAILY_RESET",
            format!("Daily reset completed for {name}"),
            Some(format!(
                "{} AVAILABLE",
                format_hms(self.event.daily_budget_seconds)
            )),
            None,
        );
    }

    fn exhaust(&mut self, i: usize, now: DateTime<Utc>, fx: &mut Fx) {
        self.freeze(i, now);
        self.racers[i].checkpoint.remaining_ms = 0;
        let id = self.racers[i].racer.id.clone();
        let name = self.racers[i].racer.display_name.clone();
        fx.msgs.push(WsMessage::SessionExhausted {
            racer_id: id.clone(),
        });
        fx.msgs.push(WsMessage::GameForceClose {
            racer_id: id.clone(),
        });
        self.set_status(i, RacerStatus::Exhausted, now, fx);
        fx.down.push((id, IngestDown::ForceClose));
        self.note(
            fx,
            now,
            ActivityKind::Status,
            Some(i),
            "SESSION_EXHAUSTED",
            format!("{name} ran out of time"),
            None,
            None,
        );
    }

    /// After a restart nobody is connected: nothing is counted for downtime.
    pub fn recover(&mut self, now: DateTime<Utc>) {
        for r in &mut self.racers {
            if matches!(
                r.racer.status,
                RacerStatus::Online | RacerStatus::Live | RacerStatus::Paused
            ) {
                r.racer.status = RacerStatus::Offline;
                if let Some(s) = r.racer.stream.as_mut() {
                    s.is_live = false;
                    s.viewers = None;
                }
            }
            r.checkpoint.at = now;
            r.last_heartbeat = None;
        }
    }

    // ---- ingest connections ---------------------------------------------------------------------

    pub fn attach_ingest(&mut self, id: &str, handle: IngestHandle) -> Option<Fx> {
        let i = self.idx(id)?;
        let mut fx = Fx::default();
        if let Some(old) = self.racers[i].ingest.replace(handle) {
            let _ = old.tx.send(IngestDown::Replaced);
        }
        self.stats_changed(&mut fx);
        Some(fx)
    }

    pub fn detach_ingest(&mut self, id: &str, conn_id: u64) -> Fx {
        let mut fx = Fx::default();
        if let Some(i) = self.idx(id)
            && self.racers[i]
                .ingest
                .as_ref()
                .is_some_and(|h| h.conn_id == conn_id)
        {
            self.racers[i].ingest = None;
            // Without HiveShock nobody vouches for the broadcast any more.
            let playing = matches!(
                self.racers[i].racer.status,
                RacerStatus::Live | RacerStatus::Paused
            );
            if let Some(stream) = self.racers[i].racer.stream.as_mut()
                && stream.viewers.is_some()
            {
                stream.viewers = None;
                stream.is_live = playing;
                fx.msgs.push(WsMessage::StreamUpdated {
                    racer_id: id.to_string(),
                    stream: stream.clone(),
                });
                fx.touch(i);
            }
            self.stats_changed(&mut fx);
        }
        fx
    }

    fn stats_changed(&mut self, fx: &mut Fx) {
        fx.stats_dirty = true;
        self.flush(fx);
    }

    // ---- admin ------------------------------------------------------------------------------

    pub fn admin_action(
        &mut self,
        id: &str,
        action: AdminAction,
        now: DateTime<Utc>,
    ) -> Result<Fx, String> {
        let i = self.idx(id).ok_or("racer not found")?;
        let mut fx = Fx::default();
        let name = self.racers[i].racer.display_name.clone();
        let status = self.racers[i].racer.status;
        match action {
            AdminAction::Pause => {
                if status != RacerStatus::Live {
                    return Err("racer is not live".into());
                }
                self.set_status(i, RacerStatus::Paused, now, &mut fx);
                fx.msgs.push(WsMessage::SessionPaused {
                    racer_id: id.into(),
                });
                self.note(
                    &mut fx,
                    now,
                    ActivityKind::Status,
                    Some(i),
                    "SESSION_PAUSED",
                    format!("{name} paused the game"),
                    None,
                    None,
                );
            }
            AdminAction::Resume => {
                if matches!(status, RacerStatus::Finished | RacerStatus::Exhausted) {
                    return Err(format!("cannot resume a {} racer", status.as_str()));
                }
                if self.racers[i].checkpoint.remaining_ms <= 0 {
                    return Err("no time left".into());
                }
                self.set_status(i, RacerStatus::Live, now, &mut fx);
                fx.msgs.push(WsMessage::SessionResumed {
                    racer_id: id.into(),
                });
            }
            AdminAction::ForceClose => {
                fx.msgs.push(WsMessage::GameForceClose {
                    racer_id: id.into(),
                });
                fx.down.push((id.into(), IngestDown::ForceClose));
                if matches!(status, RacerStatus::Live | RacerStatus::Paused) {
                    self.set_status(i, RacerStatus::Online, now, &mut fx);
                }
            }
            AdminAction::ResetDay => {
                self.daily_reset(i, now, &mut fx);
            }
            AdminAction::AdjustTime { delta_seconds } => {
                self.freeze(i, now);
                let max = self.budget_ms() * 2;
                let r = &mut self.racers[i];
                r.checkpoint.remaining_ms =
                    (r.checkpoint.remaining_ms + delta_seconds * 1000).clamp(0, max);
                fx.touch(i);
                if status == RacerStatus::Exhausted && self.racers[i].checkpoint.remaining_ms > 0 {
                    self.set_status(i, RacerStatus::Online, now, &mut fx);
                } else {
                    fx.msgs.push(WsMessage::ClockSync {
                        clock: self.clock(i, now),
                    });
                }
            }
            AdminAction::Finish { final_time_seconds } => {
                if status == RacerStatus::Finished {
                    return Err("racer already finished".into());
                }
                self.finish_racer(i, final_time_seconds, now, &mut fx);
            }
        }
        self.flush(&mut fx);
        Ok(fx)
    }

    // ---- catalog (organizer) ------------------------------------------------------------------

    fn catalog_updated(&self, fx: &mut Fx) {
        fx.msgs.push(WsMessage::CatalogUpdated {
            version: self.catalog.version(),
        });
    }

    pub fn catalog_upsert_item(&mut self, mut item: CatalogItem) -> Result<Fx, String> {
        item.id = item.id.trim().to_string();
        item.name_es = item.name_es.trim().to_string();
        item.name_en = item.name_en.trim().to_string();
        item.short = item.short.trim().to_string();
        item.icon = item
            .icon
            .map(|i| i.trim().to_string())
            .filter(|i| !i.is_empty());
        item.validate()?;
        match self.catalog.items.iter_mut().find(|i| i.id == item.id) {
            Some(existing) => *existing = item.clone(),
            None => self.catalog.items.push(item.clone()),
        }
        let mut fx = Fx::default();
        fx.ops.push(PersistOp::CatalogItem(Box::new(item)));
        self.catalog_updated(&mut fx);
        Ok(fx)
    }

    pub fn catalog_delete_item(&mut self, id: &str) -> Result<Fx, String> {
        let before = self.catalog.items.len();
        self.catalog.items.retain(|i| i.id != id);
        if self.catalog.items.len() == before {
            return Err("item not found".into());
        }
        let mut fx = Fx::default();
        fx.ops.push(PersistOp::DeleteCatalogItem(id.to_string()));
        self.catalog_updated(&mut fx);
        Ok(fx)
    }

    pub fn catalog_upsert_objective(
        &mut self,
        mut objective: CatalogObjective,
    ) -> Result<Fx, String> {
        objective.id = objective.id.trim().to_string();
        objective.name_es = objective.name_es.trim().to_string();
        objective.name_en = objective.name_en.trim().to_string();
        objective.validate()?;
        // A required objective that nobody may report would make the race impossible to finish.
        if !objective.enabled
            && self
                .event
                .rules
                .required_objective_ids
                .contains(&objective.id)
        {
            return Err(
                "this objective is required by the event rules; remove it from the rules first"
                    .into(),
            );
        }
        match self
            .catalog
            .objectives
            .iter_mut()
            .find(|o| o.id == objective.id)
        {
            Some(existing) => *existing = objective.clone(),
            None => self.catalog.objectives.push(objective.clone()),
        }
        let mut fx = Fx::default();
        fx.ops
            .push(PersistOp::CatalogObjective(Box::new(objective)));
        self.catalog_updated(&mut fx);
        Ok(fx)
    }

    pub fn catalog_delete_objective(&mut self, id: &str) -> Result<Fx, String> {
        if self
            .event
            .rules
            .required_objective_ids
            .iter()
            .any(|o| o == id)
        {
            return Err(
                "this objective is required by the event rules; remove it from the rules first"
                    .into(),
            );
        }
        let before = self.catalog.objectives.len();
        self.catalog.objectives.retain(|o| o.id != id);
        if self.catalog.objectives.len() == before {
            return Err("objective not found".into());
        }
        let mut fx = Fx::default();
        fx.ops
            .push(PersistOp::DeleteCatalogObjective(id.to_string()));
        self.catalog_updated(&mut fx);
        Ok(fx)
    }

    pub fn update_event(&mut self, p: EventPatch) -> Result<Fx, String> {
        if let Some(s) = &p.start_at_utc {
            DateTime::parse_from_rfc3339(s).map_err(|_| "startAtUtc must be RFC 3339")?;
            self.event.start_at_utc = s.clone();
        }
        if let Some(s) = &p.end_at_utc {
            DateTime::parse_from_rfc3339(s).map_err(|_| "endAtUtc must be RFC 3339")?;
            self.event.end_at_utc = Some(s.clone());
        }
        if let Some(t) = &p.daily_reset_local_time {
            parse_local_time(t).ok_or("dailyResetLocalTime must be HH:MM")?;
            self.event.daily_reset_local_time = t.clone();
        }
        if let Some(b) = p.daily_budget_seconds {
            if !(60..=86_400).contains(&b) {
                return Err("dailyBudgetSeconds must be between 60 and 86400".into());
            }
            self.event.daily_budget_seconds = b;
        }
        if let Some(ids) = &p.required_objective_ids {
            if let Some(bad) = ids.iter().find(|id| !self.catalog.is_objective(id)) {
                return Err(format!("unknown objective `{bad}`"));
            }
            self.event.rules.required_objective_ids = ids.clone();
        }
        if let Some(n) = p.name {
            self.event.name = n;
        }
        if let Some(w) = p.win_condition {
            self.event.rules.win_condition = w;
        }
        if let Some(s) = p.status {
            self.event.status = s;
        }
        let mut fx = Fx::default();
        fx.ops.push(PersistOp::Event(self.event.clone()));
        Ok(fx)
    }

    pub fn add_racer(
        &mut self,
        n: NewRacer,
        token_hash: String,
        now: DateTime<Utc>,
    ) -> Result<Fx, String> {
        if !valid_id(&n.id) {
            return Err("id must be 1-40 chars of a-z, 0-9 or '-'".into());
        }
        if self.idx(&n.id).is_some() {
            return Err("a racer with that id already exists".into());
        }
        let tz: Tz = n
            .timezone
            .parse()
            .map_err(|_| "timezone must be an IANA name")?;
        let channels = build_channels(&n.channels);
        let racer = Racer {
            id: n.id.clone(),
            display_name: n.display_name.clone(),
            slug: n.id.clone(),
            avatar_url: n.avatar_url,
            country: n.country,
            timezone: n.timezone,
            status: RacerStatus::Offline,
            elapsed_seconds: 0,
            remaining_seconds: self.event.daily_budget_seconds,
            progress_percentage: 0.0,
            current_area: None,
            current_objective: None,
            completed_objectives: vec![],
            finished_at_utc: None,
            final_time_seconds: None,
            channels,
            stream: Some(StreamState::default()),
            stats: Some(RacerStats::default()),
            items: HashMap::new(),
        };
        self.racers.push(RacerRuntime {
            racer,
            token_hash,
            tz,
            checkpoint: Checkpoint {
                remaining_ms: self.budget_ms(),
                at: now,
            },
            reset_at: next_reset_utc(now, tz, self.reset_time()),
            played_ms_total: 0,
            last_heartbeat: None,
            ingest: None,
            seen_ids: VecDeque::new(),
        });
        let i = self.racers.len() - 1;
        let mut fx = Fx::default();
        fx.ops
            .push(PersistOp::Identity(Box::new(self.identity_row(i))));
        fx.touch(i);
        fx.stats_dirty = true;
        self.flush(&mut fx);
        fx.msgs.push(WsMessage::ClockSnapshot {
            clocks: self.clocks(now),
        });
        Ok(fx)
    }

    pub fn update_racer(
        &mut self,
        id: &str,
        p: RacerPatch,
        now: DateTime<Utc>,
    ) -> Result<Fx, String> {
        let i = self.idx(id).ok_or("racer not found")?;
        if let Some(tz) = &p.timezone {
            let parsed: Tz = tz.parse().map_err(|_| "timezone must be an IANA name")?;
            self.racers[i].tz = parsed;
            self.racers[i].reset_at = next_reset_utc(now, parsed, self.reset_time());
            self.racers[i].racer.timezone = tz.clone();
        }
        let r = &mut self.racers[i].racer;
        if let Some(n) = p.display_name {
            r.display_name = n;
        }
        if p.country.is_some() {
            r.country = p.country;
        }
        if p.avatar_url.is_some() {
            r.avatar_url = p.avatar_url;
        }
        if let Some(ch) = &p.channels {
            r.channels = build_channels(ch);
        }
        let mut fx = Fx::default();
        fx.ops
            .push(PersistOp::Identity(Box::new(self.identity_row(i))));
        fx.touch(i);
        self.flush(&mut fx);
        fx.msgs.push(WsMessage::ClockSnapshot {
            clocks: self.clocks(now),
        });
        Ok(fx)
    }

    pub fn delete_racer(&mut self, id: &str, now: DateTime<Utc>) -> Result<Fx, String> {
        let i = self.idx(id).ok_or("racer not found")?;
        if let Some(h) = self.racers[i].ingest.take() {
            let _ = h.tx.send(IngestDown::Replaced);
        }
        self.racers.remove(i);
        let mut fx = Fx::default();
        fx.ops.push(PersistOp::DeleteRacer(id.to_string()));
        fx.stats_dirty = true;
        self.flush(&mut fx);
        fx.msgs.push(WsMessage::ClockSnapshot {
            clocks: self.clocks(now),
        });
        Ok(fx)
    }

    pub fn rotate_token(&mut self, id: &str, token_hash: String) -> Result<Fx, String> {
        let i = self.idx(id).ok_or("racer not found")?;
        self.racers[i].token_hash = token_hash;
        // The old token is dead: drop any live connection that used it.
        if let Some(h) = self.racers[i].ingest.take() {
            let _ = h.tx.send(IngestDown::Replaced);
        }
        let mut fx = Fx::default();
        fx.ops
            .push(PersistOp::Identity(Box::new(self.identity_row(i))));
        fx.stats_dirty = true;
        self.flush(&mut fx);
        Ok(fx)
    }

    pub fn identity_row(&self, i: usize) -> IdentityRow {
        let r = &self.racers[i];
        IdentityRow {
            id: r.racer.id.clone(),
            slug: r.racer.slug.clone(),
            display_name: r.racer.display_name.clone(),
            country: r.racer.country.clone(),
            avatar_url: r.racer.avatar_url.clone(),
            timezone: r.racer.timezone.clone(),
            token_hash: r.token_hash.clone(),
            sort_order: i as i64,
            channels: r.racer.channels.clone(),
        }
    }
}
