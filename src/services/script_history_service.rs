use chrono::Utc;
use rusqlite::{Connection, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ScriptHistoryEntry {
    pub id: i64,
    pub script_name: String,
    pub exit_code: i32,
    pub timed_out: bool,
    pub elapsed_ms: u64,
    pub stdout: String,
    pub stderr: String,
    pub started_at: String,
}

/// Records a script run into the database and enforces a maximum of 50 log records.
pub fn record_run(
    conn: &Connection,
    script_name: &str,
    exit_code: i32,
    timed_out: bool,
    elapsed_ms: u64,
    stdout: &str,
    stderr: &str,
) -> Result<()> {
    let started_at = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO script_history (script_name, exit_code, timed_out, elapsed_ms, stdout, stderr, started_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            script_name,
            exit_code,
            if timed_out { 1 } else { 0 },
            elapsed_ms as i64,
            stdout,
            stderr,
            started_at
        ],
    )?;

    // Prune script history to keep only the last 50 entries
    let _ = conn.execute(
        "DELETE FROM script_history WHERE id NOT IN (
            SELECT id FROM script_history ORDER BY id DESC LIMIT 50
        )",
        [],
    );

    Ok(())
}

/// Lists recent script execution records.
pub fn list_recent(conn: &Connection, limit: usize) -> Result<Vec<ScriptHistoryEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, script_name, exit_code, timed_out, elapsed_ms, stdout, stderr, started_at
         FROM script_history ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(rusqlite::params![limit as i64], |row| {
            Ok(ScriptHistoryEntry {
                id: row.get(0)?,
                script_name: row.get(1)?,
                exit_code: row.get(2)?,
                timed_out: row.get::<_, i32>(3)? != 0,
                elapsed_ms: row.get::<_, i64>(4)? as u64,
                stdout: row.get(5)?,
                stderr: row.get(6)?,
                started_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    Ok(rows)
}
