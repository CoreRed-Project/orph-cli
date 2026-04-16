use crate::cli::OutputFlags;
use crate::services::logger;
use anyhow::{Result, anyhow, bail};
use clap::{Args, Subcommand};
use serde::Serialize;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

#[derive(Serialize)]
struct ScriptInfo {
    name: String,
    description: String,
    path: String,
}

#[derive(Args)]
pub struct RunArgs {
    #[command(subcommand)]
    pub cmd: RunCmd,
}

impl RunArgs {
    pub fn subcommand_name(&self) -> &'static str {
        match &self.cmd {
            RunCmd::List => "list",
            RunCmd::Script { .. } => "script",
        }
    }
}

#[derive(Subcommand)]
pub enum RunCmd {
    /// List available scripts in ~/.orph/scripts/
    List,
    /// Run a script by name: `orph run <script-name> [args...] [--timeout <secs>]`
    #[command(external_subcommand)]
    Script(Vec<String>),
}

#[derive(Serialize)]
struct ScriptResult {
    script: String,
    exit_code: i32,
    timed_out: bool,
    elapsed_ms: u64,
    stdout: String,
    stderr: String,
}

fn scripts_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".orph").join("scripts")
}

/// Safety: reject names with path separators to prevent directory traversal.
fn validate_script_name(name: &str) -> Result<()> {
    if name.contains('/') || name.contains('\\') || name.starts_with('.') {
        bail!(
            "invalid script name '{}': only plain filenames are allowed",
            name
        );
    }
    Ok(())
}

/// (timeout_secs, json_override, quiet_override, verbose_override, cleaned_args)
type RunFlags = (Option<u64>, bool, bool, bool, Vec<String>);

/// Extract --timeout, --json, --quiet, --verbose from args vec.
/// Returns (timeout, json_override, quiet_override, verbose_override, cleaned_args).
fn extract_run_flags(parts: &[String]) -> Result<RunFlags> {
    let mut timeout = None;
    let mut json = false;
    let mut quiet = false;
    let mut verbose = false;
    let mut rest: Vec<String> = Vec::new();
    let mut iter = parts.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--timeout" => {
                if let Some(val) = iter.next() {
                    timeout = Some(val.parse::<u64>().map_err(|_| {
                        anyhow!(
                            "--timeout must be a positive integer (seconds), got '{}'",
                            val
                        )
                    })?);
                }
            }
            "--json" => json = true,
            "--quiet" | "-q" => quiet = true,
            "--verbose" | "-v" => verbose = true,
            _ => rest.push(arg.clone()),
        }
    }
    Ok((timeout, json, quiet, verbose, rest))
}

pub fn handle(args: RunArgs, flags: &OutputFlags) -> Result<()> {
    match args.cmd {
        RunCmd::List => list_scripts(flags),
        RunCmd::Script(parts) => run_script(&parts, flags),
    }
}

fn list_scripts(flags: &OutputFlags) -> Result<()> {
    let dir = scripts_dir();

    // Auto-create the scripts directory and seed a sample script on first use.
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
        let sample = dir.join("hello");
        std::fs::write(
            &sample,
            "#!/bin/sh\n# Say hello — edit or delete this example\necho \"hello from orph!\"\n",
        )?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&sample, std::fs::Permissions::from_mode(0o755))?;
        }
    }

    let mut infos: Vec<ScriptInfo> = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                let description = extract_description(&path);
                infos.push(ScriptInfo {
                    name: name.to_string(),
                    description,
                    path: path.display().to_string(),
                });
            }
        }
    }

    infos.sort_by(|a, b| a.name.cmp(&b.name));

    if flags.json {
        println!("{}", serde_json::to_string(&infos)?);
        return Ok(());
    }

    if infos.is_empty() {
        if !flags.quiet {
            println!("no scripts found in {}", dir.display());
            println!(
                "  hint: add executable shell scripts to that directory, then run `orph run <name>`"
            );
        }
        return Ok(());
    }

    if !flags.quiet {
        println!("scripts ({}):", dir.display());
    }
    let max_name = infos.iter().map(|i| i.name.len()).max().unwrap_or(0);
    for info in &infos {
        println!(
            "  {:<width$}  — {}",
            info.name,
            info.description,
            width = max_name
        );
    }
    Ok(())
}

/// Extract a human-readable description from the first lines of a script.
/// Rules:
///   1. Skip shebang line (`#!...`)
///   2. Take the first line starting with `#`, strip `#` and trim.
///   3. If none found, return "(no description)".
fn extract_description(path: &std::path::Path) -> String {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return "(no description)".into(),
    };
    let reader = BufReader::new(file);
    let mut found_shebang = false;

    for line in reader.lines().take(10) {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("#!") {
            found_shebang = true;
            continue;
        }
        if trimmed.starts_with('#') {
            let desc = trimmed.trim_start_matches('#').trim().to_string();
            if !desc.is_empty() {
                return desc;
            }
        }
        // First non-comment, non-empty line — stop searching
        if found_shebang {
            break;
        }
    }
    "(no description)".into()
}

