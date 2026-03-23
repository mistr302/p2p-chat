pub mod api;
pub mod settings;
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub content: String,
    pub id: uuid::Uuid,
    pub sender: Contact,
    pub status: MessageStatus,
    // TODO: date
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Contact {
    pub peer_id: String,
    pub name: String,
    pub discovery_type: DiscoveryType,
}
