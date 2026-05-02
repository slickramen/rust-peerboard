use rusqlite::{Connection, params};
use std::error::Error;
use crate::peerboard::PeerBoardMessage;

pub fn setup_db(conn: &Connection) -> Result<(), Box<dyn Error>> {
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

pub fn store_message(conn: &Connection, msg: &PeerBoardMessage) -> Result<bool, Box<dyn Error>> {
    let rows = conn.execute(
        "INSERT OR IGNORE INTO messages (message_id, peer_id, nickname, content, topic, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![msg.message_id, msg.peer_id, msg.nickname, msg.content, msg.topic, msg.timestamp],
    )?;
    Ok(rows > 0)
}

pub fn load_messages(conn: &Connection, topic: &str) -> Result<Vec<PeerBoardMessage>, Box<dyn Error>> {
    let mut stmt = conn.prepare(
        "SELECT message_id, peer_id, nickname, content, topic, timestamp
         FROM messages WHERE topic = ?1 ORDER BY timestamp ASC"
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

pub fn store_topic(conn: &Connection, topic: &str) -> Result<(), Box<dyn Error>> {
    conn.execute("INSERT OR IGNORE INTO topics (topic) VALUES (?1)", params![topic])?;
    Ok(())
}

pub fn remove_topic(conn: &Connection, topic: &str) -> Result<(), Box<dyn Error>> {
    conn.execute("DELETE FROM topics WHERE topic = ?1", params![topic])?;
    Ok(())
}

pub fn load_topics(conn: &Connection) -> Result<Vec<String>, Box<dyn Error>> {
    let mut stmt = conn.prepare("SELECT topic FROM topics")?;
    let topics = stmt.query_map([], |row| row.get::<_, String>(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(topics)
}