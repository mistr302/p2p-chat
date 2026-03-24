use std::str::FromStr;

use num_enum::TryFromPrimitive;
use p2pchat_types::{FriendRequestType, NaiveDateTime, Name};
use tokio_rusqlite::{params, rusqlite::Connection};

const SQLITE_DATETIME_FMT: &str = "%Y-%m-%d %H:%M:%S";

fn parse_sqlite_datetime(s: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(s, SQLITE_DATETIME_FMT).ok()
}

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(u8)]
pub enum NameType {
    Central = 0,
    Provided = 1,
}

pub fn insert_contact(conn: &mut Connection, peer_id: String) -> tokio_rusqlite::Result<()> {
    let mut ch_stmt = conn.prepare("INSERT INTO channels (name) VALUES (NULL)")?;
    ch_stmt.execute([])?;
    // TODO: may not be the last since were in async
    let channel_id = conn.last_insert_rowid();

    let mut stmt =
        conn.prepare("INSERT INTO contacts (peer_id, private_channel_id) VALUES (?, ?)")?;
    stmt.execute(params![peer_id, channel_id])?;
    Ok(())
}

pub fn insert_name(
    conn: &mut Connection,
    peer_id: String,
    name_type: NameType,
    content: String,
) -> tokio_rusqlite::Result<()> {
    let mut stmt = conn.prepare("INSERT INTO names (content) VALUES (?)")?;
    stmt.execute(params![content])?;
    // TODO: may not be the last since were in async
    let name_id = conn.last_insert_rowid();

    let col = match name_type {
        NameType::Central => "central_name_id",
        NameType::Provided => "provided_name_id",
    };
    let sql = format!("UPDATE contacts SET {} = ? WHERE peer_id = ?", col);
    let mut stmt = conn.prepare(&sql)?;
    stmt.execute(params![name_id, peer_id])?;
    Ok(())
}

pub fn insert_message(
    conn: &mut Connection,
    m: crate::network::chat::Message,
    channel_id: i64,
) -> tokio_rusqlite::Result<()> {
    let mut stmt =
        conn.prepare("INSERT INTO messages (id, content, channel_id) VALUES (?, ?, ?)")?;
    stmt.execute(params![m.id.to_string(), m.content, channel_id])?;
    Ok(())
}

pub fn insert_friend(conn: &mut Connection, peer_id: String) -> tokio_rusqlite::Result<()> {
    let mut stmt = conn.prepare("INSERT INTO friends (peer_id) VALUES (?)")?;
    stmt.execute(params![peer_id])?;
    Ok(())
}

pub fn delete_friend(conn: &mut Connection, peer_id: String) -> tokio_rusqlite::Result<()> {
    let mut stmt = conn.prepare("DELETE FROM friends WHERE peer_id = ?")?;
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
    let mut stmt = conn.prepare("DELETE FROM pending_friend_requests WHERE peer_id = ?")?;
    stmt.execute(params![peer_id])?;
    Ok(())
}

fn parse_contact_row(
    r: &tokio_rusqlite::rusqlite::Row,
) -> tokio_rusqlite::rusqlite::Result<p2pchat_types::Contact> {
    let peer_id: String = r.get(0)?;
    let channel_id: i64 = r.get(1)?;
    let cn_content: Option<String> = r.get(2)?;
    let cn_ttl: Option<String> = r.get(3)?;
    let pn_content: Option<String> = r.get(4)?;
    let pn_ttl: Option<String> = r.get(5)?;

    let central_name = cn_content.zip(cn_ttl).map(|(content, ttl)| Name {
        content,
        ttl: parse_sqlite_datetime(&ttl).expect("invalid ttl in names table"),
    });
    let provided_name = pn_content.zip(pn_ttl).map(|(content, ttl)| Name {
        content,
        ttl: parse_sqlite_datetime(&ttl).expect("invalid ttl in names table"),
    });

    Ok(p2pchat_types::Contact {
        peer_id,
        central_name,
        provided_name,
        channel_id,
    })
}

const CONTACT_SELECT: &str = "\
    SELECT c.peer_id, c.private_channel_id, \
           cn.content, cn.ttl, \
           pn.content, pn.ttl \
    FROM contacts AS c \
    LEFT JOIN names AS cn ON c.central_name_id = cn.id \
    LEFT JOIN names AS pn ON c.provided_name_id = pn.id";

