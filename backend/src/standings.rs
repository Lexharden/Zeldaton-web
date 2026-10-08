//! The ranking. One pure comparator, mirrored by `src/utils/standings.ts` (the website sorts its own
//! table from the racers it holds): `contract/standings-cases.json` is read by both test suites, so
//! the two cannot drift apart.
//!
//! **Finished** racers come first, in the order they crossed the line (the first one is the winner,
//! as the rule says), then by less time really played. **Racing**:
//! 1. more *required* objectives completed (the ones that count for winning);
//! 2. higher progress, compared in tenths of a percent;
//! 3. more items;
//! 4. reached that number of objectives earlier;
//! 5. less time really played;
//! 6. id.
//!
//! Nothing here depends on donations or time adjustments: they move the clock, not the race.

use std::cmp::{Ordering, Reverse};

use chrono::{DateTime, Utc};

use crate::domain::{Racer, RacerStatus, StandingEntry};

fn is_finished(r: &Racer) -> bool {
    r.status == RacerStatus::Finished || r.finished_at_utc.is_some()
}

fn parse(ts: &Option<String>) -> Option<DateTime<Utc>> {
    ts.as_deref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
}

/// Objectives completed that the event requires to win. Extra ones do not count.
pub fn required_done(racer: &Racer, required: &[String]) -> usize {
    racer
        .completed_objectives
        .iter()
        .filter(|id| required.contains(id))
        .count()
}

/// Progress in tenths of a percent: an integer, so the order is total (no NaN, no float noise).
pub fn progress_tenths(pct: f64) -> i64 {
    if pct.is_finite() {
        (pct.clamp(0.0, 100.0) * 10.0).round() as i64
    } else {
        0
    }
}

/// How many items the racer has acquired.
pub fn items_owned(racer: &Racer) -> usize {
    racer.items.values().filter(|owned| **owned).count()
}

/// What a racer is ranked by, ordered best first (`Ord`: smaller is better). Public so the panel
/// can show *why* somebody is ahead.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RankKey {
    /// Finished: when they crossed the line, then the time they played. `None` (still racing) sorts last.
    finished: (u8, Option<DateTime<Utc>>, i64),
    required_done: Reverse<usize>,
    progress_tenths: Reverse<i64>,
    items: Reverse<usize>,
    /// Reached the current number of objectives earlier; no milestone yet sorts last.
    milestone: (u8, Option<DateTime<Utc>>),
    played: i64,
    id: String,
}

pub fn rank_key(r: &Racer, required: &[String]) -> RankKey {
    let finished = if is_finished(r) {
        (
            0,
            parse(&r.finished_at_utc),
            r.final_time_seconds.unwrap_or(i64::MAX),
        )
    } else {
        (1, None, 0)
    };
    let milestone = match parse(&r.milestone_at_utc) {
        Some(at) => (0, Some(at)),
        None => (1, None),
    };
    RankKey {
        finished,
        required_done: Reverse(required_done(r, required)),
        progress_tenths: Reverse(progress_tenths(r.progress_percentage)),
        items: Reverse(items_owned(r)),
        milestone,
        played: r.played_seconds,
        id: r.id.clone(),
    }
}

pub fn compare(a: &Racer, b: &Racer, required: &[String]) -> Ordering {
    rank_key(a, required).cmp(&rank_key(b, required))
}

