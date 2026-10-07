//! The dispatcher: the one task that turns notices into Discord messages.
//!
//! Notices arrive from the engine and the organizer actions through `Hub.notices`, and the scanner
//! adds the time-based ones every few seconds. For each one the [`Policy`] decides whether it is
//! sent (switches, rehearsal, thresholds, cooldowns), the database remembers the one-off ones so a
//! restart never repeats them, and the message goes to its channel's outbox, where a worker posts
//! it in order under the rate limit.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};

use super::client::DiscordClient;
use super::outbox::{Item, Outbox};
use super::render::{Ctx, render, test_notice};
use super::scan::{RacerSnap, ScanState, Snapshot};
use super::settings::NotifySettings;
use super::{Channel, Detail, Kind, LiveInfo, Notice};
use crate::db;
use crate::domain::EventStatus;
use crate::hub::AppState;
use crate::standings;

const SCAN_EVERY: Duration = Duration::from_secs(5);

// ---- policy ------------------------------------------------------------------------------------

/// What is true right now that decides whether a notice may be sent.
#[derive(Clone, Copy, Debug)]
pub struct Gate {
    pub event_live: bool,
    pub rehearsal: bool,
    /// The kind's channel has a webhook.
    pub configured: bool,
}

/// Cooldowns by (kind, racer). The pure part of "may this be sent".
#[derive(Debug, Default)]
pub struct Policy {
    last: HashMap<(Kind, String), DateTime<Utc>>,
}

impl Policy {
    /// `Ok` when the notice may be sent now; `Err` says why not (for the debug log).
    pub fn admit(
        &self,
        n: &Notice,
        settings: &NotifySettings,
        gate: Gate,
        now: DateTime<Utc>,
    ) -> Result<(), &'static str> {
        let kind = n.kind();
        let channel = kind.channel();
        if !gate.configured {
            return Err("the channel has no webhook");
        }
        if !settings.channel_enabled(channel) {
            return Err("the channel is switched off");
        }
        if !settings.kind_enabled(kind) {
            return Err("this kind is switched off");
        }
        // The community only hears about a running, real event. Referees also hear the rest
        // (an event that just ended, a rehearsal being tested).
        if channel == Channel::Public && (!gate.event_live || gate.rehearsal) {
            return Err("the event is not live, or it is a rehearsal");
        }
        if let Detail::Jump { from, to, seconds } = &n.detail {
            let t = &settings.thresholds;
            if to - from < t.jump_percent || *seconds > t.jump_window_seconds {
                return Err("below the suspicious-jump thresholds");
            }
        }
        if let Some(cooldown) = kind.cooldown()
            && let Some(at) = self.last.get(&(kind, racer_key(n)))
            && (now - *at).to_std().is_ok_and(|d| d < cooldown)
        {
            return Err("cooling down");
        }
        Ok(())
    }

    /// Call once the notice is accepted, so the cooldown starts.
    pub fn mark(&mut self, n: &Notice, now: DateTime<Utc>) {
        if n.kind().cooldown().is_some() {
            self.last.insert((n.kind(), racer_key(n)), now);
        }
    }
}

fn racer_key(n: &Notice) -> String {
    n.racer.as_ref().map(|r| r.id.clone()).unwrap_or_default()
}

// ---- reading the race --------------------------------------------------------------------------

/// Everything the scanner needs, copied out under the lock.
fn snapshot(hub: &AppState, now: DateTime<Utc>) -> Snapshot {
    hub.read(|s| {
        let views = s.views(now);
        let leader = if s.winner.is_some() {
            None
        } else {
            standings::compute(&views)
                .first()
                .and_then(|top| views.iter().find(|r| r.id == top.racer_id))
                .filter(|r| r.progress_percentage > 0.0 || !r.completed_objectives.is_empty())
                .map(|r| r.id.clone())
        };
        let racers = (0..s.racers.len())
            .map(|i| {
                let rt = &s.racers[i];
                RacerSnap {
                    info: LiveInfo::from_state(s, i),
                    status: rt.racer.status,
                    remaining_ms: views[i].remaining_seconds * 1000,
                    is_live: rt.racer.stream.as_ref().is_some_and(|st| st.is_live),
                    connected: rt.ingest.is_some(),
                    played: rt.played_ms_total > 0 || rt.racer.progress_percentage > 0.0,
                    day_key: crate::engine::iso(rt.reset_at),
                }
            })
            .collect();
        Snapshot {
            event_live: s.event_status(now) == EventStatus::Live,
            racers,
            leader,
        }
    })
}

