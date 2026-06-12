use crate::models::diagnostics::{NetSnapshot, SysSnapshot};
use anyhow::Result;

/// One-minute load average (Linux `/proc/loadavg`). Returns `None` on unsupported OS.
pub fn loadavg_one() -> Option<f64> {
    #[cfg(target_os = "linux")]
    {
        let content = std::fs::read_to_string("/proc/loadavg").ok()?;
        let one = content.split_whitespace().next()?;
        one.parse().ok()
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

pub fn sys_snapshot_local() -> SysSnapshot {
    use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};

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

    SysSnapshot {
        cpu_percent: (cpu as f64 * 10.0).round() / 10.0,
        mem_used_mb: mem_used / 1024 / 1024,
        mem_total_mb: mem_total / 1024 / 1024,
        mem_percent: mem_pct,
        disk_used_gb: disk_used / 1024 / 1024 / 1024,
        disk_total_gb: disk_total / 1024 / 1024 / 1024,
        disk_percent: disk_pct,
    }
}

fn disk_stats(disks: &sysinfo::Disks) -> (u64, u64) {
    for disk in disks.list() {
        if disk.mount_point() == std::path::Path::new("/") {
            let total = disk.total_space();
            let avail = disk.available_space();
            let used = total.saturating_sub(avail);
            return (total, used);
        }
    }
    disks.list().iter().fold((0u64, 0u64), |(t, u), d| {
        let avail = d.available_space();
        let used = d.total_space().saturating_sub(avail);
        (t + d.total_space(), u + used)
    })
}

pub fn net_snapshot_local() -> Result<NetSnapshot> {
    #[cfg(target_os = "linux")]
    {
        net_snapshot_linux()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(NetSnapshot { interfaces: vec![] })
    }
}

#[cfg(target_os = "linux")]
fn net_snapshot_linux() -> Result<NetSnapshot> {
    use crate::models::diagnostics::NetInterface;
    use std::collections::BTreeMap;

    let mut ifs: BTreeMap<String, NetInterface> = BTreeMap::new();
    for (name, is_up, ip, is_v6) in ifaddrs_iter()? {
        let entry = ifs.entry(name.clone()).or_insert_with(|| NetInterface {
            name,
            is_up,
            operstate: None,
            ipv4: vec![],
            ipv6: vec![],
            gateway_v4: None,
        });
        entry.is_up = entry.is_up || is_up;
        if let Some(ip) = ip {
            if is_v6 {
                entry.ipv6.push(ip);
            } else {
                entry.ipv4.push(ip);
            }
        }
    }

    // operstate + default gateway best-effort
    let gw = default_gateway_v4_linux().ok().flatten();
    for entry in ifs.values_mut() {
        entry.operstate = read_operstate_linux(&entry.name).ok().flatten();
        entry.gateway_v4 = gw.clone();
    }

    Ok(NetSnapshot {
        interfaces: ifs.into_values().collect(),
    })
}

#[cfg(target_os = "linux")]
fn read_operstate_linux(ifname: &str) -> Result<Option<String>> {
    let path = format!("/sys/class/net/{}/operstate", ifname);
    let content = std::fs::read_to_string(path)?;
    Ok(Some(content.trim().to_string()))
}

#[cfg(target_os = "linux")]
fn default_gateway_v4_linux() -> Result<Option<String>> {
    let content = std::fs::read_to_string("/proc/net/route")?;
    Ok(parse_proc_net_route_default_gateway(&content))
}

#[cfg(any(test, target_os = "linux"))]
fn parse_proc_net_route_default_gateway(content: &str) -> Option<String> {
    // Format (tab-separated): Iface Destination Gateway Flags RefCnt Use Metric Mask ...
    // We want Destination == 00000000.
    let mut lines = content.lines();
    let _header = lines.next()?;
    for line in lines {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 3 {
            continue;
        }
        let destination = cols[1];
        let gateway = cols[2];
        if destination != "00000000" {
            continue;
        }
        if let Ok(raw) = u32::from_str_radix(gateway, 16) {
            // /proc/net/route encodes gateway as little-endian u32.
            let b1 = (raw & 0xFF) as u8;
            let b2 = ((raw >> 8) & 0xFF) as u8;
            let b3 = ((raw >> 16) & 0xFF) as u8;
            let b4 = ((raw >> 24) & 0xFF) as u8;
            return Some(format!("{}.{}.{}.{}", b1, b2, b3, b4));
        }
    }
    None
}

#[cfg(target_os = "linux")]
type IfAddrInfo = (String, bool, Option<String>, bool);

#[cfg(target_os = "linux")]
fn ifaddrs_iter() -> Result<Vec<IfAddrInfo>> {
    // Returns (ifname, is_up, ip_string, is_ipv6)
    use std::ffi::CStr;
    use std::net::{Ipv4Addr, Ipv6Addr};

    let mut out = Vec::new();
    let mut addrs: *mut libc::ifaddrs = std::ptr::null_mut();
    let rc = unsafe { libc::getifaddrs(&mut addrs) };
    if rc != 0 {
        return Err(anyhow::anyhow!("getifaddrs failed"));
    }

    let mut cur = addrs;
    while !cur.is_null() {
        unsafe {
            let ifa = &*cur;
            if !ifa.ifa_name.is_null() {
                let name = CStr::from_ptr(ifa.ifa_name).to_string_lossy().to_string();
                let is_up = (ifa.ifa_flags as i32 & libc::IFF_UP) != 0;

                let (ip, is_v6) = if ifa.ifa_addr.is_null() {
                    (None, false)
                } else {
                    let family = (*ifa.ifa_addr).sa_family as i32;
                    if family == libc::AF_INET {
                        let sin: *const libc::sockaddr_in =
                            ifa.ifa_addr as *const libc::sockaddr_in;
                        let addr = u32::from_be((*sin).sin_addr.s_addr);
                        let ip = Ipv4Addr::from(addr).to_string();
                        (Some(ip), false)
                    } else if family == libc::AF_INET6 {
                        let sin6: *const libc::sockaddr_in6 =
                            ifa.ifa_addr as *const libc::sockaddr_in6;
                        let octets = (*sin6).sin6_addr.s6_addr;
                        let ip = Ipv6Addr::from(octets).to_string();
                        (Some(ip), true)
                    } else {
                        (None, false)
                    }
                };

                out.push((name, is_up, ip, is_v6));
            }
            cur = (*cur).ifa_next;
        }
    }

    unsafe { libc::freeifaddrs(addrs) };
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::parse_proc_net_route_default_gateway;

    #[test]
    fn parses_default_gateway() {
        // gateway 0102A8C0 == 192.168.2.1 (little endian)
        let content = "Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT\n\
eth0\t00000000\t0102A8C0\t0003\t0\t0\t0\t00000000\t0\t0\t0\n";
        assert_eq!(
            parse_proc_net_route_default_gateway(content),
            Some("192.168.2.1".into())
        );
    }
}
