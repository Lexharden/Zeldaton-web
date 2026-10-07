//! Discord notifications: what happened in the race, told to the community and to the referees.
//!
//! One typed pipeline, three sources, one dispatcher:
//!
//! * **sources** produce [`Notice`]s: the race engine (a boss was defeated, someone finished or ran
//!   out of time...), the organizer actions (`Hub::audit_as`) and a periodic scanner for what
//!   depends on time (went live, took the lead, disconnected for long, low on time);
//! * the **dispatcher** ([`dispatch`]) applies the policy (switches, de-duplication that survives
//!   restarts, cooldowns, rehearsal), renders the message ([`render`]) and queues it per channel;
//! * each channel has an **outbox** ([`outbox`]) with a priority queue and a token bucket under
//!   Discord's rate limit, and one worker that posts the messages in order ([`client`]).
//!
//! Nothing here touches the network while the race lock is held. Adding a notification is one
//! variant in [`Kind`] and [`Detail`], one place that emits it and one arm in [`render`]; the
//! organizer panel lists the kinds from the API, so it needs no change.

use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod client;
pub mod dispatch;
pub mod outbox;
pub mod render;
pub mod scan;
pub mod settings;
pub mod text;

pub use text::LiveInfo;

/// Where a notification goes: the community channel or the referees' private one.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Public,
    Staff,
}

impl Channel {
    pub const ALL: [Channel; 2] = [Channel::Public, Channel::Staff];

    pub fn as_str(self) -> &'static str {
        match self {
            Channel::Public => "public",
            Channel::Staff => "staff",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.as_str() == s)
    }
}

/// How a channel is doing, shown in the organizer panel.
#[derive(Clone, Debug, Default)]
pub struct ChannelStatus {
    pub last_sent_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    /// Notices thrown away because the queue was full.
    pub dropped: u64,
    /// Messages waiting to be posted.
    pub queued: usize,
}

/// Lower sorts first: when a channel is saturated the low priority notices are dropped first.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Priority {
    High,
    Normal,
    Low,
}

/// Every kind of notification. The registry below is the single place that describes them.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Live,
    Winner,
    Exhausted,
    Boss,
    Leader,
    Disconnected,
    LowTime,
    DonationCap,
    Suspicious,
    EventState,
    OrganizerAction,
}

impl Kind {
    pub const ALL: [Kind; 11] = [
        Kind::Live,
        Kind::Winner,
        Kind::Exhausted,
        Kind::Boss,
        Kind::Leader,
        Kind::Disconnected,
        Kind::LowTime,
        Kind::DonationCap,
        Kind::Suspicious,
        Kind::EventState,
        Kind::OrganizerAction,
    ];

