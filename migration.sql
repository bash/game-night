CREATE TABLE organizers
    ( id INTEGER PRIMARY KEY
    , location_id INTEGER NOT NULL REFERENCES locations(id) ON DELETE CASCADE
    , user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE
    );

-- TODO: insert organizers
