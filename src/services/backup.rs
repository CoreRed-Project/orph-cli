use crate::services::paths;
use anyhow::{Context, Result, bail};
use flate2::Compression;
use flate2::write::GzEncoder;
use serde::Serialize;
use std::fs::File;
use std::path::{Path, PathBuf};
use tar::{Builder, EntryType};

#[derive(Debug, Serialize)]
pub struct BackupResult {
    pub archive: String,
    pub bytes: u64,
}

pub fn create_backup(output: Option<PathBuf>) -> Result<BackupResult> {
    let orph = paths::orph_dir();
    if !orph.exists() {
        bail!("nothing to backup — {} does not exist", orph.display());
    }

    let archive = output.unwrap_or_else(default_backup_path);
    if let Some(parent) = archive.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = File::create(&archive).with_context(|| archive.display().to_string())?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(enc);
    tar.append_dir_all(".orph", &orph)
        .context("tar append failed")?;
    tar.finish()?;
    let enc = tar.into_inner()?;
    enc.finish()?;

    let bytes = std::fs::metadata(&archive)?.len();
    Ok(BackupResult {
        archive: archive.display().to_string(),
        bytes,
    })
}

fn default_backup_path() -> PathBuf {
    let dir = paths::backups_dir();
    let _ = std::fs::create_dir_all(&dir);
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    dir.join(format!("orph-{stamp}.orphbak"))
}

pub fn restore_backup(archive: &Path) -> Result<()> {
    if !archive.exists() {
        bail!("archive not found: {}", archive.display());
    }

    let orph = paths::orph_dir();
    let parent = orph.parent().context("orph home has no parent")?;
    let file = File::open(archive)?;
    let dec = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(dec);
    tar.unpack(parent)
        .with_context(|| format!("failed to extract into {}", parent.display()))?;
    Ok(())
}

/// List entries in archive (for verification / dry info).
#[allow(dead_code)]
pub fn list_archive(archive: &Path) -> Result<Vec<String>> {
    let file = File::open(archive)?;
    let dec = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(dec);
    let mut names = Vec::new();
    for entry in tar.entries()? {
        let entry = entry?;
        let path = entry.path()?.display().to_string();
        if entry.header().entry_type() != EntryType::Directory {
            names.push(path);
        }
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn roundtrip_backup_restore() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path().join("home");
        unsafe { std::env::set_var("HOME", home.to_str().unwrap()) };
        let orph = paths::orph_dir();
        std::fs::create_dir_all(&orph).unwrap();
        std::fs::write(orph.join("test.txt"), b"orph").unwrap();

        let archive = tmp.path().join("test.orphbak");
        create_backup(Some(archive.clone())).unwrap();
        std::fs::remove_dir_all(&orph).unwrap();
        restore_backup(&archive).unwrap();
        assert!(orph.join("test.txt").exists());
        unsafe { std::env::remove_var("HOME") };
    }
}
