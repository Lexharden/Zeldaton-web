//! Pure clock logic: no I/O, fully unit-tested. Same semantics as `src/utils/{clock,time}.ts`.

use chrono::{DateTime, Duration, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;

/// A frozen remaining-time value and the instant it was taken.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Checkpoint {
    pub remaining_ms: i64,
    pub at: DateTime<Utc>,
}

impl Checkpoint {
    /// Remaining time at `now`. Only a running clock counts down; never below zero.
    pub fn remaining(&self, running: bool, now: DateTime<Utc>) -> i64 {
        if running {
            (self.remaining_ms - (now - self.at).num_milliseconds().max(0)).max(0)
        } else {
            self.remaining_ms.max(0)
        }
    }
}

pub fn parse_local_time(value: &str) -> Option<NaiveTime> {
    let (h, m) = value.split_once(':')?;
    NaiveTime::from_hms_opt(h.trim().parse().ok()?, m.trim().parse().ok()?, 0)
}

/// Resolves a wall-clock time in `tz` to UTC. Ambiguous times (DST fall-back) take the first
/// occurrence; nonexistent times (DST spring-forward gap) move forward to the first valid minute.
fn resolve_local(tz: Tz, date: NaiveDate, time: NaiveTime) -> DateTime<Utc> {
    let mut naive = date.and_time(time);
    for _ in 0..=180 {
        if let Some(dt) = tz.from_local_datetime(&naive).earliest() {
            return dt.with_timezone(&Utc);
        }
        naive += Duration::minutes(1);
    }
    Utc.from_utc_datetime(&date.and_time(time))
}

/// Next occurrence (strictly after `now`) of the daily reset wall-clock time in `tz`.
pub fn next_reset_utc(now: DateTime<Utc>, tz: Tz, reset_local: NaiveTime) -> DateTime<Utc> {
    let today = now.with_timezone(&tz).date_naive();
    let candidate = resolve_local(tz, today, reset_local);
    if candidate > now {
        candidate
    } else {
        resolve_local(tz, today.succ_opt().unwrap_or(today), reset_local)
    }
}

/// `HH:MM:SS` for activity details such as "04:00:00 AVAILABLE".
pub fn format_hms(total_seconds: i64) -> String {
    let s = total_seconds.max(0);
    format!("{:02}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(y: i32, m: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, mi, 0).unwrap()
    }

    #[test]
    fn counts_down_only_while_running() {
        let cp = Checkpoint {
            remaining_ms: 10_000,
            at: utc(2026, 1, 1, 0, 0),
        };
        let later = cp.at + Duration::seconds(3);
        assert_eq!(cp.remaining(true, later), 7_000);
        assert_eq!(cp.remaining(false, later), 10_000);
        assert_eq!(cp.remaining(true, cp.at + Duration::hours(1)), 0);
    }

    #[test]
    fn next_reset_is_in_the_racer_timezone() {
        let six = parse_local_time("06:00").unwrap();
        let madrid: Tz = "Europe/Madrid".parse().unwrap();
        // 03:00Z is 04:00 in Madrid (winter): reset later the same day at 06:00 = 05:00Z.
        assert_eq!(
            next_reset_utc(utc(2026, 1, 15, 3, 0), madrid, six),
            utc(2026, 1, 15, 5, 0)
        );
        // After 06:00 local the next reset is tomorrow.
        assert_eq!(
            next_reset_utc(utc(2026, 1, 15, 12, 0), madrid, six),
            utc(2026, 1, 16, 5, 0)
        );
    }

    #[test]
    fn mexico_city_reset_matches_event_start() {
        let six = parse_local_time("06:00").unwrap();
        let cdmx: Tz = "America/Mexico_City".parse().unwrap();
        // 7 Oct 2026, 06:00 CDMX (UTC-6, no DST) = 12:00Z.
        assert_eq!(
            next_reset_utc(utc(2026, 10, 6, 20, 0), cdmx, six),
            utc(2026, 10, 7, 12, 0)
        );
    }

    #[test]
    fn same_wall_clock_different_real_moments() {
        let six = parse_local_time("06:00").unwrap();
        let now = utc(2026, 10, 7, 0, 0);
        let tokyo: Tz = "Asia/Tokyo".parse().unwrap();
        let ny: Tz = "America/New_York".parse().unwrap();
        assert_ne!(
            next_reset_utc(now, tokyo, six),
            next_reset_utc(now, ny, six)
        );
    }

    #[test]
    fn survives_dst_transitions() {
        let six = parse_local_time("06:00").unwrap();
        let ny: Tz = "America/New_York".parse().unwrap();
        // US spring-forward is 2026-03-08: 06:00 local is 10:00Z afterwards, 11:00Z before.
        assert_eq!(
            next_reset_utc(utc(2026, 3, 7, 12, 0), ny, six),
            utc(2026, 3, 8, 10, 0)
        );
        assert_eq!(
            next_reset_utc(utc(2026, 3, 6, 12, 0), ny, six),
            utc(2026, 3, 7, 11, 0)
        );
        // A reset inside the nonexistent 02:30 hour still resolves to a valid instant.
        let gap = parse_local_time("02:30").unwrap();
        let r = next_reset_utc(utc(2026, 3, 8, 5, 0), ny, gap);
        assert!(r > utc(2026, 3, 8, 5, 0) && r < utc(2026, 3, 9, 12, 0));
    }

    #[test]
    fn parses_and_formats() {
        assert_eq!(parse_local_time("06:00"), NaiveTime::from_hms_opt(6, 0, 0));
        assert_eq!(parse_local_time("nope"), None);
        assert_eq!(format_hms(14_400), "04:00:00");
        assert_eq!(format_hms(-5), "00:00:00");
    }
}
