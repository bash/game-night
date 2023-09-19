CREATE TABLE users
    ( id INTEGER PRIMARY KEY -- This is an alias for `rowid` so we get auto-increment and last_insert_rowid() support
    , name TEXT NOT NULL
    , 'role' INTEGER NOT NULL
    , email_address TEXT NOT NULL UNIQUE
    , invited_by INTEGER NULL REFERENCES users(id) ON DELETE RESTRICT
    , campaign TEXT NULL
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

CREATE TABLE invitations
    ( id INTEGER PRIMARY KEY
    , 'role' INTEGER NOT NULL
    , created_by INTEGER NULL
    , passphrase TEXT NOT NULL UNIQUE
    , comment TEXT NOT NULL DEFAULT ''
    , used_by INTEGER NULL REFERENCES users(id)
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
    , max_participants INTEGER NOT NULL
    , strategy TEXT NOT NULL
    , description TEXT NOT NULL
    , location_id INTEGER NOT NULL REFERENCES locations(id) ON DELETE RESTRICT
    , created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT
    , open_until TEXT NOT NULl
    , closed INTEGER NOT NULL
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    , CHECK (max_participants >= min_participants)
    , CHECK (min_participants >= 2)
    );

CREATE TABLE poll_options
    ( id INTEGER PRIMARY KEY
    , poll_id INTEGER NOT NULL REFERENCES polls(id) ON DELETE CASCADE
    , datetime TEXT NOT NULL
    );

CREATE TABLE poll_answers
    ( id INTEGER PRIMARY KEY
    , poll_option_id INTEGER NOT NULL REFERENCES poll_options(id) ON DELETE CASCADE
    , user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT
    , value TEXT NOT NULL
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    , UNIQUE (poll_option_id, user_id) ON CONFLICT REPLACE
    );

CREATE TABLE events
    ( id INTEGER PRIMARY KEY
    , datetime TEXT NOT NULL
    , description TEXT NOT NULL
    , location_id INTEGER NOT NULL REFERENCES locations(id) ON DELETE RESTRICT
    , created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT
    , created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
    );

CREATE TABLE participants
    ( id INTEGER PRIMARY KEY
    , event_id INTEGER NOT NULL REFERENCES events(id) ON DELETE CASCADE
    , user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT
    );

CREATE TABLE locations
    ( id INTEGER PRIMARY KEY
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
