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
        } else if self.happiness > 70 {
            "happy"
        } else if self.happiness < 30 {
            "sad"
        } else {
            "content"
        }
    }
}
