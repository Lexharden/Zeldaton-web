//! Initial data: the event and the confirmed roster. Racers get random ingest tokens that are
//! printed once at seed time; only their SHA-256 hashes are stored.

use std::collections::VecDeque;

use chrono::{DateTime, Utc};

use crate::auth::{hash_token, random_token};
use crate::catalog::default_catalog;
use crate::db::PersistOp;
use crate::domain::*;
use crate::engine::{NewRacer, iso};
use crate::state::RaceState;

/// (id, display name). Timezones default to Mexico City until each racer's real one is known.
pub const ROSTER: [(&str, &str); 9] = [
    ("pinchiviejo", "Pinchiviejo"),
    ("ralbat", "Ralbat"),
    ("xime", "Xime"),
    ("cuaco", "Cuaco"),
    ("kahrilys", "Kahrilys"),
    ("pupperina", "Pupperina"),
    ("speaksins", "Speaksins"),
    ("perlarev", "Perlarev"),
    ("insomnia", "Insomnia"),
];

pub fn default_event() -> EventInfo {
    EventInfo {
        id: "zeldathon-2026".into(),
        name: "Zeldathon".into(),
        game: "Ocarina of Time".into(),
        edition: "2026".into(),
        status: EventStatus::Upcoming,
        // 7 Oct 2026, 06:00 Mexico City (UTC-6, no DST) = the daily reset time.
        start_at_utc: "2026-10-07T12:00:00.000Z".into(),
        end_at_utc: None,
        timezone: "America/Mexico_City".into(),
        daily_budget_seconds: 4 * 3600,
        daily_reset_local_time: "06:00".into(),
        rules: EventRules {
            win_condition:
                "First racer to complete all required objectives crosses the finish line.".into(),
            required_objective_ids: default_catalog().default_required(),
        },
        donation_time: DonationTimePolicy::default(),
    }
}

pub struct Seeded {
    pub state: RaceState,
    pub ops: Vec<PersistOp>,
    /// (racer id, plain token) — shown to the operator once.
    pub tokens: Vec<(String, String)>,
}

pub fn build(now: DateTime<Utc>) -> Seeded {
    let mut state = RaceState {
        event: default_event(),
        racers: vec![],
        activity: VecDeque::new(),
        stats: HiveShockStats::default(),
        winner: None,
        catalog: default_catalog(),
    };
    let mut ops = vec![PersistOp::Event(state.event.clone())];
    for item in &state.catalog.items {
        ops.push(PersistOp::CatalogItem(Box::new(item.clone())));
    }
    for objective in &state.catalog.objectives {
        ops.push(PersistOp::CatalogObjective(Box::new(objective.clone())));
    }
    let mut tokens = vec![];
    for (id, name) in ROSTER {
        let token = random_token();
        let fx = state
            .add_racer(
                NewRacer {
                    id: id.into(),
                    display_name: name.into(),
                    timezone: "America/Mexico_City".into(),
                    country: None,
                    avatar_url: None,
                    channels: vec![],
                },
                hash_token(&token),
                now,
            )
            .expect("seed racer is valid");
        ops.extend(fx.ops);
        tokens.push((id.to_string(), token));
    }
    ops.push(PersistOp::Counters(state.stats.clone()));
    let _ = iso(now);
    Seeded { state, ops, tokens }
}
