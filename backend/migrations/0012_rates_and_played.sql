-- What HiveShock asked for in a time donation (the server may compute its own amount from the
-- organizer's rate; both are kept so a wrong HiveShock setup can be spotted).
ALTER TABLE time_donations ADD COLUMN reported_ms INTEGER;

-- Time actually played today (the running clock only), next to the total already kept.
ALTER TABLE racer_state ADD COLUMN played_today_ms INTEGER NOT NULL DEFAULT 0;
