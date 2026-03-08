use futures::StreamExt;
use libp2p::{
    PeerId, StreamProtocol, Swarm,
    identity::{Keypair, ed25519::PublicKey},
    mdns, noise,
    request_response::{self, ProtocolSupport},
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{
    RwLock,
    mpsc::{self, UnboundedSender},
};
use tokio_rusqlite::Connection;
use uuid::Uuid;

use crate::{
    network::{
        chat::{
            ChatCommand, DirectMessageRequest, DirectMessageResponse, Message, MessageResponse,
        },
        friends::{FriendCommand, FriendRequest, FriendResponse},
        signable::sign,
    },
    settings::{Setting, SettingName, SettingValue},
    tui::types::Contact,
    tui::types::Event::EditContact,
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
    settings: Arc<RwLock<HashMap<SettingName, Setting>>>,
    tui_tx: UnboundedSender<crate::tui::types::Event>,
) -> (EventLoop, Client, mpsc::Receiver<Event>) {
    // TODO: Confiugre properly & handle errors
    // Dont generate identities on every run, create a store

    let id = {
        let s = settings.read().await;
        let s = s.get(&SettingName::KeyPair).unwrap();

        let SettingValue::Bytes(s) = s.get_value() else {
            unreachable!();
        };

        match s {
            None => {
                tracing::warn!("Keypair not located, generating a new one");
                let w = settings.write().await;
                let key = Keypair::generate_ed25519();
                let s = key.to_protobuf_encoding().unwrap();
                let setting = Setting {
                    value: SettingValue::Bytes(Some(s)),
                    constraints: None,
                };
                w.insert(SettingName::KeyPair, setting);
                key
            }
            Some(s) => Keypair::ed25519_from_bytes(s.bytes().collect::<Vec<u8>>()).unwrap(),
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
    let (event_tx, event_rx) = mpsc::channel(100);
    let client = Client {
        settings: settings.clone(),
        command_sender: command_tx,
        keys: id.clone(),
        id: PeerId::from_public_key(&id.public()),
    };
    let event_loop = EventLoop {
        swarm,
        command_rx,
        event_sender: event_tx,
        settings,
        keys: id,
        tui_tx,
        sqlite_conn,
        client: client.clone(),
    };
    (event_loop, client, event_rx)
}
#[derive(Debug)]
pub(crate) enum Event {
    InboundMessage {
        message: Message,
        sender: Box<PublicKey>,
    },
    OutboundMessageReceived {
        message_id: Uuid,
    },
    OutboundMessageInvalidSignature {
        message_id: Uuid,
    },
}
#[derive(NetworkBehaviour)]
struct Behaviour {
    mdns: mdns::tokio::Behaviour,
    direct_message:
        libp2p::request_response::cbor::Behaviour<DirectMessageRequest, DirectMessageResponse>,
    friends:
        libp2p::request_response::cbor::Behaviour<FriendRequest, signable::Signed<FriendResponse>>,
}
pub struct EventLoop {
    swarm: Swarm<Behaviour>,
    command_rx: mpsc::Receiver<Command>,
    event_sender: mpsc::Sender<Event>,
    settings: Arc<tokio::sync::RwLock<HashMap<SettingName, Setting>>>,
    keys: Keypair,
    sqlite_conn: Connection,
    tui_tx: UnboundedSender<crate::tui::types::Event>,
    client: Client,
}
#[derive(Clone)]
pub(crate) struct Client {
    pub command_sender: mpsc::Sender<Command>,
    settings: Arc<tokio::sync::RwLock<HashMap<SettingName, Setting>>>,
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
                        let _ = self.tui_tx.send(crate::tui::types::Event::AddContact(
                            crate::tui::types::Contact {
                                peer_id,
                                name: "Anonymous".to_string(),
                            },
                        ));
                        known.push(peer_id);
                    }
                    self.client.request_name(peer_id).await;
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
                request_response::Event::Message { message, .. },
            )) => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    // TODO: remove this unwrap
                    let (message, sender) = request.0.verify().expect("to be verified");
                    // if message is valid, send
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
                    self.event_sender
                        .send(Event::InboundMessage {
                            message,
                            sender: Box::new(sender),
                        })
                        .await
                        .expect("Event receiver not to be dropped.");
                }
                request_response::Message::Response { response, .. } => match response {
                    DirectMessageResponse(MessageResponse::ACK { message_id }) => {
                        self.event_sender
                            .send(Event::OutboundMessageReceived { message_id })
                            .await
                            .expect("Event receiver not to be dropped.");
                    }
                    DirectMessageResponse(MessageResponse::InvalidSignature { message_id }) => {
                        self.event_sender
                            .send(Event::OutboundMessageInvalidSignature { message_id })
                            .await
                            .expect("Event receiver not to be dropped");
                    }
                },
            },
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
                        let lock = self.settings.read().await;
                        let name = lock.get(&SettingName::Name);
                        self.swarm
                            .behaviour_mut()
                            .friends
                            .send_response(
                                channel,
                                sign(
                                    FriendResponse::RequestName {
                                        name: match name.unwrap().get_value() {
                                            SettingValue::String(val) => {
                                                val.clone().unwrap_or("Anonymous".to_string())
                                            }
                                            _ => unimplemented!("undefined behaviour"),
                                        },
                                    },
                                    &self.keys,
                                ),
                            )
                            .expect("On Name request to be sent");
                    }
                    FriendRequest::VerifyName { name } => {
                        let lock = self.settings.read().await;
                        let SettingValue::String(Some(curr_name)) = lock
                            .get(&SettingName::Name)
                            .expect("name opt to exist")
                            .get_value()
                        else {
                            unimplemented!("");
                        };
                        self.swarm
                            .behaviour_mut()
                            .friends
                            .send_response(
                                channel,
                                sign(
                                    FriendResponse::VerifyName(match name == *curr_name {
                                        true => None,
                                        false => Some(curr_name.clone()),
                                    }),
                                    &self.keys,
                                ),
                            )
                            .expect("to send res");
                    }
                    FriendRequest::AcceptFriend { decision } => {
                        self.swarm
                            .behaviour_mut()
                            .friends
                            .send_response(
                                channel,
                                sign(FriendResponse::AcceptFriendAck, &self.keys),
                            )
                            .expect("to send res");
                    }
                    FriendRequest::AddFriend => self
                        .swarm
                        .behaviour_mut()
                        .friends
                        .send_response(channel, sign(FriendResponse::AddFriendAck, &self.keys))
                        .expect("to send res"),
                },

                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    if let Some((resp, sender)) = response.verify() {
                        match resp {
                            FriendResponse::RequestName { name } => {
                                tracing::info!("Received valid name response");
                                self.tui_tx
                                    .send(EditContact(Contact {
                                        peer_id: peer,
                                        name,
                                    }))
                                    .expect("to send");
                            }
                            FriendResponse::VerifyName(name) => {}
                            FriendResponse::AddFriendAck => {}
                            FriendResponse::AcceptFriendAck => {}
                        }
                    }
                }
            },
            _ => {}
        }
    }
}
