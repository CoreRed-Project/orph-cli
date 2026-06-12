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

    // Retrieve limit configuration (telemetry_limit), fallback to 1000.
    let limit: i64 = match crate::services::config_service::get(conn, "telemetry_limit") {
        Ok(Some(entry)) => entry.value.parse().unwrap_or(1000),
        _ => 1000,
    };

    // Keep only the most recent N records.
    conn.execute(
        "DELETE FROM telemetry WHERE id NOT IN (
            SELECT id FROM telemetry ORDER BY id DESC LIMIT ?1
        )",
        rusqlite::params![limit],
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_fifo_limit() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        // Run migrations
        conn.execute_batch(
            "CREATE TABLE config (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             CREATE TABLE telemetry (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 command TEXT NOT NULL,
                 timestamp TEXT NOT NULL
             );",
        )?;

        // Test default limit is applied (but let's set a low custom limit so we don't have to write 1000 records)
        crate::services::config_service::set(&conn, "telemetry_limit", "3")?;

        record(&conn, "cmd1")?;
        record(&conn, "cmd2")?;
        record(&conn, "cmd3")?;
        record(&conn, "cmd4")?;

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM telemetry", [], |r| r.get(0))?;
        assert_eq!(count, 3);

        // Verify it kept the last 3 (cmd2, cmd3, cmd4)
        let mut stmt = conn.prepare("SELECT command FROM telemetry ORDER BY id ASC")?;
        let cmds = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(cmds, vec!["cmd2", "cmd3", "cmd4"]);

        Ok(())
    }
}
