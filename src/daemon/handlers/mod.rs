use orph_cli::ipc::Response;
use orph_cli::services::{config_service, pet_service};
use rusqlite::Connection;
use serde_json::json;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

pub fn sys_status() -> Response {
    let mut sys = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    sys.refresh_cpu_all();
    sys.refresh_memory();

    let cpus = sys.cpus();
    let cpu = if cpus.is_empty() {
        0.0_f32
    } else {
        cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32
    };
    let mem_used = sys.used_memory();
    let mem_total = sys.total_memory();
    let mem_pct = (mem_used * 100).checked_div(mem_total).unwrap_or(0);

    let disks = Disks::new_with_refreshed_list();
    let (disk_total, disk_used) = disk_stats(&disks);
    let disk_pct = (disk_used * 100).checked_div(disk_total).unwrap_or(0);

    Response::ok(json!({
        "cpu_percent":  (cpu * 10.0).round() / 10.0,
        "mem_used_mb":  mem_used / 1024 / 1024,
        "mem_total_mb": mem_total / 1024 / 1024,
        "mem_percent":  mem_pct,
        "disk_used_gb": disk_used / 1024 / 1024 / 1024,
        "disk_total_gb":disk_total / 1024 / 1024 / 1024,
        "disk_percent": disk_pct,
    }))
}

fn disk_stats(disks: &Disks) -> (u64, u64) {
    for disk in disks.list() {
        if disk.mount_point() == std::path::Path::new("/") {
            let total = disk.total_space();
            return (total, total.saturating_sub(disk.available_space()));
        }
    }
    disks.list().iter().fold((0u64, 0u64), |(t, u), d| {
        (
            t + d.total_space(),
            u + d.total_space().saturating_sub(d.available_space()),
        )
    })
}

pub fn pet_status(conn: &Connection) -> Response {
    match pet_service::get(conn) {
        Ok(pet) => Response::ok(serde_json::to_value(&pet).unwrap_or_default()),
        Err(e) => Response::error(e.to_string()),
    }
}

pub fn pet_feed(conn: &Connection) -> Response {
    match pet_service::feed(conn) {
        Ok(pet) => Response::ok(serde_json::to_value(&pet).unwrap_or_default()),
        Err(e) => Response::error(e.to_string()),
    }
}

pub fn pet_play(conn: &Connection) -> Response {
    match pet_service::play(conn) {
        Ok(pet) => Response::ok(serde_json::to_value(&pet).unwrap_or_default()),
        Err(e) => Response::error(e.to_string()),
    }
}

// ── CFG ─────────────────────────────────────────────────────────────────────

pub fn cfg_list(conn: &Connection) -> Response {
    match config_service::list(conn) {
        Ok(entries) => Response::ok(serde_json::to_value(&entries).unwrap_or_default()),
        Err(e) => Response::error(e.to_string()),
    }
}

pub fn cfg_get(conn: &Connection, payload: &serde_json::Value) -> Response {
    let key = match payload["key"].as_str() {
        Some(k) => k.to_string(),
        None => return Response::error("missing 'key' in payload"),
    };
    match config_service::get(conn, &key) {
        Ok(Some(entry)) => Response::ok(serde_json::to_value(&entry).unwrap_or_default()),
        Ok(None) => Response::ok(serde_json::Value::Null),
        Err(e) => Response::error(e.to_string()),
    }
}

pub fn cfg_set(conn: &Connection, payload: &serde_json::Value) -> Response {
    let key = match payload["key"].as_str() {
        Some(k) => k.to_string(),
        None => return Response::error("missing 'key' in payload"),
    };
    let value = match payload["value"].as_str() {
        Some(v) => v.to_string(),
        None => return Response::error("missing 'value' in payload"),
    };
    match config_service::set(conn, &key, &value) {
        Ok(()) => Response::ok(json!({"key": key, "value": value, "ok": true})),
        Err(e) => Response::error(e.to_string()),
    }
}

// ── LOGS ─────────────────────────────────────────────────────────────────────

pub fn logs_read(payload: &serde_json::Value) -> Response {
    let tail = payload["tail"].as_bool().unwrap_or(false);
    let level_filter = payload["level"].as_str().map(|l| l.to_uppercase());

    let log_path = {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        std::path::PathBuf::from(home)
            .join(".orph")
            .join("orph.log")
    };

    if !log_path.exists() {
        return Response::ok(serde_json::Value::Array(vec![]));
    }

    let file = match std::fs::File::open(&log_path) {
        Ok(f) => f,
        Err(e) => return Response::error(format!("cannot open log: {}", e)),
    };

    use std::io::BufRead;
    let reader = std::io::BufReader::new(file);

    let lines: Vec<String> = reader
        .lines()
        .map_while(Result::ok)
        .filter(|line| {
            if let Some(ref lvl) = level_filter {
                line.to_uppercase().contains(lvl.as_str())
            } else {
                true
            }
        })
        .collect();

    let output: Vec<&String> = if tail {
        let start = lines.len().saturating_sub(20);
        lines[start..].iter().collect()
    } else {
        lines.iter().collect()
    };

    Response::ok(serde_json::to_value(&output).unwrap_or_default())
}
