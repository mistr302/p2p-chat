-- Contacts table
CREATE TABLE IF NOT EXISTS contacts (
    peer_id TEXT PRIMARY KEY,
    -- public_key
    name TEXT NOT NULL
);
-- Name table
-- states if name is verified by a central server
-- Messages table
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,              -- uuid::Uuid as TEXT
    content TEXT NOT NULL,
    status INTEGER NOT NULL,          -- MessageStatus stored as integer
    contact_id TEXT NOT NULL,         -- sender
    -- TODO: date column later, e.g.: created_at INTEGER or TEXT
    FOREIGN KEY (contact_id) REFERENCES contacts(peer_id)
);

