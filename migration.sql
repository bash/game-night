PRAGMA foreign_keys = OFF;

BEGIN EXCLUSIVE TRANSACTION;

CREATE TABLE polls_
    ( id INTEGER PRIMARY KEY
    , min_participants INTEGER NOT NULL
    , strategy TEXT NOT NULL
    , open_until TEXT NOT NULl
    , close_manually INTEGER NOT NULL
    , stage TEXT NOT NULL
    , event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

INSERT INTO polls_ SELECT id, min_participants, strategy, open_until, 0 as close_manually, stage, event_id, created_at FROM polls;
DROP TABLE polls;
ALTER TABLE polls_ RENAME TO polls;

PRAGMA foreign_key_check;
COMMIT;
