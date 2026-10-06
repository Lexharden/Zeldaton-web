//! "X is live" announcements to a Discord channel, through a webhook.
//!
//! Three separate pieces so the logic can be tested without a network:
//! * [`Announcer`]: pure state machine deciding *who* to announce (a stream must stay live for a
//!   while, and one racer is not announced again for a while).
//! * [`build_message`]: pure builder of the Discord payload (an embed with links to the racer's page
//!   on the site and to their TikTok / Twitch / YouTube channels). Webhooks cannot send buttons, so
//!   the links live inside the embed.
//! * [`DiscordClient`] and [`spawn`]: the I/O. A background task scans the state every few seconds and
//!   posts; it never touches the race lock for longer than a snapshot and never blocks the race.
//!
//! The webhook URL is a secret: it comes from the environment, is never returned by the API and is
//! never logged (errors are printed without the URL).

use std::collections::HashMap;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use crate::db;
use crate::domain::{EventStatus, Platform};
use crate::hub::AppState;
use crate::state::RaceState;

/// A racer must stay live this long before the announcement (short reconnects are not "going live").
pub const LIVE_AFTER: Duration = Duration::from_secs(30);
/// The same racer is not announced again within this time, however often HiveShock reconnects.
pub const COOLDOWN: Duration = Duration::from_secs(30 * 60);
const SCAN_EVERY: Duration = Duration::from_secs(5);
/// Settings key (see `app_settings`): `"true"` when the organizer turned announcements on.
pub const ENABLED_KEY: &str = "discord_enabled";

/// Only real Discord webhook URLs are accepted (so the server can never be pointed elsewhere).
pub fn valid_webhook_url(url: &str) -> bool {
    [
        "https://discord.com/api/webhooks/",
        "https://discordapp.com/api/webhooks/",
    ]
    .iter()
    .any(|prefix| url.starts_with(prefix) && url.len() > prefix.len())
}

// ---- who to announce ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct Announcer {
    live_after: Duration,
    cooldown: Duration,
    live_since: HashMap<String, DateTime<Utc>>,
    notified: HashMap<String, DateTime<Utc>>,
}

impl Default for Announcer {
    fn default() -> Self {
        Self {
            live_after: LIVE_AFTER,
            cooldown: COOLDOWN,
            live_since: HashMap::new(),
            notified: HashMap::new(),
        }
    }
}

impl Announcer {
    /// Custom timing (the defaults are `LIVE_AFTER` and `COOLDOWN`); for tests.
    pub fn with_timing(live_after: Duration, cooldown: Duration) -> Self {
        Self {
            live_after,
            cooldown,
            ..Self::default()
        }
    }
    /// At start-up, whoever is live already counts as announced (a restart must not repeat it).
    pub fn prime(&mut self, now: DateTime<Utc>, live: &[(String, bool)]) {
        for (id, is_live) in live {
            if *is_live {
                self.live_since.insert(id.clone(), now);
                self.notified.insert(id.clone(), now);
            }
        }
    }

    /// Forgets who is live (announcements are off, or the event is not running).
    pub fn clear_live(&mut self) {
        self.live_since.clear();
    }

    /// Racers to announce now, given who is live at `now`.
    pub fn decide(&mut self, now: DateTime<Utc>, live: &[(String, bool)]) -> Vec<String> {
        let mut out = Vec::new();
        for (id, is_live) in live {
            if !is_live {
                self.live_since.remove(id);
                continue;
            }
            let since = *self.live_since.entry(id.clone()).or_insert(now);
            let steady = (now - since).to_std().is_ok_and(|d| d >= self.live_after);
            let rested = self
                .notified
                .get(id)
                .is_none_or(|at| (now - *at).to_std().is_ok_and(|d| d >= self.cooldown));
            if steady && rested {
                self.notified.insert(id.clone(), now);
                out.push(id.clone());
            }
        }
        out
    }
}

/// Announcements only happen while the event really runs, never in a rehearsal.
pub fn gates_open(enabled: bool, event_live: bool, rehearsal: bool) -> bool {
    enabled && event_live && !rehearsal
}

// ---- what to say -------------------------------------------------------------------------------

/// What the message needs to know about a racer.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveInfo {
    pub id: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub progress_pct: f64,
    pub area: Option<String>,
    pub viewers: Option<i64>,
    /// (platform, url) of each channel the racer has.
    pub channels: Vec<(Platform, String)>,
}

impl LiveInfo {
    pub fn from_state(s: &RaceState, i: usize) -> Self {
        let r = &s.racers[i].racer;
        Self {
            id: r.id.clone(),
            name: r.display_name.clone(),
            avatar_url: r.avatar_url.clone(),
            progress_pct: r.progress_percentage,
            area: r.current_area.clone(),
            viewers: r.stream.as_ref().and_then(|st| st.viewers),
            channels: r
                .channels
                .iter()
                .map(|c| (c.platform, c.url.clone()))
                .collect(),
        }
    }
}

