//! Data structures for representing a Rugpi Bakery configuration.

use std::{
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
};

use rugpi_common::{boot::BootFlow, Anyhow};
use serde::{Deserialize, Serialize};

use crate::{
    recipes::{ParameterValue, RecipeName},
    Args,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BakeryConfig {
    /// The recipes to include.
    #[serde(default)]
    pub recipes: HashSet<RecipeName>,
    /// The recipes to exclude.
    #[serde(default)]
    pub exclude: HashSet<RecipeName>,
    /// Parameters for the recipes.
    #[serde(default)]
    pub parameters: HashMap<RecipeName, HashMap<String, ParameterValue>>,
    /// Indicates whether to include firmware files in the image.
    #[serde(default)]
    pub include_firmware: IncludeFirmware,
    /// Indicates which boot flow to use for the image.
    #[serde(default)]
    pub boot_flow: BootFlow,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IncludeFirmware {
    None,
    #[default]
    Pi4,
    Pi5,
}

/// Load the configuration file from the current directory.
pub fn load_config(args: &Args) -> Anyhow<BakeryConfig> {
    let current_dir = PathBuf::try_from(env::current_dir()?)?;
    let config_path = args
        .config
        .as_deref()
        .unwrap_or_else(|| Path::new("rugpi-bakery.toml"));
    Ok(toml::from_str(&fs::read_to_string(
        current_dir.join(config_path),
    )?)?)
}
