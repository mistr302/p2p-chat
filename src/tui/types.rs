use crate::db::types::DiscoveryType;
pub use crate::db::types::MessageStatus;
use serde::{Deserialize, Serialize};
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
