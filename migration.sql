PRAGMA foreign_keys = OFF;

BEGIN EXCLUSIVE TRANSACTION;

CREATE TABLE organizers
    ( id INTEGER PRIMARY KEY
    , location_id INTEGER NOT NULL REFERENCES locations(id) ON DELETE CASCADE
    , user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE
    );

CREATE TABLE locations_
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

INSERT INTO locations_ SELECT id, nameplate as description, nameplate, street, street_number, plz, city, floor, created_at FROM locations;
DROP TABLE locations;
ALTER TABLE locations_ RENAME TO locations;

PRAGMA foreign_key_check;
COMMIT;

-- TODO: insert organizers, update location description
