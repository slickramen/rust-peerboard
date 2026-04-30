use futures::StreamExt;
use libp2p::{
    gossipsub, identity, kad, noise,
    swarm::{NetworkBehaviour, SwarmEvent, dial_opts::DialOpts},
    tcp, yamux, Multiaddr, PeerId, StreamProtocol,
};
use std::{error::Error, time::Duration};
use tokio::{io, io::AsyncBufReadExt, select, time};

#[derive(NetworkBehaviour)]
struct ChatBehaviour {
    gossipsub: gossipsub::Behaviour,
    kad: kad::Behaviour<kad::store::MemoryStore>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let key = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(key.public());
    println!("Local peer id: {peer_id}");

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_quic() // Required by PeerBoard spec
        .with_behaviour(|key| {
            // 1. GOSSIPSUB CONFIG: Optimized for small/local networks
            let gossipsub_config = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_secs(1))
                .mesh_n_low(1) // Allow meshing with even 1 peer
                .mesh_outbound_min(1)
                .mesh_n(2)
                .build()
                .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;

            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            ).map_err(|e| Box::<dyn Error + Send + Sync>::from(e.to_string()))?;

            // 2. KADEMLIA CONFIG: Strict Protocol ID for PeerBoard
            let mut kad_config = kad::Config::default();
            kad_config.set_protocol_names(vec![StreamProtocol::new("/peerboard/kad/1.0.0")]);
            
            let store = kad::store::MemoryStore::new(key.public().to_peer_id());
            let kad = kad::Behaviour::with_config(key.public().to_peer_id(), store, kad_config);

            Ok(ChatBehaviour { gossipsub, kad })
        })?
        .build();

    // Listen on all interfaces (0.0.0.0)
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    let topic = gossipsub::IdentTopic::new("peerboard/v1/general");
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    // 3. BOOTSTRAP: Using your specific university node
    let bootstrap_addr: Multiaddr = "/ip4/170.64.177.57/tcp/8000".parse()?;
    let bootstrap_peer_id: PeerId = "12D3KooWCvwqT3JUzVQczCvAVFa9EGzNqjHHSMVHVhm3RVyscCNY".parse()?;

    swarm.behaviour_mut().kad.add_address(&bootstrap_peer_id, bootstrap_addr.clone());
    
    // Explicit dial to ensure we actually hit the bootstrap
    swarm.dial(
        DialOpts::peer_id(bootstrap_peer_id)
            .addresses(vec![bootstrap_addr])
            .build(),
    )?;

    // Start discovery and self-lookup
    swarm.behaviour_mut().kad.bootstrap()?;
    swarm.behaviour_mut().kad.get_closest_peers(peer_id);

    let mut stdin = io::BufReader::new(io::stdin()).lines();
    let mut bootstrap_timer = time::interval(Duration::from_secs(30));

    println!("--- Node Ready ---");

    loop {
        select! {
            line = stdin.next_line() => {
                let line = line?.unwrap_or_default();
                if line.is_empty() { continue; }

                // Attempt publish (will fail if mesh count is 0)
                if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), line.as_bytes()) {
                    println!("Publish error: {e:?}");
                    println!("Current peers: {}", swarm.behaviour().gossipsub.all_peers().count());
                }
            }

            _ = bootstrap_timer.tick() => {
                let _ = swarm.behaviour_mut().kad.bootstrap();
            }

            event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on {address}");
                }

                // BRIDGE: When Kademlia finds a peer, explicitly notify Gossipsub
                SwarmEvent::Behaviour(ChatBehaviourEvent::Kad(kad::Event::RoutingUpdated { peer, .. })) => {
                    println!("Kademlia found peer: {peer}");
                    // This forces Gossipsub to track this peer for its next heartbeat mesh
                    swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer);
                }

                SwarmEvent::Behaviour(ChatBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source,
                    message,
                    ..
                })) => {
                    println!("\x1b[32m[{}]\x1b[0m: {}", propagation_source, String::from_utf8_lossy(&message.data));
                }

                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    println!("Connected to peer: {peer_id}");
                }

                SwarmEvent::OutgoingConnectionError { peer_id: Some(id), error, .. } => {
                    // Ignore Os Error 61 (Refused) for other students' public IPs
                    // but log others to help debugging
                    if !error.to_string().contains("Connection refused") {
                        println!("Dial error to {id}: {error}");
                    }
                }

                _ => {}
            }
        }
    }
}
