//! What the organizer can tune from the panel: a switch per channel, a switch per kind of
//! notification and the thresholds of the alerts. Stored as JSON in `app_settings` and cached in
//! the hub (the dispatcher reads it on every notice, so it must not hit the database).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqlitePool;

use super::{Channel, Kind};
use crate::db;

pub const KEY: &str = "notify_settings";
/// Before the staff channel existed this single switch controlled the live announcements.
const LEGACY_KEY: &str = "discord_enabled";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Thresholds {
    /// A racer disconnected for this long (while the event runs) alerts the referees.
    pub disconnect_minutes: i64,
    /// A racer with this much time or less left alerts the referees.
    pub low_time_minutes: i64,
    /// A progress rise of at least this many points...
    pub jump_percent: f64,
    /// ...within this many seconds is flagged as suspicious.
    pub jump_window_seconds: i64,
    /// A racer not connected this long after the start of their slot alerts the referees.
    pub no_show_minutes: i64,
    /// A slot without a referee alerts this long before it starts.
    pub uncovered_lead_minutes: i64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            disconnect_minutes: 3,
            low_time_minutes: 10,
            jump_percent: 20.0,
            jump_window_seconds: 120,
            no_show_minutes: 10,
            uncovered_lead_minutes: 30,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NotifySettings {
    /// Master switch of the community channel (off until the organizer turns it on).
    pub public_enabled: bool,
    /// Master switch of the referees' channel.
    pub staff_enabled: bool,
    /// Per-kind switches by [`Kind::key`]; a missing kind is on.
    pub kinds: HashMap<String, bool>,
    pub thresholds: Thresholds,
}

impl NotifySettings {
    pub fn kind_enabled(&self, kind: Kind) -> bool {
        self.kinds.get(kind.key()).copied().unwrap_or(true)
    }

    pub fn channel_enabled(&self, channel: Channel) -> bool {
        match channel {
            Channel::Public => self.public_enabled,
            Channel::Staff => self.staff_enabled,
        }
    }

    /// Applies a partial update from the API, all or nothing.
    ///
    /// ```json
    /// { "publicEnabled": true, "staffEnabled": false, "enabled": true /* old name of publicEnabled */,
    ///   "kinds": { "boss": false }, "thresholds": { "lowTimeMinutes": 15 } }
    /// ```
    pub fn patch(&mut self, p: &Value) -> Result<(), String> {
        let mut next = self.clone();
        let flag = |key: &str| -> Result<Option<bool>, String> {
            match p.get(key) {
                None | Some(Value::Null) => Ok(None),
                Some(Value::Bool(b)) => Ok(Some(*b)),
                Some(_) => Err(format!("{key} must be true or false")),
            }
        };
        if let Some(b) = flag("publicEnabled")?.or(flag("enabled")?) {
            next.public_enabled = b;
        }
        if let Some(b) = flag("staffEnabled")? {
            next.staff_enabled = b;
        }
        if let Some(kinds) = p.get("kinds").filter(|v| !v.is_null()) {
            let kinds = kinds.as_object().ok_or("kinds must be an object")?;
            for (key, value) in kinds {
                let kind = Kind::parse(key).ok_or_else(|| format!("unknown kind `{key}`"))?;
                let on = value
                    .as_bool()
                    .ok_or_else(|| format!("kinds.{key} must be true or false"))?;
                next.kinds.insert(kind.key().to_string(), on);
            }
        }
        if let Some(t) = p.get("thresholds").filter(|v| !v.is_null()) {
            let t = t.as_object().ok_or("thresholds must be an object")?;
            for (key, value) in t {
                match key.as_str() {
                    "disconnectMinutes" => {
                        next.thresholds.disconnect_minutes = int_in(key, value, 1, 120)?
                    }
                    "lowTimeMinutes" => {
                        next.thresholds.low_time_minutes = int_in(key, value, 1, 240)?
                    }
                    "jumpWindowSeconds" => {
                        next.thresholds.jump_window_seconds = int_in(key, value, 10, 3600)?
                    }
                    "noShowMinutes" => {
                        next.thresholds.no_show_minutes = int_in(key, value, 1, 120)?
                    }
                    "uncoveredLeadMinutes" => {
                        next.thresholds.uncovered_lead_minutes = int_in(key, value, 5, 720)?
                    }
                    "jumpPercent" => {
                        let v = value
                            .as_f64()
                            .filter(|v| (5.0..=100.0).contains(v))
                            .ok_or("jumpPercent must be between 5 and 100")?;
                        next.thresholds.jump_percent = v;
                    }
                    other => return Err(format!("unknown threshold `{other}`")),
                }
            }
        }
        *self = next;
        Ok(())
    }
}

fn int_in(key: &str, v: &Value, min: i64, max: i64) -> Result<i64, String> {
    v.as_i64()
        .filter(|n| (min..=max).contains(n))
        .ok_or_else(|| format!("{key} must be a whole number between {min} and {max}"))
}

/// Reads the settings (defaults when never saved), carrying over the old single switch.
pub async fn load(pool: &SqlitePool) -> NotifySettings {
    if let Ok(Some(json)) = db::setting(pool, KEY).await
        && let Ok(s) = serde_json::from_str(&json)
    {
        return s;
    }
    NotifySettings {
        public_enabled: matches!(db::setting(pool, LEGACY_KEY).await, Ok(Some(v)) if v == "true"),
        ..NotifySettings::default()
    }
}

pub async fn save(pool: &SqlitePool, s: &NotifySettings) -> Result<(), sqlx::Error> {
    let json = serde_json::to_string(s).unwrap_or_else(|_| "{}".into());
    db::set_setting(pool, KEY, &json).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn everything_is_off_until_the_organizer_turns_it_on_but_every_kind_is_ready() {
        let s = NotifySettings::default();
        assert!(!s.public_enabled && !s.staff_enabled);
        assert!(Kind::ALL.iter().all(|k| s.kind_enabled(*k)));
        assert_eq!(s.thresholds, Thresholds::default());
    }

    #[test]
    fn a_patch_changes_only_what_it_names() {
        let mut s = NotifySettings::default();
        s.patch(&json!({ "staffEnabled": true, "kinds": { "boss": false },
                         "thresholds": { "lowTimeMinutes": 15 } }))
            .unwrap();
        assert!(s.staff_enabled && !s.public_enabled);
        assert!(!s.kind_enabled(Kind::Boss) && s.kind_enabled(Kind::Winner));
        assert_eq!(s.thresholds.low_time_minutes, 15);
        assert_eq!(s.thresholds.disconnect_minutes, 3);
        // The old API name still switches the community channel.
        s.patch(&json!({ "enabled": true })).unwrap();
        assert!(s.public_enabled);
    }

    #[test]
    fn a_bad_patch_changes_nothing() {
        let mut s = NotifySettings::default();
        let before = s.clone();
        for bad in [
            json!({ "publicEnabled": "yes" }),
            json!({ "kinds": { "nope": true } }),
            json!({ "kinds": { "boss": 1 } }),
            json!({ "thresholds": { "disconnectMinutes": 0 } }),
            json!({ "thresholds": { "jumpPercent": 2 } }),
            json!({ "thresholds": { "surprise": 1 } }),
            // A valid part must not slip through when another part is invalid.
            json!({ "publicEnabled": true, "kinds": { "nope": true } }),
        ] {
            assert!(s.patch(&bad).is_err(), "{bad}");
            assert_eq!(s, before, "{bad}");
        }
    }

    #[test]
    fn settings_round_trip_through_json() {
        let mut s = NotifySettings::default();
        s.patch(&json!({ "publicEnabled": true, "kinds": { "leader": false } }))
            .unwrap();
        let back: NotifySettings =
            serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
        // Older or partial JSON fills the rest with defaults.
        let partial: NotifySettings = serde_json::from_str(r#"{"staffEnabled":true}"#).unwrap();
        assert!(partial.staff_enabled && partial.thresholds == Thresholds::default());
    }
}