fn gate(hub: &AppState, channel: Channel, now: DateTime<Utc>) -> Gate {
    let (event_live, rehearsal) =
        hub.read(|s| (s.event_status(now) == EventStatus::Live, s.event.rehearsal));
    Gate {
        event_live,
        rehearsal,
        configured: hub.cfg.webhook_for(channel).is_some(),
    }
}

// ---- the task ----------------------------------------------------------------------------------

struct Dispatcher {
    hub: AppState,
    policy: Policy,
    scan: ScanState,
    outboxes: HashMap<Channel, Arc<Mutex<Outbox>>>,
}

/// Starts the dispatcher and one worker per configured channel.
pub fn spawn(hub: AppState) {
    spawn_with(hub, SCAN_EVERY, ScanState::default());
}

/// `spawn` with custom timing, for tests.
pub fn spawn_with(hub: AppState, scan_every: Duration, scan: ScanState) {
    let Some(mut rx) = hub.take_notice_receiver() else {
        return;
    };
    tokio::spawn(async move {
        let mut outboxes = HashMap::new();
        for channel in Channel::ALL {
            if let Some(url) = hub.cfg.webhook_for(channel) {
                outboxes.insert(channel, start_worker(hub.clone(), channel, url));
            }
        }
        let mut d = Dispatcher {
            hub,
            policy: Policy::default(),
            scan,
            outboxes,
        };
        let now = Utc::now();
        let snap = snapshot(&d.hub, now);
        d.scan.prime(now, &snap);

        let mut tick = tokio::time::interval(scan_every);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                notice = rx.recv() => match notice {
                    Some(n) => d.handle(n).await,
                    None => break,
                },
                _ = tick.tick() => d.scan_tick().await,
            }
        }
    });
}

/// One worker per channel: posts its outbox in order, one message at a time.
fn start_worker(hub: AppState, channel: Channel, url: String) -> Arc<Mutex<Outbox>> {
    let outbox = Arc::new(Mutex::new(Outbox::standard()));
    let queue = outbox.clone();
    tokio::spawn(async move {
        let client = DiscordClient::new(url);
        loop {
            let (next, queued, dropped) = {
                let mut o = queue.lock().unwrap_or_else(|e| e.into_inner());
                let next = o.pop(Utc::now());
                (next, o.len(), o.dropped())
            };
            hub.update_channel(channel, |s| {
                s.queued = queued;
                s.dropped = dropped;
            });
            match next {
                Some(item) => {
                    let result = client.send(&item.body).await;
                    if let Err(e) = &result {
                        tracing::warn!(channel = channel.as_str(), error = %e, "discord message failed");
                    }
                    hub.record_send(channel, &result);
                }
                None => tokio::time::sleep(Duration::from_millis(200)).await,
            }
        }
    });
    outbox
}

impl Dispatcher {
    async fn scan_tick(&mut self) {
        let now = Utc::now();
        let settings = self.hub.notify_settings();
        let public_open = {
            let g = gate(&self.hub, Channel::Public, now);
            g.configured && g.event_live && !g.rehearsal && settings.public_enabled
        };
        let snap = snapshot(&self.hub, now);
        for n in self
            .scan
            .scan(&snap, &settings.thresholds, public_open, now)
        {
            self.handle(n).await;
        }
    }

