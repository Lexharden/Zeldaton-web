-- One-off Discord notices already sent (a boss defeated, a racer finished, "ran out of time" today...),
-- so a server restart never announces them twice. Emptied when the event is reset.
CREATE TABLE notices_sent (
    key     TEXT PRIMARY KEY,
    sent_at TEXT NOT NULL
);
