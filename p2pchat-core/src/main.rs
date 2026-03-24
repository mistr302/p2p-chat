mod db;
mod network;
mod tui;
use dashmap::DashMap;
use libp2p::PeerId;
use libp2p::request_response::OutboundRequestId;
use p2pchat_types::api::{
    CriticalFailure, UiClientEvent, UiClientEventId, UiClientEventRequiringDialMessage,
    UiClientEventResponse, UiClientEventResponseError, UiClientRequest, WriteEvent,
};
use p2pchat_types::settings::{SaveFile, Settings, SettingsLoadError};
use p2pchat_types::settings::{create_project_dirs, get_save_file_path};
use std::str::FromStr;
use std::{error::Error, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_util::sync::CancellationToken;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    create_project_dirs().unwrap();

    // TODO: again remove ARC got fucked
    // let sqlite = Arc::new(
    //     tokio_rusqlite::Connection::open(get_save_file_path(SaveFile::Database))
    //         .await
    //         .expect("Couldnt open sqlite connection"),
    // );
    let sqlite = Arc::new(
        tokio_rusqlite::Connection::open_in_memory()
            .await
            .expect("Couldnt open sqlite connection"),
    );

    // Open unix socket
    // TODO: handle if sock exists
    tokio::fs::remove_file("/tmp/p2p-chat.sock").await.unwrap();
    let listener = tokio::net::UnixListener::bind("/tmp/p2p-chat.sock").expect("to create");
    let mut _sock = listener.accept().await.expect("to accept");

    let (mut sock_read, mut sock_write) = _sock.0.split();

    db::migrate_db::migrate(&sqlite).await?;
    // TODO: handle better xd
    let settings = match Settings::load() {
        Err(e) => {
            write_event(
                &mut sock_write,
                Some(WriteEvent::CriticalFailure(
                    CriticalFailure::FailedToLoadSettings,
                )),
            )
            .await?;
            drop(listener);
            panic!()
        }
        Ok(s) => s,
    };
    let (api_writer_tx, mut api_writer_rx) = tokio::sync::mpsc::unbounded_channel::<WriteEvent>();

    let request_map: Arc<DashMap<OutboundRequestId, UiClientEventId>> = Arc::new(DashMap::new());

    let settings = Arc::new(settings);
    // TODO: If this fails also write a CriticalFailure
    let (event_loop, mut client, buffered) = network::new(
        sqlite.clone(),
        settings.clone(),
        api_writer_tx.clone(),
        request_map.clone(),
    )
    .await?;

    let close_app = CancellationToken::new();
    tokio::spawn(event_loop.run(Some(buffered)));
    loop {
        tokio::select! {
            _ = close_app.cancelled() => {
                break;
                // TODO: Close gracefully
            }
            req = read_event(&mut sock_read) => {
                let req = req?;
                tracing::info!("Received event: {:?}", req);
                let id = req.req_id;
                match req.event {
                    UiClientEvent::Close => {
                        close_app.cancel();
                    }
                    UiClientEvent::EventRequiringDial(ev) => {
                        let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
                        client.is_connected(PeerId::from_str(&ev.peer_id).unwrap(), tx).await;
                        let is_connected = rx.await.expect("to recv");
                        if !is_connected {
                            api_writer_tx
                                .send(crate::WriteEvent::EventResponse(UiClientEventResponse {
                                    result: Err(UiClientEventResponseError::PeerNotDialed),
                                    req_id: id,
                                }))
                                .expect("to send");
                        };
                        if is_connected {
                            match ev.event {
                                UiClientEventRequiringDialMessage::SendMessage { peer_id, message } => {
                                    client
                                        .send_message(PeerId::from_str(&peer_id).unwrap(), message, id)
                                        .await;
                                }
                                UiClientEventRequiringDialMessage::SendFriendRequest { peer_id } => {
                                    client
                                        .send_friend_request(PeerId::from_str(&peer_id).unwrap(), id)
                                        .await
                                }
                                UiClientEventRequiringDialMessage::AcceptFriendRequest { peer_id } => {
                                    client
                                        .accept_friend_req(PeerId::from_str(&peer_id).unwrap(), id)
                                        .await
                                }
                                UiClientEventRequiringDialMessage::DenyFriendRequest { peer_id } => {
                                    client
                                        .deny_friend_req(PeerId::from_str(&peer_id).unwrap(), id)
                                        .await
                                }
                            }
                        }
                    }
                    UiClientEvent::SearchUsername { username } => client.search_username(username, id).await,
                    UiClientEvent::SearchPeer { peer_id } => client.search_peer(peer_id, id).await,
                    UiClientEvent::CheckUsernameAvailability { username } => {
                        client.check_username_availability(username, id).await
                    }
                    UiClientEvent::ChangeUsername { username } => client.change_username(username, id).await,
                    UiClientEvent::LoadChatlogPage { channel_id, page } => client.load_chatlog_page(channel_id, page, id).await,
                    UiClientEvent::LoadFriends => client.load_friends(id).await,
                    UiClientEvent::LoadPendingFriendRequests => client.load_pending_friend_requests(id).await,
                    UiClientEvent::LoadIncomingFriendRequests => {
                        client.load_incoming_friend_requests(id).await
                    }
                    UiClientEvent::Dial { peer_id } => {
                        client.dial(PeerId::from_str(&peer_id).unwrap(), id).await
                    }
                }
            }
            event = api_writer_rx.recv() => {
                // TODO: handle error so it doesnt crash the app xd
                write_event(&mut sock_write, event).await?;
            }
        }
    }
    Ok(())
}
async fn write_event(
    sock_write: &mut (impl AsyncWriteExt + Unpin),
    event: Option<WriteEvent>,
) -> anyhow::Result<()> {
    tracing::info!("Writing event: {:?}", event);
    if let Some(write_event) = event {
        let serialized = postcard::to_allocvec(&write_event)?;
        sock_write.write_u64(serialized.len() as u64).await?;
        sock_write.write_all(&serialized).await?;
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
