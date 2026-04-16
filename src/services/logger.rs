use chrono::Utc;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

fn log_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".orph").join("orph.log")
}

pub fn write(level: &str, message: &str) {
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let timestamp = Utc::now().to_rfc3339();
    let line = format!("{} [{}] {}\n", timestamp, level, message);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = file.write_all(line.as_bytes());
    }
}

pub fn info(message: &str) {
    write("INFO", message);
}

pub fn error(message: &str) {
    write("ERROR", message);
}
