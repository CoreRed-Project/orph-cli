use crate::cli::OutputFlags;
use clap::{Args, CommandFactory};
use clap_complete::{Shell, generate};

#[derive(Args)]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    pub shell: Shell,
}

impl CompletionsArgs {
    pub fn subcommand_name(&self) -> &'static str {
        "completions"
    }
}

pub fn handle(args: CompletionsArgs, _flags: &OutputFlags) -> anyhow::Result<()> {
    let mut cmd = crate::cli::Cli::command();
    generate(args.shell, &mut cmd, "orph", &mut std::io::stdout());
    Ok(())
}
