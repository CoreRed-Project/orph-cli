use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Pet {
    pub name: String,
    pub hunger: u8,    // 0-100
    pub happiness: u8, // 0-100
    pub last_fed: String,
    pub last_played: String,
    pub last_updated: String,
}

impl Pet {
    pub fn mood(&self) -> &'static str {
        if self.hunger > 70 {
            "hungry"
        } else if self.happiness < 25 {
            "sad"
        } else if self.happiness > 80 {
            "playful"
        } else {
            // Check current local time for sleepiness (between 10 PM and 6 AM)
            if let Ok(local_time) = chrono::Local::now().format("%H").to_string().parse::<u32>() {
                if local_time >= 22 || local_time <= 6 {
                    return "sleepy";
                }
            }
            "content"
        }
    }
}
