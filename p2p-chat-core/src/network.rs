use base64::{Engine as _, engine::general_purpose};
use dashmap::DashMap;
use futures::StreamExt;
use libp2p::{
    PeerId, StreamProtocol, Swarm,
    identity::Keypair,
    mdns, noise,
    request_response::{self, OutboundRequestId, ProtocolSupport},
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use num_enum::TryFromPrimitive;
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio_rusqlite::{Connection, params};

use crate::{
    UiClientEventId, UiClientEventResponse, UiClientEventResponseType, WriteEvent, db::types::{DiscoveryType, MessageStatus}, network::{
        chat::{
            ChatCommand, DirectMessageRequest, DirectMessageResponse, MessageResponse,
        },
        friends::{FriendCommand, FriendRequest, FriendResponse},
    }, settings::{SettingName, SettingValue}, tui::types::Contact
};
pub mod chat;
pub mod friends;
pub mod signable;
pub mod types;
pub static REQUEST_TIMEOUT_SECS: u8 = 5;
pub static HTTP_TRACKER: &str = "localhost:8000";
pub enum CommandType {
    ChatCommand(ChatCommand),
    FriendCommand(FriendCommand),
}
pub struct Command {
    id: Uuid,
    cmd_type: CommandType
}
pub(crate) async fn new(
    sqlite_conn: Arc<Connection>,
    settings: Arc<HashMap<SettingName, SettingValue>>,
    api_writer_tx: UnboundedSender<WriteEvent>,
    request_map: Arc<DashMap<OutboundRequestId, UiClientEventId>>
) -> anyhow::Result<(EventLoop, Client)> {
    // TODO: Confiugre properly & handle errors

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
        )?
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
        })?
        .build();
    // Listen on all interfaces and whatever port the OS assigns
    swarm
        .listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
    swarm
        .listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    let (command_tx, command_rx) = mpsc::channel(100);
    let client = Client {
        settings: settings.clone(),
        command_sender: command_tx,
        keys: id.clone(),
        id: PeerId::from_public_key(&id.public()),
        request_map: request_map.clone()  //TODO: Maybe remove cuz not using it
    };
    let event_loop = EventLoop {
        swarm,
        command_rx,
        settings,
        keys: id,
        api_writer_tx,
        sqlite_conn,
        client: client.clone(),
        reqwest_client: reqwest::Client::new(),
        request_map
    };
    Ok((event_loop, client))
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
    sqlite_conn: Arc<Connection>,
    api_writer_tx: UnboundedSender<WriteEvent>,
    client: Client,
    reqwest_client: reqwest::Client,
    request_map: Arc<DashMap<OutboundRequestId, UiClientEventId>>
}
#[derive(Clone)]
pub(crate) struct Client {
    pub command_sender: mpsc::Sender<Command>,
    settings: Arc<HashMap<SettingName, SettingValue>>,
    keys: Keypair,
    pub id: PeerId,
    request_map: Arc<DashMap<OutboundRequestId, UiClientEventId>>
}
impl EventLoop {
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => self.handle_event(event).await,
                Some(command) = self.command_rx.recv() => {
                    match command.cmd_type {
                        CommandType::ChatCommand(chat) => self.handle_chat_command(chat, command.id).await,
                        CommandType::FriendCommand(friend) => self.handle_friend_command(friend, command.id).await,
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
                        known.push(peer_id);
                        // TODO: Handle uniqueness maybe select the peer_id first from sqlite and
                        // check if exists

                        let res = self
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
                        match res {
                            Ok(_) => self.client.request_name(peer_id).await,
                            Err(e) => tracing::info!("{e}"),
 
                        }
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
                                DirectMessageResponse(MessageResponse::ACK),
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
                    request_response::Message::Response { response, request_id, .. } => match response {
                        DirectMessageResponse(MessageResponse::ACK) => {
                            // TODO:
                            let client_ev_id =  self.request_map.get(&request_id).expect("to exist");            
                            self.api_writer_tx.send(crate::WriteEvent::EventResponse(crate::UiClientEventResponse { req_id: client_ev_id.0, result: Ok(UiClientEventResponseType::SendMessage)  })).expect("to send");
                            
                        }
                        // DirectMessageResponse(MessageResponse::InvalidSignature { message_id }) => {
                        //     // TODO:
                        //
                        // }
                    },
                }
            }
            SwarmEvent::Behaviour(BehaviourEvent::Friends(request_response::Event::Message {
                peer,
                message,
                ..
            })) => match message {
                request_response::Message::Request {
                    request,
                    channel,
                    ..
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
                    FriendRequest::AcceptFriend { decision } => {
                        //TODO: add the friend decision to sqlite

                        self.swarm
                            .behaviour_mut()
                            .friends
                            .send_response(
                                channel,
                                FriendResponse::AcceptFriendAck,
                            )
                            .expect("to send res");
                    }
                    FriendRequest::AddFriend => { 
                        //TODO: add the friend request to sqlite
                        self
                        .swarm
                        .behaviour_mut()
                        .friends
                        .send_response(channel, FriendResponse::AddFriendAck)
                        .expect("to send res")
                    },
                },

                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                        let client_ev_id =  self.request_map.get(&request_id).expect("to exist");            
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
                            }
                            FriendResponse::AddFriendAck => {
                                self.api_writer_tx.send(WriteEvent::EventResponse(UiClientEventResponse { req_id: client_ev_id.0, result: Ok(UiClientEventResponseType::SendFriendRequest) })).expect("to send");
                            }
                            FriendResponse::AcceptFriendAck => {
                                self.api_writer_tx.send(WriteEvent::EventResponse(UiClientEventResponse { req_id: client_ev_id.0, result: Ok(UiClientEventResponseType::AcceptFriendRequest) })).expect("to send");
                            }
                        }
                }
            },
            SwarmEvent::Behaviour(BehaviourEvent::Friends(request_response::Event::OutboundFailure {
                peer,
                request_id,
                error,
                ..
            })) => {

            },
            SwarmEvent::Behaviour(BehaviourEvent::Friends(request_response::Event::InboundFailure {
                peer,
                request_id,
                error,
                ..
            })) => {

            },

            SwarmEvent::Behaviour(BehaviourEvent::DirectMessage(request_response::Event::OutboundFailure {
                peer,
                request_id,
                error,
                ..
            })) => {

            }


            _ => {}
        }
    }
}
