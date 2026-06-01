use std::path::PathBuf;

pub fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

pub fn orph_dir() -> PathBuf {
    home_dir().join(".orph")
}

pub fn scripts_dir() -> PathBuf {
    orph_dir().join("scripts")
}

pub fn log_path() -> PathBuf {
    orph_dir().join("orph.log")
}

pub fn backups_dir() -> PathBuf {
    orph_dir().join("backups")
}