/// Makes a racer-controlled string safe inside Discord markdown: no formatting, no mention syntax.
fn escape(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars().take(60) {
        match c {
            '\\' | '*' | '_' | '~' | '`' | '|' | '>' | '<' | '[' | ']' | '(' | ')' | '#' => {
                out.push('\\');
                out.push(c);
            }
            '@' => {
                out.push('@');
                out.push('\u{200b}');
            }
            '\n' | '\r' => out.push(' '),
            _ => out.push(c),
        }
    }
    out
}

/// A URL that cannot break out of a markdown link.
fn safe_url(url: &str) -> String {
    url.replace(' ', "%20")
        .replace('(', "%28")
        .replace(')', "%29")
        .replace('<', "%3C")
        .replace('>', "%3E")
        .replace('[', "%5B")
        .replace(']', "%5D")
}

fn platform_label(p: Platform) -> &'static str {
    match p {
        Platform::Twitch => "Twitch",
        Platform::Tiktok => "TikTok",
        Platform::Youtube => "YouTube",
    }
}

/// "water-temple" -> "Water Temple" (the server does not know the site's translations).
fn pretty_area(id: &str) -> String {
    id.split('-')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// The webhook body. `public_url` is the site's address (empty = no site links); `test` marks it
/// as the panel's test message.
pub fn build_message(info: &LiveInfo, public_url: &str, test: bool, now: DateTime<Utc>) -> Value {
    let name = escape(&info.name);
    let page = (!public_url.is_empty()).then(|| format!("{public_url}/racer/{}", info.id));

    let mut lines = vec![format!(
        "🎮 Progreso: **{}%**",
        info.progress_pct.round() as i64
    )];
    if let Some(area) = info.area.as_deref().filter(|a| !a.is_empty()) {
        lines[0].push_str(&format!(" · {}", escape(&pretty_area(area))));
    }
    if let Some(viewers) = info.viewers.filter(|v| *v > 0) {
        lines.push(format!("👁 {viewers} espectadores"));
    }

    let mut links = Vec::new();
    if let Some(page) = &page {
        links.push(format!("[Seguir en Zeldatón]({})", safe_url(page)));
    }
    for (platform, url) in &info.channels {
        links.push(format!(
            "[{}]({})",
            platform_label(*platform),
            safe_url(url)
        ));
    }

    let mut embed = json!({
        "title": format!("🔴 {name} está en vivo"),
        "description": lines.join("\n"),
        "color": 0x3DDCFF,
        "footer": { "text": if test { "Zeldatón · mensaje de prueba" } else { "Zeldatón · Ocarina of Time" } },
        "timestamp": now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    });
    if let Some(page) = &page {
        embed["url"] = json!(page);
    }
    if !links.is_empty() {
        embed["fields"] =
            json!([{ "name": "Míralo aquí", "value": links.join(" · "), "inline": false }]);
    }
    let avatar = info.avatar_url.as_deref().and_then(|a| {
        if a.starts_with("https://") {
            Some(a.to_string())
        } else if a.starts_with('/') && !public_url.is_empty() {
            Some(format!("{public_url}{a}"))
        } else {
            None
        }
    });
    if let Some(avatar) = avatar {
        embed["thumbnail"] = json!({ "url": avatar });
    }
    json!({
        "username": "Zeldatón",
        // Nobody gets pinged because of a racer's display name.
        "allowed_mentions": { "parse": [] },
        "embeds": [embed],
    })
}

// ---- sending -----------------------------------------------------------------------------------

/// Last outcome, shown in the organizer panel.
#[derive(Default, Clone, Debug)]
pub struct DiscordStatus {
    pub last_sent_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

pub struct DiscordClient {
    http: reqwest::Client,
    url: String,
}

impl DiscordClient {
    pub fn new(url: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("http client");
        Self { http, url }
    }

    /// Posts `body`, retrying rate limits (429) and server errors (5xx) a few times. Errors never
    /// contain the webhook URL.
    pub async fn send(&self, body: &Value) -> Result<(), String> {
        let mut last = String::new();
        for attempt in 0..3u32 {
            match self.http.post(&self.url).json(body).send().await {
                Ok(res) if res.status().is_success() => return Ok(()),
                Ok(res) if res.status().as_u16() == 429 => {
                    let wait = res
                        .json::<Value>()
                        .await
                        .ok()
                        .and_then(|v| v["retry_after"].as_f64())
                        .unwrap_or(2.0)
                        .clamp(0.0, 10.0);
                    last = "Discord rate limit (429)".into();
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_secs_f64(wait)).await;
                    }
                }
                Ok(res) if res.status().is_server_error() => {
                    last = format!("Discord answered {}", res.status().as_u16());
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
                    }
                }
                Ok(res) => {
                    // 400/401/404...: a bad or deleted webhook. Retrying cannot help.
                    return Err(format!(
                        "Discord rejected the message ({}): check the webhook URL",
                        res.status().as_u16()
                    ));
                }
                Err(e) => {
                    last = format!("could not reach Discord: {}", e.without_url());
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_secs(1 << attempt)).await;
                    }
                }
            }
        }
        Err(last)
    }
}

