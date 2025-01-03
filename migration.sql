PRAGMA foreign_keys = OFF;

BEGIN EXCLUSIVE TRANSACTION;

CREATE TABLE users_
    ( id INTEGER PRIMARY KEY
    , name TEXT NOT NULL
    , role INTEGER NOT NULL
    , symbol TEXT NOT NULL
    , email_address TEXT NOT NULL UNIQUE
    , email_subscription TEXT NOT NULL
    , invited_by INTEGER NULL REFERENCES users(id) ON DELETE RESTRICT
    , campaign TEXT NULL
    , can_update_name INTEGER NOT NULL DEFAULT 1
    , can_answer_strongly INTEGER NOT NULL DEFAULT 0
    , can_update_symbol INTEGER NOT NULL DEFAULT 1
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    , last_active_at NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

INSERT INTO users_ SELECT id, name, role, symbol, email_address, email_subscription, invited_by, campaign, can_update_name, can_answer_strongly, 1 as can_update_symbol, created_at, last_active_at FROM users;
DROP TABLE users;
ALTER TABLE users_ RENAME TO users;

PRAGMA foreign_key_check;
COMMIT;
