use crate::cli::OutputFlags;
use crate::services::{config_service, telemetry};
use anyhow::Result;
use clap::{Args, Subcommand};
use rusqlite::Connection;

#[derive(Args)]
pub struct TelemetryArgs {
    #[command(subcommand)]
    pub cmd: Option<TelemetryCmd>,
}

impl TelemetryArgs {
    pub fn subcommand_name(&self) -> &'static str {
        match &self.cmd {
            None | Some(TelemetryCmd::List) => "list",
            Some(TelemetryCmd::Top) => "top",
        }
    }
}

#[derive(Subcommand)]
pub enum TelemetryCmd {
    /// List recent command executions (default)
    List,
    /// Show most used commands
    Top,
}

pub fn handle(args: TelemetryArgs, conn: &Connection, flags: &OutputFlags) -> Result<()> {
    // Check if telemetry is disabled and surface it clearly.
    let disabled = config_service::get(conn, "telemetry")
        .ok()
        .flatten()
        .map(|e| e.value == "disabled")
        .unwrap_or(false);

    if disabled {
        if flags.json {
            println!("{{\"status\": \"disabled\"}}");
            return Ok(());
        } else if !flags.quiet {
            println!("telemetry is disabled  (run `orph cfg set telemetry enabled` to re-enable)");
            return Ok(());
        }
    }

    match args.cmd.unwrap_or(TelemetryCmd::List) {
        TelemetryCmd::List => list(conn, flags),
        TelemetryCmd::Top => top(conn, flags),
    }
}

fn list(conn: &Connection, flags: &OutputFlags) -> Result<()> {
    let entries = telemetry::list_recent(conn, 50)?;

    if flags.json {
        println!("{}", serde_json::to_string(&entries)?);
        return Ok(());
    }

    if entries.is_empty() {
        if !flags.quiet {
            println!("no telemetry recorded yet");
        }
        return Ok(());
    }

    if !flags.quiet {
        println!("recent commands ({} entries):", entries.len());
    }
    for e in &entries {
        println!("  {}  {}", e.timestamp, e.command);
    }
    Ok(())
}

fn top(conn: &Connection, flags: &OutputFlags) -> Result<()> {
    let rows = telemetry::top_commands(conn, 20)?;

    if flags.json {
        println!("{}", serde_json::to_string(&rows)?);
        return Ok(());
    }

    if rows.is_empty() {
        if !flags.quiet {
            println!("no telemetry recorded yet");
        }
        return Ok(());
    }

    println!("top commands:");
    for r in &rows {
        println!("  {:>4}x  {}", r.count, r.command);
    }
    Ok(())
}
