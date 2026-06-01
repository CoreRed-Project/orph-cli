use chrono::Utc;
use rusqlite::{Connection, Result};
use serde::Serialize;

const EVENTS: &[&str] = &[
    "found a shiny cable in the logs ✧",
    "chased a rogue process across /tmp",
    "napped on the warm CPU heatsink",
    "discovered a forgotten config key",
    "guarded the SQLite database fiercely",
    "heard whispers from wlan0",
    "collected three interesting log lines",
    "purred at stable 5V power",
];

#[derive(Debug, Clone, Serialize)]
pub struct PetEventRecord {
    pub id: i64,
    pub message: String,
    pub created_at: String,
}

pub fn maybe_random(conn: &Connection) -> Result<Option<PetEventRecord>> {
    // ~12% chance per eligible interaction
    let roll: u8 = Utc::now().timestamp_nanos_opt().unwrap_or(0) as u8;
    if roll % 8 != 0 {
        return Ok(None);
    }
    record_random(conn).map(Some)
}

pub fn record_random(conn: &Connection) -> Result<PetEventRecord> {
    let idx = (Utc::now().timestamp() as usize) % EVENTS.len();
    record(conn, EVENTS[idx])
}

pub fn record(conn: &Connection, message: &str) -> Result<PetEventRecord> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO pet_events (message, created_at) VALUES (?1, ?2)",
        rusqlite::params![message, now],
    )?;
    let id = conn.last_insert_rowid();
    Ok(PetEventRecord {
        id,
        message: message.to_string(),
        created_at: now,
    })
}
