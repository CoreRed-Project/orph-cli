use crate::cli::OutputFlags;
use crate::ipc;
use crate::services::config_service;
use clap::{Args, Subcommand};
use rusqlite::Connection;

#[derive(Args)]
pub struct CfgArgs {
    #[command(subcommand)]
    pub cmd: CfgCmd,
}

impl CfgArgs {
    pub fn subcommand_name(&self) -> &'static str {
        match &self.cmd {
            CfgCmd::List => "list",
            CfgCmd::Get { .. } => "get",
            CfgCmd::Set { .. } => "set",
        }
    }
}

#[derive(Subcommand)]
pub enum CfgCmd {
    /// List all config entries
    List,
    /// Get a config value
    Get {
        /// Config key
        key: String,
    },
    /// Set a config value
    Set {
        /// Config key
        key: String,
        /// Config value
        value: String,
    },
}

pub fn handle(args: CfgArgs, conn: &Connection, flags: &OutputFlags) -> anyhow::Result<()> {
    match args.cmd {
        CfgCmd::List => {
            // Try daemon first.
            if let Some(resp) = ipc::send(&ipc::Request {
                command: "cfg.list".into(),
                payload: serde_json::Value::Null,
            }) && resp.is_ok()
            {
                return print_list(resp.data, flags);
            }
            // Fallback: local.
            let entries = config_service::list(conn)?;
            print_list(Some(serde_json::to_value(&entries)?), flags)
        }

        CfgCmd::Get { key } => {
            if let Some(resp) = ipc::send(&ipc::Request {
                command: "cfg.get".into(),
                payload: serde_json::json!({"key": key}),
            }) && resp.is_ok()
            {
                return print_get(resp.data, &key, flags);
            }
            // Fallback.
            match config_service::get(conn, &key)? {
                Some(e) => print_get(Some(serde_json::to_value(&e)?), &key, flags),
                None => print_get(Some(serde_json::Value::Null), &key, flags),
            }
        }

        CfgCmd::Set { key, value } => {
            if let Some(resp) = ipc::send(&ipc::Request {
                command: "cfg.set".into(),
                payload: serde_json::json!({"key": key, "value": value}),
            }) && resp.is_ok()
            {
                return print_set(&key, &value, flags);
            }
            // Fallback.
            config_service::set(conn, &key, &value)?;
            print_set(&key, &value, flags)
        }
    }
}

fn print_list(data: Option<serde_json::Value>, flags: &OutputFlags) -> anyhow::Result<()> {
    if flags.json {
        println!("{}", data.unwrap_or(serde_json::json!([])));
        return Ok(());
    }
    let entries: Vec<crate::models::config::ConfigEntry> =
        serde_json::from_value(data.unwrap_or(serde_json::json!([]))).unwrap_or_default();
    if entries.is_empty() {
        if !flags.quiet {
            println!("no config entries");
        }
    } else {
        for e in &entries {
            println!("  {} = {}", e.key, e.value);
        }
    }
    Ok(())
}

fn print_get(
    data: Option<serde_json::Value>,
    key: &str,
    flags: &OutputFlags,
) -> anyhow::Result<()> {
    match data {
        Some(serde_json::Value::Null) | None => {
            if flags.json {
                println!("null");
            } else {
                anyhow::bail!("key '{}' not found", key);
            }
        }
        Some(v) => {
            if flags.json {
                println!("{}", v);
            } else {
                let e: crate::models::config::ConfigEntry = serde_json::from_value(v)?;
                println!("{} = {}", e.key, e.value);
            }
        }
    }
    Ok(())
}

fn print_set(key: &str, value: &str, flags: &OutputFlags) -> anyhow::Result<()> {
    if flags.json {
        println!(
            "{{\"key\": \"{}\", \"value\": \"{}\", \"ok\": true}}",
            key, value
        );
    } else if !flags.quiet {
        println!("set {} = {}", key, value);
    }
    Ok(())
}
