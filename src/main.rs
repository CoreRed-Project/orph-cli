mod cli;
mod commands;
mod ipc;
mod models;
mod services;

use clap::Parser;
use cli::{Cli, Domain};

fn main() {
    let cli = Cli::parse();

    let db = services::db::init().expect("failed to initialize local database");

    let flags = cli::OutputFlags {
        json: cli.json,
        quiet: cli.quiet,
        verbose: cli.verbose,
    };

    let cmd_name = domain_name(&cli.domain);
    services::logger::info(&format!("command: {}", cmd_name));

    let result = match cli.domain {
        Domain::Sys(cmd) => commands::sys::handle(cmd, &flags),
        Domain::Core(cmd) => commands::core::handle(cmd, &flags),
        Domain::Run(cmd) => commands::run::handle(cmd, &flags),
        Domain::Logs(cmd) => commands::logs::handle(cmd, &flags),
        Domain::Pet(cmd) => commands::pet::handle(cmd, &db, &flags),
        Domain::Cfg(cmd) => commands::cfg::handle(cmd, &db, &flags),
        Domain::Telemetry(cmd) => commands::telemetry::handle(cmd, &db, &flags),
        Domain::Completions(cmd) => commands::completions::handle(cmd, &flags),
    };

    let telemetry_enabled = services::config_service::get(&db, "telemetry")
        .ok()
        .flatten()
        .map(|e| e.value != "disabled")
        .unwrap_or(true);
    if telemetry_enabled {
        let _ = services::telemetry::record(&db, &cmd_name);
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

fn domain_name(domain: &Domain) -> String {
    match domain {
        Domain::Sys(cmd) => format!("sys {}", cmd.subcommand_name()),
        Domain::Core(cmd) => format!("core {}", cmd.subcommand_name()),
        Domain::Run(cmd) => format!("run {}", cmd.subcommand_name()),
        Domain::Logs(cmd) => format!("logs {}", cmd.subcommand_name()),
        Domain::Pet(cmd) => format!("pet {}", cmd.subcommand_name()),
        Domain::Cfg(cmd) => format!("cfg {}", cmd.subcommand_name()),
        Domain::Telemetry(cmd) => format!("telemetry {}", cmd.subcommand_name()),
        Domain::Completions(cmd) => format!("completions {}", cmd.subcommand_name()),
    }
}
