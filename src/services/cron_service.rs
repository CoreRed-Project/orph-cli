use rusqlite::{Connection, Result};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CronJob {
    pub script_name: String,
    pub interval_secs: u64,
    pub last_run: Option<String>,
    pub enabled: bool,
}

pub fn register(conn: &Connection, script_name: &str, interval_secs: u64) -> Result<()> {
    conn.execute(
        "INSERT INTO cron_jobs (script_name, interval_secs, last_run, enabled)
         VALUES (?1, ?2, NULL, 1)
         ON CONFLICT(script_name) DO UPDATE SET
           interval_secs = excluded.interval_secs,
           enabled = 1",
        rusqlite::params![script_name, interval_secs as i64],
    )?;
    Ok(())
}

pub fn remove(conn: &Connection, script_name: &str) -> Result<bool> {
    let n = conn.execute(
        "DELETE FROM cron_jobs WHERE script_name = ?1",
        rusqlite::params![script_name],
    )?;
    Ok(n > 0)
}

pub fn list(conn: &Connection) -> Result<Vec<CronJob>> {
    let mut stmt = conn.prepare(
        "SELECT script_name, interval_secs, last_run, enabled FROM cron_jobs ORDER BY script_name",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(CronJob {
                script_name: row.get(0)?,
                interval_secs: row.get::<_, i64>(1)? as u64,
                last_run: row.get(2)?,
                enabled: row.get::<_, i64>(3)? != 0,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    Ok(rows)
}
