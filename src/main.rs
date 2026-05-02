pub mod peerboard {
    include!(concat!(env!("OUT_DIR"), "/peerboard.v1.rs"));
}

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    routing::get,
    Router,
};
use tower_http::{
    cors::{Any, CorsLayer},
};

use futures::{SinkExt, StreamExt};
use libp2p::{
    gossipsub, identity, kad, noise,
    swarm::{NetworkBehaviour, SwarmEvent, dial_opts::DialOpts},
    tcp, yamux, Multiaddr, PeerId,
};
use libp2p::identify;
use peerboard::PeerBoardMessage;
use prost::Message as ProstMessage;
use std::{error::Error, time::Duration};
use tokio::{io, select, sync::broadcast, time};
use chrono::Utc;
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use gossipsub::IdentTopic;
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};

#[derive(NetworkBehaviour)]
struct ChatBehaviour {
    gossipsub: gossipsub::Behaviour,
    kad: kad::Behaviour<kad::store::MemoryStore>,
    identify: identify::Behaviour,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientCommand {
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
    Unsubscribed { topic: String },
    Unsubscribe { topic: String },
    Subscribed { topic: String },
    Error { message: String },
}

fn build_message(
    peer_id: &PeerId,
    topic: &str,
    content: &str,
    nickname: &str,
) -> Result<(PeerBoardMessage, Vec<u8>), Box<dyn Error>> {
    if content.as_bytes().len() > 4096 {
        return Err("content exceeds 4096 bytes".into());
    }
    if nickname.as_bytes().len() > 32 {
        return Err("nickname exceeds 32 bytes".into());
    }
    if !topic.starts_with("peerboard/v1/") {
        return Err("topic does not begin with peerboard/v1/".into());
    }

    let timestamp = Utc::now().timestamp();

    let msg = PeerBoardMessage {
        peer_id: peer_id.to_string(),
        topic: topic.to_string(),
        content: content.to_string(),
        timestamp,
        message_id: Uuid::new_v4().to_string(),
        nickname: nickname.to_string(),
    };

    let mut buf = Vec::with_capacity(msg.encoded_len());
    msg.encode(&mut buf)?;

    Ok((msg, buf))
}

fn validate_message(msg: &PeerBoardMessage) -> bool {
    if msg.content.as_bytes().len() > 4096 {
        return false;
    }
    if msg.nickname.as_bytes().len() > 32 {
        return false;
    }
    if !msg.topic.starts_with("peerboard/v1/") {
        return false;
    }
    let now = Utc::now().timestamp();
    if msg.timestamp > now + 300 {
        return false;
    }
    true
}

fn decode_message(data: &[u8]) -> Result<PeerBoardMessage, prost::DecodeError> {
    PeerBoardMessage::decode(data)
}

fn send_ws(tx: &broadcast::Sender<String>, msg: ClientCommand) {
    if let Ok(json) = serde_json::to_string(&msg) {
        let _ = tx.send(json);
    }
}

fn setup_db(conn: &Connection) -> Result<(), Box<dyn Error>> {
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS messages (
            message_id  TEXT PRIMARY KEY,
            peer_id     TEXT NOT NULL,
            nickname    TEXT NOT NULL,
            content     TEXT NOT NULL,
            topic       TEXT NOT NULL,
            timestamp   INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS topics (
            topic TEXT PRIMARY KEY
        );
    ")?;
    Ok(())
}

fn store_message(conn: &Connection, msg: &PeerBoardMessage) -> Result<bool, Box<dyn Error>> {
    let rows = conn.execute(
        "INSERT OR IGNORE INTO messages (message_id, peer_id, nickname, content, topic, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            msg.message_id,
            msg.peer_id,
            msg.nickname,
            msg.content,
            msg.topic,
            msg.timestamp,
        ],
    )?;

    // Check if duplicate
    Ok(rows > 0)
}

fn load_messages(conn: &Connection, topic: &str) -> Result<Vec<PeerBoardMessage>, Box<dyn Error>> {
    let mut stmt = conn.prepare(
        "SELECT message_id, peer_id, nickname, content, topic, timestamp
         FROM messages
         WHERE topic = ?1
         ORDER BY timestamp ASC"
    )?;

    let msgs = stmt.query_map(params![topic], |row| {
        Ok(PeerBoardMessage {
            message_id: row.get(0)?,
            peer_id: row.get(1)?,
            nickname: row.get(2)?,
            content: row.get(3)?,
            topic: row.get(4)?,
            timestamp: row.get(5)?,
        })
    })?
    .filter_map(|r| r.ok())
    .collect();

    Ok(msgs)
}

fn store_topic(conn: &Connection, topic: &str) -> Result<(), Box<dyn Error>> {
    conn.execute("INSERT OR IGNORE INTO topics (topic) VALUES (?1)", params![topic])?;
    Ok(())
}

fn remove_topic(conn: &Connection, topic: &str) -> Result<(), Box<dyn Error>> {
    conn.execute("DELETE FROM topics WHERE topic = ?1", params![topic])?;
    Ok(())
}

fn load_topics(conn: &Connection) -> Result<Vec<String>, Box<dyn Error>> {
    let mut stmt = conn.prepare("SELECT topic FROM topics")?;
    let topics = stmt.query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(topics)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let key = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(key.public());
    println!("Local peer id: {peer_id}");

    // let nickname = {
    //     print!("Enter your nickname: ");
    //     io::stdout().flush().await?;
    //     let mut lines = io::BufReader::new(io::stdin()).lines();
    //     lines.next_line().await?.unwrap_or_default().trim().to_string()
    // };

    // if nickname.is_empty() || nickname.len() > 32 {
    //     eprintln!("Nickname must be 1-32 bytes.");
    //     return Ok(());
    // }

    let nickname = "martyn test";

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic()
        .with_behaviour(|key| {
            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .validation_mode(gossipsub::ValidationMode::Strict)
                .build()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            ).map_err(|e| Box::<dyn Error + Send + Sync>::from(e.to_string()))?;

            let kad_config = kad::Config::new(
                libp2p::StreamProtocol::new("/peerboard/kad/1.0.0")
            );

            let store = kad::store::MemoryStore::new(key.public().to_peer_id());
            let kad = kad::Behaviour::with_config(key.public().to_peer_id(), store, kad_config);

            let identify = identify::Behaviour::new(
                identify::Config::new(
                    "/peerboard/1.0.0".to_string(),
                    key.public(),
                )
            );

            Ok(ChatBehaviour { gossipsub, kad, identify })
        })?
        .build();

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
    swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;

