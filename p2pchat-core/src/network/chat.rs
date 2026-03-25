use crate::db::sql_calls::{get_message_log, insert_message};
use crate::network::{Client, EventLoop};
use crate::network::{Command, CommandType};
use libp2p::PeerId;
use p2pchat_types::api::UiClientEventResponseType;
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
    Ack,
    DeniedNotFriends,
    // InvalidSignature { message_id: Uuid },
}
pub enum ChatCommand {
    SendMessage { receiver: PeerId, message: Message },
    LoadChatLog { channel_id: i64, page: usize },
}
impl EventLoop {
    pub async fn handle_chat_command(&mut self, command: ChatCommand, req_id: Uuid) {
        match command {
            ChatCommand::SendMessage { receiver, message } => {
                let m = message.clone();
                let receiver_str = receiver.to_string();
                self.sqlite_conn
                    .call(move |c| {
                        let channel_id =
                            crate::db::sql_calls::get_contact_channel_id(c, receiver_str)?;
                        insert_message(c, m, channel_id)
                    })
                    .await
                    .expect("to write");
                // TODO: handle this error
                let id = self
                    .swarm
                    .behaviour_mut()
                    .direct_message
                    .send_request(&receiver, DirectMessageRequest(message));
                self.request_map.insert(id, crate::UiClientEventId(req_id));
            }
            ChatCommand::LoadChatLog { channel_id, page } => {
                let res = self
                    .sqlite_conn
                    .call(move |c| get_message_log(c, channel_id, page))
                    .await;
                match res {
                    Ok(log) => {
                        self.api_writer_tx
                            .send(crate::WriteEvent::EventResponse(
                                crate::UiClientEventResponse {
                                    req_id,
                                    result: Ok(UiClientEventResponseType::LoadChatlogPage(log)),
                                },
                            ))
                            .expect("to send");
                    }
                    Err(err) => {
                        // TODO: Return an error to the sock
                        tracing::info!("{err}");
                    }
                }
            }
        }
    }
}
impl Client {
    pub async fn send_message(&mut self, receiver: PeerId, message: String, req_id: Uuid) {
        let message = Message {
            content: message,
            id: uuid::Uuid::new_v4(),
        };
        self.command_sender
            .send(Command {
                id: req_id,
                cmd_type: CommandType::ChatCommand(ChatCommand::SendMessage { receiver, message }),
            })
            .await
            .expect("to send");
    }
    pub async fn load_chatlog_page(&mut self, channel_id: i64, page: usize, req_id: Uuid) {
        self.command_sender
            .send(Command {
                id: req_id,
                cmd_type: CommandType::ChatCommand(ChatCommand::LoadChatLog { channel_id, page }),
            })
            .await
            .expect("to send");
    }
}
