use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BinaryHeap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct DirEntry {
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnalyzeResult {
    pub root: String,
    pub heaviest: Vec<DirEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CleanResult {
    pub dry_run: bool,
    pub removed: Vec<String>,
    pub freed_bytes: u64,
}

struct HeapItem {
    size: u64,
    path: PathBuf,
}

impl PartialEq for HeapItem {
    fn eq(&self, other: &Self) -> bool {
        self.size == other.size
    }
}
impl Eq for HeapItem {}
impl PartialOrd for HeapItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for HeapItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.size.cmp(&other.size)
    }
}

/// Find the heaviest immediate child directories (depth 1) under `root`.
pub fn analyze_heavy_dirs(root: &Path, limit: usize) -> Result<AnalyzeResult> {
    let mut heap = BinaryHeap::new();
    if !root.exists() {
        anyhow::bail!("path not found: {}", root.display());
    }
    for entry in std::fs::read_dir(root).with_context(|| root.display().to_string())? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let size = dir_size(&path)?;
            heap.push(HeapItem { size, path });
        }
    }
    let mut heaviest = Vec::new();
    for _ in 0..limit {
        let Some(item) = heap.pop() else { break };
        heaviest.push(DirEntry {
            path: item.path.display().to_string(),
            size_bytes: item.size,
        });
    }
    heaviest.sort_by_key(|b| std::cmp::Reverse(b.size_bytes));
    Ok(AnalyzeResult {
        root: root.display().to_string(),
        heaviest,
    })
}

fn dir_size(path: &Path) -> Result<u64> {
    let mut total = 0u64;
    if path.is_file() {
        return Ok(path.metadata()?.len());
    }
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            total = total.saturating_add(dir_size(&p)?);
        } else if let Ok(m) = p.metadata() {
            total = total.saturating_add(m.len());
        }
    }
    Ok(total)
}

/// Safe cleanup: only paths under `~/.orph/tmp` and stale `orph-*` in system temp.
pub fn clean_safe(dry_run: bool) -> Result<CleanResult> {
    let mut removed = Vec::new();
    let mut freed = 0u64;

    let orph_tmp = crate::services::paths::orph_dir().join("tmp");
    if orph_tmp.exists() {
        for entry in std::fs::read_dir(&orph_tmp)? {
            let entry = entry?;
            let p = entry.path();
            let bytes = if p.is_file() {
                p.metadata().map(|m| m.len()).unwrap_or(0)
            } else {
                dir_size(&p).unwrap_or(0)
            };
            if dry_run {
                removed.push(format!("{} ({} bytes)", p.display(), bytes));
            } else if p.is_file() {
                std::fs::remove_file(&p)?;
                removed.push(p.display().to_string());
                freed += bytes;
            } else if p.is_dir() {
                std::fs::remove_dir_all(&p)?;
                removed.push(p.display().to_string());
                freed += bytes;
            }
        }
    }

    let tmp = std::env::temp_dir();
    if let Ok(entries) = std::fs::read_dir(&tmp) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with("orph-") {
                continue;
            }
            let p = entry.path();
            let bytes = dir_size(&p).unwrap_or(0);
            if dry_run {
                removed.push(format!("{} ({} bytes)", p.display(), bytes));
            } else if p.is_file() {
                std::fs::remove_file(&p)?;
                removed.push(p.display().to_string());
                freed += bytes;
            } else if p.is_dir() {
                std::fs::remove_dir_all(&p)?;
                removed.push(p.display().to_string());
                freed += bytes;
            }
        }
    }

    Ok(CleanResult {
        dry_run,
        removed,
        freed_bytes: freed,
    })
}

pub fn format_bytes(n: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if n >= GB {
        format!("{:.1} GB", n as f64 / GB as f64)
    } else if n >= MB {
        format!("{:.1} MB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.1} KB", n as f64 / KB as f64)
    } else {
        format!("{} B", n)
    }
}
