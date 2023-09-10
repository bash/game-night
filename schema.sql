CREATE TABLE users
    ( id INTEGER PRIMARY KEY -- This is an alias for `rowid` so we get auto-increment and last_insert_rowid() support
    , name TEXT NOT NULL
    , 'role' INTEGER NOT NULL
    , email_address TEXT NOT NULL UNIQUE
    , invited_by INTEGER NULL REFERENCES users(id)
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
    , user_id INTEGER NULL
    , valid_until TEXT NOT NULL
    );