    async fn handle(&mut self, n: Notice) {
        let now = Utc::now();
        let kind = n.kind();
        let channel = kind.channel();
        let settings = self.hub.notify_settings();
        // The monitor keeps the incidents whether or not Discord is on.
        if let Err(e) =
            crate::incidents::record(&self.hub.pool, &n, &settings.thresholds, now).await
        {
            tracing::warn!(error = %e, "could not record an incident");
        }
        let g = gate(&self.hub, channel, now);
        if let Err(why) = self.policy.admit(&n, &settings, g, now) {
            tracing::debug!(kind = kind.key(), why, "notice not sent");
            return;
        }
        // One-off events are sent once, even across restarts.
        if let Some(key) = &n.dedupe {
            match db::notice_reserve(&self.hub.pool, key, now).await {
                Ok(true) => {}
                Ok(false) => return,
                // Better a possible repeat than silence.
                Err(e) => tracing::warn!(error = %e, "could not record a notice key"),
            }
        }
        self.policy.mark(&n, now);
        let ctx = Ctx {
            public_url: &self.hub.cfg.public_url,
            rehearsal: g.rehearsal,
            role_id: self.hub.cfg.discord_staff_role_id.as_deref(),
            test: false,
            now,
        };
        let body = render(&n, &ctx);
        let Some(outbox) = self.outboxes.get(&channel) else {
            return;
        };
        let (queued, dropped) = {
            let mut o = outbox.lock().unwrap_or_else(|e| e.into_inner());
            o.push(Item {
                priority: kind.priority(),
                body,
            });
            (o.len(), o.dropped())
        };
        self.hub.update_channel(channel, |s| {
            s.queued = queued;
            s.dropped = dropped;
        });
    }
}

