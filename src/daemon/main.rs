// orphd — orph background daemon
// Handles IPC requests via a Unix domain socket.

mod handlers;
mod ipc_server;

use orph_cli::ipc;
use orph_cli::services;
use std::path::Path;

fn main() {
    let socket_path = ipc::SOCKET_PATH;

    // Prevent multiple instances.
    if ipc::ping() {
        eprintln!(
            "orphd: already running (socket {} is responsive)",
            socket_path
        );
        std::process::exit(1);
    }

    // Remove stale socket file if present.
    if Path::new(socket_path).exists() {
        let _ = std::fs::remove_file(socket_path);
    }

    // Clean up socket on SIGTERM / SIGINT.
    unsafe {
        libc::signal(
            libc::SIGTERM,
            handle_signal as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGINT,
            handle_signal as *const () as libc::sighandler_t,
        );
    }

    let conn = services::db::init().expect("orphd: failed to open database");
    eprintln!("orphd: database ready at {:?}", services::db::db_path());

    ipc_server::serve(&conn);
}

extern "C" fn handle_signal(_: libc::c_int) {
    let _ = std::fs::remove_file(ipc::SOCKET_PATH);
    std::process::exit(0);
}
