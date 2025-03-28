CREATE TABLE users
    ( id INTEGER PRIMARY KEY -- This is an alias for `rowid` so we get auto-increment and last_insert_rowid() support
    , name TEXT NOT NULL
    , 'role' INTEGER NOT NULL
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

CREATE TABLE invitations
    ( id INTEGER PRIMARY KEY
    , 'role' INTEGER NOT NULL
    , created_by INTEGER NULL
    , passphrase TEXT NOT NULL UNIQUE
    , comment TEXT NOT NULL DEFAULT ''
    , used_by INTEGER NULL REFERENCES users(id) ON DELETE CASCADE
    , valid_until TEXT NULL
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

CREATE TABLE email_verification_codes
    ( id INTEGER PRIMARY KEY
    , code TEXT NOT NULL UNIQUE
    , email_address TEXT NOT NULL
    , valid_until TEXT NOT NULL
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

CREATE TABLE login_tokens
    ( id INTEGER PRIMARY KEY
    , type TEXT NOT NULL
    , token TEXT NOT NULL UNIQUE
    , user_id INTEGER NULL REFERENCES users(id) ON DELETE CASCADE
    , valid_until TEXT NOT NULL
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

CREATE TABLE polls
    ( id INTEGER PRIMARY KEY
    , min_participants INTEGER NOT NULL
    , strategy TEXT NOT NULL
    , open_until TEXT NOT NULl
    , stage TEXT NOT NULL
    , event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

CREATE TABLE poll_options
    ( id INTEGER PRIMARY KEY
    , poll_id INTEGER NOT NULL REFERENCES polls(id) ON DELETE CASCADE
    , starts_at TEXT NOT NULL
    );

CREATE TABLE poll_answers
    ( id INTEGER PRIMARY KEY
    , poll_option_id INTEGER NOT NULL REFERENCES poll_options(id) ON DELETE CASCADE
    , user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE
    , value TEXT NOT NULL
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    , UNIQUE (poll_option_id, user_id) ON CONFLICT REPLACE
    );

CREATE TABLE events
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

CREATE TABLE event_emails
    ( id INTEGER PRIMARY KEY
    , event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE
    , user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE
    , message_id TEXT NOT NULL
    , subject TEXT NOT NULL
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

CREATE TABLE participants
    ( id INTEGER PRIMARY KEY
    , event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE
    , user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE
    , UNIQUE (event_id, user_id) ON CONFLICT REPLACE
    );

CREATE TABLE locations
    ( id INTEGER PRIMARY KEY
    , description TEXT NOT NULL
    , nameplate TEXT NOT NULL
    , street TEXT NOT NULL
    , street_number TEXT NOT NULL
    , plz TEXT NOT NULL
    , city TEXT NOT NULL
    , floor INTEGER NOT NULL
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    , CHECK (floor >= -128)
    , CHECK (floor <= 127)
    );

CREATE TABLE organizers
    ( id INTEGER PRIMARY KEY
    , location_id INTEGER NOT NULL REFERENCES locations(id) ON DELETE CASCADE
    , user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE
    );

CREATE TABLE groups
    ( id INTEGER PRIMARY KEY
    , name TEXT NOT NULL
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

CREATE TABLE members
    ( id INTEGER PRIMARY KEY
    , group_id INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE
    , user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );
