use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "orph",
    about = "⋆｡°✩Offline-first cyberdeck personal CLI: system stats, script runner, config, logs, and a virtual pet companion.\nRun `orph <command> --help` to explore each subsystem.✩°｡⋆",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub domain: Domain,

    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Minimal output
    #[arg(long, short, global = true)]
    pub quiet: bool,

    /// Verbose/debug output
    #[arg(long, short, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Domain {
    /// Show CPU, memory, and disk status — start here
    Sys(crate::commands::sys::SysArgs),
    /// Manage the optional orphd background daemon
    Core(crate::commands::core::CoreArgs),
    /// Run scripts from ~/.orph/scripts/ (auto-created on first use)
    Run(crate::commands::run::RunArgs),
    /// View local command logs
    Logs(crate::commands::logs::LogsArgs),
    /// Your virtual pet companion (default: status)
    Pet(crate::commands::pet::PetArgs),
    /// Get or set persistent config values
    Cfg(crate::commands::cfg::CfgArgs),
    /// Local command usage stats (never leaves your machine)
    Telemetry(crate::commands::telemetry::TelemetryArgs),
    /// Print shell completion scripts
    Completions(crate::commands::completions::CompletionsArgs),
}

pub struct OutputFlags {
    pub json: bool,
    pub quiet: bool,
    pub verbose: bool,
}
