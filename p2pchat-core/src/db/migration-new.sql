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
    -- public_key maybe after dht
    central_name_id INTEGER,
    provided_name_id INTEGER,
    private_channel_id INTEGER NOT NULL, 

    FOREIGN KEY (private_channel_id) REFERENCES channels(id),
    FOREIGN KEY (central_name_id) REFERENCES names(id),
    FOREIGN KEY (provided_name_id) REFERENCES names(id)
);
CREATE TABLE IF NOT EXISTS names (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ttl DATETIME NOT NULL DEFAULT (datetime('now', '+1 day')),
    content TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS channels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT -- if null use name as channel name
);
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,              -- uuid::Uuid as TEXT
    content TEXT NOT NULL,


    channel_id INTEGER NOT NULL,  -- TODO: changed from text to int check the sql_calls for potential errors       
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,

    FOREIGN KEY (channel_id) REFERENCES channels(id)
);

