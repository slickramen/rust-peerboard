use std::error::Error;
use libp2p::PeerId;
use chrono::Utc;
use uuid::Uuid;
use prost::Message as ProstMessage;
use crate::peerboard::PeerBoardMessage;

pub fn build_message(
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

pub fn validate_message(msg: &PeerBoardMessage) -> bool {
    if msg.content.as_bytes().len() > 4096 { return false; }
    if msg.nickname.as_bytes().len() > 32 { return false; }
    if !msg.topic.starts_with("peerboard/v1/") { return false; }
    let now = Utc::now().timestamp();
    if msg.timestamp > now + 300 { return false; }
    true
}

pub fn decode_message(data: &[u8]) -> Result<PeerBoardMessage, prost::DecodeError> {
    PeerBoardMessage::decode(data)
}