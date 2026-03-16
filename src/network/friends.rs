use libp2p::PeerId;
use serde::{Deserialize, Serialize};

use crate::network::{Client, EventLoop};

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
}
impl EventLoop {
    pub async fn handle_friend_command(&mut self, command: FriendCommand) {
        // TODO: Add everything to sqlite
        // Send re-render of contact list to tui
        match command {
            FriendCommand::RequestName { peer } => self
                .swarm
                .behaviour_mut()
                .friends
                .send_request(&peer, FriendRequest::RequestName),
            FriendCommand::AddFriend { peer } => self
                .swarm
                .behaviour_mut()
                .friends
                .send_request(&peer, FriendRequest::AddFriend),
            FriendCommand::AcceptFriend { peer, decision } => self
                .swarm
                .behaviour_mut()
                .friends
                .send_request(&peer, FriendRequest::AcceptFriend { decision }),
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
    pub async fn search_peer(&mut self, name: String) {
        unimplemented!()
    }
    pub async fn search_username(&mut self, username: String) {
        unimplemented!()
    }
    pub async fn check_username_availability(&mut self, username: String) {
        unimplemented!()
    }
    pub async fn change_username(&mut self, username: String) {
        unimplemented!()
    }
    pub async fn load_friends(&mut self) {
        unimplemented!()
    }
    pub async fn load_pending_friend_requests(&mut self) {
        unimplemented!()
    }
    pub async fn load_incoming_friend_requests(&mut self) {
        unimplemented!()
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
