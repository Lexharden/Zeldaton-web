//! Wire types. These mirror `src/types/*.ts` in the frontend one to one (camelCase JSON);
//! the `contract/` fixtures keep both sides honest.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum EventStatus {
    Upcoming,
    Live,
    Paused,
    Finished,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum RacerStatus {
    Online,
    Live,
    Paused,
    Offline,
    Exhausted,
    Finished,
}

impl RacerStatus {
    /// The daily clock only counts down while the game is actually being played.
    pub fn is_running(self) -> bool {
        self == RacerStatus::Live
    }

    pub fn as_str(self) -> &'static str {
        match self {
            RacerStatus::Online => "online",
            RacerStatus::Live => "live",
            RacerStatus::Paused => "paused",
            RacerStatus::Offline => "offline",
            RacerStatus::Exhausted => "exhausted",
            RacerStatus::Finished => "finished",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "online" => RacerStatus::Online,
            "live" => RacerStatus::Live,
            "paused" => RacerStatus::Paused,
            "exhausted" => RacerStatus::Exhausted,
            "finished" => RacerStatus::Finished,
            _ => RacerStatus::Offline,
        }
    }
}

impl EventStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            EventStatus::Upcoming => "upcoming",
            EventStatus::Live => "live",
            EventStatus::Paused => "paused",
            EventStatus::Finished => "finished",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "live" => EventStatus::Live,
            "paused" => EventStatus::Paused,
            "finished" => EventStatus::Finished,
            _ => EventStatus::Upcoming,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Twitch,
    Tiktok,
    Youtube,
}

impl Platform {
    pub fn as_str(self) -> &'static str {
        match self {
            Platform::Twitch => "twitch",
            Platform::Tiktok => "tiktok",
            Platform::Youtube => "youtube",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "twitch" => Some(Platform::Twitch),
            "tiktok" => Some(Platform::Tiktok),
            "youtube" => Some(Platform::Youtube),
            _ => None,
        }
    }

    pub fn url_for(self, handle: &str) -> String {
        match self {
            Platform::Twitch => format!("https://twitch.tv/{handle}"),
            Platform::Tiktok => format!("https://tiktok.com/@{handle}"),
            Platform::Youtube => format!("https://youtube.com/@{handle}"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EventRules {
    pub win_condition: String,
    pub required_objective_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EventInfo {
    pub id: String,
    pub name: String,
    pub game: String,
    pub edition: String,
    pub status: EventStatus,
    pub start_at_utc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_at_utc: Option<String>,
    pub timezone: String,
    pub daily_budget_seconds: i64,
    pub daily_reset_local_time: String,
    pub rules: EventRules,
    /// How much time viewer donations may add or remove (the streamer sets the rate in HiveShock).
    #[serde(default)]
    pub donation_time: DonationTimePolicy,
    /// Test run: the site shows a notice and the organizer can reset everything before the real event.
    #[serde(default)]
    pub rehearsal: bool,
}

/// Organizer limits for time from donations. HiveShock converts TikTok diamonds / Twitch bits into
/// seconds with the streamer's own rate; the server applies them only within these limits.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct DonationTimePolicy {
    pub enabled: bool,
    /// Donations may add time.
    pub allow_add: bool,
    /// Donations may take time away.
    pub allow_remove: bool,
    /// Largest change a single donation can make (seconds).
    pub max_seconds_per_donation: i64,
    /// Most time donations can add to one racer per day (seconds; resets with the daily budget).
    pub max_added_seconds_per_day: i64,
    /// Most time donations can remove from one racer per day (seconds).
    pub max_removed_seconds_per_day: i64,
    /// Seconds one TikTok diamond is worth. When set, **the server** computes the time of a
    /// diamond donation (`amount` x this), keeping only the direction (add or remove) that HiveShock
    /// sent, so every racer has the same rate whatever their HiveShock says. `None`: HiveShock's number.
    pub seconds_per_diamond: Option<i64>,
    /// The same for Twitch bits.
    pub seconds_per_bit: Option<i64>,
}

impl Default for DonationTimePolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            allow_add: true,
            allow_remove: true,
            max_seconds_per_donation: 3600,
            max_added_seconds_per_day: 4 * 3600,
            max_removed_seconds_per_day: 4 * 3600,
            seconds_per_diamond: Some(3),
            seconds_per_bit: None,
        }
    }
}

