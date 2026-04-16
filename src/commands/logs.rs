use crate::cli::OutputFlags;
use crate::ipc;
use clap::Args;
use std::io::{self, BufRead};
use std::path::PathBuf;

#[derive(Args)]
pub struct LogsArgs {
    /// Stream the last N log entries (like tail -n); omit N to show all
    #[arg(long, short = 'n', value_name = "N")]
    pub tail: Option<usize>,

    /// Follow log output in real-time (like tail -f)
    #[arg(long, short = 'f')]
    pub follow: bool,

    /// Filter by level (info, warn, error)
    #[arg(long)]
    pub level: Option<String>,
}

impl LogsArgs {
    pub fn subcommand_name(&self) -> &'static str {
        if self.follow { "follow" } else { "view" }
    }
}

fn log_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".orph").join("orph.log")
}

pub fn handle(args: LogsArgs, flags: &OutputFlags) -> anyhow::Result<()> {
    // Try daemon first.
    if let Some(resp) = ipc::send(&ipc::Request {
        command: "logs.read".into(),
        payload: serde_json::json!({
            "follow": args.follow,
            "tail":   args.tail,
            "level":  args.level,
        }),
    }) && resp.is_ok()
        && let Some(data) = resp.data
    {
        let lines: Vec<String> = serde_json::from_value(data).unwrap_or_default();
        return print_lines(&lines, flags);
    }

    // Fallback: local.
    let path = log_path();
    if !path.exists() {
        if flags.json {
            println!("[]");
        } else if !flags.quiet {
            println!("no log file found at {}", path.display());
        }
        return Ok(());
    }

    let file = std::fs::File::open(&path)?;
    let reader = io::BufReader::new(file);
    let level_filter = args.level.as_deref().map(|l| l.to_uppercase());

    let lines: Vec<String> = reader
        .lines()
        .map_while(Result::ok)
        .filter(|line| {
            if let Some(ref lvl) = level_filter {
                line.to_uppercase().contains(lvl.as_str())
            } else {
                true
            }
        })
        .collect();

    let output: Vec<String> = if let Some(n) = args.tail {
        let start = lines.len().saturating_sub(n);
        lines[start..].to_vec()
    } else {
        lines
    };

    print_lines(&output, flags)
}

fn print_lines(lines: &[String], flags: &OutputFlags) -> anyhow::Result<()> {
    if flags.json {
        println!("{}", serde_json::to_string(lines)?);
    } else {
        for line in lines {
            println!("{}", line);
        }
    }
    Ok(())
}