    let bootstrap_peer_id: PeerId = "12D3KooWCvwqT3JUzVQczCvAVFa9EGzNqjHHSMVHVhm3RVyscCNY".parse()?;
    let addrs = vec![
        "/ip4/170.64.177.57/tcp/8000".parse::<Multiaddr>()?,
        "/ip4/170.64.177.57/udp/8000/quic-v1".parse::<Multiaddr>()?,
    ];
    swarm.behaviour_mut().kad.add_address(&bootstrap_peer_id, addrs[0].clone());
    swarm.behaviour_mut().kad.add_address(&bootstrap_peer_id, addrs[1].clone());
    swarm.dial(
        DialOpts::peer_id(bootstrap_peer_id)
            .addresses(addrs)
            .build(),
    )?;

    swarm.behaviour_mut().kad.bootstrap()?;
    swarm.behaviour_mut().kad.get_closest_peers(peer_id);

    let (to_swarm_tx, mut to_swarm_rx) =
        tokio::sync::mpsc::unbounded_channel::<ClientCommand>();

    let (broadcast_tx, _) = broadcast::channel::<String>(256);
    let broadcast_tx2 = broadcast_tx.clone();
    
    let db = Arc::new(Mutex::new(Connection::open("peerboard.db")?));
    setup_db(&db.lock().unwrap())?;

    store_topic(&db.lock().unwrap(), "peerboard/v1/general")?;

    let persisted_topics = load_topics(&db.lock().unwrap())?;
    println!("Persisted topics from DB: {:?}", persisted_topics);

