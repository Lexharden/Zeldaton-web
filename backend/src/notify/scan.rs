//! What depends on the passage of time, checked every few seconds against a snapshot of the race:
//! who went live, who took the lead, who has been disconnected for long, who is running out of
//! time. Pure: the clock is injected, so every rule is tested without waiting.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

use chrono::{DateTime, Utc};

use super::settings::Thresholds;
use super::{Detail, LiveInfo, Notice};
use crate::domain::RacerStatus;

/// A racer must stay live this long before the announcement (short reconnects are not "going live").
pub const LIVE_AFTER: Duration = Duration::from_secs(30);
/// The same racer is not announced as live again within this time.
pub const LIVE_COOLDOWN: Duration = Duration::from_secs(30 * 60);
/// A new leader must hold the lead this long before it is announced...
const LEAD_STEADY: Duration = Duration::from_secs(60);
/// ...and the lead is not announced again within this time.
const LEAD_COOLDOWN: Duration = Duration::from_secs(5 * 60);

// ---- who went live -----------------------------------------------------------------------------

#[derive(Debug)]
pub struct Announcer {
    live_after: Duration,
    cooldown: Duration,
    live_since: HashMap<String, DateTime<Utc>>,
    notified: HashMap<String, DateTime<Utc>>,
}

impl Default for Announcer {
    fn default() -> Self {
        Self::with_timing(LIVE_AFTER, LIVE_COOLDOWN)
    }
}

impl Announcer {
    /// Custom timing, for tests.
    pub fn with_timing(live_after: Duration, cooldown: Duration) -> Self {
        Self {
            live_after,
            cooldown,
            live_since: HashMap::new(),
            notified: HashMap::new(),
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

    /// Forgets who is live (the event is not running).
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

// ---- the snapshot ------------------------------------------------------------------------------

/// One racer as the scanner sees them.
#[derive(Clone, Debug)]
pub struct RacerSnap {
    pub info: LiveInfo,
    pub status: RacerStatus,
    pub remaining_ms: i64,
    /// The site shows them as live (broadcasting or playing).
    pub is_live: bool,
    /// HiveShock is connected.
    pub connected: bool,
    /// Changes with the racer's daily reset: a once-a-day alert is keyed on it.
    pub day_key: String,
}

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub event_live: bool,
    pub racers: Vec<RacerSnap>,
    /// First place, only when the lead is meaningful (someone progressed and nobody has won yet).
    pub leader: Option<String>,
}

/// The game is being played (its clock runs or is paused).
fn is_playing(status: RacerStatus) -> bool {
    matches!(status, RacerStatus::Live | RacerStatus::Paused)
}

// ---- the scanner -------------------------------------------------------------------------------

#[derive(Debug, Default)]
struct Lead {
    announced: Option<String>,
    candidate: Option<(String, DateTime<Utc>)>,
    last_announced_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Default)]
pub struct ScanState {
    announcer: Announcer,
    lead: Lead,
    offline_since: HashMap<String, DateTime<Utc>>,
    /// Racers seen playing (live or paused) while connected, and not seen stopping since. Only
    /// these can *drop*: someone who closed the game on purpose, or has not started, is not an
    /// incident, because nobody is expected to be connected at any fixed hour.
    was_playing: HashSet<String>,
    disconnect_alerted: HashSet<String>,
    low_alerted: HashSet<String>,
}

impl ScanState {
    pub fn with_announcer(announcer: Announcer) -> Self {
        Self {
            announcer,
            ..Self::default()
        }
    }

    /// At start-up: whoever is live, leading or connected right now is the starting point, not news.
    pub fn prime(&mut self, now: DateTime<Utc>, snap: &Snapshot) {
        let live: Vec<_> = snap
            .racers
            .iter()
            .map(|r| (r.info.id.clone(), r.is_live))
            .collect();
        self.announcer.prime(now, &live);
        self.lead.announced = snap.leader.clone();
        for r in snap
            .racers
            .iter()
            .filter(|r| r.connected && is_playing(r.status))
        {
            self.was_playing.insert(r.info.id.clone());
        }
    }

