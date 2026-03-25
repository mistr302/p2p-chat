use base64::{Engine as _, engine::general_purpose};
use dashmap::DashMap;
use futures::StreamExt;
use libp2p::{
    Multiaddr, PeerId, StreamProtocol, Swarm, dcutr,
    identity::Keypair,
    mdns, noise,
    request_response::{self, OutboundRequestId, ProtocolSupport},
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
};
use num_enum::TryFromPrimitive;
use std::sync::Arc;
use std::time::Duration;
use std::{collections::HashMap, str::FromStr};
use tokio::sync::{
    Mutex,
    mpsc::{self, UnboundedSender},
    oneshot,
};
use tokio_rusqlite::{Connection, params};
use uuid::Uuid;

use crate::{
    UiClientEventId, UiClientEventResponse, WriteEvent,
    db::{
        sql_calls::{
            delete_friend_request, get_contact, get_contact_channel_id, insert_contact,
            insert_friend, insert_friend_request, insert_message, insert_name,
        },
        types::{DiscoveryType, MessageStatus},
    },
    network::{
        chat::{ChatCommand, DirectMessageRequest, DirectMessageResponse, MessageResponse},
        friends::{FriendCommand, FriendRequest, FriendResponse},
    },
    tui::types::Contact,
};
use p2pchat_types::{
    FriendRequestType,
    api::{
        DcutrConnectionEvent, DcutrConnectionSuccess, RelayConnectionError,
        RelayServerConnectionEvent, UiClientEventRequiringDial, UiClientEventResponseType,
        UiClientRequest,
    },
    settings::{SettingName, SettingValue},
};
#[derive(Debug)]
pub struct UiClientRequestRequiringDial {
    pub event: UiClientEventRequiringDial,
    pub id: Uuid,
}
pub mod chat;
pub mod friends;
pub mod signable;
pub mod types;
// TODO: !IMPORTANT! Add the relay addr
pub static RELAY_ADDR: &str = "";
// TODO: !IMPORTANT! Add the http addr
pub static HTTP_TRACKER: &str = "127.0.0.1:8000";
// pub static HTTP_TRACKER: &str = "localhost:8000";
pub enum CommandType {
    ChatCommand(ChatCommand),
    FriendCommand(FriendCommand),
    Dial {
        peer_id: PeerId,
    },
    IsPeerConnected {
        sender: oneshot::Sender<bool>,
        peer_id: PeerId,
    },
    BufferEvent {
        ev: UiClientRequestRequiringDial,
    },
}
pub struct Command {
    id: Uuid,
    cmd_type: CommandType,
}
pub(crate) async fn new(
    sqlite_conn: Arc<Connection>,
    settings: Arc<HashMap<SettingName, SettingValue>>,
    api_writer_tx: UnboundedSender<WriteEvent>,
    request_map: Arc<DashMap<OutboundRequestId, UiClientEventId>>,
) -> anyhow::Result<(EventLoop, Client, Vec<SwarmEvent<BehaviourEvent>>)> {
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
        .with_relay_client(noise::Config::new, yamux::Config::default)?
        .with_behaviour(|key, relay_client| {
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
            let identify = libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
                "/p2pchat/1.0.0".to_string(),
                key.public(),
            ));
            let dcutr = libp2p::dcutr::Behaviour::new(key.public().to_peer_id());
            Ok(Behaviour {
                relay_client,
                identify,
                dcutr,
                mdns,
                direct_message,
                friends,
            })
        })?
        .with_swarm_config(|cfg| cfg.with_idle_connection_timeout(Duration::from_secs(60 * 10)))
        .build();

    // Listen on all interfaces and whatever port the OS assigns
    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    let mut swarm_event_buffer = vec![];
    // Wait to listen on all interfaces.
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(1);
    loop {
        tokio::select! {
            event = swarm.next() => {
                match event.unwrap() {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        tracing::info!(%address, "Listening on address");
                    }
                    event => swarm_event_buffer.push(event),
                }
            }
            _ = tokio::time::sleep_until(deadline) => {
                // Likely listening on all interfaces now, thus continuing by breaking the loop.
                break;
            }
        }
    }
    // dial relay
    let mut relay_connections = Vec::new();
    let relay_addr = Multiaddr::from_str(RELAY_ADDR)?;
    let res = swarm.dial(relay_addr.clone());
    match res {
        Ok(_) => {
            // Step 2: after connection, request a reservation (circuit relay listen)
            let relay_reservation_addr = relay_addr
                .clone()
                .with(libp2p::multiaddr::Protocol::P2pCircuit); // appends /p2p-circuit
            swarm.listen_on(relay_reservation_addr); // TODO: !IMPORTANT! handle this error
            relay_connections.push(relay_addr);
        }
        Err(e) => {
            tracing::error!("Failed to dial relay: {e}");
            api_writer_tx
                .send(WriteEvent::RelayServerConnection(
                    RelayServerConnectionEvent(Err(RelayConnectionError::DialError)),
                ))
                .expect("to send");
        }
    }

    let (command_tx, command_rx) = mpsc::channel(100);
    let client = Client {
        settings: settings.clone(),
        command_sender: command_tx,
        keys: id.clone(),
        id: PeerId::from_public_key(&id.public()),
        request_map: request_map.clone(), //TODO: Maybe remove cuz not using it
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
        request_map,
        relay_connections: Arc::new(Mutex::new(relay_connections)),
        request_buffer: HashMap::new(),
    };
    Ok((event_loop, client, swarm_event_buffer))
}
#[derive(NetworkBehaviour)]
pub struct Behaviour {
    mdns: mdns::tokio::Behaviour,
    direct_message:
        libp2p::request_response::cbor::Behaviour<DirectMessageRequest, DirectMessageResponse>,
    friends: libp2p::request_response::cbor::Behaviour<FriendRequest, FriendResponse>,
    relay_client: libp2p::relay::client::Behaviour,
    identify: libp2p::identify::Behaviour,
    dcutr: libp2p::dcutr::Behaviour,
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
    request_map: Arc<DashMap<OutboundRequestId, UiClientEventId>>,
    relay_connections: Arc<Mutex<Vec<Multiaddr>>>,
    // TODO: find out if i can do this safely
    request_buffer: HashMap<PeerId, Vec<UiClientRequestRequiringDial>>,
}
#[derive(Clone)]
pub(crate) struct Client {
    pub command_sender: mpsc::Sender<Command>,
    settings: Arc<HashMap<SettingName, SettingValue>>,
    keys: Keypair,
    pub id: PeerId,
    request_map: Arc<DashMap<OutboundRequestId, UiClientEventId>>,
}
impl Client {
    pub async fn dial(&self, peer_id: PeerId, req_id: Uuid) {
        self.command_sender
            .send(Command {
                id: req_id,
                cmd_type: CommandType::Dial { peer_id },
            })
            .await
            .expect("to send");
    }
    pub async fn is_connected(
        &mut self,
        peer_id: PeerId,
        sender: tokio::sync::oneshot::Sender<bool>,
    ) {
        self.command_sender
            .send(Command {
                id: Uuid::new_v4(),
                cmd_type: CommandType::IsPeerConnected { sender, peer_id },
            })
            .await
            .expect("to send");
    }
    pub async fn buffer_event(&self, ev: UiClientRequestRequiringDial) {
        self.command_sender
            .send(Command {
                id: Uuid::new_v4(),
                cmd_type: CommandType::BufferEvent { ev },
            })
            .await
            .expect("to send");
    }
    // pub async fn send_event_req_dial(&self) {
    //     self.command_sender.send(value)
    // }
}
impl EventLoop {
    pub async fn run(mut self, buffered_events: Option<Vec<SwarmEvent<BehaviourEvent>>>) {
        if let Some(buffered) = buffered_events {
            for ev in buffered {
                self.handle_event(ev).await;
            }
        }
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => self.handle_event(event).await,
                Some(command) = self.command_rx.recv() => {
                    match command.cmd_type {
                        CommandType::ChatCommand(chat) => self.handle_chat_command(chat, command.id).await,
                        CommandType::FriendCommand(friend) => self.handle_friend_command(friend, command.id).await,
                        CommandType::Dial { peer_id } => self.dial_peer(peer_id).await,
                        CommandType::IsPeerConnected { sender, peer_id } => {
                            let res = self.swarm.is_connected(&peer_id);
                            tracing::info!("checking if peer is connected: {res}");
                            sender.send(res).expect("to send");
                        }
                        CommandType::BufferEvent { ev } => {
                            match self.request_buffer.get_mut(&PeerId::from_str(&ev.event.peer_id).unwrap()) {
                                Some(v) => {
                                    v.push(ev);
                                }
                                None => {
                                    self.request_buffer.insert(PeerId::from_str(&ev.event.peer_id).unwrap(), vec![ev]);
                                }
                            }
                        }
                    }
                },
            }
        }
    }
    async fn dial_peer(&mut self, peer_id: PeerId) {
        tracing::info!("dialing peer {peer_id}");
        // attempt to dial as a known peer
        let _res = self.swarm.dial(peer_id);
        tracing::info!("{:?}", _res);
        // dial over the  relay
        for conn in self.relay_connections.lock().await.iter() {
            // TODO: this could be a bit too much to dial every relay just for one
            // connection, use dht after
            match self.swarm.dial(
                conn.clone()
                    .with(libp2p::multiaddr::Protocol::P2pCircuit)
                    .with_p2p(peer_id)
                    .unwrap(),
            ) {
                Ok(_) => break, // TODO: this may be bs
                Err(e) => {
                    tracing::error!("failed to dial peer on relay: {conn}");
                }
            }
        }
    }
    async fn handle_event(&mut self, event: SwarmEvent<BehaviourEvent>) {
        match event {
            SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                let mut known = Vec::<PeerId>::new();
                for (peer_id, multiaddr) in list {
                    tracing::info!("{peer_id} peer connected at {multiaddr}!");
                    // Add the discovered address to the swarm's address book
                    self.swarm.add_peer_address(peer_id, multiaddr);

                    // TODO: implement known as a map
                    if !known.contains(&peer_id) {
                        known.push(peer_id);
                        // TODO: Handle uniqueness maybe select the peer_id first from sqlite and
                        // check if exists
                        let r = self
                            .sqlite_conn
                            .call(move |c| get_contact(c, peer_id.to_string()))
                            .await;
                        if let Ok(contact) = r {
                            let name = contact
                                .central_name
                                .as_ref()
                                .or(contact.provided_name.as_ref())
                                .map(|n| n.content.clone());
                            self.api_writer_tx
                                .send(WriteEvent::DiscoverMdnsContact {
                                    peer_id: contact.peer_id,
                                    name,
                                })
                                .expect("to send");
                        } else {
                            let res = self
                                .sqlite_conn
                                .call(move |c| insert_contact(c, peer_id.to_string()))
                                .await;
                            match res {
                                Ok(_) => {
                                    self.client.request_name(peer_id).await;

                                    // TODO: bruh im actually ashamed of ts
                                    self.client.buffer_event(UiClientRequestRequiringDial {
                                        id: Uuid::new_v4(),
                                        event: UiClientEventRequiringDial {
                                            peer_id: peer_id.to_string(),
                                            event: p2pchat_types::api::UiClientEventRequiringDialMessage::ResolveName,
                                        }
                                    }).await;
                                }
                                Err(e) => tracing::info!("{e}"),
                            }
                            self.api_writer_tx
                                .send(WriteEvent::DiscoverMdnsContact {
                                    peer_id: peer_id.to_string(),
                                    name: None,
                                })
                                .expect("to send");
                        }
                        // TODO: Send the mdns record
                    }
                }
            }
            SwarmEvent::Behaviour(BehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                for (peer_id, multiaddr) in list {
                    self.api_writer_tx
                        .send(WriteEvent::MdnsPeerDisconnected {
                            peer_id: peer_id.to_string(),
                        })
                        .expect("receiver not to be dropped");
                    tracing::info!("{peer_id} expired mDNS, removed {multiaddr}");
                }
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                tracing::info!("Local node is listening on {address}");
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                // TODO:
            }
            SwarmEvent::ConnectionEstablished {
                peer_id,
                connection_id,
                endpoint,
                ..
            } => {
                // TODO: flush the requests
                if let Some(requests) = self.request_buffer.remove(&peer_id) {
                    for req in requests {
                        tracing::info!("Flushing request: {:?}", req);
                        crate::resolve_event_req_dial(req.event, req.id, &mut self.client).await;
                    }
                }
            }
            SwarmEvent::Behaviour(BehaviourEvent::Dcutr(ev)) => {
                // TODO: add connection_id if libp2p allows it to be public someday
                if ev.result.is_ok() {
                    self.api_writer_tx
                        .send(WriteEvent::DcutrConnection(DcutrConnectionEvent(Ok(
                            DcutrConnectionSuccess {
                                peer_id: ev.remote_peer_id.to_string(),
                            },
                        ))))
                        .expect("to send");
                }
            }
            SwarmEvent::Behaviour(BehaviourEvent::RelayClient(ev)) => match ev {
                libp2p::relay::client::Event::ReservationReqAccepted { relay_peer_id, .. } => {
                    // TODO: figure ts out
                }
                _ => {}
            },
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
                        let contact = self
                            .sqlite_conn
                            .call(move |c| {
                                let contact = get_contact(c, peer.to_string())?;
                                insert_message(c, m, contact.channel_id)?;
                                Ok::<_, tokio_rusqlite::Error>(contact)
                            })
                            .await
                            .unwrap();

                        // if message is valid, send ack
                        self.swarm
                            .behaviour_mut()
                            .direct_message
                            .send_response(channel, DirectMessageResponse(MessageResponse::Ack))
                            .expect("to be sent");
                        // TODO: Send to ui through the api
                        let message = crate::tui::types::Message {
                            content: message.content,
                            id: message.id,
                            sender: contact,
                            created_at: p2pchat_types::chrono::Local::now().naive_local(),
                        };
                        self.api_writer_tx
                            .send(WriteEvent::ReceiveMessage(message))
                            .expect("to send");
                    }
                    request_response::Message::Response {
                        response,
                        request_id,
                        ..
                    } => {
                        let client_ev_id = self.request_map.get(&request_id).expect("to exist");
                        match response {
                            DirectMessageResponse(MessageResponse::Ack) => {
                                // TODO:
                                self.api_writer_tx
                                    .send(crate::WriteEvent::EventResponse(
                                        crate::UiClientEventResponse {
                                            req_id: client_ev_id.0,
                                            result: Ok(UiClientEventResponseType::SendMessage),
                                        },
                                    ))
                                    .expect("to send");
                            }
                            DirectMessageResponse(MessageResponse::DeniedNotFriends) => {}
                        }
                    }
                }
            }
            SwarmEvent::Behaviour(BehaviourEvent::Friends(request_response::Event::Message {
                peer,
                message,
                ..
            })) => match message {
                request_response::Message::Request {
                    request, channel, ..
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
                                            // TODO: actually maybe crash xd
                                        }
                                        _ => unimplemented!("undefined behaviour"),
                                    },
                                },
                            )
                            .expect("On Name request to be sent");
                    }
                    FriendRequest::AcceptFriend { decision } => {
                        //TODO: add the friend decision to sqlite
                        // TODO: ts could def fail
                        self.sqlite_conn
                            .call(move |c| {
                                if decision {
                                    insert_friend(c, peer.to_string())?;
                                }
                                delete_friend_request(c, peer.to_string())
                            })
                            .await
                            .expect("to work :sob:");
                        self.api_writer_tx
                            .send(WriteEvent::ReceiveFriendRequestResponse { decision })
                            .expect("to send");
                        self.swarm
                            .behaviour_mut()
                            .friends
                            .send_response(channel, FriendResponse::AcceptFriendAck)
                            .expect("to send res");
                    }
                    FriendRequest::AddFriend => {
                        self.sqlite_conn
                            .call(move |c| {
                                insert_friend_request(
                                    c,
                                    FriendRequestType::Incoming,
                                    peer.to_string(),
                                )
                            })
                            .await
                            .expect("to work :sob:");
                        self.api_writer_tx
                            .send(WriteEvent::ReceiveFriendRequest)
                            .expect("to send");

                        self.swarm
                            .behaviour_mut()
                            .friends
                            .send_response(channel, FriendResponse::AddFriendAck)
                            .expect("to send res")
                    }
                },

                request_response::Message::Response {
                    request_id,
                    response,
                } => {
                    let client_ev_id = self.request_map.get(&request_id).expect("to exist");
                    match response {
                        FriendResponse::RequestName { name } => {
                            tracing::info!("Received valid name response");
                            let n = name.clone();
                            let res = self
                                .sqlite_conn
                                .call(move |c| {
                                    insert_name(
                                        c,
                                        peer.to_string(),
                                        crate::db::sql_calls::NameType::Provided,
                                        name,
                                    )
                                })
                                .await;
                            match res {
                                Ok(_) => self
                                    .api_writer_tx
                                    .send(WriteEvent::MdnsNameResolved {
                                        peer_id: peer.to_string(),
                                        name: n,
                                    })
                                    .expect("to send"),
                                Err(err) => tracing::info!("{err}"),
                            }
                        }
                        FriendResponse::AddFriendAck => {
                            self.api_writer_tx
                                .send(WriteEvent::EventResponse(UiClientEventResponse {
                                    req_id: client_ev_id.0,
                                    result: Ok(UiClientEventResponseType::SendFriendRequest),
                                }))
                                .expect("to send");
                        }
                        FriendResponse::AcceptFriendAck => {
                            self.api_writer_tx
                                .send(WriteEvent::EventResponse(UiClientEventResponse {
                                    req_id: client_ev_id.0,
                                    result: Ok(UiClientEventResponseType::AcceptFriendRequest),
                                }))
                                .expect("to send");
                        }
                    }
                }
            },
            SwarmEvent::Behaviour(BehaviourEvent::Friends(
                request_response::Event::OutboundFailure {
                    peer,
                    request_id,
                    error,
                    ..
                },
            )) => {}
            SwarmEvent::Behaviour(BehaviourEvent::Friends(
                request_response::Event::InboundFailure {
                    peer,
                    request_id,
                    error,
                    ..
                },
            )) => {}

            SwarmEvent::Behaviour(BehaviourEvent::DirectMessage(
                request_response::Event::OutboundFailure {
                    peer,
                    request_id,
                    error,
                    ..
                },
            )) => {}

            _ => {}
        }
    }
}
