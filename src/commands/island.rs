use crate::cli::OutputFlags;
use crate::tui;
use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct IslandArgs {}

impl IslandArgs {
    pub fn subcommand_name(&self) -> &'static str {
        "open"
    }
}

pub fn handle(_args: IslandArgs, _flags: &OutputFlags) -> Result<()> {
    tui::island::run(_flags)
}
