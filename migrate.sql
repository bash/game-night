PRAGMA foreign_keys = OFF;

BEGIN EXCLUSIVE TRANSACTION;
ALTER TABLE users RENAME TO users_;

CREATE TABLE users
    ( id INTEGER PRIMARY KEY
    , name TEXT NOT NULL
    , 'role' INTEGER NOT NULL
    , email_address TEXT NOT NULL UNIQUE
    , email_subscription TEXT NOT NULL
    , invited_by INTEGER NULL REFERENCES users(id) ON DELETE RESTRICT
    , campaign TEXT NULL
    , can_update_name INTEGER NOT NULL DEFAULT 1
    , can_answer_strongly INTEGER NOT NULL DEFAULT 0
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

INSERT INTO users SELECT id, name, role, email_address, 'subscribed' as email_subscription, invited_by, campaign, can_update_name, can_answer_strongly, created_at FROM users_;
DROP TABLE users_;
COMMIT;
