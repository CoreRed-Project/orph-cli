use chrono::Utc;
use rusqlite::{Connection, Result};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

static IS_READ_ONLY: AtomicBool = AtomicBool::new(false);

pub fn is_read_only() -> bool {
    IS_READ_ONLY.load(Ordering::Relaxed)
}

pub fn db_path() -> PathBuf {
    let base = dirs_home();
    base.join(".orph").join("orph.db")
}

fn dirs_home() -> PathBuf {
    if let Ok(home) = std::env::var("ORPH_HOME") {
        return PathBuf::from(home);
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home);
    }
    std::env::temp_dir().join("orph")
}

pub fn init() -> Result<Connection> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    match Connection::open(&path) {
        Ok(conn) => {
            if create_tables(&conn).is_err() {
                IS_READ_ONLY.store(true, Ordering::Relaxed);
                let mem_conn = Connection::open_in_memory()?;
                create_tables(&mem_conn)?;
                Ok(mem_conn)
            } else {
                Ok(conn)
            }
        }
        Err(_) => {
            IS_READ_ONLY.store(true, Ordering::Relaxed);
            let mem_conn = Connection::open_in_memory()?;
            create_tables(&mem_conn)?;
            Ok(mem_conn)
        }
    }
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

        CREATE TABLE IF NOT EXISTS cron_jobs (
            script_name TEXT PRIMARY KEY,
            interval_secs INTEGER NOT NULL,
            last_run TEXT,
            enabled INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE IF NOT EXISTS pet_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS script_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            script_name TEXT NOT NULL,
            exit_code INTEGER NOT NULL,
            timed_out INTEGER NOT NULL,
            elapsed_ms INTEGER NOT NULL,
            stdout TEXT NOT NULL,
            stderr TEXT NOT NULL,
            started_at TEXT NOT NULL
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

    // Create telemetry command index
    let _ = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_telemetry_command ON telemetry(command)",
        [],
    );

    // Create script history index
    let _ = conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_script_history_name ON script_history(script_name)",
        [],
    );

    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageStats {
    pub configs: i64,
    pub telemetry: i64,
    pub cron_jobs: i64,
    pub scripts: i64,
}

pub fn get_storage_stats(conn: &Connection) -> Result<StorageStats> {
    let configs: i64 = conn
        .query_row("SELECT COUNT(*) FROM config", [], |r| r.get(0))
        .unwrap_or(0);
    let telemetry: i64 = conn
        .query_row("SELECT COUNT(*) FROM telemetry", [], |r| r.get(0))
        .unwrap_or(0);
    let cron_jobs: i64 = conn
        .query_row("SELECT COUNT(*) FROM cron_jobs", [], |r| r.get(0))
        .unwrap_or(0);

    let scripts_dir = crate::services::paths::scripts_dir();
    let scripts = std::fs::read_dir(scripts_dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .count() as i64
        })
        .unwrap_or(0);

    Ok(StorageStats {
        configs,
        telemetry,
        cron_jobs,
        scripts,
    })
}
