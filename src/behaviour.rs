use libp2p::{
    gossipsub, kad, identify,
    swarm::NetworkBehaviour,
};
use std::error::Error;
use tokio::io;

#[derive(NetworkBehaviour)]
pub struct ChatBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kad: kad::Behaviour<kad::store::MemoryStore>,
    pub identify: identify::Behaviour,
}

pub fn build_behaviour(key: &libp2p::identity::Keypair) -> Result<ChatBehaviour, Box<dyn Error + Send + Sync>> {
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
}