    let active_topics: Arc<Mutex<HashMap<String, IdentTopic>>> = Arc::new(Mutex::new(HashMap::new()));

    for t in &persisted_topics {
        let ident = gossipsub::IdentTopic::new(t);
        match swarm.behaviour_mut().gossipsub.subscribe(&ident) {
            Ok(_) => {
                active_topics.lock().unwrap().insert(t.clone(), ident);
            }
            Err(e) => eprintln!("Failed to resubscribe to {t}: {e}"),
        }
    }

    let active_topics_ws = active_topics.clone();

    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    {
        let conn = db.lock().unwrap();
        let mut stmt = conn.prepare("SELECT message_id FROM messages")?;
        let ids = stmt.query_map([], |row| row.get::<_, String>(0))?;
        for id in ids {
            seen_ids.insert(id?);
        }
    }

    println!("Loaded {} message IDs from store", seen_ids.len());

    let db_ws = db.clone();

    tokio::spawn(async move {
    let app = Router::new()
        .route("/ws", get(move |ws: WebSocketUpgrade| {
            let tx = broadcast_tx2.clone();
            let to_swarm = to_swarm_tx.clone();
            let db = db_ws.clone();
            let active_topics_ws = active_topics_ws.clone();

            async move {
                ws.on_upgrade(move |socket: WebSocket| async move {
                    let mut rx = tx.subscribe();
                    let (mut sender, mut receiver) = socket.split(); // add this

                    let initial_topics: Vec<String> = active_topics_ws.lock().unwrap().keys().cloned().collect();

                    let init = ClientCommand::Init {
                        user_id: peer_id.to_string(),
                        username: nickname.to_string(),
                        topics: initial_topics.clone(),
                    };

                    if let Ok(json) = serde_json::to_string(&init) {
                        let _ = sender.send(Message::Text(json.into())).await;
                    }

                    // replay history for each subscribed topic
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
                                Ok(cmd) => { 
                                    let _ = to_swarm.send(cmd); 
                                }
                                Err(e) => eprintln!("Deserialize error: {e}"),
                            }
                        }
                    });

                    tokio::select! {
                        _ = send_task => {},
                        _ = recv_task => {},
                    }
                })
            }
        }))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await.unwrap();
        println!("API server listening on http://127.0.0.1:{port}");
        axum::serve(listener, app).await.unwrap();
    });

    let mut bootstrap_timer = time::interval(Duration::from_secs(30));

    println!("--- Node Ready ---");

    loop {
        select! {
            _ = bootstrap_timer.tick() => {
                let _ = swarm.behaviour_mut().kad.bootstrap();
            }

            Some(cmd) = to_swarm_rx.recv() => {
                match cmd {
                    ClientCommand::Subscribe { topic: name } => {
                        if !active_topics.lock().unwrap().contains_key(&name) {
                            let t = gossipsub::IdentTopic::new(&name);
                            match swarm.behaviour_mut().gossipsub.subscribe(&t) {
                                Ok(_) => {
                                    store_topic(&db.lock().unwrap(), &name)?;
                                    active_topics.lock().unwrap().insert(name.clone(), t);
                                    send_ws(&broadcast_tx, ClientCommand::Subscribed { topic: name.clone() });

                                    // replay stored messages for this topic
                                    let msgs = load_messages(&db.lock().unwrap(), &name).unwrap_or_default();
                                    for msg in msgs {
                                        let ws_msg = ClientCommand::Chat {
                                            peer_id: Some(msg.peer_id),
                                            nickname: Some(msg.nickname),
                                            content: msg.content,
                                            timestamp: Some(msg.timestamp),
                                            message_id: Some(msg.message_id),
                                            topic: msg.topic,
                                        };
                                        send_ws(&broadcast_tx, ws_msg);
                                    }
                                }
                                Err(e) => {
                                    send_ws(&broadcast_tx, ClientCommand::Error {
                                        message: format!("subscribe failed: {e}"),
                                    });
                                }
                            }
                        }
                    }

                    ClientCommand::Unsubscribe { topic: name } => {
                        if let Some(t) = active_topics.lock().unwrap().remove(&name) {
                            swarm.behaviour_mut().gossipsub.unsubscribe(&t);
                            remove_topic(&db.lock().unwrap(), &name)?;
                            send_ws(&broadcast_tx, ClientCommand::Unsubscribed { topic: name });
                        }
                    }

                    ClientCommand::Chat { topic, content, .. } => {
                        let t = active_topics.lock().unwrap().get(&topic).cloned()
                            .unwrap_or_else(|| gossipsub::IdentTopic::new(&topic));

                        match build_message(&peer_id, &topic, &content, nickname) { 
                            Ok((msg, payload)) => {
                                if let Err(e) = swarm.behaviour_mut().gossipsub.publish(t, payload) {
                                    eprintln!("Publish error: {e:?}");
                                }

                                seen_ids.insert(msg.message_id.clone());

                                if let Err(e) = store_message(&db.lock().unwrap(), &msg) {
                                    eprintln!("DB error: {e}");
                                }

                                let ws_msg = ClientCommand::Chat {
                                    peer_id: Some(msg.peer_id.clone()),
                                    nickname: Some(msg.nickname.clone()),
                                    content: msg.content.clone(),
                                    timestamp: Some(msg.timestamp),
                                    message_id: Some(msg.message_id.clone()),
                                    topic: topic.clone(),
                                };

                                println!(
                                    "\x1b[32m[{}]\x1b[0m ({}): {}",
                                    msg.nickname, msg.peer_id, msg.content
                                );

                                send_ws(&broadcast_tx, ws_msg);
                            }
                            Err(e) => eprintln!("Encode error: {e}"),
                        }
                    }

                    _ => {}
                }
            }

            event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on {address}");
                }

                SwarmEvent::Behaviour(ChatBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed {
                    peer_id, topic
                })) => {
                    println!("Peer {peer_id} subscribed to {topic}");
                }

                SwarmEvent::Behaviour(ChatBehaviourEvent::Gossipsub(gossipsub::Event::GossipsubNotSupported {
                    peer_id
                })) => {
                    // println!("Peer {peer_id} does not support Gossipsub");
                }

                SwarmEvent::Behaviour(ChatBehaviourEvent::Kad(event)) => {
                    match event {
                        kad::Event::RoutingUpdated { peer, .. } => {
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                        }
                        kad::Event::OutboundQueryProgressed { result, .. } => {
                            match result {
                                kad::QueryResult::GetClosestPeers(Ok(ok)) => {
                                    println!("Kad returned {} close peers", ok.peers.len());
                                    for peer in ok.peers {
                                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer.peer_id);
                                    }
                                }
                                kad::QueryResult::Bootstrap(Ok(ok)) => {
                                    if ok.num_remaining == 0 {
                                        swarm.behaviour_mut().kad.get_closest_peers(peer_id);
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }

                SwarmEvent::Behaviour(ChatBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    message,
                    ..
                })) => {
                    match decode_message(&message.data) {
                        Ok(msg) => {
                            if !validate_message(&msg) {
                                continue;
                            }
                            if !seen_ids.insert(msg.message_id.clone()) {
                                continue;
                            }

                            // store msgs
                            if let Err(e) = store_message(&db.lock().unwrap(), &msg) {
                                eprintln!("DB error: {e}");
                            }

                            println!(
                                "\x1b[32m[{}]\x1b[0m ({}): {}",
                                msg.nickname, msg.peer_id, msg.content
                            );
                            let json = serde_json::json!({
                                "type": "chat",
                                "nickname": msg.nickname,
                                "peer_id": msg.peer_id,
                                "content": msg.content,
                                "timestamp": msg.timestamp,
                                "message_id": msg.message_id,
                                "topic": message.topic.to_string(),
                            }).to_string();
                            let _ = broadcast_tx.send(json);
                        }

                        Err(_) => {}
                    }
                }

                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                }

                SwarmEvent::OutgoingConnectionError { peer_id: Some(id), error, .. } => {
                    if !error.to_string().contains("Connection refused") {
                        // println!("Dial error to {id}: {error}");
                    }
                }

                _ => {}
            }
        }
    }
}