    /// Stable id used in the settings and the API.
    pub fn key(self) -> &'static str {
        match self {
            Kind::Live => "live",
            Kind::Winner => "winner",
            Kind::Exhausted => "exhausted",
            Kind::Boss => "boss",
            Kind::Leader => "leader",
            Kind::Disconnected => "disconnected",
            Kind::LowTime => "low_time",
            Kind::DonationCap => "donation_cap",
            Kind::Suspicious => "suspicious",
            Kind::EventState => "event_state",
            Kind::OrganizerAction => "organizer_action",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.key() == s)
    }

    pub fn channel(self) -> Channel {
        match self {
            Kind::Live | Kind::Winner | Kind::Exhausted | Kind::Boss | Kind::Leader => {
                Channel::Public
            }
            _ => Channel::Staff,
        }
    }

    pub fn priority(self) -> Priority {
        match self {
            Kind::Winner
            | Kind::Exhausted
            | Kind::Disconnected
            | Kind::LowTime
            | Kind::Suspicious
            | Kind::EventState => Priority::High,
            Kind::Boss | Kind::Live | Kind::DonationCap | Kind::OrganizerAction => Priority::Normal,
            Kind::Leader => Priority::Low,
        }
    }

    /// Shown in the organizer panel.
    pub fn label(self) -> &'static str {
        match self {
            Kind::Live => "Un corredor entra en vivo",
            Kind::Winner => "Un corredor termina el juego (ganador)",
            Kind::Exhausted => "A un corredor se le acaba el tiempo",
            Kind::Boss => "Un corredor derrota a un jefe",
            Kind::Leader => "Cambia el líder de la carrera",
            Kind::Disconnected => "Un corredor lleva mucho tiempo desconectado",
            Kind::LowTime => "A un corredor le queda poco tiempo",
            Kind::DonationCap => "Se agota el tope diario de donaciones de un corredor",
            Kind::Suspicious => "Progreso sospechoso (salto brusco)",
            Kind::EventState => "El evento se inicia, pausa, reanuda, finaliza o reinicia",
            Kind::OrganizerAction => "Un organizador actúa sobre un corredor",
        }
    }

    /// Critical notices may mention the referees' role.
    pub fn critical(self) -> bool {
        matches!(self, Kind::Disconnected | Kind::LowTime | Kind::Suspicious)
    }

    /// The same racer is not notified of this kind again within this time (in addition to the
    /// idempotency keys that make a one-off event fire once).
    pub fn cooldown(self) -> Option<Duration> {
        match self {
            Kind::Live => Some(Duration::from_secs(30 * 60)),
            Kind::Leader => Some(Duration::from_secs(5 * 60)),
            Kind::Suspicious => Some(Duration::from_secs(5 * 60)),
            _ => None,
        }
    }
}

/// What a notice says. Each variant belongs to exactly one [`Kind`].
#[derive(Clone, Debug, PartialEq)]
pub enum Detail {
    Live,
    /// `place` 1 is the winner.
    Winner {
        place: u32,
        final_seconds: Option<i64>,
    },
    Exhausted,
    Boss {
        boss: String,
        count: Option<i64>,
    },
    Leader {
        previous: Option<String>,
    },
    /// `back`: the racer reconnected after an alert.
    Disconnected {
        minutes: i64,
        back: bool,
    },
    LowTime {
        minutes_left: i64,
    },
    DonationCap {
        adding: bool,
        limit_seconds: i64,
        viewer: Option<String>,
    },
    /// Candidate from the engine (any notable rise); the dispatcher applies the organizer's thresholds.
    Jump {
        from: f64,
        to: f64,
        seconds: i64,
    },
    EventState {
        state: String,
        actor: String,
        reason: Option<String>,
    },
    Organizer {
        actor: String,
        action: String,
        reason: Option<String>,
        detail: Option<String>,
    },
}

impl Detail {
    pub fn kind(&self) -> Kind {
        match self {
            Detail::Live => Kind::Live,
            Detail::Winner { .. } => Kind::Winner,
            Detail::Exhausted => Kind::Exhausted,
            Detail::Boss { .. } => Kind::Boss,
            Detail::Leader { .. } => Kind::Leader,
            Detail::Disconnected { .. } => Kind::Disconnected,
            Detail::LowTime { .. } => Kind::LowTime,
            Detail::DonationCap { .. } => Kind::DonationCap,
            Detail::Jump { .. } => Kind::Suspicious,
            Detail::EventState { .. } => Kind::EventState,
            Detail::Organizer { .. } => Kind::OrganizerAction,
        }
    }
}

/// One thing worth telling somebody.
#[derive(Clone, Debug)]
pub struct Notice {
    /// The racer concerned, as they were when it happened.
    pub racer: Option<LiveInfo>,
    pub detail: Detail,
    /// Idempotency key: a notice with a key that was already sent is dropped, even after a restart
    /// (and the keys are forgotten when the event is reset).
    pub dedupe: Option<String>,
    pub at: DateTime<Utc>,
}

impl Notice {
    pub fn new(racer: Option<LiveInfo>, detail: Detail, at: DateTime<Utc>) -> Self {
        Self {
            racer,
            detail,
            dedupe: None,
            at,
        }
    }

    pub fn keyed(mut self, key: impl Into<String>) -> Self {
        self.dedupe = Some(key.into());
        self
    }

