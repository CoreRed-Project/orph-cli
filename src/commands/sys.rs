use crate::cli::OutputFlags;
use crate::ipc;
use anyhow::Result;
use clap::{Args, Subcommand};
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

#[derive(Args)]
pub struct SysArgs {
    #[command(subcommand)]
    pub cmd: SysCmd,
}

impl SysArgs {
    pub fn subcommand_name(&self) -> &'static str {
        match self.cmd {
            SysCmd::Status => "status",
            SysCmd::Info => "info",
        }
    }
}

#[derive(Subcommand)]
pub enum SysCmd {
    /// Show system health status
    Status,
    /// Show detailed system information
    Info,
}

pub fn handle(args: SysArgs, flags: &OutputFlags) -> Result<()> {
    match args.cmd {
        SysCmd::Status => {
            // Try daemon first; fall back to local if not running.
            if let Some(resp) = ipc::send(&ipc::Request {
                command: "sys.status".into(),
                payload: serde_json::Value::Null,
            }) && let Some(data) = resp.data
            {
                if flags.json {
                    println!("{}", data);
                } else {
                    print_status_from_json(&data, flags);
                }
                return Ok(());
            }
            // Fallback: local
            if !flags.quiet && !flags.json {
                eprintln!("  [daemon offline — running in local fallback mode]");
            }
            let mut sys = System::new_with_specifics(
                RefreshKind::nothing()
                    .with_cpu(CpuRefreshKind::everything())
                    .with_memory(MemoryRefreshKind::everything()),
            );
            std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
            sys.refresh_cpu_all();
            sys.refresh_memory();
            status(&sys, flags);
        }
        SysCmd::Info => {
            // info only needs cpu count and memory totals — no CPU usage sampling needed.
            let sys = System::new_with_specifics(
                RefreshKind::nothing()
                    .with_cpu(CpuRefreshKind::nothing())
                    .with_memory(MemoryRefreshKind::everything()),
            );
            info(&sys, flags);
        }
    }
    Ok(())
}

/// Format daemon sys.status response in human mode.
fn print_status_from_json(data: &serde_json::Value, flags: &OutputFlags) {
    let cpu = data["cpu_percent"].as_f64().unwrap_or(0.0);
    let mem_used = data["mem_used_mb"].as_u64().unwrap_or(0);
    let mem_tot = data["mem_total_mb"].as_u64().unwrap_or(0);
    let mem_pct = data["mem_percent"].as_u64().unwrap_or(0);
    let disk_u = data["disk_used_gb"].as_u64().unwrap_or(0);
    let disk_t = data["disk_total_gb"].as_u64().unwrap_or(0);
    let disk_pct = data["disk_percent"].as_u64().unwrap_or(0);

    if flags.quiet {
        println!("cpu={:.1}% mem={}% disk={}%", cpu, mem_pct, disk_pct);
        return;
    }
    println!("sys status");
    println!("  cpu    : {:.1}%", cpu);
    println!("  memory : {} / {} MB ({}%)", mem_used, mem_tot, mem_pct);
    if disk_t > 0 {
        println!("  disk   : {} / {} GB ({}%)", disk_u, disk_t, disk_pct);
    } else {
        println!("  disk   : unavailable");
    }
}

fn status(sys: &System, flags: &OutputFlags) {
    let cpus = sys.cpus();
    let cpu = if cpus.is_empty() {
        0.0_f32
    } else {
        cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32
    };
    let mem_used = sys.used_memory();
    let mem_total = sys.total_memory();
    let mem_pct = if mem_total > 0 {
        mem_used * 100 / mem_total
    } else {
        0
    };

    // Disk: aggregate all physical disks, pick root mount or sum all.
    let disks = Disks::new_with_refreshed_list();
    let (disk_total, disk_used) = disk_stats(&disks);
    let disk_pct = if disk_total > 0 {
        disk_used * 100 / disk_total
    } else {
        0
    };

    if flags.json {
        println!(
            "{{\"cpu_percent\": {:.1}, \"mem_used_mb\": {}, \"mem_total_mb\": {}, \"mem_percent\": {}, \
             \"disk_used_gb\": {}, \"disk_total_gb\": {}, \"disk_percent\": {}}}",
            cpu,
            mem_used / 1024 / 1024,
            mem_total / 1024 / 1024,
            mem_pct,
            disk_used / 1024 / 1024 / 1024,
            disk_total / 1024 / 1024 / 1024,
            disk_pct,
        );
    } else if flags.quiet {
        println!("cpu={:.1}% mem={}% disk={}%", cpu, mem_pct, disk_pct);
    } else {
        println!("sys status");
        println!("  cpu    : {:.1}%", cpu);
        println!(
            "  memory : {} / {} MB ({}%)",
            mem_used / 1024 / 1024,
            mem_total / 1024 / 1024,
            mem_pct
        );
        if disk_total > 0 {
            println!(
                "  disk   : {} / {} GB ({}%)",
                disk_used / 1024 / 1024 / 1024,
                disk_total / 1024 / 1024 / 1024,
                disk_pct
            );
        } else {
            println!("  disk   : unavailable");
        }
    }
}

/// Returns (total_bytes, used_bytes) for the root filesystem ("/") if found,
/// otherwise sums all disks. Returns (0, 0) if no disks are available.
fn disk_stats(disks: &Disks) -> (u64, u64) {
    // Prefer the disk mounted at "/" (Linux/macOS root)
    for disk in disks.list() {
        if disk.mount_point() == std::path::Path::new("/") {
            let total = disk.total_space();
            let avail = disk.available_space();
            let used = total.saturating_sub(avail);
            return (total, used);
        }
    }
    // Fallback: sum all disks
    let (total, used) = disks.list().iter().fold((0u64, 0u64), |(t, u), d| {
        let avail = d.available_space();
        let used = d.total_space().saturating_sub(avail);
        (t + d.total_space(), u + used)
    });
    (total, used)
}

fn info(sys: &System, flags: &OutputFlags) {
    let hostname = System::host_name().unwrap_or_else(|| "unknown".into());
    let os = System::long_os_version().unwrap_or_else(|| "unknown".into());
    let kernel = System::kernel_version().unwrap_or_else(|| "unknown".into());
    let uptime = System::uptime();
    let cpu_count = sys.cpus().len();

    if flags.json {
        println!(
            "{{\"hostname\": \"{}\", \"os\": \"{}\", \"kernel\": \"{}\", \"uptime_s\": {}, \"cpu_count\": {}}}",
            hostname, os, kernel, uptime, cpu_count
        );
    } else {
        println!("sys info");
        println!("  hostname : {}", hostname);
        println!("  os       : {}", os);
        println!("  kernel   : {}", kernel);
        println!("  uptime   : {}s", uptime);
        println!("  cpus     : {}", cpu_count);
    }
}
