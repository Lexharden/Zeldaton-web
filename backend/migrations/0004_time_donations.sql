-- Time from viewer donations (TikTok diamonds, Twitch bits). The streamer decides in HiveShock how
-- many seconds a donation adds or removes; the organizer's policy (event.donation_time) caps it.

-- Organizer policy as JSON (see DonationTimePolicy); '{}' = the defaults.
ALTER TABLE event ADD COLUMN donation_time TEXT NOT NULL DEFAULT '{}';

-- What donations already added / removed today, for the daily caps. Reset with the daily budget.
ALTER TABLE racer_state ADD COLUMN donation_added_ms INTEGER NOT NULL DEFAULT 0;
ALTER TABLE racer_state ADD COLUMN donation_removed_ms INTEGER NOT NULL DEFAULT 0;

-- The official ledger: one row per donation HiveShock reported, with what was actually applied.
-- (racer_id, client_id) is unique so a retried message is never applied twice, even after a restart.
CREATE TABLE time_donations (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    ts           TEXT NOT NULL,
    racer_id     TEXT NOT NULL,
    client_id    TEXT NOT NULL,
    platform     TEXT NOT NULL,
    currency     TEXT NOT NULL,
    amount       INTEGER NOT NULL,
    gift         TEXT,
    gift_count   INTEGER,
    viewer       TEXT,
    requested_ms INTEGER NOT NULL,
    applied_ms   INTEGER NOT NULL,
    limited_by   TEXT,
    UNIQUE (racer_id, client_id)
);
CREATE INDEX time_donations_recent ON time_donations (id DESC);
CREATE INDEX time_donations_racer ON time_donations (racer_id, id DESC);
