CREATE TABLE poll_options_new
    ( id INTEGER PRIMARY KEY
    , poll_id INTEGER NOT NULL REFERENCES polls(id) ON DELETE CASCADE
    , starts_at TEXT NOT NULL
    , ends_at TEXT NOT NULL
    , CHECK (unixepoch(ends_at) - unixepoch(starts_at) >= 0)
    );

-- The assumed duration (in code) up until this change was 4 hours.
INSERT INTO poll_options_new (id, poll_id, starts_at, ends_at)
SELECT id, poll_id, datetime AS starts_at,  strftime('%Y-%m-%dT%H:%M:%SZ', datetime, '+04:00') as ends_at
FROM poll_options;

DROP TABLE poll_options;
ALTER TABLE poll_options_new RENAME TO poll_options;

CREATE TABLE events_new
    ( id INTEGER PRIMARY KEY
    , starts_at TEXT NOT NULL
    , ends_at TEXT NOT NULL
    , description TEXT NOT NULL
    , location_id INTEGER NOT NULL REFERENCES locations(id) ON DELETE RESTRICT
    , created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    , CHECK (unixepoch(ends_at) - unixepoch(starts_at) >= 0)
    );

-- The assumed duration (in code) up until this change was 4 hours.
INSERT INTO events_new (id, starts_at, ends_at, description, location_id, created_by, created_at)
SELECT id, datetime as starts_at, strftime('%Y-%m-%dT%H:%M:%SZ', datetime, '+04:00') as ends_at, description, location_id, created_by, created_at
FROM events;

PRAGMA foreign_keys = OFF;
DROP TABLE events;
ALTER TABLE events_new RENAME TO events;
