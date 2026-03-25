pub mod api;
pub mod settings;
pub mod signable;
pub use chrono;
pub use chrono::{DateTime, NaiveDateTime};
pub use libp2p::identity::Keypair;
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};

// HTTP Tracker constants
pub static HTTP_TRACKER: &str = "127.0.0.1:8000";

// HTTP Tracker request/response types
#[derive(Debug, Serialize, Deserialize)]
pub struct PeerSearchResponse {
    pub peer_id: String,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsernamePayload {
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub peer_id: String,
    pub username: String,
}
#[derive(Debug, PartialEq, Clone, TryFromPrimitive, Serialize, Deserialize)]
#[repr(u8)]
pub enum DiscoveryType {
    Mdns = 0,
    Tracker = 1,
    You = 2,
}

#[derive(Debug, TryFromPrimitive)]
#[repr(u8)]
pub enum FriendRequestType {
    Incoming = 0,
    Outgoing = 1,
}

#[derive(Debug, Clone, TryFromPrimitive, Serialize, Deserialize)]
#[repr(u8)]
pub enum MessageStatus {
    ReceivedNotRead = 0,
    ReceivedRead = 1,
    SentOffNotRead = 2,
    SentOffRead = 3,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Name {
    pub content: String,
    pub ttl: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub content: String,
    pub id: uuid::Uuid,
    pub sender: Contact,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contact {
    pub peer_id: String,
    pub central_name: Option<Name>,
    pub provided_name: Option<Name>,
    pub channel_id: i64,
}
