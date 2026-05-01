pub mod peerboard {
    include!(concat!(env!("OUT_DIR"), "/peerboard.v1.rs"));
}

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
    routing::post, 
    Json,
    http::HeaderValue,
};
use tower_http::cors::{Any, CorsLayer};
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
use tokio::{io, io::AsyncBufReadExt, io::AsyncWriteExt, select, sync::broadcast, time};
use chrono::Utc;
use uuid::Uuid;

#[derive(NetworkBehaviour)]
struct ChatBehaviour {
    gossipsub: gossipsub::Behaviour,
    kad: kad::Behaviour<kad::store::MemoryStore>,
    identify: identify::Behaviour,
}

use serde::Serialize;

#[derive(Serialize)]
struct ChatMessageDto {
    peer_id: String,
    nickname: String,
    content: String,
    timestamp: i64,
    message_id: String,
    topic: String,
}

fn build_message(
    peer_id: &PeerId,
    topic: &str,
    content: &str,
    nickname: &str,
) -> Result<(PeerBoardMessage, Vec<u8>), Box<dyn Error>> {
    if content.len() > 4096 {
        return Err("content exceeds 4096 bytes".into());
    }
    if nickname.len() > 32 {
        return Err("nickname exceeds 32 bytes".into());
    }

    let msg = PeerBoardMessage {
        peer_id: peer_id.to_string(),
        topic: topic.to_string(),
        content: content.to_string(),
        timestamp: Utc::now().timestamp(),
        message_id: Uuid::new_v4().to_string(),
        nickname: nickname.to_string(),
    };

    let mut buf = Vec::with_capacity(msg.encoded_len());
    msg.encode(&mut buf)?;

    Ok((msg, buf))
}

fn decode_message(data: &[u8]) -> Result<PeerBoardMessage, prost::DecodeError> {
    PeerBoardMessage::decode(data)
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

    let nickname = "anon";

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

    let topic = gossipsub::IdentTopic::new("peerboard/v1/general");
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

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

    let (to_swarm_tx, mut to_swarm_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // ── Axum WebSocket server ──────────────────────────────────────────────
    let (broadcast_tx, _) = broadcast::channel::<String>(256);
    let broadcast_tx2 = broadcast_tx.clone();

    tokio::spawn(async move {
        let app = Router::new()
            .route("/ws", get(move |ws: WebSocketUpgrade| {
                let tx = broadcast_tx2.clone();
                let to_swarm = to_swarm_tx.clone();

                async move {
                    ws.on_upgrade(move |socket: WebSocket| async move {
                        let mut rx = tx.subscribe();
                        let (mut sender, mut receiver) = socket.split();

                        let send_task = tokio::spawn(async move {
                            while let Ok(msg) = rx.recv().await {
                                if sender.send(Message::Text(msg)).await.is_err() {
                                    break;
                                }
                            }
                        });

                        let recv_task = tokio::spawn(async move {
                            while let Some(Ok(Message::Text(text))) = receiver.next().await {
                                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                                    if let Some(content) = v.get("content").and_then(|c| c.as_str()) {
                                        let _ = to_swarm.send(content.to_string());
                                    }
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
    // ──────────────────────────────────────────────────────────────────────

    let mut stdin = io::BufReader::new(io::stdin()).lines();
    let mut bootstrap_timer = time::interval(Duration::from_secs(30));

    println!("--- Node Ready ---");

    loop {
        select! {
            _ = bootstrap_timer.tick() => {
                let _ = swarm.behaviour_mut().kad.bootstrap();
            }

            Some(content) = to_swarm_rx.recv() => {
                match build_message(&peer_id, topic.hash().as_str(), &content, &nickname) {
                    Ok((msg, payload)) => {
                        if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), payload) {
                            eprintln!("Publish error: {e:?}");
                        }

                        let dto = ChatMessageDto {
                            peer_id: msg.peer_id,
                            nickname: msg.nickname,
                            content: msg.content,
                            timestamp: msg.timestamp,
                            message_id: msg.message_id,
                            topic: msg.topic,
                        };

                        let json = serde_json::to_string(&dto).unwrap();
                        let _ = broadcast_tx.send(json);
                    }
                    Err(e) => eprintln!("Encode error: {e}"),
                }
            }

            event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on {address}");
                }

                SwarmEvent::Behaviour(ChatBehaviourEvent::Gossipsub(gossipsub::Event::Subscribed {
                    peer_id, topic
                })) => {
                    // println!("Peer {peer_id} subscribed to {topic}");
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
                            println!(
                                "\x1b[32m[{}]\x1b[0m ({}): {}",
                                msg.nickname, msg.peer_id, msg.content
                            );
                            let json = serde_json::json!({
                                "nickname": msg.nickname,
                                "peer_id": msg.peer_id,
                                "content": msg.content,
                                "timestamp": msg.timestamp,
                                "message_id": msg.message_id,
                                "topic": msg.topic,
                            }).to_string();
                            let _ = broadcast_tx.send(json);
                        }
                        Err(_) => {
                            println!("[raw]: {}", String::from_utf8_lossy(&message.data));
                            let _ = broadcast_tx.send(
                                String::from_utf8_lossy(&message.data).to_string()
                            );
                        }
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