/// The panel's test message: a sample of what `channel` receives, posted straight away.
pub async fn send_test(hub: &AppState, channel: Channel) -> Result<(), String> {
    let url = hub.cfg.webhook_for(channel).ok_or(match channel {
        Channel::Public => "DISCORD_WEBHOOK_URL is not set",
        Channel::Staff => "DISCORD_STAFF_WEBHOOK_URL is not set",
    })?;
    let info = hub
        .read(|s| (!s.racers.is_empty()).then(|| LiveInfo::from_state(s, 0)))
        .ok_or("there are no racers yet")?;
    let now = Utc::now();
    let ctx = Ctx {
        public_url: &hub.cfg.public_url,
        rehearsal: hub.read(|s| s.event.rehearsal),
        role_id: None,
        test: true,
        now,
    };
    let body = render(&test_notice(channel, info, now), &ctx);
    let result = DiscordClient::new(url).send(&body).await;
    hub.record_send(channel, &result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notify::settings::Thresholds;
    use chrono::TimeZone;

    fn t(secs: i64) -> DateTime<Utc> {
        Utc.timestamp_opt(1_790_000_000 + secs, 0).unwrap()
    }
    fn info(id: &str) -> LiveInfo {
        LiveInfo {
            id: id.into(),
            name: id.into(),
            avatar_url: None,
            progress_pct: 10.0,
            area: None,
            viewers: None,
            channels: vec![],
        }
    }
    fn notice(id: &str, d: Detail) -> Notice {
        Notice::new(Some(info(id)), d, t(0))
    }
    fn on() -> NotifySettings {
        NotifySettings {
            public_enabled: true,
            staff_enabled: true,
            ..NotifySettings::default()
        }
    }
    const LIVE: Gate = Gate {
        event_live: true,
        rehearsal: false,
        configured: true,
    };

    #[test]
    fn switches_decide_per_channel_and_per_kind() {
        let p = Policy::default();
        let boss = notice(
            "ana",
            Detail::Boss {
                boss: "gohma".into(),
                count: None,
            },
        );
        let cut = notice(
            "ana",
            Detail::Disconnected {
                minutes: 3,
                back: false,
            },
        );
        assert!(p.admit(&boss, &on(), LIVE, t(0)).is_ok());
        assert!(p.admit(&cut, &on(), LIVE, t(0)).is_ok());
        let off = NotifySettings::default();
        assert!(
            p.admit(&boss, &off, LIVE, t(0)).is_err(),
            "everything starts off"
        );
        // Each channel has its own switch.
        let only_staff = NotifySettings {
            public_enabled: false,
            staff_enabled: true,
            ..off.clone()
        };
        assert!(p.admit(&boss, &only_staff, LIVE, t(0)).is_err());
        assert!(p.admit(&cut, &only_staff, LIVE, t(0)).is_ok());
        // And each kind.
        let mut no_boss = on();
        no_boss.kinds.insert("boss".into(), false);
        assert!(p.admit(&boss, &no_boss, LIVE, t(0)).is_err());
        assert!(
            p.admit(&notice("ana", Detail::Exhausted), &no_boss, LIVE, t(0))
                .is_ok()
        );
        // A channel without a webhook never sends.
        assert!(
            p.admit(
                &boss,
                &on(),
                Gate {
                    configured: false,
                    ..LIVE
                },
                t(0)
            )
            .is_err()
        );
    }

    #[test]
    fn the_community_only_hears_a_running_real_event_but_referees_hear_more() {
        let p = Policy::default();
        let boss = notice(
            "ana",
            Detail::Boss {
                boss: "gohma".into(),
                count: None,
            },
        );
        let cut = notice(
            "ana",
            Detail::Disconnected {
                minutes: 3,
                back: false,
            },
        );
        let ended = notice(
            "ana",
            Detail::EventState {
                state: "finalizado".into(),
                actor: "x".into(),
                reason: None,
            },
        );
        for g in [
            Gate {
                event_live: false,
                ..LIVE
            },
            Gate {
                rehearsal: true,
                ..LIVE
            },
        ] {
            assert!(p.admit(&boss, &on(), g, t(0)).is_err());
            assert!(
                p.admit(&cut, &on(), g, t(0)).is_ok(),
                "referees can test in a rehearsal"
            );
        }
        assert!(
            p.admit(
                &ended,
                &on(),
                Gate {
                    event_live: false,
                    ..LIVE
                },
                t(0)
            )
            .is_ok()
        );
    }

    #[test]
    fn a_jump_must_clear_the_organizers_thresholds() {
        let p = Policy::default();
        let mut s = on();
        s.thresholds = Thresholds {
            jump_percent: 20.0,
            jump_window_seconds: 120,
            ..Thresholds::default()
        };
        let jump = |from, to, secs| {
            notice(
                "ana",
                Detail::Jump {
                    from,
                    to,
                    seconds: secs,
                },
            )
        };
        assert!(p.admit(&jump(10.0, 35.0, 60), &s, LIVE, t(0)).is_ok());
        assert!(
            p.admit(&jump(10.0, 25.0, 60), &s, LIVE, t(0)).is_err(),
            "too small"
        );
        assert!(
            p.admit(&jump(10.0, 35.0, 500), &s, LIVE, t(0)).is_err(),
            "too slow to be a jump"
        );
    }

    #[test]
    fn cooldowns_are_per_kind_and_per_racer() {
        let mut p = Policy::default();
        let live = |id| notice(id, Detail::Live);
        assert!(p.admit(&live("ana"), &on(), LIVE, t(0)).is_ok());
        p.mark(&live("ana"), t(0));
        assert!(
            p.admit(&live("ana"), &on(), LIVE, t(600)).is_err(),
            "30 minutes"
        );
        assert!(
            p.admit(&live("beto"), &on(), LIVE, t(600)).is_ok(),
            "another racer"
        );
        assert!(p.admit(&live("ana"), &on(), LIVE, t(31 * 60)).is_ok());
        // Kinds without a cooldown (one-offs are de-duplicated by key instead) are never held back.
        let boss = notice(
            "ana",
            Detail::Boss {
                boss: "gohma".into(),
                count: None,
            },
        );
        p.mark(&boss, t(0));
        assert!(p.admit(&boss, &on(), LIVE, t(1)).is_ok());
    }
}