impl DonationTimePolicy {
    /// Upper bound for every limit: two days of play.
    pub const MAX_SECONDS: i64 = 2 * 86_400;

    pub fn validate(&self) -> Result<(), String> {
        if !(1..=Self::MAX_SECONDS).contains(&self.max_seconds_per_donation) {
            return Err("maxSecondsPerDonation must be between 1 and 172800".into());
        }
        for (name, rate) in [
            ("secondsPerDiamond", self.seconds_per_diamond),
            ("secondsPerBit", self.seconds_per_bit),
        ] {
            if rate.is_some_and(|r| !(1..=3600).contains(&r)) {
                return Err(format!("{name} must be between 1 and 3600"));
            }
        }
        for (name, v) in [
            ("maxAddedSecondsPerDay", self.max_added_seconds_per_day),
            ("maxRemovedSecondsPerDay", self.max_removed_seconds_per_day),
        ] {
            if !(0..=Self::MAX_SECONDS).contains(&v) {
                return Err(format!("{name} must be between 0 and 172800"));
            }
        }
        Ok(())
    }
}

/// What a donation was paid in.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum DonationCurrency {
    /// TikTok gifts.
    Diamonds,
    /// Twitch cheers.
    Bits,
}

impl DonationCurrency {
    pub fn as_str(self) -> &'static str {
        match self {
            DonationCurrency::Diamonds => "diamonds",
            DonationCurrency::Bits => "bits",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub platform: Platform,
    pub handle: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StreamState {
    pub is_live: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewers: Option<i64>,
}

/// Which Link the racer is playing right now.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum LinkAge {
    Child,
    Adult,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RacerStats {
    /// The Link being played (child or adult).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub age: Option<LinkAge>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hearts: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_hearts: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rupees: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skulltulas: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bosses_defeated: Option<i64>,
}

impl RacerStats {
    /// Overlays the fields present in `patch` (telemetry is partial by design).
    pub fn merge(&mut self, patch: &RacerStats) {
        if patch.hearts.is_some() {
            self.hearts = patch.hearts;
        }
        if patch.max_hearts.is_some() {
            self.max_hearts = patch.max_hearts;
        }
        if patch.rupees.is_some() {
            self.rupees = patch.rupees;
        }
        if patch.skulltulas.is_some() {
            self.skulltulas = patch.skulltulas;
        }
        if patch.bosses_defeated.is_some() {
            self.bosses_defeated = patch.bosses_defeated;
        }
        if patch.age.is_some() {
            self.age = patch.age;
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Racer {
    pub id: String,
    pub display_name: String,
    pub slug: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    pub timezone: String,
    pub status: RacerStatus,
    pub elapsed_seconds: i64,
    /// Time really played (the game running), since the last daily reset and over the whole event.
    /// Unlike `elapsed_seconds` (the daily budget minus what is left) donations and organizer
    /// adjustments do not change it.
    #[serde(default)]
    pub played_today_seconds: i64,
    #[serde(default)]
    pub played_seconds: i64,
    pub remaining_seconds: i64,
    pub progress_percentage: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_area: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_objective: Option<String>,
    pub completed_objectives: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at_utc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_time_seconds: Option<i64>,
    pub channels: Vec<Channel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<StreamState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<RacerStats>,
    pub items: HashMap<String, bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StandingEntry {
    pub racer_id: String,
    pub rank: u32,
}

/// Authoritative clock snapshot. The frontend only interpolates it between snapshots.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClockState {
    pub racer_id: String,
    pub server_time_utc: String,
    pub remaining_ms: i64,
    pub status: RacerStatus,
    pub reset_at_utc: String,
    /// Time really played today / in total, as of `server_time_utc` (it keeps growing while the
    /// clock runs, like `remaining_ms` keeps shrinking).
    #[serde(default)]
    pub played_today_ms: i64,
    #[serde(default)]
    pub played_total_ms: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StreamInfo {
    pub racer_id: String,
    pub is_live: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub viewers: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ActivityKind {
    Area,
    Item,
    Boss,
    Status,
    Reset,
    Finish,
    /// Time added or removed by viewer donations.
    Time,
    System,
}

impl ActivityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ActivityKind::Area => "area",
            ActivityKind::Item => "item",
            ActivityKind::Boss => "boss",
            ActivityKind::Status => "status",
            ActivityKind::Reset => "reset",
            ActivityKind::Finish => "finish",
            ActivityKind::Time => "time",
            ActivityKind::System => "system",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s {
            "area" => ActivityKind::Area,
            "item" => ActivityKind::Item,
            "boss" => ActivityKind::Boss,
            "status" => ActivityKind::Status,
            "reset" => ActivityKind::Reset,
            "finish" => ActivityKind::Finish,
            "time" => ActivityKind::Time,
            _ => ActivityKind::System,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActivityItem {
    pub id: String,
    pub timestamp_utc: String,
    pub kind: ActivityKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub racer_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub racer_name: Option<String>,
    /// English fallback sentence; clients compose localized text from `code` + `subject`.
    pub message: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HiveShockStats {
    pub connected_racers: i64,
    pub game_events: i64,
    pub item_events: i64,
    pub progress_events: i64,
    pub chat_events: i64,
}

/// Partial game progress; every field is optional so telemetry can grow incrementally.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GameProgressPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percentage: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_area: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_objective: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_objectives: Option<Vec<String>>,
}

/// Public WebSocket messages. Tag names match `src/types/websocket.ts` exactly.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(
    tag = "type",
    rename_all = "SCREAMING_SNAKE_CASE",
    rename_all_fields = "camelCase"
)]
pub enum WsMessage {
    ClockSnapshot {
        clocks: Vec<ClockState>,
    },
    ClockSync {
        clock: ClockState,
    },
    DailyReset {
        racer_id: String,
        clock: ClockState,
    },
    SessionStarted {
        racer_id: String,
    },
    SessionPaused {
        racer_id: String,
    },
    SessionResumed {
        racer_id: String,
    },
    SessionExhausted {
        racer_id: String,
    },
    GameForceClose {
        racer_id: String,
    },
    GameProgress {
        racer_id: String,
        progress: GameProgressPatch,
    },
    ItemAcquired {
        racer_id: String,
        item: String,
    },
    AreaChanged {
        racer_id: String,
        area: String,
    },
    BossDefeated {
        racer_id: String,
        boss: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        bosses_defeated: Option<i64>,
    },
    StatsUpdated {
        racer_id: String,
        stats: RacerStats,
    },
    GameFinished {
        racer_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        final_time_seconds: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        finished_at_utc: Option<String>,
    },
    RacerStatusChanged {
        racer_id: String,
        status: RacerStatus,
    },
    LiveActivity {
        activity: ActivityItem,
    },
    HiveshockStatsUpdated {
        stats: HiveShockStats,
    },
    StreamUpdated {
        racer_id: String,
        stream: StreamState,
    },
    /// The organizer changed the catalog: clients reload it. `version` is the new content hash.
    CatalogUpdated {
        version: String,
    },
    /// The event itself changed in a way live patches cannot express (reset, rehearsal switched):
    /// clients reload everything.
    EventUpdated,
}

/// Wire frame: the message plus a top-level `serverTimeUtc`, which the frontend socket
/// reads on every message to keep its server-time offset fresh.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame<'a> {
    #[serde(flatten)]
    pub message: &'a WsMessage,
    pub server_time_utc: String,
}

pub fn encode_frame(message: &WsMessage, server_time_utc: String) -> String {
    serde_json::to_string(&Frame {
        message,
        server_time_utc,
    })
    .expect("frame serializes")
}

/// Client -> server messages on the public socket.
#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PublicClientMessage {
    ClockSyncRequest,
    Ping,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn frame_has_tag_camel_case_fields_and_server_time() {
        let msg = WsMessage::GameProgress {
            racer_id: "ralbat".into(),
            progress: GameProgressPatch {
                percentage: Some(76.0),
                ..Default::default()
            },
        };
        let v: serde_json::Value = serde_json::from_str(&encode_frame(&msg, "T".into())).unwrap();
        assert_eq!(
            v,
            json!({"type":"GAME_PROGRESS","racerId":"ralbat","progress":{"percentage":76.0},"serverTimeUtc":"T"})
        );
    }

    #[test]
    fn hiveshock_stats_tag_matches_frontend() {
        let msg = WsMessage::HiveshockStatsUpdated {
            stats: HiveShockStats::default(),
        };
        let v: serde_json::Value = serde_json::from_str(&encode_frame(&msg, "T".into())).unwrap();
        assert_eq!(v["type"], "HIVESHOCK_STATS_UPDATED");
        assert_eq!(v["stats"]["connectedRacers"], 0);
    }
}
