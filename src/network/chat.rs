use crate::network::Command;
use crate::network::signable::{Signed, sign};
use crate::network::{Client, EventLoop};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectMessageRequest(pub Message);
#[derive(Debug, Serialize, Deserialize)]
pub struct DirectMessageResponse(pub MessageResponse);

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Message {
    pub content: String,
    pub id: Uuid,
}
#[derive(Debug, Serialize, Deserialize)]
pub enum MessageResponse {
    ACK { message_id: Uuid },
    InvalidSignature { message_id: Uuid },
}
pub enum ChatCommand {
    SendMessage { receiver: PeerId, message: Message },
    // ReadMessage {
    //     receiver: PeerId,
    // },
}
impl EventLoop {
    pub async fn handle_chat_command(&mut self, command: ChatCommand) {
        match command {
            ChatCommand::SendMessage { receiver, message } => {
                self.swarm
                    .behaviour_mut()
                    .direct_message
                    .send_request(&receiver, DirectMessageRequest(message));
            } // ChatCommand::ReadMessage { receiver } => {
              //     todo!()
              //     // self.swarm
              //     //     .behaviour_mut()
              //     //     .direct_message
              //     //     .send_request(&receiver, DirectMessageRequest(1));
              // }
        }
    }
}
impl Client {
    pub async fn send_message(&mut self, receiver: PeerId, message: String) {
        let message = Message {
            content: message,
            id: uuid::Uuid::new_v4(),
        };
        self.command_sender
            .send(Command::ChatCommand(ChatCommand::SendMessage {
                receiver,
                message,
            }))
            .await
            .expect("to send");
    }
    pub async fn load_chatlog_page(&mut self) {
        unimplemented!()
    }
}
