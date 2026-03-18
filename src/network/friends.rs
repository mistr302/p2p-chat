use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    db::sql_calls::{get_friends, get_incoming_friend_requests, get_pending_friend_requests},
    network::{Client, CommandType, EventLoop, HTTP_TRACKER, signable::sign},
};

#[derive(Debug, Serialize, Deserialize)]
pub enum FriendRequest {
    RequestName,
    VerifyName { name: String },
    AddFriend,
    AcceptFriend { decision: bool },
}
#[derive(Debug, Serialize, Deserialize)]
pub enum FriendResponse {
    RequestName { name: String },
    VerifyName(Option<String>),
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
struct RegisterRequest {
    pub_key: String,
    content: String,
    sig: String,
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
                self.swarm
                    .behaviour_mut()
                    .friends
                    .send_request(&peer, FriendRequest::RequestName);
            }
            FriendCommand::AddFriend { peer } => {
                self.swarm
                    .behaviour_mut()
                    .friends
                    .send_request(&peer, FriendRequest::AddFriend);
            }
            FriendCommand::AcceptFriend { peer, decision } => {
                self.swarm
                    .behaviour_mut()
                    .friends
                    .send_request(&peer, FriendRequest::AcceptFriend { decision });
            }
            FriendCommand::SearchPeer { id } => {
                let url = format!("http://{}/find-by-id?q={}", HTTP_TRACKER, id);
                match self.reqwest_client.get(&url).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.json::<PeerSearchResponse>().await {
                                Ok(result) => {
                                    self.api_writer_tx
                                        .send(crate::WriteEvent::EventResponse(
                                            crate::UiClientEventResponse {
                                                result: Ok(
                                                    crate::UiClientEventResponseType::SearchPeer {
                                                        username: result.username,
                                                    },
                                                ),
                                                req_id,
                                            },
                                        ))
                                        .expect("to send");
                                }
                                Err(e) => {
                                    tracing::error!("Failed to parse SearchPeer response: {e}");
                                }
                            }
                        } else {
                            tracing::error!(
                                "SearchPeer request failed with status: {}",
                                response.status()
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("SearchPeer request error: {e}");
                    }
                }
            }
            FriendCommand::SearchUsername { username } => {
                let url = format!("http://{}/find-by-name?q={}", HTTP_TRACKER, username);
                match self.reqwest_client.get(&url).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.json::<PeerSearchResponse>().await {
                                Ok(result) => {
                                    self.api_writer_tx
                                        .send(crate::WriteEvent::EventResponse(
                                            crate::UiClientEventResponse {
                                                req_id,
                                                result: Ok(
                                                    crate::UiClientEventResponseType::SearchUsername {
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
                        } else {
                            tracing::error!(
                                "SearchUsername request failed with status: {}",
                                response.status()
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("SearchUsername request error: {e}");
                    }
                }
            }
            FriendCommand::CheckUsernameAvailability { username } => {
                let url = format!("http://{}/find-by-name?q={}", HTTP_TRACKER, username);
                match self.reqwest_client.get(&url).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.json::<PeerSearchResponse>().await {
                                Ok(_) => {
                                    // Username exists, so it's NOT available
                                    self.api_writer_tx
                                        .send(crate::WriteEvent::EventResponse(
                                            crate::UiClientEventResponse {
                                                req_id,
                                                result: Ok(
                                                    crate::UiClientEventResponseType::CheckUsernameAvailability(
                                                        false,
                                                    ),
                                                ),
                                            },
                                        ))
                                        .expect("to send");
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Failed to parse CheckUsernameAvailability response: {e}"
                                    );
                                    // TODO: Handle parse error properly
                                }
                            }
                        } else {
                            // Username not found, so it's available
                            self.api_writer_tx
                                .send(crate::WriteEvent::EventResponse(
                                    crate::UiClientEventResponse {
                                        req_id,
                                        result: Ok(
                                            crate::UiClientEventResponseType::CheckUsernameAvailability(
                                                true,
                                            ),
                                        ),
                                    },
                                ))
                                .expect("to send");
                        }
                    }
                    Err(e) => {
                        tracing::error!("CheckUsernameAvailability request error: {e}");
                        // TODO: Handle request error properly
                    }
                }
            }
            FriendCommand::ChangeUsername { username } => {
                let payload = UsernamePayload { username };
                let signed = sign(payload, &self.keys);

                let url = format!("http://{}/register", HTTP_TRACKER);
                match self.reqwest_client.post(&url).json(&signed).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            match response.json::<RegisterResponse>().await {
                                Ok(result) => {
                                    self.api_writer_tx
                                        .send(crate::WriteEvent::EventResponse(
                                            crate::UiClientEventResponse {
                                                req_id,
                                                result: Ok(
                                                    crate::UiClientEventResponseType::ChangeUsername,
                                                ),
                                            },
                                        ))
                                        .expect("to send");
                                    tracing::info!(
                                        "Username changed successfully to: {}",
                                        result.username
                                    );
                                }
                                Err(e) => {
                                    tracing::error!("Failed to parse ChangeUsername response: {e}");
                                }
                            }
                        } else {
                            tracing::error!(
                                "ChangeUsername request failed with status: {}",
                                response.status()
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("ChangeUsername request error: {e}");
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
                                    result: Ok(crate::UiClientEventResponseType::LoadFriends(f)),
                                },
                            ))
                            .expect("to send");
                    }
                    Err(e) => {
                        tracing::info!("{e}");
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
                                        crate::UiClientEventResponseType::LoadPendingFriendRequests(
                                            requests,
                                        ),
                                    ),
                                },
                            ))
                            .expect("to send");
                    }
                    Err(e) => {
                        tracing::info!("{e}");
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
                                        crate::UiClientEventResponseType::LoadIncomingFriendRequests(
                                            requests,
                                        ),
                                    ),
                                },
                            ))
                            .expect("to send");
                    }
                    Err(e) => {
                        tracing::info!("{e}");
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
    pub async fn send_friend_request(&mut self, peer: PeerId) {
        self.command_sender
            .send(super::Command {
                // TODO: pass in the actual id instead of generating
                id: Uuid::new_v4(),
                cmd_type: CommandType::FriendCommand(FriendCommand::AddFriend { peer }),
            })
            .await
            .expect("to send request");
    }
    pub async fn accept_friend_req(&mut self, peer: PeerId) {
        self.command_sender
            .send(super::Command {
                // TODO: pass in the actual id instead of generating
                id: Uuid::new_v4(),
                cmd_type: CommandType::FriendCommand(FriendCommand::AcceptFriend {
                    peer,
                    decision: true,
                }),
            })
            .await
            .expect("to send request");
    }
    pub async fn deny_friend_req(&mut self, peer: PeerId) {
        self.command_sender
            .send(super::Command {
                // TODO: pass in the actual id instead of generating
                id: Uuid::new_v4(),
                cmd_type: CommandType::FriendCommand(FriendCommand::AcceptFriend {
                    peer,
                    decision: false,
                }),
            })
            .await
            .expect("to send request");
    }
    pub async fn search_peer(&mut self, id: String) {
        self.command_sender
            .send(super::Command {
                // TODO: pass in the actual id instead of generating
                id: Uuid::new_v4(),
                cmd_type: CommandType::FriendCommand(FriendCommand::SearchPeer { id }),
            })
            .await
            .expect("to send request");
    }
    pub async fn search_username(&mut self, username: String) {
        self.command_sender
            .send(super::Command {
                // TODO: pass in the actual id instead of generating
                id: Uuid::new_v4(),
                cmd_type: CommandType::FriendCommand(FriendCommand::SearchUsername { username }),
            })
            .await
            .expect("to send request");
    }
    pub async fn check_username_availability(&mut self, username: String) {
        self.command_sender
            .send(super::Command {
                // TODO: pass in the actual id instead of generating
                id: Uuid::new_v4(),
                cmd_type: CommandType::FriendCommand(FriendCommand::CheckUsernameAvailability {
                    username,
                }),
            })
            .await
            .expect("to send request");
    }
    pub async fn change_username(&mut self, username: String) {
        self.command_sender
            .send(super::Command {
                // TODO: pass in the actual id instead of generating
                id: Uuid::new_v4(),
                cmd_type: CommandType::FriendCommand(FriendCommand::ChangeUsername { username }),
            })
            .await
            .expect("to send request");
    }
    pub async fn load_friends(&mut self) {
        self.command_sender
            .send(super::Command {
                // TODO: pass in the actual id instead of generating
                id: Uuid::new_v4(),
                cmd_type: CommandType::FriendCommand(FriendCommand::LoadFriends),
            })
            .await
            .expect("to send request");
    }
    pub async fn load_pending_friend_requests(&mut self) {
        self.command_sender
            .send(super::Command {
                // TODO: pass in the actual id instead of generating
                id: Uuid::new_v4(),
                cmd_type: CommandType::FriendCommand(FriendCommand::LoadPendingFriendRequests),
            })
            .await
            .expect("to send request");
    }
    pub async fn load_incoming_friend_requests(&mut self) {
        self.command_sender
            .send(super::Command {
                // TODO: pass in the actual id instead of generating
                id: Uuid::new_v4(),
                cmd_type: CommandType::FriendCommand(FriendCommand::LoadIncomingFriendRequests),
            })
            .await
            .expect("to send request");
    }
}

// Name exchange -- Will occur when there is no name linked to PubKey
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
