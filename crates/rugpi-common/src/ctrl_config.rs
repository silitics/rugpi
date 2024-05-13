use std::{fs, path::Path};

use anyhow::bail;
use serde::{Deserialize, Serialize};

use crate::{disk::repart::PartitionSchema, Anyhow};

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
    pub fn system_size_bytes(&self) -> Anyhow<u64> {
        size_to_bytes(self.system_size())
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

/// Converts a size string to bytes.
fn size_to_bytes(size: &str) -> Anyhow<u64> {
    if size.is_empty() {
        bail!("invalid system size: must not be empty");
    }
    let unit = &size[size.len() - 1..];
    let size = size[..size.len() - 1].parse::<u64>()?;
    match unit {
        "G" => Ok(size * (1 << 30)),
        "M" => Ok(size * (1 << 20)),
        _ => bail!("unsupported unit {unit}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_to_bytes() {
        assert_eq!(size_to_bytes("128M").unwrap(), 134217728);
        assert_eq!(size_to_bytes("4G").unwrap(), 4294967296);
    }
}