fn run_script(parts: &[String], flags: &OutputFlags) -> Result<()> {
    if parts.is_empty() {
        bail!("no script specified");
    }

    let script_name = &parts[0];
    validate_script_name(script_name)?;

    let (timeout_secs, json_override, quiet_override, verbose_override, script_args) =
        extract_run_flags(&parts[1..])?;

    // Merge global flags with any flags passed after the script name
    let json = flags.json || json_override;
    let _quiet = flags.quiet || quiet_override;
    let verbose = flags.verbose || verbose_override;

    let script_path = scripts_dir().join(script_name);
    if !script_path.exists() {
        bail!(
            "script '{}' not found in {}",
            script_name,
            scripts_dir().display()
        );
    }

    if verbose {
        eprintln!("[verbose] running: {}", script_path.display());
        if let Some(t) = timeout_secs {
            eprintln!("[verbose] timeout: {}s", t);
        }
    }

    logger::info(&format!("running script: {}", script_name));

    let start = Instant::now();

    let mut child = std::process::Command::new(&script_path)
        .args(&script_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                anyhow::anyhow!("failed to run script '{}': permission denied\n  hint: make the script executable with `chmod +x {}`", script_name, script_path.display())
            } else {
                anyhow::anyhow!("failed to run script '{}': {}", script_name, e)
            }
        })?;

    // Read stdout/stderr in background threads to avoid pipe deadlock.
    let mut stdout_reader = child.stdout.take().expect("stdout piped");
    let mut stderr_reader = child.stderr.take().expect("stderr piped");

    let stdout_thread = std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = stdout_reader.read_to_string(&mut buf);
        buf
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut buf = String::new();
        let _ = stderr_reader.read_to_string(&mut buf);
        buf
    });

    // Wait for process, honouring optional timeout.
    #[allow(unused_assignments)]
    let mut final_status: Option<std::process::ExitStatus> = None;

    let timed_out = match timeout_secs {
        Some(secs) => {
            let deadline = Duration::from_secs(secs);
            loop {
                match child.try_wait()? {
                    Some(status) => {
                        final_status = Some(status);
                        break false;
                    }
                    None => {
                        if start.elapsed() >= deadline {
                            let _ = child.kill();
                            final_status = child.wait().ok(); // reap to avoid zombie
                            break true;
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        }
        None => {
            final_status = child.wait().ok();
            false
        }
    };

    let elapsed_ms = start.elapsed().as_millis() as u64;

    // Only wait for reader threads when the process exited normally.
    // On timeout the child was killed, but sub-processes (e.g. `sleep`) may still
    // hold the pipe open; joining would block. Timed-out output is discarded.
    let (stdout, stderr) = if timed_out {
        drop(stdout_thread);
        drop(stderr_thread);
        (String::new(), String::new())
    } else {
        (
            stdout_thread.join().unwrap_or_default(),
            stderr_thread.join().unwrap_or_default(),
        )
    };

    let exit_code: i32 = if timed_out {
        -1
    } else {
        final_status.and_then(|s| s.code()).unwrap_or(-1)
    };

    logger::info(&format!(
        "script '{}' finished: exit_code={} elapsed={}ms timed_out={}",
        script_name, exit_code, elapsed_ms, timed_out
    ));
    if exit_code != 0 {
        logger::error(&format!(
            "script '{}' non-zero exit: code={} timed_out={}",
            script_name, exit_code, timed_out
        ));
    }

    if json {
        let result = ScriptResult {
            script: script_name.clone(),
            exit_code,
            timed_out,
            elapsed_ms,
            stdout,
            stderr,
        };
        println!("{}", serde_json::to_string(&result)?);
        if timed_out || exit_code != 0 {
            std::process::exit(exit_code.max(1) as i32);
        }
    } else {
        // Human mode: print captured output, then surface errors.
        if !stdout.is_empty() {
            print!("{}", stdout);
        }
        if !stderr.is_empty() {
            eprint!("{}", stderr);
        }
        if timed_out {
            bail!(
                "script '{}' timed out after {}s",
                script_name,
                timeout_secs.unwrap_or(0)
            );
        }
        if exit_code != 0 {
            bail!("script '{}' exited with code {}", script_name, exit_code);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[test]
    fn extract_description_from_comment() {
        let f = write_temp("#!/bin/bash\n# backs up home directory\necho hi\n");
        assert_eq!(extract_description(f.path()), "backs up home directory");
    }

    #[test]
    fn extract_description_no_comment() {
        let f = write_temp("echo hi\n");
        assert_eq!(extract_description(f.path()), "(no description)");
    }

    #[test]
    fn extract_description_only_shebang() {
        let f = write_temp("#!/usr/bin/env python3\n");
        assert_eq!(extract_description(f.path()), "(no description)");
    }
}
