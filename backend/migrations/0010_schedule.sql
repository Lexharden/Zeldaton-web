-- The streams' schedule, who referees each one, and the referees' log book.
-- Slots are planning, not race data: they survive an event reset. Notes belong to the run and are
-- emptied with it.
CREATE TABLE schedule_slots (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    racer_id   TEXT NOT NULL REFERENCES racers(id) ON DELETE CASCADE,
    start_utc  TEXT NOT NULL,
    end_utc    TEXT NOT NULL,
    note       TEXT,
    created_by TEXT NOT NULL
);
CREATE INDEX schedule_slots_time ON schedule_slots (start_utc);
CREATE INDEX schedule_slots_racer ON schedule_slots (racer_id, start_utc);

-- A slot can have several referees; a referee can take several slots.
CREATE TABLE slot_assignments (
    slot_id INTEGER NOT NULL REFERENCES schedule_slots(id) ON DELETE CASCADE,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (slot_id, user_id)
);

CREATE TABLE referee_notes (
    id       INTEGER PRIMARY KEY AUTOINCREMENT,
    ts       TEXT NOT NULL,
    racer_id TEXT REFERENCES racers(id) ON DELETE SET NULL,
    author   TEXT NOT NULL,
    text     TEXT NOT NULL
);
CREATE INDEX referee_notes_ts ON referee_notes (id DESC);