pub fn compute(racers: &[Racer], required: &[String]) -> Vec<StandingEntry> {
    let mut keyed: Vec<(RankKey, &Racer)> =
        racers.iter().map(|r| (rank_key(r, required), r)).collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    keyed
        .into_iter()
        .enumerate()
        .map(|(i, (_, r))| StandingEntry {
            racer_id: r.id.clone(),
            rank: i as u32 + 1,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::collections::HashMap;

    fn racer(id: &str) -> Racer {
        Racer {
            id: id.into(),
            display_name: id.into(),
            slug: id.into(),
            avatar_url: None,
            country: None,
            timezone: "UTC".into(),
            status: RacerStatus::Live,
            elapsed_seconds: 100,
            played_today_seconds: 0,
            played_seconds: 0,
            remaining_seconds: 100,
            progress_percentage: 0.0,
            current_area: None,
            current_objective: None,
            completed_objectives: vec![],
            finished_at_utc: None,
            final_time_seconds: None,
            milestone_at_utc: None,
            channels: vec![],
            stream: None,
            stats: None,
            items: HashMap::new(),
        }
    }

    fn ids(list: &[Racer], required: &[&str]) -> Vec<String> {
        let required: Vec<String> = required.iter().map(|s| s.to_string()).collect();
        compute(list, &required)
            .into_iter()
            .map(|s| s.racer_id)
            .collect()
    }

    #[test]
    fn progress_is_an_integer_so_the_order_is_total() {
        assert_eq!(progress_tenths(41.649), 416);
        assert_eq!(progress_tenths(41.65), 417);
        assert_eq!(progress_tenths(f64::NAN), 0);
        assert_eq!(progress_tenths(f64::INFINITY), 0);
        assert_eq!(progress_tenths(150.0), 1000);
        assert_eq!(progress_tenths(-3.0), 0);
    }

    #[test]
    fn extra_objectives_do_not_count_and_items_only_break_ties() {
        let mut a = racer("a");
        a.completed_objectives = vec!["x".into(), "y".into(), "z".into()]; // none required
        let mut b = racer("b");
        b.completed_objectives = vec!["r1".into()];
        assert_eq!(ids(&[a.clone(), b.clone()], &["r1", "r2"]), ["b", "a"]);
        // Same objectives and progress: more items first.
        let mut c = racer("c");
        c.completed_objectives = vec!["r1".into()];
        c.items.insert("hookshot".into(), true);
        c.items.insert("bow".into(), false); // listed but not owned
        assert_eq!(ids(&[b, c], &["r1", "r2"]), ["c", "b"]);
    }

    #[test]
    fn donations_and_time_adjustments_do_not_move_anybody() {
        let mut a = racer("a");
        let mut b = racer("b");
        a.progress_percentage = 30.0;
        b.progress_percentage = 30.0;
        // `a` lost an hour of clock to donations, `b` gained one: the old tie-break would have split them.
        a.elapsed_seconds = 9000;
        b.elapsed_seconds = 100;
        a.remaining_seconds = 5400;
        b.remaining_seconds = 18_000;
        assert_eq!(
            ids(&[a, b], &[]),
            ["a", "b"],
            "both untouched: only id decides"
        );
    }

    #[test]
    fn the_order_never_depends_on_the_order_of_the_input() {
        let mut list = vec![];
        for (i, id) in ["d", "a", "c", "b", "e"].iter().enumerate() {
            let mut r = racer(id);
            r.progress_percentage = (i % 3) as f64 * 10.0;
            list.push(r);
        }
        let expected = ids(&list, &[]);
        for shift in 0..list.len() {
            let mut rotated = list.clone();
            rotated.rotate_left(shift);
            assert_eq!(ids(&rotated, &[]), expected, "rotation {shift}");
            rotated.reverse();
            assert_eq!(ids(&rotated, &[]), expected, "reversed {shift}");
        }
    }

    #[test]
    fn the_comparator_is_a_total_order() {
        // Every pair, both ways, over a small grid of values: antisymmetric and transitive.
        let mut pool = vec![];
        for (n, pct, items, mile, played) in [
            (0, 0.0, 0, None, 0),
            (1, 10.0, 1, Some("2026-10-07T13:00:00Z"), 100),
            (1, 10.04, 1, Some("2026-10-07T12:00:00Z"), 100),
            (1, 10.0, 2, None, 50),
            (2, 5.0, 0, Some("2026-10-07T14:00:00Z"), 10),
            (2, 5.0, 0, Some("2026-10-07T14:00:00Z"), 10),
        ] {
            let mut r = racer(&format!("r{}", pool.len()));
            r.completed_objectives = (0..n).map(|i| format!("o{i}")).collect();
            r.progress_percentage = pct;
            for i in 0..items {
                r.items.insert(format!("i{i}"), true);
            }
            r.milestone_at_utc = mile.map(str::to_string);
            r.played_seconds = played;
            pool.push(r);
        }
        let req: Vec<String> = (0..5).map(|i| format!("o{i}")).collect();
        for a in &pool {
            for b in &pool {
                assert_eq!(compare(a, b, &req), compare(b, a, &req).reverse());
                for c in &pool {
                    if compare(a, b, &req).is_le() && compare(b, c, &req).is_le() {
                        assert!(compare(a, c, &req).is_le(), "{} {} {}", a.id, b.id, c.id);
                    }
                }
            }
        }
    }

    /// The cases the website's test reads too: both implementations must agree on every one.
    #[test]
    fn the_shared_golden_cases_hold() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../contract/standings-cases.json"
        );
        let doc: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        for case in doc["cases"].as_array().unwrap() {
            let required: Vec<String> = case["required"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect();
            let racers: Vec<Racer> = case["racers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| {
                    let mut r = racer(v["id"].as_str().unwrap());
                    if let Some(s) = v["status"].as_str() {
                        r.status = serde_json::from_value(Value::String(s.into())).unwrap();
                    }
                    r.completed_objectives = v["objectives"]
                        .as_array()
                        .map(|a| a.iter().map(|x| x.as_str().unwrap().to_string()).collect())
                        .unwrap_or_default();
                    r.progress_percentage = v["progress"].as_f64().unwrap_or(0.0);
                    for i in 0..v["items"].as_u64().unwrap_or(0) {
                        r.items.insert(format!("item{i}"), true);
                    }
                    r.milestone_at_utc = v["milestoneAt"].as_str().map(str::to_string);
                    r.played_seconds = v["played"].as_i64().unwrap_or(0);
                    r.finished_at_utc = v["finishedAt"].as_str().map(str::to_string);
                    r.final_time_seconds = v["finalTime"].as_i64();
                    r.elapsed_seconds = v["elapsed"].as_i64().unwrap_or(100);
                    r
                })
                .collect();
            let want: Vec<&str> = case["order"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap())
                .collect();
            let got: Vec<String> = compute(&racers, &required)
                .into_iter()
                .map(|s| s.racer_id)
                .collect();
            assert_eq!(got, want, "case `{}`", case["name"].as_str().unwrap());
            // Whatever order they arrive in, the result is the same.
            let mut reversed = racers.clone();
            reversed.reverse();
            let again: Vec<String> = compute(&reversed, &required)
                .into_iter()
                .map(|s| s.racer_id)
                .collect();
            assert_eq!(
                again,
                want,
                "case `{}` reversed",
                case["name"].as_str().unwrap()
            );
        }
    }
}
