-- The event is called Zeldatón (with an accent). Renames the stored name only if it was never customised.
UPDATE event SET name = 'Zeldatón' WHERE name = 'Zeldathon';
