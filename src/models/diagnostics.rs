use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SysSnapshot {
    pub cpu_percent: f64,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
    pub mem_percent: u64,
    pub disk_used_gb: u64,
    pub disk_total_gb: u64,
    pub disk_percent: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetInterface {
    pub name: String,
    pub is_up: bool,
    pub operstate: Option<String>,
    pub ipv4: Vec<String>,
    pub ipv6: Vec<String>,
    pub gateway_v4: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetSnapshot {
    pub interfaces: Vec<NetInterface>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSnapshot {
    pub soc_temp_c: Option<f64>,
    pub throttled_now: bool,
    pub under_voltage_now: bool,
    pub arm_capped_now: bool,
    pub throttled_ever: bool,
    pub under_voltage_ever: bool,
    pub raw_throttle: Option<String>,
}

impl HealthSnapshot {
    /// Parse Raspberry Pi `get_throttled` hex flags when present.
    pub fn with_parsed_flags(mut self) -> Self {
        let Some(raw) = &self.raw_throttle else {
            return self;
        };
        let hex = raw.trim().strip_prefix("0x").unwrap_or(raw.trim());
        let Ok(flags) = u32::from_str_radix(hex, 16) else {
            return self;
        };
        self.under_voltage_now = flags & 0x1 != 0;
        self.arm_capped_now = flags & 0x2 != 0;
        self.throttled_now = flags & 0x4 != 0;
        self.under_voltage_ever = flags & 0x10000 != 0;
        self.throttled_ever = flags & 0x40000 != 0;
        self
    }
}