    /// `public_open`: the community channel can receive notices right now. When it cannot (switched
    /// off, rehearsal...) what it would announce is *not* remembered as announced, so turning it on
    /// later does not replay stale news; the lead is simply followed silently.
    pub fn scan(
        &mut self,
        snap: &Snapshot,
        t: &Thresholds,
        public_open: bool,
        now: DateTime<Utc>,
    ) -> Vec<Notice> {
        if !snap.event_live {
            self.announcer.clear_live();
            self.offline_since.clear();
            self.was_playing.clear();
            self.lead.candidate = None;
            return Vec::new();
        }
        let mut out = Vec::new();
        let info_of = |id: &str| {
            snap.racers
                .iter()
                .find(|r| r.info.id == id)
                .map(|r| r.info.clone())
        };

        // Went live.
        let live: Vec<_> = snap
            .racers
            .iter()
            .map(|r| (r.info.id.clone(), r.is_live))
            .collect();
        if public_open {
            for id in self.announcer.decide(now, &live) {
                out.push(Notice::new(info_of(&id), Detail::Live, now));
            }
        } else {
            self.announcer.clear_live();
        }

        // Took the lead.
        match (&snap.leader, self.lead.announced.clone()) {
            _ if !public_open => {
                self.lead.announced = snap.leader.clone();
                self.lead.candidate = None;
            }
            (None, _) => self.lead.candidate = None,
            (Some(l), None) => {
                // The first leader is the start of the race, not news.
                self.lead.announced = Some(l.clone());
            }
            (Some(l), Some(current)) if *l == current => self.lead.candidate = None,
            (Some(l), Some(current)) => {
                let since = match &self.lead.candidate {
                    Some((id, since)) if id == l => *since,
                    _ => {
                        self.lead.candidate = Some((l.clone(), now));
                        now
                    }
                };
                let steady = (now - since).to_std().is_ok_and(|d| d >= LEAD_STEADY);
                let rested = self
                    .lead
                    .last_announced_at
                    .is_none_or(|at| (now - at).to_std().is_ok_and(|d| d >= LEAD_COOLDOWN));
                if steady && rested {
                    out.push(Notice::new(
                        info_of(l),
                        Detail::Leader {
                            previous: info_of(&current).map(|i| i.name),
                        },
                        now,
                    ));
                    self.lead.announced = Some(l.clone());
                    self.lead.candidate = None;
                    self.lead.last_announced_at = Some(now);
                }
            }
        }

        for r in &snap.racers {
            let id = &r.info.id;
            let out_of_play = matches!(r.status, RacerStatus::Finished | RacerStatus::Exhausted);

            // Disconnected for long, and back again.
            if r.connected {
                // Playing now, or stopped on purpose (the game was closed: back to waiting).
                if is_playing(r.status) {
                    self.was_playing.insert(id.clone());
                } else {
                    self.was_playing.remove(id);
                }
                self.offline_since.remove(id);
                if self.disconnect_alerted.remove(id) {
                    out.push(Notice::new(
                        Some(r.info.clone()),
                        Detail::Disconnected {
                            minutes: 0,
                            back: true,
                        },
                        now,
                    ));
                }
            } else if !out_of_play && self.was_playing.contains(id) {
                let since = *self.offline_since.entry(id.clone()).or_insert(now);
                let minutes = (now - since).num_minutes();
                if minutes >= t.disconnect_minutes && self.disconnect_alerted.insert(id.clone()) {
                    out.push(Notice::new(
                        Some(r.info.clone()),
                        Detail::Disconnected {
                            minutes,
                            back: false,
                        },
                        now,
                    ));
                }
            } else {
                self.offline_since.remove(id);
            }

            // Running out of time (once per day).
            let playing = matches!(r.status, RacerStatus::Live | RacerStatus::Paused);
            if playing && r.remaining_ms > 0 && r.remaining_ms <= t.low_time_minutes * 60_000 {
                let key = format!("low:{id}:{}", r.day_key);
                if self.low_alerted.insert(key.clone()) {
                    out.push(
                        Notice::new(
                            Some(r.info.clone()),
                            Detail::LowTime {
                                minutes_left: (r.remaining_ms + 59_999) / 60_000,
                            },
                            now,
                        )
                        .keyed(key),
                    );
                }
            }
        }
        out
    }
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

    // ---- went live

    #[test]
    fn a_stream_is_announced_once_after_staying_live_for_a_while() {
        let mut a = Announcer::default();
        assert!(a.decide(t(0), &live(&[("ana", true)])).is_empty());
        assert!(a.decide(t(20), &live(&[("ana", true)])).is_empty());
        assert_eq!(a.decide(t(31), &live(&[("ana", true)])), ["ana"]);
        assert!(a.decide(t(60), &live(&[("ana", true)])).is_empty());
    }

