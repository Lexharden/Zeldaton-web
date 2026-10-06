-- Small organizer settings (key/value): whether the public donors board is shown, Discord on/off...
CREATE TABLE app_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Donors the organizer chose not to show on the public site (still counted and visible in the panel).
-- `viewer_key` is lower(trim(viewer)): the same handle in another case is the same person.
CREATE TABLE hidden_donors (
    platform   TEXT NOT NULL,
    viewer_key TEXT NOT NULL,
    PRIMARY KEY (platform, viewer_key)
);
