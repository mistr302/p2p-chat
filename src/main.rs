mod db;
mod network;
mod settings;
mod tui;
use crate::settings::Settings;
use crate::settings::{create_project_dirs, get_save_file_path};
use dashmap::DashMap;
use libp2p::PeerId;
use libp2p::request_response::OutboundRequestId;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::{error::Error, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
#[derive(Deserialize, Serialize, Clone)]
struct UiClientRequest {
    req_id: Uuid,
    event: UiClientEvent,
}
#[derive(Deserialize, Serialize, Clone)]
enum UiClientEvent {
    SendMessage { peer_id: String, message: String },
    SendFriendRequest { peer_id: String },
    AcceptFriendRequest { peer_id: String },
    DenyFriendRequest { peer_id: String },
    SearchUsername { username: String },
    SearchPeer { peer_id: String },
    CheckUsernameAvailability { username: String },
    ChangeUsername { username: String },
    LoadChatlogPage { from_peer_id: String, page: usize },
    LoadFriends,
    LoadPendingFriendRequests,
    LoadIncomingFriendRequests,
    Close,
}
#[derive(Deserialize, Serialize)]
pub enum UiClientEventResponseError {}
#[derive(Deserialize, Serialize)]
pub struct UiClientEventResponse {
    req_id: Uuid,
    result: Result<UiClientEventResponseType, UiClientEventResponseError>,
}
#[derive(Deserialize, Serialize)]
pub enum UiClientEventResponseType {
    SendMessage,
    SendFriendRequest,
    AcceptFriendRequest,
    DenyFriendRequest,
    SearchPeer { username: String },
    SearchUsername { peer_id: String },
    CheckUsernameAvailability(bool),
    ChangeUsername,
    LoadChatlogPage(Vec<crate::tui::types::Message>),
    LoadFriends(Vec<crate::tui::types::Contact>),
    LoadPendingFriendRequests(Vec<crate::tui::types::Contact>),
    LoadIncomingFriendRequests(Vec<crate::tui::types::Contact>),
}
#[derive(Deserialize, Serialize)]
pub enum WriteEvent {
    ReceiveMessage(tui::types::Message),
    ReceiveFriendRequest,
    DiscoverMdnsContact,
    MdnsNameChanged { peer_id: String, name: String },
    EventResponse(UiClientEventResponse),
}
pub struct UiClientEventId(Uuid);
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    create_project_dirs().unwrap();

    let sqlite = Arc::new(
        tokio_rusqlite::Connection::open(get_save_file_path(settings::SaveFile::Database))
            .await
            .expect("Couldnt open sqlite connection"),
    );
    // TODO: Make the hashmap for the ui_request_id -> network_request_id
    let request_map: Arc<DashMap<OutboundRequestId, UiClientEventId>> = Arc::new(DashMap::new());
    // let sqlite = tokio_rusqlite::Connection::open_in_memory()
    //     .await
    //     .expect("Couldnt open sqlite connection");

    // Open unix socket
    let listener = tokio::net::UnixListener::bind("/tmp/p2p-chat.sock").expect("to create");
    let mut _sock = listener.accept().await.expect("to accept");

    let (mut sock_read, mut sock_write) = _sock.0.split();

    db::migrate_db::migrate(&sqlite).await?;
    // TODO: Write an error to sock if failed to load settings
    let settings = Settings::load()?;
    let (api_writer_tx, mut api_writer_rx) = tokio::sync::mpsc::unbounded_channel::<WriteEvent>();

    let settings = Arc::new(settings);
    let (event_loop, mut client) = network::new(
        sqlite.clone(),
        settings.clone(),
        api_writer_tx.clone(),
        request_map.clone(),
    )
    .await?;
    let close_app = CancellationToken::new();
    tokio::spawn(event_loop.run());
    tokio::select! {
        _ = close_app.cancelled() => {
            return Ok(());
        }
        req = read_event(&mut sock_read) => {
            let request = req.clone()?;
            match request.event {
                UiClientEvent::Close => {
                    close_app.cancel();
                }
                UiClientEvent::SendMessage { peer_id, message } => {
                    client
                        .send_message(PeerId::from_str(&peer_id).unwrap(), message, req?.req_id)
                        .await;
                }
                UiClientEvent::SendFriendRequest { peer_id } => {
                    client
                        .send_friend_request(PeerId::from_str(&peer_id).unwrap(), req?.req_id)
                        .await
                }
                UiClientEvent::AcceptFriendRequest { peer_id } => {
                    client
                        .accept_friend_req(PeerId::from_str(&peer_id).unwrap(), req?.req_id)
                        .await
                }
                UiClientEvent::DenyFriendRequest { peer_id } => {
                    client
                        .deny_friend_req(PeerId::from_str(&peer_id).unwrap(), req?.req_id)
                        .await
                }
                UiClientEvent::SearchUsername { username } => client.search_username(username, req?.req_id).await,
                UiClientEvent::SearchPeer { peer_id } => client.search_peer(peer_id, req?.req_id).await,
                UiClientEvent::CheckUsernameAvailability { username } => {
                    client.check_username_availability(username, req?.req_id).await
                }
                UiClientEvent::ChangeUsername { username } => client.change_username(username, req?.req_id).await,
                UiClientEvent::LoadChatlogPage { from_peer_id, page } => client.load_chatlog_page(from_peer_id.to_string(), page, req?.req_id).await,
                UiClientEvent::LoadFriends => client.load_friends(req?.req_id).await,
                UiClientEvent::LoadPendingFriendRequests => client.load_pending_friend_requests(req?.req_id).await,
                UiClientEvent::LoadIncomingFriendRequests => {
                    client.load_incoming_friend_requests(req?.req_id).await
                }
            }
        }
        event = api_writer_rx.recv() => {
            if let Some(write_event) = event {
                let serialized = postcard::to_allocvec(&write_event)?;
                sock_write.write_u64(serialized.len() as u64).await?;
                sock_write.write_all(&serialized).await?;
            }
        }
    }
    Ok(())
}
#[derive(Clone, Debug)]
enum ReadEventError {
    ReadError(String),
    PostCardSerializeError(String),
}
impl Error for ReadEventError {}
impl std::fmt::Display for ReadEventError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReadError(s) => write!(f, "Error reading event: {}", s),
            Self::PostCardSerializeError(s) => write!(f, "Error while deserializing data: {}", s),
        }
    }
}
impl ReadEventError {
    fn from_io_error(err: std::io::Error) -> Self {
        Self::ReadError(err.to_string())
    }
}
async fn read_event(
    sock_read: &mut (impl AsyncReadExt + Unpin),
) -> Result<UiClientRequest, ReadEventError> {
    let bytes = sock_read
        .read_u64()
        .await
        .map_err(ReadEventError::from_io_error)?;
    let mut buf = vec![0u8; bytes as usize];
    sock_read
        .read_exact(&mut buf)
        .await
        .map_err(ReadEventError::from_io_error)?;
    match postcard::from_bytes(&buf) {
        Err(e) => Err(ReadEventError::PostCardSerializeError(e.to_string())),
        Ok(event) => Ok(event),
    }
}
