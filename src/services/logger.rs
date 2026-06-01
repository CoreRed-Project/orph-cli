use chrono::Utc;
use std::fs::OpenOptions;
use std::io::Write;

pub fn write(level: &str, message: &str) {
    let path = crate::services::paths::log_path();
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
