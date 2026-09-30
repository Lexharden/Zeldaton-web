-- Game catalog: items and objectives, editable from the organizer panel. An empty catalog is
-- seeded with the factory one (src/catalog.rs) the first time the server starts.
CREATE TABLE catalog_items (
    id         TEXT PRIMARY KEY,
    grp        TEXT NOT NULL,
    age        TEXT NOT NULL CHECK (age IN ('child', 'adult', 'both')),
    name_es    TEXT NOT NULL,
    name_en    TEXT NOT NULL,
    short      TEXT NOT NULL,
    icon       TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    enabled    INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE catalog_objectives (
    id         TEXT PRIMARY KEY,
    age        TEXT NOT NULL CHECK (age IN ('child', 'adult', 'both')),
    name_es    TEXT NOT NULL,
    name_en    TEXT NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    required   INTEGER NOT NULL DEFAULT 1,
    enabled    INTEGER NOT NULL DEFAULT 1
);
