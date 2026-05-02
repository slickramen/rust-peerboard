use std::path::PathBuf;
use std::error::Error;
use libp2p::identity;

pub fn load_or_generate_keypair() -> Result<identity::Keypair, Box<dyn Error>> {
    let path = keypair_path();

    if path.exists() {
        let bytes = std::fs::read(&path)?;
        let keypair = identity::Keypair::from_protobuf_encoding(&bytes)?;
        println!("Loaded existing keypair from {}", path.display());
        Ok(keypair)
    } else {
        let keypair = identity::Keypair::generate_ed25519();
        save_keypair(&keypair, &path)?;
        println!("Generated new keypair, saved to {}", path.display());
        Ok(keypair)
    }
}

fn save_keypair(keypair: &identity::Keypair, path: &PathBuf) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = keypair.to_protobuf_encoding()?;
    std::fs::write(path, bytes)?;
    Ok(())
}

fn keypair_path() -> PathBuf {
    if let Ok(path) = std::env::var("PEERBOARD_KEY") {
        return PathBuf::from(path);
    }

    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".peerboard").join("identity.key")
}