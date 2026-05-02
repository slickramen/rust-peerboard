use crate::protocol::*;
use crate::db::*;
use axum::extract::ws::{Message, WebSocket};
use futures::{SinkExt, StreamExt};
use tokio::sync::{broadcast, mpsc};
use std::sync::{Arc, Mutex};
use rusqlite::Connection;
use libp2p::PeerId;

pub async fn handle_socket(
    socket: WebSocket,
    peer_id: PeerId,
    nickname: String,
    tx: broadcast::Sender<String>,
    to_swarm: mpsc::UnboundedSender<ClientCommand>,
    db: Arc<Mutex<Connection>>,
    active_topics: Arc<Mutex<std::collections::HashMap<String, libp2p::gossipsub::IdentTopic>>>,
) {
    let mut rx = tx.subscribe();
    let (mut sender, mut receiver) = socket.split();

    let initial_topics: Vec<String> = active_topics.lock().unwrap().keys().cloned().collect();

    let init = ClientCommand::Init {
        user_id: peer_id.to_string(),
        username: nickname.to_string(),
        topics: initial_topics.clone(),
    };

    if let Ok(json) = serde_json::to_string(&init) {
        let _ = sender.send(Message::Text(json.into())).await;
    }

    for topic in &initial_topics {
        let msgs = {
            let conn = db.lock().unwrap();
            load_messages(&conn, topic).unwrap_or_default()
        };
        for msg in msgs {
            let ws_msg = ClientCommand::Chat {
                peer_id: Some(msg.peer_id),
                nickname: Some(msg.nickname),
                content: msg.content,
                timestamp: Some(msg.timestamp),
                message_id: Some(msg.message_id),
                topic: msg.topic,
            };
            if let Ok(json) = serde_json::to_string(&ws_msg) {
                let _ = sender.send(Message::Text(json.into())).await;
            }
        }
    }

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            match serde_json::from_str::<ClientCommand>(&text) {
                Ok(cmd) => { let _ = to_swarm.send(cmd); }
                Err(e) => eprintln!("Deserialize error: {e}"),
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}

pub fn send_ws(tx: &broadcast::Sender<String>, msg: ClientCommand) {
    if let Ok(json) = serde_json::to_string(&msg) {
        let _ = tx.send(json);
    }
}
