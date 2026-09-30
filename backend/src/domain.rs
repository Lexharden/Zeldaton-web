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

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RacerStats {
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
