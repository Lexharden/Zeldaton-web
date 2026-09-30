use std::cmp::Ordering;

use crate::domain::{Racer, RacerStatus, StandingEntry};

fn is_finished(r: &Racer) -> bool {
    r.status == RacerStatus::Finished || r.finished_at_utc.is_some()
}

/// Ranking order. Kept identical to `compareRacers` in `src/utils/standings.ts`:
/// finished first (earliest final time), then more objectives, higher progress,
/// less time used, and finally name.
pub fn compare(a: &Racer, b: &Racer) -> Ordering {
    let (fa, fb) = (is_finished(a), is_finished(b));
    if fa != fb {
        return if fa {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }
    if fa && fb {
        let ta = a.final_time_seconds.unwrap_or(i64::MAX);
        let tb = b.final_time_seconds.unwrap_or(i64::MAX);
        if ta != tb {
            return ta.cmp(&tb);
        }
    }
    b.completed_objectives
        .len()
        .cmp(&a.completed_objectives.len())
        .then(
            b.progress_percentage
                .partial_cmp(&a.progress_percentage)
                .unwrap_or(Ordering::Equal),
        )
        .then(a.elapsed_seconds.cmp(&b.elapsed_seconds))
        .then(
            a.display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase()),
        )
}

pub fn compute(racers: &[Racer]) -> Vec<StandingEntry> {
    let mut sorted: Vec<&Racer> = racers.iter().collect();
    sorted.sort_by(|a, b| compare(a, b));
    sorted
        .into_iter()
        .enumerate()
        .map(|(i, r)| StandingEntry {
            racer_id: r.id.clone(),
            rank: i as u32 + 1,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn racer(id: &str, done: usize, pct: f64) -> Racer {
        Racer {
            id: id.into(),
            display_name: id.into(),
            slug: id.into(),
            avatar_url: None,
            country: None,
            timezone: "UTC".into(),
            status: RacerStatus::Live,
            elapsed_seconds: 100,
            remaining_seconds: 100,
            progress_percentage: pct,
            current_area: None,
            current_objective: None,
            completed_objectives: (0..done).map(|i| i.to_string()).collect(),
            finished_at_utc: None,
            final_time_seconds: None,
            channels: vec![],
            stream: None,
            stats: None,
            items: HashMap::new(),
        }
    }

    #[test]
    fn ranks_by_objectives_then_progress() {
        let list = [
            racer("a", 1, 15.0),
            racer("b", 2, 25.0),
            racer("c", 1, 19.0),
        ];
        let ids: Vec<_> = compute(&list).into_iter().map(|s| s.racer_id).collect();
        assert_eq!(ids, ["b", "c", "a"]);
    }

    #[test]
    fn finished_first_by_final_time() {
        let mut b = racer("b", 10, 100.0);
        b.status = RacerStatus::Finished;
        b.final_time_seconds = Some(200);
        let mut c = racer("c", 10, 100.0);
        c.status = RacerStatus::Finished;
        c.final_time_seconds = Some(100);
        let list = [racer("a", 3, 90.0), b, c];
        let ranked = compute(&list);
        assert_eq!(ranked[0].racer_id, "c");
        assert_eq!(ranked[0].rank, 1);
        assert_eq!(ranked[2].racer_id, "a");
    }
}
