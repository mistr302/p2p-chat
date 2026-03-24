use std::str::FromStr;

use libp2p::PeerId;
use num_enum::TryFromPrimitive;
use p2pchat_types::FriendRequestType;
use tokio_rusqlite::{params, rusqlite::Connection};

use crate::db::types::DiscoveryType;
pub fn insert_message(
    conn: &mut Connection,
    m: crate::network::chat::Message,
    message_status: crate::db::types::MessageStatus,
    peer_id: String,
) -> tokio_rusqlite::Result<()> {
    let mut stmt =
        conn.prepare("INSERT INTO messages (id, content, status, contact_id) VALUES (?, ?, ?, ?)")?;
    stmt.execute(params![
        m.id.to_string(),
        m.content,
        message_status as u8,
        peer_id.to_string()
    ])?;
    Ok(())
}
pub fn insert_friend(conn: &mut Connection, peer_id: String) -> tokio_rusqlite::Result<()> {
    let mut stmt = conn.prepare("INSERT INTO friends (peer_id) VALUES (?)")?;
    stmt.execute(params![peer_id])?;
    Ok(())
}
pub fn insert_friend_request(
    conn: &mut Connection,
    request_type: FriendRequestType,
    peer_id: String,
) -> tokio_rusqlite::Result<()> {
    let mut stmt =
        conn.prepare("INSERT INTO pending_friend_requests (request_type, peer_id) VALUES (?, ?)")?;
    stmt.execute(params![request_type as u8, peer_id])?;
    Ok(())
}
pub fn delete_friend_request(conn: &mut Connection, peer_id: String) -> tokio_rusqlite::Result<()> {
    let mut stmt = conn.prepare("DELETE from pending_friend_requests WHERE peer_id = ?")?;
    stmt.execute(params![peer_id])?;
    Ok(())
}

// TODO: Use the page variable
pub fn get_message_log(
    conn: &mut Connection,
    peer_id: String,
    page: usize,
) -> tokio_rusqlite::Result<Vec<crate::tui::types::Message>> {
    let sql = "SELECT m.id, m.content, m.status, c.name, c.discovery_type FROM messages AS m INNER JOIN contacts AS c ON m.contact_id = c.peer_id WHERE contact_id = ?";
    let mut stmt = conn.prepare(sql)?;

    let mut rows = stmt.query(params![peer_id])?;
    let mut log = Vec::new();
    while let Ok(Some(r)) = rows.next() {
        let m = crate::tui::types::Message {
            id: uuid::Uuid::from_str(r.get::<usize, String>(0)?.as_ref()).unwrap(),
            content: r.get(1)?,
            status: crate::db::types::MessageStatus::try_from_primitive(r.get(2)?).unwrap(),
            sender: crate::tui::types::Contact {
                name: r.get(3)?,
                discovery_type: crate::db::types::DiscoveryType::try_from_primitive(r.get(4)?)
                    .unwrap(),
                peer_id: peer_id.to_string(),
            },
        };
        log.push(m);
    }
    Ok(log)
}

pub fn get_contact(
    conn: &mut Connection,
    peer_id: String,
) -> tokio_rusqlite::Result<crate::tui::types::Contact> {
    // TODO: can fail on name being none
    let mut stmt =
        conn.prepare("SELECT peer_id, discovery_type, name from contacts WHERE peer_id = ?")?;
    let res = stmt.query_one(params![peer_id], |r| {
        let c = crate::tui::types::Contact {
            peer_id: r.get(0)?,
            discovery_type: DiscoveryType::try_from_primitive(r.get(1)?).unwrap(),
            name: r.get(2)?,
        };
        Ok(c)
    })?;
    Ok(res)
}
pub fn get_friends(
    conn: &mut Connection,
) -> tokio_rusqlite::Result<Vec<crate::tui::types::Contact>> {
    let mut stmt = conn.prepare("SELECT c.peer_id, c.discovery_type, c.name FROM contacts AS c INNER JOIN friends AS f ON c.peer_id = f.peer_id")?;

    let mut rows = stmt.query([])?;
    let mut contacts = Vec::new();
    while let Ok(Some(r)) = rows.next() {
        let c = crate::tui::types::Contact {
            peer_id: r.get(0)?,
            discovery_type: DiscoveryType::try_from_primitive(r.get(1)?).unwrap(),
            name: r.get(2)?,
        };
        contacts.push(c);
    }
    Ok(contacts)
}
pub fn get_pending_friend_requests(
    conn: &mut Connection,
) -> tokio_rusqlite::Result<Vec<crate::tui::types::Contact>> {
    let mut stmt = conn.prepare("SELECT c.peer_id, c.discovery_type, c.name FROM contacts AS c INNER JOIN pending_friend_requests AS p ON c.peer_id = p.peer_id WHERE p.request_type = ?")?;

    let mut rows = stmt.query(params![crate::db::types::FriendRequestType::Outgoing as u8])?;
    let mut contacts = Vec::new();
    while let Ok(Some(r)) = rows.next() {
        let c = crate::tui::types::Contact {
            peer_id: r.get(0)?,
            discovery_type: DiscoveryType::try_from_primitive(r.get(1)?).unwrap(),
            name: r.get(2)?,
        };
        contacts.push(c);
    }
    Ok(contacts)
}
pub fn get_incoming_friend_requests(
    conn: &mut Connection,
) -> tokio_rusqlite::Result<Vec<crate::tui::types::Contact>> {
    let mut stmt = conn.prepare("SELECT c.peer_id, c.discovery_type, c.name FROM contacts AS c INNER JOIN pending_friend_requests AS p ON c.peer_id = p.peer_id WHERE p.request_type = ?")?;

    let mut rows = stmt.query(params![crate::db::types::FriendRequestType::Incoming as u8])?;
    let mut contacts = Vec::new();
    while let Ok(Some(r)) = rows.next() {
        let c = crate::tui::types::Contact {
            peer_id: r.get(0)?,
            discovery_type: DiscoveryType::try_from_primitive(r.get(1)?).unwrap(),
            name: r.get(2)?,
        };
        contacts.push(c);
    }
    Ok(contacts)
}
