PRAGMA foreign_keys = OFF;

BEGIN EXCLUSIVE TRANSACTION;

CREATE TABLE events_
    ( id INTEGER PRIMARY KEY
    , starts_at TEXT NULL
    , title TEXT NOT NULL
    , description TEXT NOT NULL
    , location_id INTEGER NOT NULL REFERENCES locations(id) ON DELETE RESTRICT
    , created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

CREATE TABLE polls_
    ( id INTEGER PRIMARY KEY
    , min_participants INTEGER NOT NULL
    , max_participants INTEGER NOT NULL
    , strategy TEXT NOT NULL
    , open_until TEXT NOT NULl
    , closed INTEGER NOT NULL
    , event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    , CHECK (max_participants >= min_participants)
    , CHECK (min_participants >= 2)
    );

INSERT INTO events_ SELECT * FROM events;
DROP TABLE events;
ALTER TABLE events_ RENAME TO events;

INSERT INTO polls_ SELECT id, min_participants, max_participants, strategy, open_until, closed, id, created_at FROM polls;
--                                                                                              ^^
--                                                                              our poll and event ids just *happen* to line up until now.
DROP TABLE polls;
ALTER TABLE polls_ RENAME TO polls;

CREATE TABLE event_emails
    ( id INTEGER PRIMARY KEY
    , event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE
    , user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE
    , message_id TEXT NOT NULL
    , subject TEXT NOT NULL
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

PRAGMA foreign_key_check;
COMMIT;
