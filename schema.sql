CREATE TABLE users
    ( name TEXT NOT NULL
    , 'role' INTEGER NOT NULL
    , email_address TEXT NOT NULL
    , invited_by INTEGER NULL
    , campaign TEXT NULL
    -- , FOREIGN KEY (invited_by) REFERENCES users(rowid)
    , UNIQUE (email_address)
    );

CREATE TABLE invitations
    ( 'role' INTEGER NOT NULL
    , created_by INTEGER NULL
    , passphrase TEXT NOT NULL
    , valid_until TEXT NULL
    , UNIQUE (passphrase)
    );

CREATE TABLE email_verification_codes
    ( code TEXT NOT NULL
    , email_address TEXT NOT NULL
    , valid_until TEXT NOT NULL
    , UNIQUE (code)
    );

CREATE TABLE login_tokens
    ( type TEXT NOT NULL
    , token TEXT NOT NULL
    , user_id INTEGER NULL
    , valid_until TEXT NOT NULL
    , UNIQUE (token)
    );