// TODO: Use the page variable
pub fn get_message_log(
    conn: &mut Connection,
    channel_id: i64,
    page: usize,
) -> tokio_rusqlite::Result<Vec<p2pchat_types::Message>> {
    let sql = "SELECT m.id, m.content, m.created_at \
               FROM messages AS m \
               WHERE m.channel_id = ? \
               ORDER BY m.created_at ASC";
    let mut stmt = conn.prepare(sql)?;

    let mut rows = stmt.query(params![channel_id])?;
    let mut log = Vec::new();
    while let Ok(Some(r)) = rows.next() {
        let m = p2pchat_types::Message {
            id: uuid::Uuid::from_str(r.get::<usize, String>(0)?.as_ref()).unwrap(),
            content: r.get(1)?,
            sender: p2pchat_types::Contact {
                peer_id: String::new(),
                central_name: None,
                provided_name: None,
                channel_id,
            },
            created_at: parse_sqlite_datetime(r.get::<usize, String>(2)?.as_ref())
                .expect("couldnt convert str to datetime"),
        };
        log.push(m);
    }
    Ok(log)
}

pub fn get_contact(
    conn: &mut Connection,
    peer_id: String,
) -> tokio_rusqlite::Result<p2pchat_types::Contact> {
    let sql = format!("{} WHERE c.peer_id = ?", CONTACT_SELECT);
    let mut stmt = conn.prepare(&sql)?;
    let res = stmt.query_one(params![peer_id], |r| parse_contact_row(r))?;
    Ok(res)
}

pub fn get_contact_channel_id(
    conn: &mut Connection,
    peer_id: String,
) -> tokio_rusqlite::Result<i64> {
    let mut stmt = conn.prepare("SELECT private_channel_id FROM contacts WHERE peer_id = ?")?;
    let id = stmt.query_one(params![peer_id], |r| r.get(0))?;
    Ok(id)
}

fn get_contacts_by_query(
    conn: &mut Connection,
    where_clause: &str,
    params: &[&dyn tokio_rusqlite::rusqlite::types::ToSql],
) -> tokio_rusqlite::Result<Vec<p2pchat_types::Contact>> {
    let sql = format!("{} {}", CONTACT_SELECT, where_clause);
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params)?;
    let mut contacts = Vec::new();
    while let Ok(Some(r)) = rows.next() {
        contacts.push(parse_contact_row(r)?);
    }
    Ok(contacts)
}

pub fn get_friends(conn: &mut Connection) -> tokio_rusqlite::Result<Vec<p2pchat_types::Contact>> {
    get_contacts_by_query(
        conn,
        "INNER JOIN friends AS f ON c.peer_id = f.peer_id",
        &[],
    )
}

pub fn get_pending_friend_requests(
    conn: &mut Connection,
) -> tokio_rusqlite::Result<Vec<p2pchat_types::Contact>> {
    get_contacts_by_query(
        conn,
        "INNER JOIN pending_friend_requests AS p ON c.peer_id = p.peer_id WHERE p.request_type = ?",
        &[&(FriendRequestType::Outgoing as u8)],
    )
}