    #[test]
    fn short_blips_do_not_count_and_reconnects_do_not_repeat() {
        let mut a = Announcer::default();
        a.decide(t(0), &live(&[("ana", true)]));
        a.decide(t(10), &live(&[("ana", false)]));
        assert!(a.decide(t(15), &live(&[("ana", true)])).is_empty());
        assert_eq!(a.decide(t(50), &live(&[("ana", true)])), ["ana"]);
        a.decide(t(60), &live(&[("ana", false)]));
        a.decide(t(70), &live(&[("ana", true)]));
        assert!(a.decide(t(200), &live(&[("ana", true)])).is_empty());
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

    // ---- the rest

    fn snap(racers: Vec<RacerSnap>, leader: Option<&str>) -> Snapshot {
        Snapshot {
            event_live: true,
            racers,
            leader: leader.map(str::to_string),
        }
    }
    fn racer(id: &str) -> RacerSnap {
        RacerSnap {
            info: LiveInfo {
                id: id.into(),
                name: id.to_uppercase(),
                avatar_url: None,
                progress_pct: 30.0,
                area: None,
                viewers: None,
                channels: vec![],
            },
            status: RacerStatus::Live,
            remaining_ms: 3 * 3600 * 1000,
            is_live: false,
            connected: true,
            day_key: "d1".into(),
        }
    }
    fn kinds(n: &[Notice]) -> Vec<String> {
        n.iter().map(|n| format!("{:?}", n.kind())).collect()
    }

    #[test]
    fn nothing_is_reported_while_the_event_is_not_running() {
        let mut s = ScanState::default();
        let mut r = racer("ana");
        r.connected = false;
        let mut sn = snap(vec![r], Some("ana"));
        sn.event_live = false;
        assert!(s.scan(&sn, &Thresholds::default(), true, t(0)).is_empty());
        assert!(
            s.scan(&sn, &Thresholds::default(), true, t(9999))
                .is_empty()
        );
    }

    #[test]
    fn the_lead_changes_only_when_the_new_leader_holds_it_and_not_in_bursts() {
        let th = Thresholds::default();
        let mut s = ScanState::default();
        let ana = || racer("ana");
        let beto = || racer("beto");
        // The first leader is the start of the race: silent.
        assert!(
            s.scan(&snap(vec![ana(), beto()], Some("ana")), &th, true, t(0))
                .is_empty()
        );
        // Beto overtakes but has not held it for a minute yet.
        assert!(
            s.scan(&snap(vec![ana(), beto()], Some("beto")), &th, true, t(10))
                .is_empty()
        );
        assert!(
            s.scan(&snap(vec![ana(), beto()], Some("beto")), &th, true, t(50))
                .is_empty()
        );
        let n = s.scan(&snap(vec![ana(), beto()], Some("beto")), &th, true, t(71));
        assert_eq!(n.len(), 1);
        assert_eq!(
            n[0].detail,
            Detail::Leader {
                previous: Some("ANA".into())
            }
        );
        assert_eq!(n[0].racer.as_ref().unwrap().id, "beto");
        // A quick flip back and forth is a blip, never announced.
        s.scan(&snap(vec![ana(), beto()], Some("ana")), &th, true, t(80));
        assert!(
            s.scan(&snap(vec![ana(), beto()], Some("beto")), &th, true, t(100))
                .is_empty()
        );
        // A real change right after is held back by the cooldown, then announced.
        s.scan(&snap(vec![ana(), beto()], Some("ana")), &th, true, t(120));
        assert!(
            s.scan(&snap(vec![ana(), beto()], Some("ana")), &th, true, t(200))
                .is_empty()
        );
        let n = s.scan(&snap(vec![ana(), beto()], Some("ana")), &th, true, t(400));
        assert_eq!(kinds(&n), ["Leader"]);
    }

    #[test]
    fn a_long_disconnection_alerts_once_and_the_return_is_reported() {
        let th = Thresholds {
            disconnect_minutes: 3,
            ..Thresholds::default()
        };
        let mut s = ScanState::default();
        let online = racer("ana");
        let mut offline = racer("ana");
        offline.connected = false;
        assert!(
            s.scan(&snap(vec![online.clone()], None), &th, true, t(0))
                .is_empty()
        );
        // Drops: nothing before the threshold.
        assert!(
            s.scan(&snap(vec![offline.clone()], None), &th, true, t(10))
                .is_empty()
        );
        assert!(
            s.scan(&snap(vec![offline.clone()], None), &th, true, t(150))
                .is_empty()
        );
        let n = s.scan(&snap(vec![offline.clone()], None), &th, true, t(200));
        assert_eq!(n.len(), 1);
        assert_eq!(
            n[0].detail,
            Detail::Disconnected {
                minutes: 3,
                back: false
            }
        );
        // Still offline: no repeats.
        assert!(
            s.scan(&snap(vec![offline.clone()], None), &th, true, t(500))
                .is_empty()
        );
        // Back.
        let n = s.scan(&snap(vec![online.clone()], None), &th, true, t(600));
        assert_eq!(
            n[0].detail,
            Detail::Disconnected {
                minutes: 0,
                back: true
            }
        );
        assert!(
            s.scan(&snap(vec![online], None), &th, true, t(610))
                .is_empty()
        );
    }

    /// A racer as seen after their connection dropped: the engine marks them offline.
    fn dropped(id: &str) -> RacerSnap {
        let mut r = racer(id);
        r.connected = false;
        r.status = RacerStatus::Offline;
        r
    }
    fn waiting(id: &str) -> RacerSnap {
        let mut r = racer(id);
        r.status = RacerStatus::Online;
        r
    }
    fn paused(id: &str) -> RacerSnap {
        let mut r = racer(id);
        r.status = RacerStatus::Paused;
        r
    }

    #[test]
    fn closing_the_game_on_purpose_is_not_a_disconnection() {
        let th = Thresholds::default(); // 3 minutes
        let mut s = ScanState::default();
        assert!(
            s.scan(&snap(vec![racer("ana")], None), &th, false, t(0))
                .is_empty()
        );
        // The session ends (back to waiting) and then HiveShock is closed.
        assert!(
            s.scan(&snap(vec![waiting("ana")], None), &th, false, t(60))
                .is_empty()
        );
        for secs in [70, 400, 4000] {
            assert!(
                s.scan(&snap(vec![dropped("ana")], None), &th, false, t(secs))
                    .is_empty(),
                "{secs}"
            );
        }
    }

    #[test]
    fn a_drop_in_the_middle_of_a_game_is_alerted_once_and_closed_when_back() {
        let th = Thresholds::default();
        let mut s = ScanState::default();
        // Paused counts as playing too.
        s.scan(&snap(vec![paused("ana")], None), &th, false, t(0));
        assert!(
            s.scan(&snap(vec![dropped("ana")], None), &th, false, t(10))
                .is_empty()
        );
        let n = s.scan(&snap(vec![dropped("ana")], None), &th, false, t(200));
        assert_eq!(kinds(&n), ["Disconnected"]);
        assert!(
            s.scan(&snap(vec![dropped("ana")], None), &th, false, t(900))
                .is_empty()
        );
        let n = s.scan(&snap(vec![waiting("ana")], None), &th, false, t(1000));
        assert_eq!(
            n[0].detail,
            Detail::Disconnected {
                minutes: 0,
                back: true
            }
        );
    }

    #[test]
    fn stopping_after_a_drop_does_not_alert_a_second_time_for_the_same_racer() {
        let th = Thresholds::default();
        let mut s = ScanState::default();
        s.scan(&snap(vec![racer("ana")], None), &th, false, t(0));
        s.scan(&snap(vec![dropped("ana")], None), &th, false, t(200));
        // Back, closes the game properly, goes away: no new alert.
        s.scan(&snap(vec![waiting("ana")], None), &th, false, t(300));
        for secs in [310, 700, 5000] {
            assert!(
                s.scan(&snap(vec![dropped("ana")], None), &th, false, t(secs))
                    .is_empty()
            );
        }
    }

    #[test]
    fn a_pause_of_the_event_forgets_who_was_playing_and_a_restart_remembers_who_still_is() {
        let th = Thresholds::default();
        let mut s = ScanState::default();
        s.scan(&snap(vec![racer("ana")], None), &th, false, t(0));
        let mut halted = snap(vec![dropped("ana")], None);
        halted.event_live = false;
        s.scan(&halted, &th, false, t(10));
        // The event runs again: nobody was playing as far as the scanner knows.
        for secs in [20, 400] {
            assert!(
                s.scan(&snap(vec![dropped("ana")], None), &th, false, t(secs))
                    .is_empty()
            );
        }
        // After a restart whoever is connected and playing is already tracked.
        let mut s = ScanState::default();
        s.prime(t(0), &snap(vec![racer("beto")], None));
        s.scan(&snap(vec![dropped("beto")], None), &th, false, t(10));
        let n = s.scan(&snap(vec![dropped("beto")], None), &th, false, t(200));
        assert_eq!(kinds(&n), ["Disconnected"]);
    }

    #[test]
    fn nobody_is_alerted_for_not_being_connected_yet_or_for_being_done() {
        let th = Thresholds::default();
        let mut s = ScanState::default();
        let mut never = racer("nuevo");
        never.connected = false;
        let mut done = racer("fin");
        done.connected = false;
        done.status = RacerStatus::Finished;
        let mut out = racer("sin-tiempo");
        out.connected = false;
        out.status = RacerStatus::Exhausted;
        let sn = snap(vec![never, done, out], None);
        for secs in [0, 400, 4000] {
            assert!(s.scan(&sn, &th, true, t(secs)).is_empty());
        }
    }

    #[test]
    fn low_time_alerts_once_per_day_with_a_key_for_the_database() {
        let th = Thresholds {
            low_time_minutes: 10,
            ..Thresholds::default()
        };
        let mut s = ScanState::default();
        let mut r = racer("ana");
        r.remaining_ms = 11 * 60_000;
        assert!(
            s.scan(&snap(vec![r.clone()], None), &th, true, t(0))
                .is_empty()
        );
        r.remaining_ms = 9 * 60_000 + 10_000;
        let n = s.scan(&snap(vec![r.clone()], None), &th, true, t(60));
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].detail, Detail::LowTime { minutes_left: 10 });
        assert_eq!(n[0].dedupe.as_deref(), Some("low:ana:d1"));
        r.remaining_ms = 5 * 60_000;
        assert!(
            s.scan(&snap(vec![r.clone()], None), &th, true, t(120))
                .is_empty(),
            "once per day"
        );
        // A new day is a new alert.
        r.day_key = "d2".into();
        r.remaining_ms = 8 * 60_000;
        assert_eq!(
            s.scan(&snap(vec![r.clone()], None), &th, true, t(180))
                .len(),
            1
        );
        // Not playing (or already at zero): not "low", it is exhausted and has its own notice.
        r.day_key = "d3".into();
        r.status = RacerStatus::Online;
        assert!(s.scan(&snap(vec![r], None), &th, true, t(240)).is_empty());
    }

