mod db;
mod ipc;
mod network;
mod settings;
mod setup_tui;
mod tui;
use crate::db::types::DiscoveryType;
use crate::settings::Settings;
use crate::settings::{SettingName, SettingValue, create_project_dirs, get_save_file_path};
use crate::tui::types::{Contact, MessageStatus, Tui};
use libp2p::PeerId;
use libp2p::identity::PublicKey;
use num_enum::TryFromPrimitive;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::{error::Error, sync::Arc};
use tokio::io::AsyncReadExt;
use tokio_rusqlite::params;
use tokio_util::sync::CancellationToken;

#[derive(Deserialize, Serialize)]
enum UiClientEvent {
    SendMessage { peer_id: String, message: String },
    SendFriendRequest { peer_id: String },
    AcceptFriendRequest { peer_id: String },
    SearchUsername { username: String },
    CheckUsernameAvailability { username: String },
    ChangeUsername { username: String },
    LoadChatlogPage,
    LoadFriends,
    LoadPendingFriendRequests,
    LoadIncomingFriendRequests,
}
#[derive(Deserialize, Serialize)]
enum UiClientEventResponse {
    SendMessage,
    SendFriendRequest,
    AcceptFriendRequest,
    SearchUsername,
    CheckUsernameAvailability,
    ChangeUsername,
    LoadChatlogPage,
    LoadFriends,
    LoadPendingFriendRequests,
    LoadIncomingFriendRequests,
}
#[derive(Deserialize, Serialize)]
enum WriteEvent {
    ReceiveMessage(tui::types::Message),
    ReceiveFriendRequest,
    DiscoverMdnsContact,
    EventResponse(UiClientEventResponse),
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let mut args = std::env::args().skip(1);
    if matches!(args.next().as_deref(), Some("setup")) {
        setup_tui::run_setup()?;
        return Ok(());
    }
    create_project_dirs().unwrap();
    // TODO: add an actual sqlite file
    let sqlite = tokio_rusqlite::Connection::open(get_save_file_path(settings::SaveFile::Database))
        .await
        .expect("Couldnt open sqlite connection");
    // Open unix socket
    let mut _sock = tokio::net::UnixStream::connect("/tmp/p2p-chat.sock")
        .await
        .expect("to connect");
    let (mut sock_read, sock_write) = _sock.split();
    // let sqlite = tokio_rusqlite::Connection::open_in_memory()
    //     .await
    //     .expect("Couldnt open sqlite connection");
    db::migrate_db::migrate(&sqlite)
        .await
        .expect("Failed to migrate database");

    let settings = Settings::load();
    // TODO: Check all required settings while loading and return result when loading
    let (api_writer_tx, mut api_writer_rx) = tokio::sync::mpsc::unbounded_channel();

    let settings = Arc::new(settings);
    let (event_loop, mut client) =
        network::new(sqlite.clone(), settings.clone(), api_writer_tx.clone()).await;
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
                UiClientEvent::SearchUsername { username } => client.search_username(username).await,
                UiClientEvent::CheckUsernameAvailability { username } => {
                    client.check_username_availability(username).await
                }
                UiClientEvent::ChangeUsername { username } => client.change_username(username).await,
                UiClientEvent::LoadChatlogPage => client.load_chatlog_page().await,
                UiClientEvent::LoadFriends => client.load_friends().await,
                UiClientEvent::LoadPendingFriendRequests => client.load_pending_friend_requests().await,
                UiClientEvent::LoadIncomingFriendRequests => {
                    client.load_incoming_friend_requests().await
                }
            }
        }
        event = api_writer_rx.recv() => {

        }
    }
    loop {
        let bytes: u64 = sock_read.read_u64().await?;
        let mut buf = Vec::with_capacity(bytes as usize);

        sock_read.read_exact(&mut buf).await.unwrap();
        let event: UiClientEvent = postcard::from_bytes(&buf).unwrap();
    }
}
async fn read_event(sock_read: &mut (impl AsyncReadExt + Unpin)) -> anyhow::Result<UiClientEvent> {
    let bytes = sock_read.read_u64().await?;
    let mut buf = vec![0u8; bytes as usize];
    sock_read.read_exact(&mut buf).await?;
    let event = postcard::from_bytes(&buf)?;
    Ok(event)
}