pub fn get_incoming_friend_requests(
    conn: &mut Connection,
) -> tokio_rusqlite::Result<Vec<p2pchat_types::Contact>> {
    get_contacts_by_query(
        conn,
        "INNER JOIN pending_friend_requests AS p ON c.peer_id = p.peer_id WHERE p.request_type = ?",
        &[&(FriendRequestType::Incoming as u8)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_rusqlite::rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE channels (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT
            );
            CREATE TABLE names (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ttl DATETIME DEFAULT (datetime('now', '+1 day')),
                content TEXT NOT NULL
            );
            CREATE TABLE contacts (
                peer_id TEXT PRIMARY KEY,
                central_name_id INTEGER,
                provided_name_id INTEGER,
                private_channel_id INTEGER NOT NULL,
                FOREIGN KEY (private_channel_id) REFERENCES channels(id),
                FOREIGN KEY (central_name_id) REFERENCES names(id),
                FOREIGN KEY (provided_name_id) REFERENCES names(id)
            );
            CREATE TABLE friends (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                peer_id TEXT,
                FOREIGN KEY (peer_id) REFERENCES contacts(peer_id)
            );
            CREATE TABLE pending_friend_requests (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                request_type INTEGER NOT NULL,
                peer_id TEXT NOT NULL,
                FOREIGN KEY (peer_id) REFERENCES contacts(peer_id)
            );
            CREATE TABLE messages (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                channel_id TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (channel_id) REFERENCES channels(id)
            );",
        )
        .unwrap();
        conn
    }

    #[test]
    fn test_insert_contact_creates_channel_and_contact() {
        let mut conn = setup_db();
        insert_contact(&mut conn, "peer1".into()).unwrap();

        let channel_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM channels", [], |r| r.get(0))
            .unwrap();
        assert_eq!(channel_count, 1);

        let (peer_id, channel_id): (String, i64) = conn
            .query_row(
                "SELECT peer_id, private_channel_id FROM contacts WHERE peer_id = ?",
                params!["peer1"],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(peer_id, "peer1");
        assert_eq!(channel_id, 1);
    }

    #[test]
    fn test_multiple_contacts_get_separate_channels() {
        let mut conn = setup_db();
        insert_contact(&mut conn, "peer1".into()).unwrap();
        insert_contact(&mut conn, "peer2".into()).unwrap();

        let ch1 = get_contact_channel_id(&mut conn, "peer1".into()).unwrap();
        let ch2 = get_contact_channel_id(&mut conn, "peer2".into()).unwrap();
        assert_ne!(ch1, ch2);
    }

    #[test]
    fn test_insert_name_central() {
        let mut conn = setup_db();
        insert_contact(&mut conn, "peer1".into()).unwrap();
        insert_name(&mut conn, "peer1".into(), NameType::Central, "Alice".into()).unwrap();

        let contact = get_contact(&mut conn, "peer1".into()).unwrap();
        assert!(contact.central_name.is_some());
        assert_eq!(contact.central_name.unwrap().content, "Alice");
        assert!(contact.provided_name.is_none());
    }

    #[test]
    fn test_insert_name_provided() {
        let mut conn = setup_db();
        insert_contact(&mut conn, "peer1".into()).unwrap();
        insert_name(&mut conn, "peer1".into(), NameType::Provided, "Bob".into()).unwrap();

        let contact = get_contact(&mut conn, "peer1".into()).unwrap();
        assert!(contact.provided_name.is_some());
        assert_eq!(contact.provided_name.unwrap().content, "Bob");
        assert!(contact.central_name.is_none());
    }

    #[test]
    fn test_insert_name_both() {
        let mut conn = setup_db();
        insert_contact(&mut conn, "peer1".into()).unwrap();
        insert_name(
            &mut conn,
            "peer1".into(),
            NameType::Central,
            "CentralName".into(),
        )
        .unwrap();
        insert_name(
            &mut conn,
            "peer1".into(),
            NameType::Provided,
            "ProvidedName".into(),
        )
        .unwrap();

        let contact = get_contact(&mut conn, "peer1".into()).unwrap();
        assert_eq!(contact.central_name.unwrap().content, "CentralName");
        assert_eq!(contact.provided_name.unwrap().content, "ProvidedName");
    }

    #[test]
    fn test_name_has_ttl() {
        let mut conn = setup_db();
        insert_contact(&mut conn, "peer1".into()).unwrap();
        insert_name(&mut conn, "peer1".into(), NameType::Central, "Alice".into()).unwrap();

        let contact = get_contact(&mut conn, "peer1".into()).unwrap();
        // ttl is set by the DB default: datetime('now', '+1 day')
        let ttl = contact.central_name.unwrap().ttl;
        assert!(ttl > p2pchat_types::chrono::Local::now().naive_local());
    }

    #[test]
    fn test_get_contact_no_names() {
        let mut conn = setup_db();
        insert_contact(&mut conn, "peer1".into()).unwrap();

        let contact = get_contact(&mut conn, "peer1".into()).unwrap();
        assert_eq!(contact.peer_id, "peer1");
        assert!(contact.central_name.is_none());
        assert!(contact.provided_name.is_none());
    }

    #[test]
    fn test_get_contact_channel_id() {
        let mut conn = setup_db();
        insert_contact(&mut conn, "peer1".into()).unwrap();

        let ch = get_contact_channel_id(&mut conn, "peer1".into()).unwrap();
        let contact = get_contact(&mut conn, "peer1".into()).unwrap();
        assert_eq!(ch, contact.channel_id);
    }

    #[test]
    fn test_insert_message_and_get_message_log() {
        let mut conn = setup_db();
        insert_contact(&mut conn, "peer1".into()).unwrap();
        let channel_id = get_contact_channel_id(&mut conn, "peer1".into()).unwrap();

        let msg = crate::network::chat::Message {
            content: "hello".into(),
            id: uuid::Uuid::new_v4(),
        };
        let msg_id = msg.id;
        insert_message(&mut conn, msg, channel_id).unwrap();

        let log = get_message_log(&mut conn, channel_id, 0).unwrap();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0].content, "hello");
        assert_eq!(log[0].id, msg_id);
    }

    #[test]
    fn test_message_log_empty_channel() {
        let mut conn = setup_db();
        insert_contact(&mut conn, "peer1".into()).unwrap();
        let channel_id = get_contact_channel_id(&mut conn, "peer1".into()).unwrap();

        let log = get_message_log(&mut conn, channel_id, 0).unwrap();
        assert!(log.is_empty());
    }

    #[test]
    fn test_messages_isolated_per_channel() {
        let mut conn = setup_db();
        insert_contact(&mut conn, "peer1".into()).unwrap();
        insert_contact(&mut conn, "peer2".into()).unwrap();
        let ch1 = get_contact_channel_id(&mut conn, "peer1".into()).unwrap();
        let ch2 = get_contact_channel_id(&mut conn, "peer2".into()).unwrap();

        insert_message(
            &mut conn,
            crate::network::chat::Message {
                content: "for peer1".into(),
                id: uuid::Uuid::new_v4(),
            },
            ch1,
        )
        .unwrap();
        insert_message(
            &mut conn,
            crate::network::chat::Message {
                content: "for peer2".into(),
                id: uuid::Uuid::new_v4(),
            },
            ch2,
        )
        .unwrap();

        let log1 = get_message_log(&mut conn, ch1, 0).unwrap();
        assert_eq!(log1.len(), 1);
        assert_eq!(log1[0].content, "for peer1");

        let log2 = get_message_log(&mut conn, ch2, 0).unwrap();
        assert_eq!(log2.len(), 1);
        assert_eq!(log2[0].content, "for peer2");
    }

    #[test]
    fn test_insert_and_delete_friend() {
        let mut conn = setup_db();
        insert_contact(&mut conn, "peer1".into()).unwrap();
        insert_friend(&mut conn, "peer1".into()).unwrap();

        let friends = get_friends(&mut conn).unwrap();
        assert_eq!(friends.len(), 1);
        assert_eq!(friends[0].peer_id, "peer1");

        delete_friend(&mut conn, "peer1".into()).unwrap();
        let friends = get_friends(&mut conn).unwrap();
        assert_eq!(friends.len(), 0);
    }

    #[test]
    fn test_friends_list_shows_names() {
        let mut conn = setup_db();
        insert_contact(&mut conn, "peer1".into()).unwrap();
        insert_name(
            &mut conn,
            "peer1".into(),
            NameType::Provided,
            "Alice".into(),
        )
        .unwrap();
        insert_friend(&mut conn, "peer1".into()).unwrap();

        let friends = get_friends(&mut conn).unwrap();
        assert_eq!(friends.len(), 1);
        assert_eq!(friends[0].provided_name.as_ref().unwrap().content, "Alice");
    }

    #[test]
    fn test_insert_and_delete_friend_request() {
        let mut conn = setup_db();
        insert_contact(&mut conn, "peer1".into()).unwrap();
        insert_friend_request(&mut conn, FriendRequestType::Outgoing, "peer1".into()).unwrap();

        let pending = get_pending_friend_requests(&mut conn).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].peer_id, "peer1");

        let incoming = get_incoming_friend_requests(&mut conn).unwrap();
        assert_eq!(incoming.len(), 0);

        delete_friend_request(&mut conn, "peer1".into()).unwrap();
        let pending = get_pending_friend_requests(&mut conn).unwrap();
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn test_incoming_friend_request() {
        let mut conn = setup_db();
        insert_contact(&mut conn, "peer1".into()).unwrap();
        insert_friend_request(&mut conn, FriendRequestType::Incoming, "peer1".into()).unwrap();

        let incoming = get_incoming_friend_requests(&mut conn).unwrap();
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].peer_id, "peer1");

        let pending = get_pending_friend_requests(&mut conn).unwrap();
        assert_eq!(pending.len(), 0);
    }

    #[test]
    fn test_delete_friend_nonexistent_is_ok() {
        let mut conn = setup_db();
        // should not error even if no rows match
        delete_friend(&mut conn, "nobody".into()).unwrap();
    }

    #[test]
    fn test_delete_friend_request_nonexistent_is_ok() {
        let mut conn = setup_db();
        delete_friend_request(&mut conn, "nobody".into()).unwrap();
    }
}
