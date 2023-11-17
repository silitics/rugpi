//! Data structures for representing a Rugpi Bakery configuration.

use std::{
    collections::{HashMap, HashSet},
    env, fs,
};

use camino::Utf8PathBuf;
use rugpi_common::Anyhow;
use serde::{Deserialize, Serialize};

use crate::recipes::{ParameterValue, RecipeName};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BakeryConfig {
    /// The recipes to include.
    #[serde(default)]
    pub recipes: HashSet<RecipeName>,
    /// Include firmware files for Pi 4.
    #[serde(default)]
    pub include_firmware: IncludeFirmware,
    /// The recipes to exclude.
    #[serde(default)]
    pub exclude: HashSet<RecipeName>,
    /// Parameters for the recipes.
    #[serde(default)]
    pub parameters: HashMap<RecipeName, HashMap<String, ParameterValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IncludeFirmware {
    None,
    #[default]
    Pi4,
}

/// Load the configuration file from the current directory.
pub fn load_config() -> Anyhow<BakeryConfig> {
    let current_dir = Utf8PathBuf::try_from(env::current_dir()?)?;
    Ok(toml::from_str(&fs::read_to_string(
        current_dir.join("rugpi-bakery.toml"),
    )?)?)
}
