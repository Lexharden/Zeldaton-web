-- One row per racer and game day (the day between two of that racer's daily resets, named by the
-- local date it started on). Counters are added to as things happen; `partial` marks days that began
-- before this table existed: their counts were rebuilt from the activity, donation and audit logs and
-- time played, sessions, progress and viewers were not measured for the part already gone.
CREATE TABLE racer_days (
    racer_id          TEXT NOT NULL REFERENCES racers(id) ON DELETE CASCADE,
    day               TEXT NOT NULL,
    played_ms         INTEGER NOT NULL DEFAULT 0,
    sessions          INTEGER NOT NULL DEFAULT 0,
    objectives        INTEGER NOT NULL DEFAULT 0,
    items             INTEGER NOT NULL DEFAULT 0,
    bosses            INTEGER NOT NULL DEFAULT 0,
    areas             INTEGER NOT NULL DEFAULT 0,
    donations         INTEGER NOT NULL DEFAULT 0,
    donation_added_ms   INTEGER NOT NULL DEFAULT 0,
    donation_removed_ms INTEGER NOT NULL DEFAULT 0,
    donation_capped   INTEGER NOT NULL DEFAULT 0,
    diamonds          INTEGER NOT NULL DEFAULT 0,
    bits              INTEGER NOT NULL DEFAULT 0,
    adjust_ms         INTEGER NOT NULL DEFAULT 0,
    exhausted         INTEGER NOT NULL DEFAULT 0,
    force_closed      INTEGER NOT NULL DEFAULT 0,
    progress_start    REAL,
    progress_end      REAL,
    peak_viewers      INTEGER,
    partial           INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (racer_id, day)
);
