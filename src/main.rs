pub mod peerboard {
    include!(concat!(env!("OUT_DIR"), "/peerboard.v1.rs"));
}

mod db;
mod message;
mod protocol;
mod keypair;
mod ws;
mod bootstrap;

use db::*;
use message::*;
use protocol::*;
use keypair::load_or_generate_keypair;
use ws::{
    handle_socket,
    send_ws,
};
use bootstrap::bootstrap_node;

use axum::{
    extract::ws::{WebSocketUpgrade},
    routing::get,
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use futures::{StreamExt};
use libp2p::{
    gossipsub, kad, noise,
    swarm::{SwarmEvent, dial_opts::DialOpts},
    tcp, yamux, PeerId,
};
use libp2p::identify;
use std::{error::Error, time::Duration};
use tokio::{io, select, sync::broadcast, time};
use std::collections::HashMap;
use gossipsub::IdentTopic;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

#[derive(libp2p::swarm::NetworkBehaviour)]
struct ChatBehaviour {
    gossipsub: gossipsub::Behaviour,
    kad: kad::Behaviour<kad::store::MemoryStore>,
    identify: identify::Behaviour,
}

pub fn get_nickname() -> String {
    let args: Vec<String> = std::env::args().collect();
    args.windows(2)
        .find_map(|w| {
            if w[0] == "--nickname" || w[0] == "-n" {
                Some(w[1].clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "anon".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let key = load_or_generate_keypair()?;
    let peer_id = PeerId::from(key.public());
    println!("Local peer id: {peer_id}");

    let nickname = get_nickname();

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

    let bootstrap = bootstrap_node();
    for addr in &bootstrap.addrs {
        swarm.behaviour_mut().kad.add_address(&bootstrap.peer_id, addr.clone());
    }
    swarm.dial(
        DialOpts::peer_id(bootstrap.peer_id)
            .addresses(bootstrap.addrs)
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

    let loaded_topics = load_topics(&db.lock().unwrap())?;
    println!("Loaded topics from DB: {:?}", loaded_topics);

    let active_topics: Arc<Mutex<HashMap<String, IdentTopic>>> = Arc::new(Mutex::new(HashMap::new()));

    for t in &loaded_topics {
        let ident = gossipsub::IdentTopic::new(t);
        match swarm.behaviour_mut().gossipsub.subscribe(&ident) {
            Ok(_) => {
                active_topics.lock().unwrap().insert(t.clone(), ident);
            }
            Err(e) => eprintln!("Unable to resubscribe to {t}: {e}"),
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
    let nickname_ws = nickname.clone();

    tokio::spawn(async move {
        let app = Router::new()
            .route("/ws", get(move |ws: WebSocketUpgrade| {
                let tx = broadcast_tx2.clone();
                let to_swarm = to_swarm_tx.clone();
                let db = db_ws.clone();
                let active_topics_ws = active_topics_ws.clone();

                async move {
                    ws.on_upgrade(move |socket| handle_socket(
                        socket, peer_id, nickname_ws,
                        tx, to_swarm, db, active_topics_ws,
                    ))
                }
            }))
            .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any));

        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await.unwrap();
        println!("API server listening on http://127.0.0.1:{port}");
        axum::serve(listener, app).await.unwrap();
    });

    let mut bootstrap_timer = time::interval(Duration::from_secs(30));

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

                                    let msgs = load_messages(&db.lock().unwrap(), &name).unwrap_or_default();
                                    for msg in msgs {
                                        let ws_msg = make_chat_command(&msg, &msg.topic.clone());
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

                        match build_message(&peer_id, &topic, &content, nickname.as_str()) { 
                            Ok((msg, payload)) => {
                                if let Err(e) = swarm.behaviour_mut().gossipsub.publish(t, payload) {
                                    eprintln!("Publish error: {e:?}");
                                }

                                seen_ids.insert(msg.message_id.clone());

                                if let Err(e) = store_message(&db.lock().unwrap(), &msg) {
                                    eprintln!("DB error: {e}");
                                }

                                let ws_msg = make_chat_command(&msg, &topic);

                                println!(
                                    "[{}] ({}): {}",
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
                                "[{}] ({}): {}",
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

                _ => {}
            }
        }
    }
}
