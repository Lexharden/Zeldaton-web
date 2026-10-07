-- Referee monitor. Incidents are what the scanner and the engine detect (long disconnection, low time,
-- suspicious jump, donation cap, out of time), kept so a referee who joins late can review them even
-- when Discord is off. Emptied when the event is reset.
CREATE TABLE incidents (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    ts          TEXT NOT NULL,
    kind        TEXT NOT NULL,
    severity    TEXT NOT NULL CHECK (severity IN ('info','warn','error')),
    racer_id    TEXT,
    racer_name  TEXT,
    message     TEXT NOT NULL,
    payload     TEXT NOT NULL DEFAULT '{}',
    status      TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','reviewed','dismissed')),
    reviewed_by TEXT,
    reviewed_at TEXT,
    note        TEXT,
    -- When the condition itself ended (the racer reconnected...), independent of the review.
    ended_at    TEXT
);
CREATE INDEX incidents_status ON incidents (status, id DESC);
CREATE INDEX incidents_racer ON incidents (racer_id, kind, id DESC);

-- When each referee last opened the monitor, for the "since your last visit" summary.
ALTER TABLE users ADD COLUMN monitor_seen_at TEXT;