    #[test]
    fn while_the_community_channel_is_closed_nothing_stale_is_replayed_later() {
        let th = Thresholds::default();
        let mut s = ScanState::with_announcer(Announcer::with_timing(
            Duration::from_secs(1),
            LIVE_COOLDOWN,
        ));
        let mut ana = racer("ana");
        ana.is_live = true;
        let sn = snap(vec![ana, racer("beto")], Some("ana"));
        // Closed (switched off or rehearsal): she has been live for ages and led all along.
        for secs in [0, 100, 200] {
            assert!(s.scan(&sn, &th, false, t(secs)).is_empty());
        }
        // Opened: the live stream counts from now, and the lead (followed silently) is not "news".
        assert!(s.scan(&sn, &th, true, t(300)).is_empty());
        let n = s.scan(&sn, &th, true, t(302));
        assert_eq!(kinds(&n), ["Live"]);
        // The referees' alerts do not depend on the community channel.
        let mut off = racer("beto");
        off.connected = false;
        let both = snap(vec![racer("ana"), off], None);
        s.scan(&both, &th, false, t(310));
        let n = s.scan(&both, &th, false, t(310 + 200));
        assert_eq!(kinds(&n), ["Disconnected"]);
    }

    #[test]
    fn a_restart_does_not_repeat_what_was_already_true() {
        let th = Thresholds::default();
        let mut s = ScanState::with_announcer(Announcer::with_timing(
            Duration::from_secs(1),
            LIVE_COOLDOWN,
        ));
        let mut ana = racer("ana");
        ana.is_live = true;
        let sn = snap(vec![ana, racer("beto")], Some("ana"));
        s.prime(t(0), &sn);
        // Same picture after the restart: no "went live", no "took the lead".
        assert!(s.scan(&sn, &th, true, t(5)).is_empty());
        assert!(s.scan(&sn, &th, true, t(100)).is_empty());
    }
}
