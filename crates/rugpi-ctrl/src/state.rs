use std::fs;

use serde::{Deserialize, Serialize};

/// Configuration of the state management subsystem.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateConfig {
    /// Indicates the state to persist.
    pub persist: Vec<Persist>,
}

impl StateConfig {
    /// Creates a default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Merges the other configuration into this configuration.
    pub fn merge(&mut self, other: StateConfig) {
        self.persist.extend(other.persist);
    }
}

/// Indicates the state to persist.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Persist {
    /// Persist a directory.
    Directory { directory: String },
    /// Persist a file.
    File {
        file: String,
        default: Option<String>,
    },
}

pub fn load_state_config() -> StateConfig {
    let mut combined = StateConfig::new();
    if let Ok(read_dir) = fs::read_dir("/etc/rugpi/state") {
        for entry in read_dir {
            entry
                .ok()
                .and_then(|entry| fs::read_to_string(entry.path()).ok())
                .and_then(|config| toml::from_str(&config).ok())
                .map(|config| combined.merge(config));
        }
    }
    combined
}
