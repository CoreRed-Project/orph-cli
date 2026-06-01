use crate::cli::OutputFlags;
use crate::services::{paths, storage};
use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
pub struct StorageArgs {
    #[command(subcommand)]
    pub cmd: StorageCmd,
}

impl StorageArgs {
    pub fn subcommand_name(&self) -> &'static str {
        match self.cmd {
            StorageCmd::Analyze { .. } => "analyze",
            StorageCmd::Clean { .. } => "clean",
        }
    }
}

#[derive(Subcommand)]
pub enum StorageCmd {
    /// Find heaviest directories under a path (default: ~/.orph)
    Analyze {
        #[arg(default_value = "~/.orph")]
        path: String,
        #[arg(long, default_value = "10")]
        top: usize,
    },
    /// Safe cleanup of orph temp paths (dry-run by default)
    Clean {
        /// Actually delete files
        #[arg(long)]
        apply: bool,
    },
}

pub fn handle(args: StorageArgs, flags: &OutputFlags) -> Result<()> {
    match args.cmd {
        StorageCmd::Analyze { path, top } => {
            let root = expand_path(&path);
            let result = storage::analyze_heavy_dirs(&root, top)?;
            if flags.json {
                println!("{}", serde_json::to_string(&result)?);
                return Ok(());
            }
            println!("storage analyze — {}", result.root);
            if result.heaviest.is_empty() {
                println!("  (no subdirectories)");
            } else {
                for e in &result.heaviest {
                    println!("  {:>10}  {}", storage::format_bytes(e.size_bytes), e.path);
                }
            }
        }
        StorageCmd::Clean { apply } => {
            let dry_run = !apply;
            let result = storage::clean_safe(dry_run)?;
            if flags.json {
                println!("{}", serde_json::to_string(&result)?);
                return Ok(());
            }
            println!(
                "storage clean — {} (freed {})",
                if dry_run { "dry-run" } else { "applied" },
                storage::format_bytes(result.freed_bytes)
            );
            for p in &result.removed {
                println!("  {}", p);
            }
            if dry_run && !result.removed.is_empty() {
                println!("  hint: re-run with `--apply` to delete");
            }
        }
    }
    Ok(())
}

fn expand_path(path: &str) -> PathBuf {
    if path == "~" || path.starts_with("~/") {
        let home = paths::home_dir();
        if path == "~" {
            home
        } else {
            home.join(path.trim_start_matches("~/"))
        }
    } else {
        PathBuf::from(path)
    }
}
