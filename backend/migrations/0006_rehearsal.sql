-- Rehearsal mode: while on, the site says the data is a test and the organizer may wipe it with
-- "reset event". Off by default (and for the real event).
ALTER TABLE event ADD COLUMN rehearsal INTEGER NOT NULL DEFAULT 0;
