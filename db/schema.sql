CREATE TABLE IF NOT EXISTS announcements (
    id INTEGER PRIMARY KEY,
    title TEXT,
    body TEXT,
    date TEXT,
    -- TODO: should reference users table?
    author TEXT
)
