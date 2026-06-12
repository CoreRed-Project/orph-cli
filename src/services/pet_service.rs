use crate::models::pet::Pet;
use crate::services::logger;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, Result};

/// Decay rates per hour
const HUNGER_RATE: f64 = 5.0; // hunger increases 5 points/hour
const HAPPINESS_RATE: f64 = 3.0; // happiness decreases 3 points/hour

/// Pure decay calculation — testable without DB.
pub fn calculate_decay(hunger: u8, happiness: u8, elapsed_hours: f64) -> (u8, u8) {
    let new_hunger = (hunger as f64 + HUNGER_RATE * elapsed_hours)
        .clamp(0.0, 100.0)
        .round() as u8;
    let new_happiness = (happiness as f64 - HAPPINESS_RATE * elapsed_hours)
        .clamp(0.0, 100.0)
        .round() as u8;
    (new_hunger, new_happiness)
}

/// Fetch raw pet row without applying decay.
fn fetch(conn: &Connection) -> Result<Pet> {
    conn.query_row(
        "SELECT name, hunger, happiness, last_fed, last_played, last_updated FROM pet WHERE id = 1",
        [],
        |row| {
            Ok(Pet {
                name: row.get(0)?,
                hunger: row.get::<_, i64>(1)? as u8,
                happiness: row.get::<_, i64>(2)? as u8,
                last_fed: row.get(3)?,
                last_played: row.get(4)?,
                last_updated: row.get(5)?,
            })
        },
    )
}

/// Apply time-based decay and persist to DB. Returns updated Pet.
fn apply_decay(conn: &Connection) -> Result<Pet> {
    let pet = fetch(conn)?;
    let now = Utc::now();

    let last: DateTime<Utc> = pet.last_updated.parse::<DateTime<Utc>>().unwrap_or(now);

    let mut elapsed_seconds = (now - last).num_seconds();
    if elapsed_seconds < -10 {
        // Backwards clock jump! Recover immediately.
        let now_str = now.to_rfc3339();
        conn.execute(
            "UPDATE pet SET last_updated = ?1 WHERE id = 1",
            rusqlite::params![now_str],
        )?;
        return fetch(conn);
    }
    if elapsed_seconds <= 0 {
        return Ok(pet);
    }
    if elapsed_seconds > 86400 {
        elapsed_seconds = 86400; // Cap at 24 hours
    }

    let elapsed_hours = elapsed_seconds as f64 / 3600.0;
    let (new_hunger, new_happiness) = calculate_decay(pet.hunger, pet.happiness, elapsed_hours);

    let stats_changed = new_hunger != pet.hunger || new_happiness != pet.happiness;
    let now_str = now.to_rfc3339();

    if stats_changed {
        conn.execute(
            "UPDATE pet SET hunger = ?1, happiness = ?2, last_updated = ?3 WHERE id = 1",
            rusqlite::params![new_hunger as i64, new_happiness as i64, now_str],
        )?;

        logger::info(&format!(
            "pet decay applied: elapsed={:.2}h hunger {}→{} happiness {}→{}",
            elapsed_hours, pet.hunger, new_hunger, pet.happiness, new_happiness
        ));

        fetch(conn)
    } else {
        Ok(pet)
    }
}

/// Public read: applies decay first, then returns state.
pub fn get(conn: &Connection) -> Result<Pet> {
    apply_decay(conn)
}

pub fn feed(conn: &Connection) -> Result<Pet> {
    apply_decay(conn)?;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE pet SET hunger = MAX(0, hunger - 40), happiness = MIN(100, happiness + 10), last_fed = ?1, last_updated = ?1 WHERE id = 1",
        rusqlite::params![now],
    )?;
    fetch(conn)
}

pub fn play(conn: &Connection) -> Result<Pet> {
    apply_decay(conn)?;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE pet SET happiness = MIN(100, happiness + 20), hunger = MIN(100, hunger + 5), last_played = ?1, last_updated = ?1 WHERE id = 1",
        rusqlite::params![now],
    )?;
    fetch(conn)
}

pub fn rename(conn: &Connection, name: &str) -> Result<Pet> {
    conn.execute(
        "UPDATE pet SET name = ?1 WHERE id = 1",
        rusqlite::params![name],
    )?;
    fetch(conn)
}
