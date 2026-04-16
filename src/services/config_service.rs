use crate::models::config::ConfigEntry;
use rusqlite::{Connection, Result};

pub fn list(conn: &Connection) -> Result<Vec<ConfigEntry>> {
    let mut stmt = conn.prepare("SELECT key, value FROM config ORDER BY key")?;
    let entries = stmt
        .query_map([], |row| {
            Ok(ConfigEntry {
                key: row.get(0)?,
                value: row.get(1)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;
    Ok(entries)
}

pub fn get(conn: &Connection, key: &str) -> Result<Option<ConfigEntry>> {
    let mut stmt = conn.prepare("SELECT key, value FROM config WHERE key = ?1")?;
    let mut rows = stmt.query_map(rusqlite::params![key], |row| {
        Ok(ConfigEntry {
            key: row.get(0)?,
            value: row.get(1)?,
        })
    })?;
    rows.next().transpose()
}

pub fn set(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO config (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )?;
    Ok(())
}
