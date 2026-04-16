use chrono::Utc;
use rusqlite::{Connection, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct TelemetryEntry {
    pub id: i64,
    pub command: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct CommandCount {
    pub command: String,
    pub count: i64,
}

pub fn record(conn: &Connection, command: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO telemetry (command, timestamp) VALUES (?1, ?2)",
        rusqlite::params![command, now],
    )?;
    Ok(())
}

/// Returns the N most recent telemetry entries.
pub fn list_recent(conn: &Connection, limit: i64) -> Result<Vec<TelemetryEntry>> {
    let mut stmt =
        conn.prepare("SELECT id, command, timestamp FROM telemetry ORDER BY id DESC LIMIT ?1")?;
    let entries = stmt
        .query_map(rusqlite::params![limit], |row| {
            Ok(TelemetryEntry {
                id: row.get(0)?,
                command: row.get(1)?,
                timestamp: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    Ok(entries)
}

/// Returns commands ordered by usage frequency.
pub fn top_commands(conn: &Connection, limit: i64) -> Result<Vec<CommandCount>> {
    let mut stmt = conn.prepare(
        "SELECT command, COUNT(*) as count FROM telemetry GROUP BY command ORDER BY count DESC LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![limit], |row| {
            Ok(CommandCount {
                command: row.get(0)?,
                count: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    Ok(rows)
}
