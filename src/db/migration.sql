CREATE TABLE IF NOT EXISTS pending_friend_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    request_type INTEGER NOT NULL,
    peer_id TEXT NOT NULL,
    FOREIGN KEY (peer_id) REFERENCES contacts(peer_id)
);

CREATE TABLE IF NOT EXISTS friends (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    peer_id TEXT,
    
    FOREIGN KEY (peer_id) REFERENCES contacts(peer_id)
);
-- Contacts table
CREATE TABLE IF NOT EXISTS contacts (
    peer_id TEXT PRIMARY KEY,
    discovery_type INTEGER NOT NULL,
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

