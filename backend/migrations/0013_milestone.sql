-- When each racer last changed their number of completed required objectives, to rank who reached
-- the same point first.
ALTER TABLE racer_state ADD COLUMN milestone_at TEXT;
