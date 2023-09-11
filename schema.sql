CREATE TABLE users
    ( id INTEGER PRIMARY KEY -- This is an alias for `rowid` so we get auto-increment and last_insert_rowid() support
    , name TEXT NOT NULL
    , 'role' INTEGER NOT NULL
    , email_address TEXT NOT NULL UNIQUE
    , invited_by INTEGER NULL REFERENCES users(id) ON DELETE RESTRICT
    , campaign TEXT NULL
    );

CREATE TABLE invitations
    ( id INTEGER PRIMARY KEY
    , 'role' INTEGER NOT NULL
    , created_by INTEGER NULL
    , passphrase TEXT NOT NULL UNIQUE
    , valid_until TEXT NULL
    );

CREATE TABLE email_verification_codes
    ( id INTEGER PRIMARY KEY
    , code TEXT NOT NULL UNIQUE
    , email_address TEXT NOT NULL
    , valid_until TEXT NOT NULL
    );

CREATE TABLE login_tokens
    ( id INTEGER PRIMARY KEY
    , type TEXT NOT NULL
    , token TEXT NOT NULL UNIQUE
    , user_id INTEGER NULL REFERENCES users(id) ON DELETE CASCADE
    , valid_until TEXT NOT NULL
    );

CREATE TABLE polls
    ( id INTEGER PRIMARY KEY
    , min_participants INTEGER NOT NULL
    , max_participants INTEGER NOT NULL
    , strategy TEXT NOT NULL
    , description TEXT NOT NULL
    , created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT
    , open_until TEXT NOT NULl
    , closed INTEGER NOT NULL
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
    , UNIQUE (poll_option_id, user_id) ON CONFLICT REPLACE
    );
