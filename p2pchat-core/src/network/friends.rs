use libp2p::PeerId;
use p2pchat_types::api::{UiClientEventResponseError, UiClientEventResponseType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::sql_calls::{
        delete_friend_request, get_friends, get_incoming_friend_requests,
        get_pending_friend_requests, insert_friend, insert_friend_request,
    },
    network::{Client, CommandType, EventLoop, UiClientRequestRequiringDial, signable::sign},
};

#[derive(Debug, Serialize, Deserialize)]
pub enum FriendRequest {
    RequestName,
    AddFriend,
    AcceptFriend { decision: bool },
}
#[derive(Debug, Serialize, Deserialize)]
pub enum FriendResponse {
    RequestName { name: String },
    AddFriendAck,
    AcceptFriendAck,
}
pub enum FriendCommand {
    RequestName { peer: PeerId },
    AddFriend { peer: PeerId },
    AcceptFriend { peer: PeerId, decision: bool },
    SearchPeer { id: String },
    SearchUsername { username: String },
    CheckUsernameAvailability { username: String },
    ChangeUsername { username: String },
    LoadFriends,
    LoadPendingFriendRequests,
    LoadIncomingFriendRequests,
}

#[derive(Debug, Serialize, Deserialize)]
struct PeerSearchResponse {
    peer_id: String,
    username: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct UsernamePayload {
    username: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RegisterResponse {
    peer_id: String,
    username: String,
}

impl EventLoop {
    pub async fn handle_friend_command(&mut self, command: FriendCommand, req_id: Uuid) {
        // TODO: Add everything to sqlite
        // Send re-render of contact list to tui
        match command {
            FriendCommand::RequestName { peer } => {
                // TODO: this is not a ui client event
                let id = self
                    .swarm
                    .behaviour_mut()
                    .friends
                    .send_request(&peer, FriendRequest::RequestName);
                self.request_map.insert(id, crate::UiClientEventId(req_id));
            }
            FriendCommand::AddFriend { peer } => {
                // TODO: Add to sqlite as pending
                self.sqlite_conn
                    .call(move |c| {
                        insert_friend_request(
                            c,
                            p2pchat_types::FriendRequestType::Outgoing,
                            peer.to_string(),
                        )
                    })
                    .await
                    .expect("to work :sob:");

                let id = self
                    .swarm
                    .behaviour_mut()
                    .friends
                    .send_request(&peer, FriendRequest::AddFriend);
                self.request_map.insert(id, crate::UiClientEventId(req_id));
                // the event is written in swarm
            }
            FriendCommand::AcceptFriend { peer, decision } => {
                self.sqlite_conn
                    .call(move |c| {
                        if decision {
                            insert_friend(c, peer.to_string())?;
                        }
                        delete_friend_request(c, peer.to_string())
                    })
                    .await
                    .expect("to work :sob:");

                let id = self
                    .swarm
                    .behaviour_mut()
                    .friends
                    .send_request(&peer, FriendRequest::AcceptFriend { decision });
                self.request_map.insert(id, crate::UiClientEventId(req_id));
            }
            FriendCommand::SearchPeer { id } => {
                let url = format!("http://{}/find-by-id?q={}", self.client.http_tracker, id);
                match self.reqwest_client.get(&url).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.json::<PeerSearchResponse>().await {
                                Ok(result) => {
                                    // TODO: KINDA BIG create a contact
                                    self.api_writer_tx
                                        .send(crate::WriteEvent::EventResponse(
                                            crate::UiClientEventResponse {
                                                result: Ok(UiClientEventResponseType::SearchPeer {
                                                    username: result.username,
                                                }),
                                                req_id,
                                            },
                                        ))
                                        .expect("to send");
                                }
                                Err(e) => {
                                    tracing::error!("Failed to parse SearchPeer response: {e}");
                                }
                            }
                        } else if response.status().is_server_error() {
                            tracing::error!(
                                "SearchPeer request failed with status: {}",
                                response.status()
                            );
                        } else {
                            self.api_writer_tx
                                .send(crate::WriteEvent::EventResponse(
                                    crate::UiClientEventResponse {
                                        result: Err(UiClientEventResponseError::PeerSearchNotFound),
                                        req_id,
                                    },
                                ))
                                .expect("to send");
                        }
                    }
                    Err(e) => {
                        tracing::error!("SearchPeer request error: {e}");
                        self.api_writer_tx
                            .send(crate::WriteEvent::EventResponse(
                                crate::UiClientEventResponse {
                                    result: Err(UiClientEventResponseError::PeerSearchTrackerConnectionFailed),
                                    req_id,
                                },
                            ))
                            .expect("to send");
                    }
                }
            }
            FriendCommand::SearchUsername { username } => {
                let url = format!(
                    "http://{}/find-by-name?q={}",
                    self.client.http_tracker, username
                );
                match self.reqwest_client.get(&url).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.json::<PeerSearchResponse>().await {
                                // TODO: create a contact and add to sqlite
                                // maybe add ttl to names and try to fetch from local db first?
                                Ok(result) => {
                                    // TODO: KINDA BIG create a contact
                                    self.api_writer_tx
                                        .send(crate::WriteEvent::EventResponse(
                                            crate::UiClientEventResponse {
                                                req_id,
                                                result: Ok(
                                                    UiClientEventResponseType::SearchUsername {
                                                        peer_id: result.peer_id,
                                                    },
                                                ),
                                            },
                                        ))
                                        .expect("to send");
                                }
                                Err(e) => {
                                    tracing::error!("Failed to parse SearchUsername response: {e}");
                                }
                            }
                        } else if response.status().is_server_error() {
                            self.api_writer_tx
                                .send(crate::WriteEvent::EventResponse(
                                    crate::UiClientEventResponse {
                                        result: Err(
                                            UiClientEventResponseError::PeerSearchServerError,
                                        ),
                                        req_id,
                                    },
                                ))
                                .expect("to send");

                            tracing::error!(
                                "SearchPeer request failed with status: {}",
                                response.status()
                            );
                        } else {
                            self.api_writer_tx
                                .send(crate::WriteEvent::EventResponse(
                                    crate::UiClientEventResponse {
                                        result: Err(UiClientEventResponseError::PeerSearchNotFound),
                                        req_id,
                                    },
                                ))
                                .expect("to send");
                        }
                    }
                    Err(e) => {
                        tracing::error!("SearchUsername request error: {e}");
                        self.api_writer_tx
                            .send(crate::WriteEvent::EventResponse(
                                crate::UiClientEventResponse {
                                    result: Err(UiClientEventResponseError::PeerSearchTrackerConnectionFailed),
                                    req_id,
                                },
                            ))
                            .expect("to send");
                    }
                }
            }
            FriendCommand::LoadFriends => {
                let friends = self.sqlite_conn.call(get_friends).await;
                match friends {
                    Ok(f) => {
                        self.api_writer_tx
                            .send(crate::WriteEvent::EventResponse(
                                crate::UiClientEventResponse {
                                    req_id,
                                    result: Ok(UiClientEventResponseType::LoadFriends(f)),
                                },
                            ))
                            .expect("to send");
                    }
                    Err(e) => {
                        tracing::error!("{e}");
                    }
                }
            }
            // TODO: Send errors to the api
            FriendCommand::LoadPendingFriendRequests => {
                let pending_requests = self.sqlite_conn.call(get_pending_friend_requests).await;
                match pending_requests {
                    Ok(requests) => {
                        self.api_writer_tx
                            .send(crate::WriteEvent::EventResponse(
                                crate::UiClientEventResponse {
                                    req_id,
                                    result: Ok(
                                        UiClientEventResponseType::LoadPendingFriendRequests(
                                            requests,
                                        ),
                                    ),
                                },
                            ))
                            .expect("to send");
                    }
                    Err(e) => {
                        tracing::error!("{e}");
                    }
                }
            }
            FriendCommand::LoadIncomingFriendRequests => {
                let incoming_requests = self.sqlite_conn.call(get_incoming_friend_requests).await;
                match incoming_requests {
                    Ok(requests) => {
                        self.api_writer_tx
                            .send(crate::WriteEvent::EventResponse(
                                crate::UiClientEventResponse {
                                    req_id,
                                    result: Ok(
                                        UiClientEventResponseType::LoadIncomingFriendRequests(
                                            requests,
                                        ),
                                    ),
                                },
                            ))
                            .expect("to send");
                    }
                    Err(e) => {
                        tracing::error!("{e}");
                    }
                }
            }
        };
    }
}
impl Client {
    pub async fn request_name(&mut self, peer: PeerId) {
        self.command_sender
            .send(super::Command {
                // TODO: pass in the actual id instead of generating
                id: Uuid::new_v4(),
                cmd_type: CommandType::FriendCommand(FriendCommand::RequestName { peer }),
            })
            .await
            .expect("to send request");
        tracing::info!("Sending name req");
    }
    pub async fn send_friend_request(&mut self, peer: PeerId, req_id: Uuid) {
        self.command_sender
            .send(super::Command {
                id: req_id,
                cmd_type: CommandType::FriendCommand(FriendCommand::AddFriend { peer }),
            })
            .await
            .expect("to send request");
    }
    pub async fn accept_friend_req(&mut self, peer: PeerId, req_id: Uuid) {
        self.command_sender
            .send(super::Command {
                id: req_id,
                cmd_type: CommandType::FriendCommand(FriendCommand::AcceptFriend {
                    peer,
                    decision: true,
                }),
            })
            .await
            .expect("to send request");
    }
    pub async fn deny_friend_req(&mut self, peer: PeerId, req_id: Uuid) {
        self.command_sender
            .send(super::Command {
                id: req_id,
                cmd_type: CommandType::FriendCommand(FriendCommand::AcceptFriend {
                    peer,
                    decision: false,
                }),
            })
            .await
            .expect("to send request");
    }
    pub async fn search_peer(&mut self, id: String, req_id: Uuid) {
        self.command_sender
            .send(super::Command {
                id: req_id,
                cmd_type: CommandType::FriendCommand(FriendCommand::SearchPeer { id }),
            })
            .await
            .expect("to send request");
    }
    pub async fn search_username(&mut self, username: String, req_id: Uuid) {
        self.command_sender
            .send(super::Command {
                id: req_id,
                cmd_type: CommandType::FriendCommand(FriendCommand::SearchUsername { username }),
            })
            .await
            .expect("to send request");
    }
    pub async fn check_username_availability(&mut self, username: String, req_id: Uuid) {
        self.command_sender
            .send(super::Command {
                id: req_id,
                cmd_type: CommandType::FriendCommand(FriendCommand::CheckUsernameAvailability {
                    username,
                }),
            })
            .await
            .expect("to send request");
    }
    pub async fn change_username(&mut self, username: String, req_id: Uuid) {
        self.command_sender
            .send(super::Command {
                id: req_id,
                cmd_type: CommandType::FriendCommand(FriendCommand::ChangeUsername { username }),
            })
            .await
            .expect("to send request");
    }
    pub async fn load_friends(&mut self, req_id: Uuid) {
        self.command_sender
            .send(super::Command {
                id: req_id,
                cmd_type: CommandType::FriendCommand(FriendCommand::LoadFriends),
            })
            .await
            .expect("to send request");
    }
    pub async fn load_pending_friend_requests(&mut self, req_id: Uuid) {
        self.command_sender
            .send(super::Command {
                id: req_id,
                cmd_type: CommandType::FriendCommand(FriendCommand::LoadPendingFriendRequests),
            })
            .await
            .expect("to send request");
    }
    pub async fn load_incoming_friend_requests(&mut self, req_id: Uuid) {
        self.command_sender
            .send(super::Command {
                id: req_id,
                cmd_type: CommandType::FriendCommand(FriendCommand::LoadIncomingFriendRequests),
            })
            .await
            .expect("to send request");
    }
}

// Name exchange -- Will occur when there is no name linked to PeerId
// What is your name?
// My name is: xxxx
// acknowledged

// Name verification -- Will reoccur based on ttl values
// Is your name still xxxx?
// yes / no

// Friend request
// I wanna be ur friend
// request acknowledged

// AcceptFriendRequest
// I want / dont want to be ur friend
// acknowledged
