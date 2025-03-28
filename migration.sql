PRAGMA foreign_keys = OFF;

BEGIN EXCLUSIVE TRANSACTION;

UPDATE polls SET stage = 'blocked' WHERE stage = 'open' AND close_manually = 1;

CREATE TABLE polls_
    ( id INTEGER PRIMARY KEY
    , min_participants INTEGER NOT NULL
    , strategy TEXT NOT NULL
    , open_until TEXT NOT NULl
    , stage TEXT NOT NULL
    , event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

INSERT INTO polls_ SELECT id, min_participants, strategy, open_until, stage, event_id, created_at FROM polls;
DROP TABLE polls;
ALTER TABLE polls_ RENAME TO polls;

CREATE TABLE events_
    ( id INTEGER PRIMARY KEY
    , starts_at TEXT NULL
    , title TEXT NOT NULL
    , description TEXT NOT NULL
    , location_id INTEGER NOT NULL REFERENCES locations(id) ON DELETE RESTRICT
    , restrict_to INTEGER NULL REFERENCES groups(id) ON DELETE RESTRICT
    , cancelled INTEGER NOT NULL
    , parent_id INTEGER NULL REFERENCES events(id) ON DELETE RESTRICT
    , created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

INSERT INTO events_ SELECT id, starts_at, title, description, location_id, restrict_to, cancelled, NULL as parent_id, created_by, created_at FROM events;
DROP TABLE events;
ALTER TABLE events_ RENAME TO events;

CREATE TABLE poll_options_
    ( id INTEGER PRIMARY KEY
    , poll_id INTEGER NOT NULL REFERENCES polls(id) ON DELETE CASCADE
    , starts_at TEXT NOT NULL
    , promote INTEGER NOT NULL
    );

INSERT INTO poll_options_ SELECT id, poll_id, starts_at, 0 as promote FROM poll_options;
DROP TABLE poll_options;
ALTER TABLE poll_options_ RENAME TO poll_options;

PRAGMA foreign_key_check;
COMMIT;
