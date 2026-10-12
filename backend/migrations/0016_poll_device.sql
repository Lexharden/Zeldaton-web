-- Second identity for a vote: a random per-browser token (stored hashed). It catches the same
-- browser voting again from another address, e.g. when a dual-stack connection switches between
-- IPv4 and IPv6, or moves from wifi to mobile data. Either identity matching blocks the vote.
ALTER TABLE poll_votes ADD COLUMN device_hash TEXT;
CREATE UNIQUE INDEX poll_votes_device ON poll_votes (device_hash) WHERE device_hash IS NOT NULL;
