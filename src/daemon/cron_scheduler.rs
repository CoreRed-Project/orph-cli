use chrono::Utc;
use orph_cli::services::{cron_service, db, logger, script_runner};
use rusqlite::Connection;
use std::time::Duration;

const TICK: Duration = Duration::from_secs(15);

pub fn spawn() {
    std::thread::spawn(|| {
        loop {
            if let Err(e) = tick() {
                logger::error(&format!("cron scheduler: {}", e));
            }
            std::thread::sleep(TICK);
        }
    });
}

fn tick() -> anyhow::Result<()> {
    let conn = db::init()?;
    let due = due_jobs(&conn)?;
    for job in due {
        logger::info(&format!(
            "cron: running '{}' (every {}s)",
            job.script_name, job.interval_secs
        ));
        match script_runner::run_isolated(&job.script_name, &[], Some(300)) {
            Ok(result) => {
                logger::info(&format!(
                    "cron: '{}' exit={} elapsed={}ms",
                    job.script_name, result.exit_code, result.elapsed_ms
                ));
                if result.exit_code != 0 {
                    logger::error(&format!(
                        "cron: '{}' failed with code {}",
                        job.script_name, result.exit_code
                    ));
                }
            }
            Err(e) => logger::error(&format!("cron: '{}' error: {}", job.script_name, e)),
        }
        mark_run(&conn, &job.script_name)?;
    }
    Ok(())
}

fn due_jobs(conn: &Connection) -> anyhow::Result<Vec<cron_service::CronJob>> {
    let jobs = cron_service::list(conn)?;
    let now = Utc::now();
    Ok(jobs
        .into_iter()
        .filter(|job| job.enabled)
        .filter(|job| match &job.last_run {
            None => true,
            Some(ts) => {
                let Ok(last) = ts.parse::<chrono::DateTime<Utc>>() else {
                    return true;
                };
                (now - last).num_seconds() >= job.interval_secs as i64
            }
        })
        .collect())
}

fn mark_run(conn: &Connection, script_name: &str) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE cron_jobs SET last_run = ?1 WHERE script_name = ?2",
        rusqlite::params![now, script_name],
    )?;
    Ok(())
}
