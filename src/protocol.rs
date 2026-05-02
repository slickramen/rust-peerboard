use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::peerboard::PeerBoardMessage;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientCommand {
    Init {
        user_id: String,
        username: String,
        topics: Vec<String>,
    },
    Chat {
        topic: String,
        content: String,
        #[serde(default)]
        peer_id: Option<String>,
        #[serde(default)]
        nickname: Option<String>,
        #[serde(default)]
        timestamp: Option<i64>,
        #[serde(default)]
        message_id: Option<String>,
    },
    Subscribe { topic: String },
    Unsubscribe { topic: String },
    Subscribed { topic: String },
    Unsubscribed { topic: String },
    Error { message: String },
}

pub fn make_chat_command(msg: &PeerBoardMessage, topic: &str) -> ClientCommand {
    ClientCommand::Chat {
        peer_id: Some(msg.peer_id.clone()),
        nickname: Some(msg.nickname.clone()),
        content: msg.content.clone(),
        timestamp: Some(msg.timestamp),
        message_id: Some(msg.message_id.clone()),
        topic: topic.to_string(),
    }
}