async fn enabled(hub: &AppState) -> bool {
    matches!(db::setting(&hub.pool, ENABLED_KEY).await, Ok(Some(v)) if v == "true")
}

/// Everything the scan needs from the race, copied out under the lock.
fn snapshot(hub: &AppState, now: DateTime<Utc>) -> (bool, bool, Vec<(LiveInfo, bool)>) {
    hub.read(|s| {
        let live = (0..s.racers.len())
            .map(|i| {
                let is_live = s.racers[i]
                    .racer
                    .stream
                    .as_ref()
                    .is_some_and(|st| st.is_live);
                (LiveInfo::from_state(s, i), is_live)
            })
            .collect();
        (
            s.event_status(now) == EventStatus::Live,
            s.event.rehearsal,
            live,
        )
    })
}

fn record(hub: &AppState, result: &Result<(), String>) {
    let mut status = hub.discord.lock().unwrap_or_else(|e| e.into_inner());
    match result {
        Ok(()) => {
            status.last_sent_at = Some(Utc::now());
            status.last_error = None;
        }
        Err(e) => status.last_error = Some(e.clone()),
    }
}

/// Sends the panel's test message with the first racer's data.
pub async fn send_test(hub: &AppState) -> Result<(), String> {
    let url = hub
        .cfg
        .discord_webhook_url
        .clone()
        .ok_or("DISCORD_WEBHOOK_URL is not set")?;
    let info = hub
        .read(|s| (!s.racers.is_empty()).then(|| LiveInfo::from_state(s, 0)))
        .ok_or("there are no racers yet")?;
    let body = build_message(&info, &hub.cfg.public_url, true, Utc::now());
    let result = DiscordClient::new(url).send(&body).await;
    record(hub, &result);
    result
}

/// Starts the background task (only when a webhook is configured).
pub fn spawn(hub: AppState) {
    spawn_with(hub, SCAN_EVERY, Announcer::default());
}

