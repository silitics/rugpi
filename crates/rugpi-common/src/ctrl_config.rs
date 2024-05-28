use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::{
    disk::{parse_size, repart::PartitionSchema},
    utils::units::NumBytes,
    Anyhow,
};

pub const CTRL_CONFIG_PATH: &str = "/etc/rugpi/ctrl.toml";

/// Structure of the Rugpi Ctrl configuration file.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    /// The size of the system partition(s).
    pub system_size: Option<String>,
    pub partition_schema: Option<PartitionSchema>,
    /// Indicates what to do with the overlay.
    #[serde(default)]
    pub overlay: Overlay,
}

impl Config {
    /// The size of the system partition(s) (defaults to `4G`).
    pub fn system_size(&self) -> &str {
        self.system_size.as_deref().unwrap_or("4G")
    }

    /// The size of the system partition(s) in bytes.
    pub fn system_size_bytes(&self) -> Anyhow<NumBytes> {
        Ok(parse_size(self.system_size())?)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Overlay {
    Persist,
    #[default]
    Discard,
}

/// Loads the Rugpi Ctrl configuration.
pub fn load_config(path: impl AsRef<Path>) -> Anyhow<Config> {
    let path = path.as_ref();
    if path.exists() {
        Ok(toml::from_str(&fs::read_to_string(path)?)?)
    } else {
        Ok(Config::default())
    }
}
