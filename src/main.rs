mod cli;
mod commands;
mod ipc;
mod models;
mod services;
mod tui;

use clap::Parser;
use cli::{Cli, Domain};

fn main() {
    let cli = Cli::parse();

    let flags = cli::OutputFlags {
        json: cli.json,
        quiet: cli.quiet,
        verbose: cli.verbose,
    };

    let cmd_name = domain_name(&cli.domain);
    services::logger::info(&format!("command: {}", cmd_name));

    let mut db = None;
    let result: anyhow::Result<()> = match cli.domain {
        Domain::Sys(cmd) => commands::sys::handle(cmd, &flags),
        Domain::Health(cmd) => commands::health::handle(cmd, &flags),
        Domain::Storage(cmd) => commands::storage::handle(cmd, &flags),
        Domain::Core(cmd) => commands::core::handle(cmd, &flags),
        Domain::Island(cmd) => commands::island::handle(cmd, &flags),
        Domain::Run(cmd) => with_db(&mut db, |conn| commands::run::handle(cmd, &flags, conn)),
        Domain::Logs(cmd) => commands::logs::handle(cmd, &flags),
        Domain::Pet(cmd) => with_db(&mut db, |conn| commands::pet::handle(cmd, conn, &flags)),
        Domain::Cfg(cmd) => with_db(&mut db, |conn| commands::cfg::handle(cmd, conn, &flags)),
        Domain::Telemetry(cmd) => with_db(&mut db, |conn| {
            commands::telemetry::handle(cmd, conn, &flags)
        }),
        Domain::Completions(cmd) => commands::completions::handle(cmd, &flags),
    };

    if result.is_ok() {
        if db.is_none() {
            db = services::db::init().ok();
        }
        if let Some(conn) = db.as_ref() {
            let telemetry_enabled = services::config_service::get(conn, "telemetry")
                .ok()
                .flatten()
                .map(|e| e.value != "disabled")
                .unwrap_or(true);
            if telemetry_enabled {
                let _ = services::telemetry::record(conn, &cmd_name);
            }
        }
    }

    if let Err(e) = result {
        services::logger::error(&format!("command '{}' failed: {}", cmd_name, e));
        if flags.json {
            eprintln!("{{\"error\": \"{}\"}}", e);
        } else {
            eprintln!("error: {}", e);
        }
        std::process::exit(1);
    }
}

fn with_db<F>(db: &mut Option<rusqlite::Connection>, f: F) -> anyhow::Result<()>
where
    F: FnOnce(&rusqlite::Connection) -> anyhow::Result<()>,
{
    let conn = services::db::init()
        .map_err(|e| anyhow::anyhow!("failed to initialize local database: {}", e))?;
    let result = f(&conn);
    *db = Some(conn);
    result
}

fn domain_name(domain: &Domain) -> String {
    match domain {
        Domain::Sys(cmd) => format!("sys {}", cmd.subcommand_name()),
        Domain::Health(cmd) => format!("health {}", cmd.subcommand_name()),
        Domain::Storage(cmd) => format!("storage {}", cmd.subcommand_name()),
        Domain::Core(cmd) => format!("core {}", cmd.subcommand_name()),
        Domain::Island(cmd) => format!("island {}", cmd.subcommand_name()),
        Domain::Run(cmd) => format!("run {}", cmd.subcommand_name()),
        Domain::Logs(cmd) => format!("logs {}", cmd.subcommand_name()),
        Domain::Pet(cmd) => format!("pet {}", cmd.subcommand_name()),
        Domain::Cfg(cmd) => format!("cfg {}", cmd.subcommand_name()),
        Domain::Telemetry(cmd) => format!("telemetry {}", cmd.subcommand_name()),
        Domain::Completions(cmd) => format!("completions {}", cmd.subcommand_name()),
    }
}
