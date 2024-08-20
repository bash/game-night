PRAGMA foreign_keys = OFF;

BEGIN EXCLUSIVE TRANSACTION;

CREATE TABLE poll_options_
    ( id INTEGER PRIMARY KEY
    , poll_id INTEGER NOT NULL REFERENCES polls(id) ON DELETE CASCADE
    , starts_at TEXT NOT NULL
    );

CREATE TABLE events_
    ( id INTEGER PRIMARY KEY
    , starts_at TEXT NOT NULL
    , title TEXT NOT NULL
    , description TEXT NOT NULL
    , location_id INTEGER NOT NULL REFERENCES locations(id) ON DELETE RESTRICT
    , created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

INSERT INTO poll_options_ SELECT id, poll_id, starts_at FROM poll_options;
DROP TABLE poll_options;
ALTER TABLE poll_options_ RENAME TO poll_options;

INSERT INTO events_ SELECT id, starts_at, title, description, location_id, created_by, created_at FROM events;
DROP TABLE events;
ALTER TABLE events_ RENAME TO events;

PRAGMA foreign_key_check;
COMMIT;
