CREATE TABLE web_push_subscriptions
    ( id INTEGER PRIMARY KEY
    , endpoint TEXT NOT NULL UNIQUE ON CONFLICT ABORT
    , keys TEXT NOT NULL
    , expiration_time TEXT NULL
    , user_id INTEGER NULL REFERENCES users(id) ON DELETE CASCADE
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );
