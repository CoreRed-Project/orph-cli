use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigEntry {
    pub key: String,
    pub value: String,
}
