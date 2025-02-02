use std::fs;
use std::path::Path;

use reportify::{Report, ResultExt};
use serde::{Deserialize, Serialize};

use crate::disk::parse_size;
use crate::disk::repart::PartitionSchema;
use crate::system::SystemError;
use crate::utils::units::NumBytes;

pub const CTRL_CONFIG_PATH: &str = "/etc/rugpi/ctrl.toml";

/// Structure of the Rugix Ctrl configuration file.
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
    pub fn system_size_bytes(&self) -> Result<NumBytes, Report<SystemError>> {
        parse_size(self.system_size()).whatever("unable to parse system size")
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Overlay {
    Persist,
    #[default]
    Discard,
}

/// Loads the Rugix Ctrl configuration.
pub fn load_config(path: impl AsRef<Path>) -> Result<Config, Report<SystemError>> {
    let path = path.as_ref();
    if path.exists() {
        Ok(
            toml::from_str(&fs::read_to_string(path).whatever("unable to read config")?)
                .whatever("unable to parse config")?,
        )
    } else {
        Ok(Config::default())
    }
}
