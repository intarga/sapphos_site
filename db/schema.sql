CREATE TABLE IF NOT EXISTS announcements (
    id INTEGER PRIMARY KEY,
    title TEXT,
    body TEXT,
    date TEXT,
    -- TODO: should reference users table?
    author TEXT
);

CREATE TABLE IF NOT EXISTS tower_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    data BLOB NOT NULL,
    expiry INTEGER NOT NULL
);
