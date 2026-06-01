use crate::models::diagnostics::HealthSnapshot;

pub fn snapshot_local() -> HealthSnapshot {
    HealthSnapshot {
        soc_temp_c: read_soc_temp_c(),
        throttled_now: false,
        under_voltage_now: false,
        arm_capped_now: false,
        throttled_ever: false,
        under_voltage_ever: false,
        raw_throttle: read_throttle_raw(),
    }
    .with_parsed_flags()
}

fn read_soc_temp_c() -> Option<f64> {
    #[cfg(target_os = "linux")]
    {
        let milli: i64 = std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp")
            .ok()?
            .trim()
            .parse()
            .ok()?;
        return Some(milli as f64 / 1000.0);
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

fn read_throttle_raw() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        const PATHS: &[&str] = &[
            "/sys/devices/platform/soc/soc:firmware/get_throttled",
            "/sys/kernel/debug/raspberrypi/v3io/v3io",
        ];
        for path in PATHS {
            if let Ok(s) = std::fs::read_to_string(path) {
                let t = s.trim();
                if !t.is_empty() {
                    return Some(t.to_string());
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::models::diagnostics::HealthSnapshot;

    #[test]
    fn parses_throttle_hex() {
        let mut h = HealthSnapshot {
            soc_temp_c: None,
            throttled_now: false,
            under_voltage_now: false,
            arm_capped_now: false,
            throttled_ever: false,
            under_voltage_ever: false,
            raw_throttle: Some("0x50005".into()),
        };
        h = h.with_parsed_flags();
        assert!(h.under_voltage_now);
        assert!(h.throttled_ever);
    }
}
