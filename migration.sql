PRAGMA foreign_keys = OFF;

BEGIN EXCLUSIVE TRANSACTION;

CREATE TABLE polls_
    ( id INTEGER PRIMARY KEY
    , min_participants INTEGER NOT NULL
    , max_participants INTEGER NOT NULL
    , strategy TEXT NOT NULL
    , title TEXT NOT NULL
    , description TEXT NOT NULL
    , location_id INTEGER NOT NULL REFERENCES locations(id) ON DELETE RESTRICT
    , created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT
    , open_until TEXT NOT NULl
    , closed INTEGER NOT NULL
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    , CHECK (max_participants >= min_participants)
    , CHECK (min_participants >= 2)
    );

CREATE TABLE events_
    ( id INTEGER PRIMARY KEY
    , starts_at TEXT NOT NULL
    , ends_at TEXT NOT NULL
    , title TEXT NOT NULL
    , description TEXT NOT NULL
    , location_id INTEGER NOT NULL REFERENCES locations(id) ON DELETE RESTRICT
    , created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    , CHECK (unixepoch(ends_at) - unixepoch(starts_at) >= 0)
    );

INSERT INTO polls_ SELECT id, min_participants, max_participants, strategy, '', description, location_id, created_by, open_until, closed, created_at FROM polls;
DROP TABLE polls;
ALTER TABLE polls_ RENAME TO polls;

INSERT INTO events_ SELECT id, starts_at, ends_at, '', description, location_id, created_by, created_at FROM events;
DROP TABLE events;
ALTER TABLE events_ RENAME TO events;

PRAGMA foreign_key_check;
COMMIT;
