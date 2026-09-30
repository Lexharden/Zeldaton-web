//! Game catalogs. Mirrors `src/config/event.ts` in the frontend: ids are what telemetry reports.

pub const OBJECTIVES: [&str; 10] = [
    "kokiri-forest",
    "deku-tree",
    "dodongos-cavern",
    "jabu-jabu",
    "forest-temple",
    "fire-temple",
    "water-temple",
    "shadow-temple",
    "spirit-temple",
    "ganons-castle",
];

pub const ITEMS: [&str; 9] = [
    "master-sword",
    "hookshot",
    "longshot",
    "bow",
    "bombs",
    "boomerang",
    "megaton-hammer",
    "iron-boots",
    "mirror-shield",
];

pub fn is_objective(id: &str) -> bool {
    OBJECTIVES.contains(&id)
}

pub fn is_item(id: &str) -> bool {
    ITEMS.contains(&id)
}
