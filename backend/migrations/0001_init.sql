CREATE TABLE event (
    id                     TEXT PRIMARY KEY,
    name                   TEXT NOT NULL,
    game                   TEXT NOT NULL,
    edition                TEXT NOT NULL,
    status                 TEXT NOT NULL,
    start_at_utc           TEXT NOT NULL,
    end_at_utc             TEXT,
    timezone               TEXT NOT NULL,
    daily_budget_seconds   INTEGER NOT NULL,
    daily_reset_local_time TEXT NOT NULL,
    win_condition          TEXT NOT NULL,
    required_objective_ids TEXT NOT NULL
);

CREATE TABLE racers (
    id           TEXT PRIMARY KEY,
    slug         TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    country      TEXT,
    avatar_url   TEXT,
    timezone     TEXT NOT NULL,
    token_hash   TEXT NOT NULL UNIQUE,
    sort_order   INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE channels (
    racer_id TEXT NOT NULL REFERENCES racers(id) ON DELETE CASCADE,
    platform TEXT NOT NULL,
    handle   TEXT NOT NULL,
    url      TEXT NOT NULL,
    PRIMARY KEY (racer_id, platform)
);

CREATE TABLE racer_state (
    racer_id             TEXT PRIMARY KEY REFERENCES racers(id) ON DELETE CASCADE,
    status               TEXT NOT NULL,
    remaining_ms         INTEGER NOT NULL,
    checkpoint_at        TEXT NOT NULL,
    reset_at             TEXT NOT NULL,
    played_ms_total      INTEGER NOT NULL DEFAULT 0,
    progress_pct         REAL NOT NULL DEFAULT 0,
    current_area         TEXT,
    current_objective    TEXT,
    completed_objectives TEXT NOT NULL DEFAULT '[]',
    items                TEXT NOT NULL DEFAULT '{}',
    stats                TEXT NOT NULL DEFAULT '{}',
    finished_at          TEXT,
    final_time_seconds   INTEGER,
    last_heartbeat_at    TEXT,
    stream_live          INTEGER NOT NULL DEFAULT 0,
    viewers              INTEGER,
    thumbnail_url        TEXT
);

CREATE TABLE activity (
    id        TEXT PRIMARY KEY,
    ts        TEXT NOT NULL,
    kind      TEXT NOT NULL,
    racer_id  TEXT,
    racer_name TEXT,
    message   TEXT NOT NULL,
    code      TEXT NOT NULL,
    detail    TEXT,
    subject   TEXT
);
CREATE INDEX activity_ts ON activity (ts DESC);

CREATE TABLE hiveshock_counters (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    game_events     INTEGER NOT NULL DEFAULT 0,
    item_events     INTEGER NOT NULL DEFAULT 0,
    progress_events INTEGER NOT NULL DEFAULT 0,
    chat_events     INTEGER NOT NULL DEFAULT 0
);

-- The "official session log": every organizer action lands here.
CREATE TABLE audit_log (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    ts       TEXT NOT NULL,
    actor    TEXT NOT NULL,
    action   TEXT NOT NULL,
    racer_id TEXT,
    payload  TEXT NOT NULL DEFAULT '{}'
);
