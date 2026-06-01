use crate::cli::OutputFlags;
use crate::ipc;
use crate::services::backup;
use anyhow::Result;
use clap::{Args, Subcommand};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;

#[derive(Args)]
pub struct CoreArgs {
    #[command(subcommand)]
    pub cmd: CoreCmd,
}

impl CoreArgs {
    pub fn subcommand_name(&self) -> &'static str {
        match self.cmd {
            CoreCmd::Status => "status",
            CoreCmd::Start => "start",
            CoreCmd::Stop => "stop",
            CoreCmd::Backup { .. } => "backup",
            CoreCmd::Restore { .. } => "restore",
        }
    }
}

#[derive(Subcommand)]
pub enum CoreCmd {
    /// Show daemon status
    Status,
    /// Start the orphd daemon in the background
    Start,
    /// Stop the orphd daemon
    Stop,
    /// Bundle ~/.orph/ into a compressed snapshot
    Backup {
        /// Output path (default: ~/.orph/backups/orph-<timestamp>.orphbak)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Restore ~/.orph/ from a snapshot archive
    Restore {
        /// Path to .orphbak archive
        archive: PathBuf,
    },
}

pub fn handle(args: CoreArgs, flags: &OutputFlags) -> Result<()> {
    match args.cmd {
        CoreCmd::Status => {
            status(flags);
            Ok(())
        }
        CoreCmd::Start => {
            start(flags);
        }
        CoreCmd::Stop => {
            stop(flags);
        }
        CoreCmd::Backup { output } => backup_cmd(output, flags),
        CoreCmd::Restore { archive } => restore_cmd(archive, flags),
    }
}

fn backup_cmd(output: Option<PathBuf>, flags: &OutputFlags) -> Result<()> {
    let result = backup::create_backup(output)?;
    if flags.json {
        println!("{}", serde_json::to_string(&result)?);
    } else if !flags.quiet {
        println!("backup created: {}", result.archive);
        println!("  size: {} bytes", result.bytes);
    }
    Ok(())
}

fn restore_cmd(archive: PathBuf, flags: &OutputFlags) -> Result<()> {
    backup::restore_backup(&archive)?;
    if flags.json {
        println!(
            "{{\"restored\": true, \"archive\": \"{}\"}}",
            archive.display()
        );
    } else if !flags.quiet {
        println!("restored from {}", archive.display());
    }
    Ok(())
}

fn status(flags: &OutputFlags) {
    let running = ipc::ping();
    let state = if running { "running" } else { "offline" };
    let version = env!("CARGO_PKG_VERSION");

    if flags.json {
        println!(
            "{{\"daemon\": \"orphd\", \"status\": \"{}\", \"socket\": \"{}\", \"version\": \"{}\"}}",
            state,
            ipc::socket_path_display(),
            version
        );
    } else if flags.quiet {
        println!("orphd={}", state);
    } else {
        println!("core status");
        println!("  daemon  : orphd");
        println!("  status  : {}", state);
        println!("  socket  : {}", ipc::socket_path_display());
        println!("  version : {}", version);
    }
}

fn start(flags: &OutputFlags) -> ! {
    // Check if already running
    if ipc::ping() {
        if flags.json {
            println!("{{\"daemon\": \"orphd\", \"status\": \"already_running\"}}");
        } else {
            println!("orphd is already running");
        }
        std::process::exit(0);
    }

    // Resolve orphd binary path: same directory as the current executable.
    // Fallback: search PATH via `which orphd`.
    let orphd_path = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("orphd")))
        .filter(|p| p.exists())
        .or_else(|| {
            std::process::Command::new("which")
                .arg("orphd")
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| {
                    String::from_utf8(o.stdout)
                        .ok()
                        .map(|s| std::path::PathBuf::from(s.trim()))
                })
        })
        .unwrap_or_else(|| std::path::PathBuf::from("orphd"));

    if !orphd_path.exists() {
        if !flags.json {
            eprintln!("error: orphd binary not found at {}", orphd_path.display());
            eprintln!("  hint: ensure both `orph` and `orphd` are installed in the same directory");
            eprintln!(
                "        run `make install` or copy both binaries to the same location on $PATH"
            );
        } else {
            eprintln!("{{\"error\": \"orphd binary not found\"}}");
        }
        std::process::exit(1);
    }

    let child = std::process::Command::new(&orphd_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .spawn();

    match child {
        Ok(_) => {
            // Give daemon a moment to bind its socket.
            std::thread::sleep(std::time::Duration::from_millis(300));
            let ok = ipc::ping();
            if flags.json {
                println!(
                    "{{\"daemon\": \"orphd\", \"status\": \"{}\"}}",
                    if ok { "started" } else { "start_failed" }
                );
            } else if ok {
                println!("orphd started");
            } else {
                eprintln!("orphd may not have started correctly");
                std::process::exit(1);
            }
        }
        Err(e) => {
            if flags.json {
                eprintln!("{{\"error\": \"{}\"}}", e);
            } else {
                eprintln!("error: failed to start orphd: {}", e);
            }
            std::process::exit(1);
        }
    }
    std::process::exit(0);
}

fn stop(flags: &OutputFlags) -> ! {
    if !ipc::ping() {
        if flags.json {
            println!("{{\"daemon\": \"orphd\", \"status\": \"not_running\"}}");
        } else {
            println!("orphd is not running");
        }
        std::process::exit(0);
    }

    // Send a shutdown request (best-effort; daemon exits on SIGTERM).
    // We kill by finding the PID from the socket, but simplest is:
    // send a special "shutdown" command.
    let req = ipc::Request {
        command: "shutdown".into(),
        payload: serde_json::Value::Null,
    };
    let _ = ipc::send(&req);

    // Give it a moment to exit.
    std::thread::sleep(std::time::Duration::from_millis(300));

    let still_up = ipc::ping();
    if flags.json {
        println!(
            "{{\"daemon\": \"orphd\", \"status\": \"{}\"}}",
            if still_up { "stop_failed" } else { "stopped" }
        );
    } else if still_up {
        eprintln!("orphd may still be running");
        std::process::exit(1);
    } else {
        println!("orphd stopped");
    }
    std::process::exit(0);
}