/// `spawn` with custom timing, for tests.
pub fn spawn_with(hub: AppState, scan_every: Duration, mut announcer: Announcer) {
    let Some(url) = hub.cfg.discord_webhook_url.clone() else {
        return;
    };
    tokio::spawn(async move {
        let client = DiscordClient::new(url);
        let now = Utc::now();
        let (_, _, racers) = snapshot(&hub, now);
        announcer.prime(
            now,
            &racers
                .iter()
                .map(|(i, l)| (i.id.clone(), *l))
                .collect::<Vec<_>>(),
        );
        let mut interval = tokio::time::interval(scan_every);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let now = Utc::now();
            let on = enabled(&hub).await;
            let (event_live, rehearsal, racers) = snapshot(&hub, now);
            if !gates_open(on, event_live, rehearsal) {
                announcer.clear_live();
                continue;
            }
            let flags: Vec<(String, bool)> =
                racers.iter().map(|(i, l)| (i.id.clone(), *l)).collect();
            for id in announcer.decide(now, &flags) {
                let Some((info, _)) = racers.iter().find(|(i, _)| i.id == id) else {
                    continue;
                };
                let body = build_message(info, &hub.cfg.public_url, false, now);
                let result = client.send(&body).await;
                if let Err(e) = &result {
                    tracing::warn!(racer = %id, error = %e, "discord announcement failed");
                }
                record(&hub, &result);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1_790_000_000 + secs, 0).unwrap()
    }
    fn live(ids: &[(&str, bool)]) -> Vec<(String, bool)> {
        ids.iter().map(|(i, l)| (i.to_string(), *l)).collect()
    }

    #[test]
    fn only_discord_webhooks_are_accepted() {
        assert!(valid_webhook_url("https://discord.com/api/webhooks/1/abc"));
        assert!(valid_webhook_url(
            "https://discordapp.com/api/webhooks/1/abc"
        ));
        for bad in [
            "",
            "https://discord.com/api/webhooks/",
            "http://discord.com/api/webhooks/1/abc",
            "https://evil.example/api/webhooks/1/abc",
            "https://discord.com.evil.example/api/webhooks/1/abc",
        ] {
            assert!(!valid_webhook_url(bad), "{bad}");
        }
    }

    #[test]
    fn a_stream_is_announced_once_after_staying_live_for_a_while() {
        let mut a = Announcer::default();
        assert!(a.decide(t(0), &live(&[("ana", true)])).is_empty()); // just went live
        assert!(a.decide(t(20), &live(&[("ana", true)])).is_empty()); // not steady yet
        assert_eq!(a.decide(t(31), &live(&[("ana", true)])), ["ana"]);
        assert!(a.decide(t(60), &live(&[("ana", true)])).is_empty()); // already announced
    }

    #[test]
    fn short_blips_do_not_count_and_reconnects_do_not_repeat() {
        let mut a = Announcer::default();
        a.decide(t(0), &live(&[("ana", true)]));
        a.decide(t(10), &live(&[("ana", false)])); // dropped before 30 s
        assert!(a.decide(t(15), &live(&[("ana", true)])).is_empty()); // the clock restarted
        assert_eq!(a.decide(t(50), &live(&[("ana", true)])), ["ana"]);
        // Reconnects within the cooldown never announce again...
        a.decide(t(60), &live(&[("ana", false)]));
        a.decide(t(70), &live(&[("ana", true)]));
        assert!(a.decide(t(200), &live(&[("ana", true)])).is_empty());
        // ...but a later, separate stream does.
        a.decide(t(300), &live(&[("ana", false)]));
        a.decide(t(2000), &live(&[("ana", true)]));
        assert_eq!(a.decide(t(2040), &live(&[("ana", true)])), ["ana"]);
    }

    #[test]
    fn racers_are_tracked_independently_and_a_restart_does_not_repeat() {
        let mut a = Announcer::default();
        a.prime(t(0), &live(&[("ana", true), ("beto", false)]));
        assert!(
            a.decide(t(40), &live(&[("ana", true), ("beto", false)]))
                .is_empty()
        );
        a.decide(t(41), &live(&[("ana", true), ("beto", true)]));
        assert_eq!(
            a.decide(t(80), &live(&[("ana", true), ("beto", true)])),
            ["beto"]
        );
    }

    #[test]
    fn announcements_need_the_switch_a_running_event_and_no_rehearsal() {
        assert!(gates_open(true, true, false));
        assert!(!gates_open(false, true, false));
        assert!(!gates_open(true, false, false));
        assert!(!gates_open(true, true, true));
    }

    fn info() -> LiveInfo {
        LiveInfo {
            id: "ralbat".into(),
            name: "Ral*bat_ @everyone [x](y)".into(),
            avatar_url: Some("/api/media/racers/ralbat-1.png".into()),
            progress_pct: 41.6,
            area: Some("water-temple".into()),
            viewers: Some(120),
            channels: vec![
                (Platform::Twitch, "https://twitch.tv/ralbat".into()),
                (Platform::Tiktok, "https://tiktok.com/@ra lbat)".into()),
            ],
        }
    }

    #[test]
    fn the_message_links_the_site_and_each_channel() {
        let m = build_message(&info(), "https://zeldaton.example", false, t(0));
        let e = &m["embeds"][0];
        assert_eq!(e["url"], "https://zeldaton.example/racer/ralbat");
        let fields = e["fields"][0]["value"].as_str().unwrap();
        assert!(fields.starts_with("[Seguir en Zeldatón](https://zeldaton.example/racer/ralbat)"));
        assert!(fields.contains("[Twitch](https://twitch.tv/ralbat)"));
        // A handle with a space or a parenthesis cannot break the markdown link.
        assert!(fields.contains("[TikTok](https://tiktok.com/@ra%20lbat%29)"));
        let d = e["description"].as_str().unwrap();
        assert!(
            d.contains("**42%**") && d.contains("Water Temple") && d.contains("120 espectadores")
        );
        assert_eq!(
            e["thumbnail"]["url"],
            "https://zeldaton.example/api/media/racers/ralbat-1.png"
        );
        assert_eq!(e["footer"]["text"], "Zeldatón · Ocarina of Time");
    }

    #[test]
    fn a_hostile_name_cannot_ping_or_format() {
        let m = build_message(&info(), "https://zeldaton.example", false, t(0));
        assert_eq!(m["allowed_mentions"], json!({ "parse": [] }));
        let title = m["embeds"][0]["title"].as_str().unwrap();
        assert!(
            title.contains("Ral\\*bat\\_ @\u{200b}everyone \\[x\\]\\(y\\)"),
            "{title}"
        );
    }

    #[test]
    fn it_degrades_without_site_url_channels_or_avatar() {
        let mut bare = info();
        bare.channels.clear();
        bare.avatar_url = None;
        bare.viewers = None;
        bare.area = None;
        let m = build_message(&bare, "", true, t(0));
        let e = &m["embeds"][0];
        assert!(
            e.get("url").is_none() && e.get("fields").is_none() && e.get("thumbnail").is_none()
        );
        assert_eq!(e["description"], "🎮 Progreso: **42%**");
        assert_eq!(e["footer"]["text"], "Zeldatón · mensaje de prueba");
    }
}
