CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY NOT NULL,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tower_sessions (
    -- must be TEXT because it comes from the session manager as a 128 bit int, which can't fit in
    -- INTEGER
    id TEXT PRIMARY KEY NOT NULL,
    data BLOB NOT NULL,
    expiry INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS announcements (
    id INTEGER PRIMARY KEY NOT NULL,
    title TEXT,
    body TEXT,
    date TEXT,
    -- TODO: should reference users table?
    author TEXT
);
