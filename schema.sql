CREATE TABLE users
    ( name TEXT NOT NULL
    , 'role' INTEGER NOT NULL
    , email_address TEXT NOT NULL
    , invited_by INTEGER NULL
    -- , FOREIGN KEY (invited_by) REFERENCES users(rowid)
    , UNIQUE (email_address)
    );

CREATE TABLE invitations
    ( 'role' INTEGER NOT NULL
    , created_by INTEGER NULL
    , passphrase TEXT NOT NULL
    , UNIQUE (passphrase)
    );
