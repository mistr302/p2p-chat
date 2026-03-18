mod db;
mod network;
mod settings;
mod setup_tui;
mod tui;
use crate::settings::Settings;
use crate::settings::{create_project_dirs, get_save_file_path};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::{error::Error, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Deserialize, Serialize)]
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
}
#[derive(Deserialize, Serialize)]
pub enum UiClientEventResponse {
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
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    create_project_dirs().unwrap();
    let mut args = std::env::args().skip(1);
    if matches!(args.next().as_deref(), Some("setup")) {
        setup_tui::run_setup()?;
        return Ok(());
    }
    // TODO: add an actual sqlite file
    let sqlite = Arc::new(
        tokio_rusqlite::Connection::open(get_save_file_path(settings::SaveFile::Database))
            .await
            .expect("Couldnt open sqlite connection"),
    );
    // let sqlite = tokio_rusqlite::Connection::open_in_memory()
    //     .await
    //     .expect("Couldnt open sqlite connection");

    // Open unix socket
    let listener = tokio::net::UnixListener::bind("/tmp/p2p-chat.sock").expect("to create");
    let mut _sock = listener.accept().await.expect("to accept");

    let (mut sock_read, mut sock_write) = _sock.0.split();

    db::migrate_db::migrate(&sqlite).await?;

    let settings = Settings::load()?;
    // TODO: Check all required settings while loading and return result when loading
    let (api_writer_tx, mut api_writer_rx) = tokio::sync::mpsc::unbounded_channel::<WriteEvent>();

    let settings = Arc::new(settings);
    let (event_loop, mut client) =
        network::new(sqlite.clone(), settings.clone(), api_writer_tx.clone()).await?;
    // let token = CancellationToken::new();
    // let child_token = token.child_token();
    tokio::spawn(event_loop.run());
    tokio::select! {
        event = read_event(&mut sock_read) => {
            match event? {
                UiClientEvent::SendMessage { peer_id, message } => {
                    client
                        .send_message(PeerId::from_str(&peer_id).unwrap(), message)
                        .await;
                }
                UiClientEvent::SendFriendRequest { peer_id } => {
                    client
                        .send_friend_request(PeerId::from_str(&peer_id).unwrap())
                        .await
                }
                UiClientEvent::AcceptFriendRequest { peer_id } => {
                    client
                        .accept_friend_req(PeerId::from_str(&peer_id).unwrap())
                        .await
                }
                UiClientEvent::DenyFriendRequest { peer_id } => {
                    client
                        .deny_friend_req(PeerId::from_str(&peer_id).unwrap())
                        .await
                }
                UiClientEvent::SearchUsername { username } => client.search_username(username).await,
                UiClientEvent::SearchPeer { peer_id } => client.search_peer(peer_id).await,
                UiClientEvent::CheckUsernameAvailability { username } => {
                    client.check_username_availability(username).await
                }
                UiClientEvent::ChangeUsername { username } => client.change_username(username).await,
                UiClientEvent::LoadChatlogPage { from_peer_id, page } => client.load_chatlog_page(from_peer_id.to_string(), page).await,
                UiClientEvent::LoadFriends => client.load_friends().await,
                UiClientEvent::LoadPendingFriendRequests => client.load_pending_friend_requests().await,
                UiClientEvent::LoadIncomingFriendRequests => {
                    client.load_incoming_friend_requests().await
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
async fn read_event(sock_read: &mut (impl AsyncReadExt + Unpin)) -> anyhow::Result<UiClientEvent> {
    let bytes = sock_read.read_u64().await?;
    let mut buf = vec![0u8; bytes as usize];
    sock_read.read_exact(&mut buf).await?;
    let event = postcard::from_bytes(&buf)?;
    Ok(event)
}
