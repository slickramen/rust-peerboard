use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

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

pub fn send_ws(tx: &broadcast::Sender<String>, msg: ClientCommand) {
    if let Ok(json) = serde_json::to_string(&msg) {
        let _ = tx.send(json);
    }
}