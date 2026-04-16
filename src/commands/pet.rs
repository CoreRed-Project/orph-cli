use crate::cli::OutputFlags;
use crate::ipc;
use crate::services::{logger, pet_service};
use clap::{Args, Subcommand};
use rusqlite::Connection;

#[derive(Args)]
pub struct PetArgs {
    #[command(subcommand)]
    pub cmd: Option<PetCmd>,
}

impl PetArgs {
    pub fn subcommand_name(&self) -> &'static str {
        match &self.cmd {
            None | Some(PetCmd::Status) => "status",
            Some(PetCmd::Feed) => "feed",
            Some(PetCmd::Play) => "play",
            Some(PetCmd::Rename { .. }) => "rename",
        }
    }
}

#[derive(Subcommand)]
pub enum PetCmd {
    /// Show pet status
    Status,
    /// Feed your pet
    Feed,
    /// Play with your pet
    Play,
    /// Rename your pet
    Rename {
        /// New name
        name: String,
    },
}

pub fn handle(args: PetArgs, conn: &Connection, flags: &OutputFlags) -> anyhow::Result<()> {
    let cmd = args.cmd.unwrap_or(PetCmd::Status);
    let ipc_cmd = match &cmd {
        PetCmd::Status => Some("pet.status"),
        PetCmd::Feed => Some("pet.feed"),
        PetCmd::Play => Some("pet.play"),
        PetCmd::Rename { .. } => None,
    };

    // Try daemon first for supported commands.
    if let Some(ipc_cmd_str) = ipc_cmd {
        if let Some(resp) = ipc::send(&ipc::Request {
            command: ipc_cmd_str.into(),
            payload: serde_json::Value::Null,
        }) {
            if resp.is_ok() {
                if let Some(data) = resp.data {
                    let pet: crate::models::pet::Pet = serde_json::from_value(data)?;
                    let action = match &cmd {
                        PetCmd::Feed => Some("feed"),
                        PetCmd::Play => Some("play"),
                        _ => None,
                    };
                    print_pet(&pet, flags, action)?;
                    return Ok(());
                }
            }
        }
        // Daemon not available — running in local fallback mode.
        if !flags.quiet && !flags.json {
            eprintln!("  [daemon offline — running in local fallback mode]");
        }
    }

    // Fallback: local execution.
    match cmd {
        PetCmd::Status => {
            let pet = pet_service::get(conn)?;
            print_pet(&pet, flags, None)?;
        }
        PetCmd::Feed => {
            let pet = pet_service::feed(conn)?;
            logger::info(&format!(
                "pet '{}' fed (hunger={}, happiness={})",
                pet.name, pet.hunger, pet.happiness
            ));
            print_pet(&pet, flags, Some("feed"))?;
        }
        PetCmd::Play => {
            let pet = pet_service::play(conn)?;
            logger::info(&format!(
                "pet '{}' played (happiness={})",
                pet.name, pet.happiness
            ));
            print_pet(&pet, flags, Some("play"))?;
        }
        PetCmd::Rename { name } => {
            let pet = pet_service::rename(conn, &name)?;
            logger::info(&format!("pet renamed to '{}'", pet.name));
            if flags.json {
                println!("{}", serde_json::to_string(&pet)?);
            } else {
                println!("⋆｡°✩ your pet is now called {} ✩°｡⋆", pet.name);
            }
        }
    }
    Ok(())
}

fn print_pet(
    pet: &crate::models::pet::Pet,
    flags: &OutputFlags,
    action: Option<&str>,
) -> anyhow::Result<()> {
    if flags.json {
        println!("{}", serde_json::to_string(pet)?);
        return Ok(());
    }

    let mood = pet.mood();
    let (icon, text) = match (action, mood) {
        (Some("feed"), _) => (
            "ฅ^•ﻌ•^ฅ",
            format!("{} just ate and feels better~", pet.name),
        ),
        (Some("play"), _) => ("(≧◡≦)", format!("{} had so much fun!! ✧", pet.name)),
        (_, "happy") => ("⋆｡°✩", format!("your pet {} feels happy ✩°｡⋆", pet.name)),
        (_, "hungry") => ("૮₍ ˶•⤙•˶ ₎ა", format!("{} is hungry...", pet.name)),
        (_, "sad") => ("(╥_╥)", format!("{} feels lonely...", pet.name)),
        _ => ("(◕‿◕✿)", format!("{} is doing okay~", pet.name)),
    };

    if flags.quiet {
        println!(
            "{} {} | hunger={} happiness={}",
            icon, pet.name, pet.hunger, pet.happiness
        );
        return Ok(());
    }

    println!("{} {}", icon, text);
    println!("  name      : {}", pet.name);
    println!("  hunger    : {}/100", pet.hunger);
    println!("  happiness : {}/100", pet.happiness);
    println!("  mood      : {}", mood);
    println!("  last fed  : {}", pet.last_fed);
    println!("  last play : {}", pet.last_played);
    Ok(())
}
