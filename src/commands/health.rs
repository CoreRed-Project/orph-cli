use crate::cli::OutputFlags;
use crate::services::health;
use anyhow::Result;
use clap::Args;

#[derive(Args)]
pub struct HealthArgs {}

impl HealthArgs {
    pub fn subcommand_name(&self) -> &'static str {
        "status"
    }
}

pub fn handle(_args: HealthArgs, flags: &OutputFlags) -> Result<()> {
    let snap = health::snapshot_local();

    if flags.json {
        println!("{}", serde_json::to_string(&snap)?);
        return Ok(());
    }

    if flags.quiet {
        let temp = snap
            .soc_temp_c
            .map(|t| format!("{t:.1}"))
            .unwrap_or_else(|| "n/a".into());
        println!(
            "temp_c={} throttled={} undervolt={}",
            temp, snap.throttled_now, snap.under_voltage_now
        );
        return Ok(());
    }

    println!("orph health");
    match snap.soc_temp_c {
        Some(t) => println!("  soc temp     : {:.1}°C", t),
        None => println!("  soc temp     : unavailable"),
    }
    println!(
        "  throttled    : {}",
        if snap.throttled_now {
            "yes (now)"
        } else {
            "no"
        }
    );
    println!(
        "  under-voltage: {}",
        if snap.under_voltage_now {
            "yes (now) — check power supply"
        } else {
            "no"
        }
    );
    if snap.throttled_ever || snap.under_voltage_ever {
        println!(
            "  history      : throttled_ever={} undervolt_ever={}",
            snap.throttled_ever, snap.under_voltage_ever
        );
    }
    if let Some(raw) = &snap.raw_throttle {
        println!("  raw flags    : {}", raw);
    }
    Ok(())
}
