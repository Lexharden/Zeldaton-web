-- Who voted: the name each voter typed. Only the organizers see it (the public page shows totals
-- only); votes cast before this column existed have none.
ALTER TABLE poll_votes ADD COLUMN voter_name TEXT;
