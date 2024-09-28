PRAGMA foreign_keys = OFF;

BEGIN EXCLUSIVE TRANSACTION;

CREATE TABLE polls_
    ( id INTEGER PRIMARY KEY
    , min_participants INTEGER NOT NULL
    , max_participants INTEGER NOT NULL
    , strategy TEXT NOT NULL
    , open_until TEXT NOT NULl
    , stage TEXT NOT NULL
    , event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    , CHECK (max_participants >= min_participants)
    , CHECK (min_participants >= 2)
    );

INSERT INTO polls_ SELECT id, min_participants, max_participants, strategy, open_until, iif(closed, 'closed', 'open') as stage, event_id, created_at FROM polls;
--                                                                                          ^^^^^^   ^^^^^^
--                                                                                   We assume that all polls in the database were
--                                                                                   finalized successfully if closed.
DROP TABLE polls;
ALTER TABLE polls_ RENAME TO polls;

PRAGMA foreign_key_check;
COMMIT;
