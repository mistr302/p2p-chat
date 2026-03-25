use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UiClientRequest {
    pub req_id: Uuid,
    pub event: UiClientEvent,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UiClientEventRequiringDial {
    pub peer_id: String,
    pub event: UiClientEventRequiringDialMessage,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum UiClientEventRequiringDialMessage {
    ResolveName,
    SendMessage { peer_id: String, message: String },
    SendFriendRequest { peer_id: String },
    AcceptFriendRequest { peer_id: String },
    DenyFriendRequest { peer_id: String },
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum UiClientEvent {
    EventRequiringDial(UiClientEventRequiringDial),
    SearchUsername { username: String },
    SearchPeer { peer_id: String },
    LoadChatlogPage { channel_id: i64, page: usize },
    LoadFriends,
    LoadPendingFriendRequests,
    LoadIncomingFriendRequests,
    Close,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct UiClientEventResponse {
    pub req_id: Uuid,
    pub result: Result<UiClientEventResponseType, UiClientEventResponseError>,
}
#[derive(Deserialize, Serialize, Debug)]
pub enum UiClientEventResponseType {
    SendMessage,
    SendFriendRequest,
    AcceptFriendRequest,
    DenyFriendRequest,
    SearchPeer { username: String },
    SearchUsername { peer_id: String },
    CheckUsernameAvailability(bool),
    ChangeUsername,
    LoadChatlogPage(Vec<crate::Message>),
    LoadFriends(Vec<crate::Contact>),
    LoadPendingFriendRequests(Vec<crate::Contact>),
    LoadIncomingFriendRequests(Vec<crate::Contact>),
}
#[derive(Deserialize, Serialize, Debug)]
pub enum UiClientEventResponseError {
    MessageDeniedNotFriends,
    NetworkError,
    PeerNotDialed,
    SqliteError,
    PeerSearchNotFound,
    PeerSearchServerError,
    PeerSearchTrackerConnectionFailed,
    ChangeNameUsernameExists,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct RelayConnectionSuccess {
    pub relay_addr: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum RelayConnectionError {
    DialError,
    ParseAddrError,
    ReservationError,
}
#[derive(Deserialize, Serialize, Debug)]
pub enum DcutrConnectionError {}
#[derive(Deserialize, Serialize, Debug)]
pub struct DcutrConnectionSuccess {
    pub peer_id: String,
}
#[derive(Deserialize, Serialize, Debug)]

pub struct RelayServerConnectionEvent(pub Result<RelayConnectionSuccess, RelayConnectionError>);
#[derive(Deserialize, Serialize, Debug)]

pub struct DcutrConnectionEvent(pub Result<DcutrConnectionSuccess, DcutrConnectionError>); // THIS CUZ
// ITS KINDA COOL TO KNOW XD

#[derive(Deserialize, Serialize, Debug)]
pub enum CriticalFailure {
    FailedToLoadSettings,
}
#[derive(Deserialize, Serialize, Debug)]
pub enum WriteEvent {
    CriticalFailure(CriticalFailure),
    ReceiveMessage(crate::Message),
    ReceiveFriendRequest,
    // If decision is true -> in tui refetch contact list and pending friend requests, refetch
    // pending friend requests either way
    ReceiveFriendRequestResponse {
        decision: bool,
    },
    DiscoverMdnsContact {
        // This means the mdns contact is connected
        peer_id: String,
        name: Option<String>, // None -> waiting for name; Some -> name
    },
    PeerDisconnected {
        // TODO:
        peer_id: String,
    },
    MdnsNameResolved {
        peer_id: String,
        name: String,
    },
    RelayServerConnection(RelayServerConnectionEvent),
    DcutrConnection(DcutrConnectionEvent),
    EventResponse(UiClientEventResponse),
}
pub struct UiClientEventId(pub Uuid);
