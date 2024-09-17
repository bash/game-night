PRAGMA foreign_keys = OFF;

BEGIN EXCLUSIVE TRANSACTION;

CREATE TABLE users_
    ( id INTEGER PRIMARY KEY -- This is an alias for `rowid` so we get auto-increment and last_insert_rowid() support
    , name TEXT NOT NULL
    , 'role' INTEGER NOT NULL
    , email_address TEXT NOT NULL UNIQUE
    , email_subscription TEXT NOT NULL
    , invited_by INTEGER NULL REFERENCES users(id) ON DELETE RESTRICT
    , campaign TEXT NULL
    , can_update_name INTEGER NOT NULL DEFAULT 1
    , can_answer_strongly INTEGER NOT NULL DEFAULT 0
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    , last_active_at NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

INSERT INTO users_ SELECT id, name, role, email_address, email_subscription, invited_by, campaign, can_update_name, can_answer_strongly, created_at, created_at as last_active_at FROM users;
DROP TABLE users;
ALTER TABLE users_ RENAME TO users;

PRAGMA foreign_key_check;
COMMIT;

-- Estimate last_active_at based on poll answers.
UPDATE users SET last_active_at = max(users.last_active_at, coalesce((SELECT max(created_at) FROM poll_answers WHERE user_id = users.id), ''));
