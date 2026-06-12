use crate::services::paths;
use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::io::Read;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize)]
pub struct ScriptRunResult {
    pub script: String,
    pub exit_code: i32,
    pub timed_out: bool,
    pub elapsed_ms: u64,
    pub stdout: String,
    pub stderr: String,
}

pub fn validate_script_name(name: &str) -> Result<()> {
    if name.contains('/') || name.contains('\\') || name.starts_with('.') {
        bail!(
            "invalid script name '{}': only plain filenames are allowed",
            name
        );
    }
    Ok(())
}

pub fn script_path(name: &str) -> PathBuf {
    paths::scripts_dir().join(name)
}

/// Run a script with a minimal environment (inherits only PATH/HOME/USER).
pub fn run_isolated(
    name: &str,
    args: &[String],
    timeout_secs: Option<u64>,
) -> Result<ScriptRunResult> {
    validate_script_name(name)?;
    let path = script_path(name);
    if !path.exists() {
        bail!(
            "script '{}' not found in {}",
            name,
            paths::scripts_dir().display()
        );
    }

    let start = Instant::now();
    let mut cmd = std::process::Command::new(&path);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("HOME", paths::home_dir())
        .env(
            "USER",
            std::env::var("USER").unwrap_or_else(|_| "orph".into()),
        );

    let mut child = cmd.spawn().with_context(|| {
        format!(
            "failed to spawn script '{}' — is it executable?",
            path.display()
        )
    })?;

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

    let (final_status, timed_out) = match timeout_secs {
        Some(secs) => {
            let deadline = Duration::from_secs(secs);
            loop {
                match child.try_wait()? {
                    Some(status) => {
                        break (Some(status), false);
                    }
                    None if start.elapsed() >= deadline => {
                        let _ = child.kill();
                        break (child.wait().ok(), true);
                    }
                    None => std::thread::sleep(Duration::from_millis(100)),
                }
            }
        }
        None => (child.wait().ok(), false),
    };

    let elapsed_ms = start.elapsed().as_millis() as u64;
    let (stdout, stderr) = if timed_out {
        (String::new(), String::new())
    } else {
        (
            stdout_thread.join().unwrap_or_default(),
            stderr_thread.join().unwrap_or_default(),
        )
    };

    let exit_code = if timed_out {
        -1
    } else {
        final_status.and_then(|s| s.code()).unwrap_or(-1)
    };

    // Record this script run in the sqlite history
    if let Ok(conn) = crate::services::db::init() {
        let _ = crate::services::script_history_service::record_run(
            &conn, name, exit_code, timed_out, elapsed_ms, &stdout, &stderr,
        );
    }

    Ok(ScriptRunResult {
        script: name.to_string(),
        exit_code,
        timed_out,
        elapsed_ms,
        stdout,
        stderr,
    })
}
