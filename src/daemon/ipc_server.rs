use orph_cli::ipc::{Request, Response, SOCKET_PATH};
use rusqlite::Connection;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;

/// Blocking IPC server loop. Handles one connection at a time.
pub fn serve(conn: &Connection) {
    let listener = UnixListener::bind(SOCKET_PATH)
        .expect("failed to bind socket — is another orphd already running?");

    eprintln!("orphd: listening on {}", SOCKET_PATH);

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
                let mut line = String::new();

                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => continue,
                    Ok(_) => {}
                }

                let (response, shutdown) = match serde_json::from_str::<Request>(line.trim()) {
                    Ok(req) => dispatch(&req, conn),
                    Err(e) => (Response::error(format!("invalid request: {}", e)), false),
                };

                let mut out = serde_json::to_string(&response).unwrap_or_default();
                out.push('\n');
                let _ = stream.write_all(out.as_bytes());
                drop(stream);

                if shutdown {
                    let _ = std::fs::remove_file(SOCKET_PATH);
                    std::process::exit(0);
                }
            }
            Err(e) => eprintln!("orphd: accept error: {}", e),
        }
    }
}

/// Returns (Response, should_shutdown).
fn dispatch(req: &Request, conn: &Connection) -> (Response, bool) {
    let resp = match req.command.as_str() {
        "ping" => Response::ok(serde_json::json!({"pong": true})),
        "sys.status" => super::handlers::sys_status(),
        "pet.status" => super::handlers::pet_status(conn),
        "pet.feed" => super::handlers::pet_feed(conn),
        "pet.play" => super::handlers::pet_play(conn),
        "cfg.list" => super::handlers::cfg_list(conn),
        "cfg.get" => super::handlers::cfg_get(conn, &req.payload),
        "cfg.set" => super::handlers::cfg_set(conn, &req.payload),
        "logs.read" => super::handlers::logs_read(&req.payload),
        "shutdown" => Response::ok(serde_json::json!({"shutdown": true})),
        other => Response::error(format!("unknown command: {}", other)),
    };
    let shutdown = req.command == "shutdown";
    (resp, shutdown)
}
