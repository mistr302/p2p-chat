mod db;
mod network;
mod settings;
mod tui;
use crate::settings::{create_project_dirs, get_save_file_path};
use crate::tui::types::Tui;
use crate::{network::Event, settings::Settings};
use libp2p::identity::PublicKey;
use std::{error::Error, sync::Arc};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    create_project_dirs().unwrap();
    // TODO: add an actual sqlite file
    let sqlite = tokio_rusqlite::Connection::open(get_save_file_path(settings::SaveFile::Database))
        .await
        .expect("Couldnt open sqlite connection");
    // let sqlite = tokio_rusqlite::Connection::open_in_memory()
    //     .await
    //     .expect("Couldnt open sqlite connection");
    db::migrate_db::migrate(&sqlite)
        .await
        .expect("Failed to migrate database");

    let settings = Settings::load().await;
    // Settings::save(&settings).await;
    let tui = Tui::new();
    let tui_tx = tui.event_tx.clone();

    let settings = Arc::new(RwLock::new(settings));
    let (event_loop, client, mut network_event) =
        network::new(sqlite.clone(), settings.clone(), tui_tx.clone()).await;
    let token = CancellationToken::new();
    let child_token = token.child_token();
    tokio::spawn(event_loop.run());
    tokio::spawn(tui::run(client, token, tui));
    loop {
        // Read full lines from stdin
        tokio::select! {
            _ = child_token.cancelled() => {
                // TODO: Handle gracefully
                return Ok(())
            }
            Some(event) = network_event.recv() => {
                match event {
                    Event::InboundMessage { message, sender } => {
                        tracing::info!("recived message: {}: {}", sender.to_bytes().iter().map(|b| b.to_string()).collect::<String>(), message.content);
                        // TODO: maybe find out if peer id isnt already being sent in libp2p
                        let peer_id = PublicKey::from(*sender).to_peer_id();
                        // // pull name from sqlite
                        // sqlite
                        //     .call(move |c| {
                        //         let mut stmt = c.prepare("SELECT peer_id, name FROM contacts WHERE peer_id LIKE ?1")?;
                        //         stmt.query_one([peer_id.to_string()], |r| {
                        //             Ok(Contact {
                        //                 peer_id,
                        //                 name: r.get(1)?,
                        //             })
                        //         })
                        //     })
                        //     .await.unwrap();
                        let message = crate::tui::types::Message {
                            id: message.id,
                            content: message.content,
                            status: crate::tui::types::MessageStatus::ReceivedNotRead,
                            sender: crate::tui::types::Contact {
                                name: "Anonymous".to_string(),
                                peer_id
                                // peer_id: PeerId::, // I really need that peer_id
                            },
                        };
                        // TODO: save to sqlite

                        // send to tui
                        let _ = tui_tx.send(crate::tui::types::Event::MessageReceived(message));
                    }
                    Event::OutboundMessageReceived { message_id } => {
                        tracing::info!("{} message was received!", message_id);
                    },
                    Event::OutboundMessageInvalidSignature { message_id } => {
                        tracing::info!("outbound messsage has invalid sig");
                    },
                }
            }
        }
    }
}
