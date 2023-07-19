use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    pub system_size: Option<String>,
}

impl Config {
    pub fn system_size(&self) -> &str {
        self.system_size.as_deref().unwrap_or("4G")
    }
}
