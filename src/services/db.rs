use chrono::Utc;
use rusqlite::{Connection, Result};
use std::path::PathBuf;

pub fn db_path() -> PathBuf {
    let base = dirs_home();
    base.join(".orph").join("orph.db")
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn init() -> Result<Connection> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let conn = Connection::open(&path)?;
    create_tables(&conn)?;
    Ok(conn)
}

fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS pet (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            hunger INTEGER NOT NULL DEFAULT 30,
            happiness INTEGER NOT NULL DEFAULT 70,
            last_fed TEXT NOT NULL,
            last_played TEXT NOT NULL,
            last_updated TEXT NOT NULL DEFAULT ''
        );

        CREATE TABLE IF NOT EXISTS config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS telemetry (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            command TEXT NOT NULL,
            timestamp TEXT NOT NULL
        );
        ",
    )?;

    // Migrate: add last_updated column if it doesn't exist yet
    let _ = conn.execute(
        "ALTER TABLE pet ADD COLUMN last_updated TEXT NOT NULL DEFAULT ''",
        [],
    );

    // Seed default pet if not exists
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT OR IGNORE INTO pet (id, name, hunger, happiness, last_fed, last_played, last_updated)
         VALUES (1, 'Bit', 30, 70, ?1, ?2, ?3)",
        rusqlite::params![now, now, now],
    )?;

    // Backfill last_updated for existing rows that have empty string
    conn.execute(
        "UPDATE pet SET last_updated = last_fed WHERE last_updated = ''",
        [],
    )?;

    Ok(())
}
