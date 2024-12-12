PRAGMA foreign_keys = OFF;

BEGIN EXCLUSIVE TRANSACTION;

CREATE TABLE events_
    ( id INTEGER PRIMARY KEY
    , starts_at TEXT NULL
    , title TEXT NOT NULL
    , description TEXT NOT NULL
    , location_id INTEGER NOT NULL REFERENCES locations(id) ON DELETE RESTRICT
    , restrict_to INTEGER NULL REFERENCES groups(id) ON DELETE RESTRICT
    , cancelled INTEGER NOT NULL DEFAULT 0
    , created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

INSERT INTO events_ SELECT id, starts_at, title, description, location_id, restrict_to, 0 as cancelled, created_by, created_at FROM events;
DROP TABLE events;
ALTER TABLE events_ RENAME TO events;

PRAGMA foreign_key_check;
COMMIT;
