-- "Who will win?" public poll: one vote per connection. The key is a salted hash of the voter's
-- address (the address itself is never stored); the primary key is what makes a second vote from the
-- same connection impossible, even when two requests arrive at the same instant.
CREATE TABLE poll_votes (
    ip_hash    TEXT PRIMARY KEY,
    racer_id   TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX poll_votes_racer ON poll_votes (racer_id);
