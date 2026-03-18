use libp2p::PeerId;
use serde::{Deserialize, Serialize};

use crate::{
    db::sql_calls::{get_friends, get_incoming_friend_requests, get_pending_friend_requests},
    network::{Client, EventLoop},
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
impl EventLoop {
    pub async fn handle_friend_command(&mut self, command: FriendCommand) {
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
            FriendCommand::SearchPeer { id } => unimplemented!(),
            FriendCommand::SearchUsername { username } => unimplemented!(),
            FriendCommand::CheckUsernameAvailability { username } => unimplemented!(),
            FriendCommand::ChangeUsername { username } => unimplemented!(),
            FriendCommand::LoadFriends => {
                let friends = self.sqlite_conn.call(get_friends).await;
                match friends {
                    Ok(f) => {
                        self.api_writer_tx
                            .send(crate::WriteEvent::EventResponse(
                                crate::UiClientEventResponse::LoadFriends(f),
                            ))
                            .expect("to send");
                    }
                    Err(e) => {
                        tracing::info!("{e}");
                    }
                }
            }
            FriendCommand::LoadPendingFriendRequests => {
                let pending_requests = self.sqlite_conn.call(get_pending_friend_requests).await;
                match pending_requests {
                    Ok(requests) => {
                        self.api_writer_tx
                            .send(crate::WriteEvent::EventResponse(
                                crate::UiClientEventResponse::LoadPendingFriendRequests(requests),
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
                                crate::UiClientEventResponse::LoadIncomingFriendRequests(requests),
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
            .send(super::Command::FriendCommand(FriendCommand::RequestName {
                peer,
            }))
            .await
            .expect("to send request");
        tracing::info!("Sending name req");
    }
    pub async fn send_friend_request(&mut self, peer: PeerId) {
        self.command_sender
            .send(super::Command::FriendCommand(FriendCommand::AddFriend {
                peer,
            }))
            .await
            .expect("to send request");
    }
    pub async fn accept_friend_req(&mut self, peer: PeerId) {
        self.command_sender
            .send(super::Command::FriendCommand(FriendCommand::AcceptFriend {
                peer,
                decision: true,
            }))
            .await
            .expect("to send request");
    }
    pub async fn deny_friend_req(&mut self, peer: PeerId) {
        self.command_sender
            .send(super::Command::FriendCommand(FriendCommand::AcceptFriend {
                peer,
                decision: false,
            }))
            .await
            .expect("to send request");
    }
    pub async fn search_peer(&mut self, id: String) {
        self.command_sender
            .send(super::Command::FriendCommand(FriendCommand::SearchPeer {
                id,
            }))
            .await
            .expect("to send request");
    }
    pub async fn search_username(&mut self, username: String) {
        self.command_sender
            .send(super::Command::FriendCommand(
                FriendCommand::SearchUsername { username },
            ))
            .await
            .expect("to send request");
    }
    pub async fn check_username_availability(&mut self, username: String) {
        self.command_sender
            .send(super::Command::FriendCommand(
                FriendCommand::CheckUsernameAvailability { username },
            ))
            .await
            .expect("to send request");
    }
    pub async fn change_username(&mut self, username: String) {
        self.command_sender
            .send(super::Command::FriendCommand(
                FriendCommand::ChangeUsername { username },
            ))
            .await
            .expect("to send request");
    }
    pub async fn load_friends(&mut self) {
        self.command_sender
            .send(super::Command::FriendCommand(FriendCommand::LoadFriends))
            .await
            .expect("to send request");
    }
    pub async fn load_pending_friend_requests(&mut self) {
        self.command_sender
            .send(super::Command::FriendCommand(
                FriendCommand::LoadPendingFriendRequests,
            ))
            .await
            .expect("to send request");
    }
    pub async fn load_incoming_friend_requests(&mut self) {
        self.command_sender
            .send(super::Command::FriendCommand(
                FriendCommand::LoadIncomingFriendRequests,
            ))
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
