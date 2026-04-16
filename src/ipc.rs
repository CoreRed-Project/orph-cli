//! Shared IPC types and client used by both `orph` CLI and `orphd` daemon.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;

pub const SOCKET_PATH: &str = "/tmp/orphd.sock";

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub command: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub status: String, // "ok" | "error"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Response {
    #[allow(dead_code)]
    pub fn ok(data: serde_json::Value) -> Self {
        Self {
            status: "ok".into(),
            data: Some(data),
            error: None,
        }
    }
    #[allow(dead_code)]
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            status: "error".into(),
            data: None,
            error: Some(msg.into()),
        }
    }
    pub fn is_ok(&self) -> bool {
        self.status == "ok"
    }
}

/// Attempt to send a request to orphd. Returns `None` if daemon is not running.
pub fn send(req: &Request) -> Option<Response> {
    let mut stream = UnixStream::connect(SOCKET_PATH).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .ok()?;

    let mut line = serde_json::to_string(req).ok()?;
    line.push('\n');
    stream.write_all(line.as_bytes()).ok()?;

    let mut reader = BufReader::new(&stream);
    let mut resp_line = String::new();
    reader.read_line(&mut resp_line).ok()?;
    serde_json::from_str(resp_line.trim()).ok()
}

/// Returns true if the daemon is reachable and responding.
pub fn ping() -> bool {
    let req = Request {
        command: "ping".into(),
        payload: serde_json::Value::Null,
    };
    matches!(send(&req), Some(r) if r.is_ok())
}