    pub fn kind(&self) -> Kind {
        self.detail.kind()
    }
}

/// The organizer actions (see `Hub::audit_as`) that referees want to hear about.
pub fn from_audit(
    actor: &str,
    action: &str,
    racer: Option<LiveInfo>,
    payload: &serde_json::Value,
    at: DateTime<Utc>,
) -> Option<Notice> {
    let reason = payload["reason"]
        .as_str()
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .map(str::to_string);
    if let Some(state) = action.strip_prefix("event.") {
        let label = match state {
            "start" => "iniciado",
            "pause" => "pausado",
            "resume" => "reanudado",
            "finish" => "finalizado",
            "reset" => "reiniciado",
            _ => return None,
        };
        return Some(Notice::new(
            None,
            Detail::EventState {
                state: label.into(),
                actor: actor.into(),
                reason,
            },
            at,
        ));
    }
    let racer_action = action.strip_prefix("racer.")?;
    let (label, detail) = match racer_action {
        "pause" => ("pausó el juego de", None),
        "resume" => ("reanudó el juego de", None),
        "force-close" => ("cerró el juego de", None),
        "reset-day" => ("reinició el día de", None),
        "adjust-time" => {
            let delta = payload["deltaSeconds"].as_i64()?;
            (
                "ajustó el tiempo de",
                Some(format!(
                    "{}{}",
                    if delta >= 0 { "+" } else { "−" },
                    text::hms(delta.abs())
                )),
            )
        }
        "finish" => (
            "marcó como terminado a",
            payload["finalTimeSeconds"]
                .as_i64()
                .map(|s| format!("tiempo final {}", text::hms(s))),
        ),
        _ => return None,
    };
    racer.as_ref()?;
    Some(Notice::new(
        racer,
        Detail::Organizer {
            actor: actor.into(),
            action: label.into(),
            reason,
            detail,
        },
        at,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_registry_is_consistent() {
        for kind in Kind::ALL {
            assert_eq!(Kind::parse(kind.key()), Some(kind), "{kind:?}");
            assert!(!kind.label().is_empty());
        }
        let keys: std::collections::HashSet<_> = Kind::ALL.iter().map(|k| k.key()).collect();
        assert_eq!(keys.len(), Kind::ALL.len(), "duplicate keys");
        // Referee alerts never reach the community channel.
        for k in [
            Kind::Disconnected,
            Kind::LowTime,
            Kind::Suspicious,
            Kind::EventState,
        ] {
            assert_eq!(k.channel(), Channel::Staff);
        }
        assert_eq!(Kind::Winner.channel(), Channel::Public);
    }

    fn info() -> LiveInfo {
        LiveInfo {
            id: "ralbat".into(),
            name: "Ralbat".into(),
            avatar_url: None,
            progress_pct: 10.0,
            area: None,
            viewers: None,
            channels: vec![],
        }
    }

    #[test]
    fn organizer_actions_become_staff_notices_with_who_and_why() {
        let now = Utc::now();
        let n = from_audit(
            "ana",
            "racer.adjust-time",
            Some(info()),
            &json!({ "deltaSeconds": -90, "reason": " lag del stream " }),
            now,
        )
        .unwrap();
        assert_eq!(
            n.detail,
            Detail::Organizer {
                actor: "ana".into(),
                action: "ajustó el tiempo de".into(),
                reason: Some("lag del stream".into()),
                detail: Some("−00:01:30".into()),
            }
        );
        let e = from_audit("ana", "event.pause", None, &json!({ "reason": null }), now).unwrap();
        assert_eq!(e.kind(), Kind::EventState);
        // Things referees do not need to hear about.
        for action in [
            "racer.update",
            "racer.token.rotate",
            "auth.login",
            "event.update",
            "catalog.item.save",
        ] {
            assert!(
                from_audit("ana", action, Some(info()), &json!({}), now).is_none(),
                "{action}"
            );
        }
        // A racer action without a known racer says nothing.
        assert!(from_audit("ana", "racer.force-close", None, &json!({}), now).is_none());
    }
}
