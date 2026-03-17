use base64::{Engine as _, engine::general_purpose};
use futures::StreamExt;
use libp2p::{
    PeerId, StreamProtocol, Swarm,
    identity::{Keypair, ed25519::PublicKey},
    mdns, noise,
    request_response::{self, ProtocolSupport},
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use num_enum::TryFromPrimitive;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_rusqlite::{Connection, params};
use uuid::Uuid;

use crate::{
    WriteEvent, db::types::{DiscoveryType, MessageStatus}, network::{
        chat::{
            ChatCommand, DirectMessageRequest, DirectMessageResponse, Message, MessageResponse,
        },
        friends::{FriendCommand, FriendRequest, FriendResponse},
        signable::sign,
    }, settings::{SettingName, SettingValue}, tui::types::{Contact, Event::EditContactName}
};
pub mod chat;
pub mod friends;
pub mod signable;

pub enum Command {
    ChatCommand(ChatCommand),
    FriendCommand(FriendCommand),
}
pub(crate) async fn new(
    sqlite_conn: Connection,
    settings: Arc<HashMap<SettingName, SettingValue>>,
    api_writer_tx: UnboundedSender<WriteEvent>,
) -> (EventLoop, Client) {
    // TODO: Confiugre properly & handle errors
    // Dont generate identities on every run, create a store

    let id = match settings.get(&SettingName::KeyPair) {
        Some(SettingValue::String(Some(s))) => {
            let bytes = general_purpose::STANDARD
                .decode(s)
                .expect("Couldnt decode saved keypair");
            Keypair::from_protobuf_encoding(&bytes).expect("Couldnt parse the saved keypair")
        }
        Some(SettingValue::Bytes(Some(s))) => {
            Keypair::from_protobuf_encoding(s).expect("Couldnt parse the saved keypair")
        }
        _ => {
            panic!("Keypair missing. Run `app-bin setup` to generate settings.");
        }
    };

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(id.clone())
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )
        .unwrap()
        .with_quic()
        .with_behaviour(|key| {
            let mdns =
                mdns::tokio::Behaviour::new(mdns::Config::default(), key.public().to_peer_id())?;
            let direct_message = libp2p::request_response::cbor::Behaviour::new(
                [(
                    StreamProtocol::new("/direct-message/1"),
                    ProtocolSupport::Full,
                )],
                request_response::Config::default(),
            );
            let friends = libp2p::request_response::cbor::Behaviour::new(
                [(StreamProtocol::new("/friends/1"), ProtocolSupport::Full)],
                request_response::Config::default(),
            );
            Ok(Behaviour {
                mdns,
                direct_message,
                friends,
            })
        })
        .unwrap()
        .build();
    // Listen on all interfaces and whatever port the OS assigns
    swarm
        .listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse().unwrap())
        .unwrap();
    swarm
        .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
        .unwrap();
    let (command_tx, command_rx) = mpsc::channel(100);
    let client = Client {
        settings: settings.clone(),
        command_sender: command_tx,
        keys: id.clone(),
        id: PeerId::from_public_key(&id.public()),
    };
    let event_loop = EventLoop {
        swarm,
        command_rx,
        settings,
        keys: id,
        api_writer_tx,
        sqlite_conn,
        client: client.clone(),
    };
    (event_loop, client)
}
#[derive(NetworkBehaviour)]
struct Behaviour {
    mdns: mdns::tokio::Behaviour,
    direct_message:
        libp2p::request_response::cbor::Behaviour<DirectMessageRequest, DirectMessageResponse>,
    friends:
        libp2p::request_response::cbor::Behaviour<FriendRequest, FriendResponse>,
}
pub struct EventLoop {
    swarm: Swarm<Behaviour>,
    command_rx: mpsc::Receiver<Command>,
    settings: Arc<HashMap<SettingName, SettingValue>>,
    keys: Keypair,
    sqlite_conn: Connection,
    api_writer_tx: UnboundedSender<WriteEvent>,
    client: Client,
}
#[derive(Clone)]
pub(crate) struct Client {
    pub command_sender: mpsc::Sender<Command>,
    settings: Arc<HashMap<SettingName, SettingValue>>,
    keys: Keypair,
    pub id: PeerId,
}
impl EventLoop {
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => self.handle_event(event).await,
                Some(command) = self.command_rx.recv() => {
                    match command {
                        Command::ChatCommand(chat) => self.handle_chat_command(chat).await,
                        Command::FriendCommand(friend) => self.handle_friend_command(friend).await,
                    }
                },
            }
        }
    }
    async fn handle_event(&mut self, event: SwarmEvent<BehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                let mut known = Vec::<PeerId>::new();
                for (peer_id, _multiaddr) in list {
                    tracing::info!("{peer_id} peer connected!");
                    // Maybe dial and get locally set name
                    if !known.contains(&peer_id) {
                        // let _ = self.tui_tx.send(crate::tui::types::Event::AddContact(
                        //     crate::tui::types::Contact {
                        //         peer_id: peer_id.to_string(),
                        //         name: "Anonymous".to_string(),
                        //         discovery_type: DiscoveryType::Mdns,
                        //     },
                        // ));
                        known.push(peer_id);

                        let _ = self
                            .sqlite_conn
                            .call(move |c| {
                                let mut stmt = c.prepare(
                                    "INSERT INTO contacts(peer_id, discovery_type) VALUES(?, ?)",
                                )?;
                                stmt.execute(params![
                                    peer_id.to_string(),
                                    DiscoveryType::Mdns as u8
                                ])
                            })
                            .await;
                        self.client.request_name(peer_id).await;
                        // TODO: Send the mdns record
                    }
                }
            }
            SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                for (peer_id, _multiaddr) in list {
                    tracing::info!("{peer_id} expired mDNS");
                }
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                tracing::info!("Local node is listening on {address}");
            }
            SwarmEvent::Behaviour(BehaviourEvent::DirectMessage(
                request_response::Event::Message { message, peer, .. },
            )) => {
                match message {
                    request_response::Message::Request {
                        request, channel, ..
                    } => {
                        let message = request.0;
                        let peer_id = peer;

                        let m = message.clone();
                        self.sqlite_conn
                            .call(move |c| {
                                let mut stmt = c.prepare("INSERT INTO messages (id, content, status, contact_id) VALUES (?, ?, ?, ?)")?;
                                stmt.execute(params![m.id.to_string(), m.content, (MessageStatus::ReceivedNotRead as u8), peer_id.to_string()])
                            })
                            .await.unwrap();

                        let contact = self.sqlite_conn 
                            .call(move |c| {
                                let mut stmt = c.prepare("SELECT name, discovery_type FROM contacts WHERE peer_id LIKE ?1")?;
                                stmt.query_one([peer_id.to_string()], |r| {
                                    Ok(Contact {
                                        peer_id: peer_id.to_string(),
                                        name: r.get(0)?,
                                        discovery_type: DiscoveryType::try_from_primitive(r.get(1)?).unwrap(),
                                    })
                                })
                            })
                            .await.unwrap();

                        // if message is valid, send ack
                        self.swarm
                            .behaviour_mut()
                            .direct_message
                            .send_response(
                                channel,
                                DirectMessageResponse(MessageResponse::ACK {
                                    message_id: message.id,
                                }),
                            )
                            .expect("to be sent");
                        // TODO: Send to ui through the api
                        let message = crate::tui::types::Message{
                            content: message.content,
                            id: message.id,
                            sender: contact,
                            status: crate::db::types::MessageStatus::ReceivedNotRead,
                        };
                        self.api_writer_tx.send(WriteEvent::ReceiveMessage(message)).expect("to send");
                    }
                    request_response::Message::Response { response, .. } => match response {
                        DirectMessageResponse(MessageResponse::ACK { message_id }) => {
                            // TODO:
                        }
                        DirectMessageResponse(MessageResponse::InvalidSignature { message_id }) => {
                            // TODO:

                        }
                    },
                }
            }
            SwarmEvent::Behaviour(BehaviourEvent::Friends(request_response::Event::Message {
                peer,
                connection_id,
                message,
            })) => match message {
                request_response::Message::Request {
                    request_id,
                    request,
                    channel,
                } => match request {
                    FriendRequest::RequestName => {
                        let name = self.settings.get(&SettingName::Name);
                        self.swarm
                            .behaviour_mut()
                            .friends
                            .send_response(
                                channel,
                                    FriendResponse::RequestName {
                                        name: match name.unwrap() {
                                            SettingValue::String(val) => {
                                                val.clone().unwrap_or("Anonymous".to_string())
                                            }
                                            _ => unimplemented!("undefined behaviour"),
                                        },
                                    },
                             
                            )
                            .expect("On Name request to be sent");
                    }
                    FriendRequest::VerifyName { name } => {
                        let SettingValue::String(Some(curr_name)) = self
                            .settings
                            .get(&SettingName::Name)
                            .expect("name opt to exist")
                        else {
                            unimplemented!("");
                        };
                        self.swarm
                            .behaviour_mut()
                            .friends
                            .send_response(
                                channel,
                                    FriendResponse::VerifyName(match name == *curr_name {
                                        true => None,
                                        false => Some(curr_name.clone()),
                                    }),
                            )
                            .expect("to send res");
                    }
                    FriendRequest::AcceptFriend { decision } => {
                        self.swarm
                            .behaviour_mut()
                            .friends
                            .send_response(
                                channel,
                                FriendResponse::AcceptFriendAck,
                            )
                            .expect("to send res");
                    }
                    FriendRequest::AddFriend => self
                        .swarm
                        .behaviour_mut()
                        .friends
                        .send_response(channel, FriendResponse::AddFriendAck)
                        .expect("to send res"),
                },

                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                        match response {
                            FriendResponse::RequestName { name } => {
                                tracing::info!("Received valid name response");
                                let n = name.clone();
                                let res = self.sqlite_conn
                                    .call(move |c| {
                                        let mut stmt = c.prepare(
                                            "UPDATE contacts SET name=? WHERE peer_id = ?",
                                        )?;
                                        stmt.execute(params![name, peer.to_string()])
                                    })
                                    .await;
                                match res {
                                    Ok(_) => self.api_writer_tx.send(WriteEvent::MdnsNameChanged { peer_id: peer.to_string(), name: n }).expect("to send"),
                                    Err(err) => tracing::info!("{err}")
                                }
                                // TODO: After success send to ui over API
                            }
                            FriendResponse::VerifyName(name) => {}
                            FriendResponse::AddFriendAck => {}
                            FriendResponse::AcceptFriendAck => {}
                        }
                }
            },
            _ => {}
        }
    }
}
