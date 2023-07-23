use std::{fs, path::Path};

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
    Directory {
        /// The path of the directory to persist.    
        directory: String,
    },
    /// Persist a file.
    File {
        /// The path to the file to persist.
        file: String,
        /// The default content to write to the file.
        default: Option<String>,
    },
}

/// The default directory with the configurations for state management.
pub const STATE_CONFIG_DIR: &str = "/etc/rugpi/state";

/// Loads the state configuration from the provided directory.
pub fn load_state_config(dir: impl AsRef<Path>) -> StateConfig {
    let mut combined = StateConfig::new();
    if let Ok(read_dir) = fs::read_dir(dir) {
        for entry in read_dir {
            if let Some(config) = entry
                .ok()
                .and_then(|entry| fs::read_to_string(entry.path()).ok())
                .and_then(|config| toml::from_str(&config).ok())
            {
                combined.merge(config);
            }
        }
    }
    combined
}
