//! Project configuration.

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use anyhow::Context;
use rugpi_common::{boot::BootFlow, Anyhow};
use serde::{Deserialize, Serialize};

use super::{
    recipes::{ParameterValue, RecipeName},
    repositories::Source,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BakeryConfig {
    /// The repositories to use.
    #[serde(default)]
    pub repositories: HashMap<String, Source>,
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
    /// The target architecture to build an image for.
    #[serde(default)]
    pub architecture: Architecture,
    /// Indicates which boot flow to use for the image.
    #[serde(default)]
    pub boot_flow: BootFlow,
}

impl BakeryConfig {
    /// Load the configuration from the given path.
    pub fn load(path: &Path) -> Anyhow<Self> {
        toml::from_str(
            &fs::read_to_string(path)
                .with_context(|| format!("reading configuration file from {path:?}"))?,
        )
        .with_context(|| format!("loading configuration file from {path:?}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IncludeFirmware {
    None,
    #[default]
    Pi4,
    Pi5,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Architecture {
    #[default]
    Arm64,
    Armhf,
}

impl Architecture {
    pub fn as_str(self) -> &'static str {
        match self {
            Architecture::Arm64 => "arm64",
            Architecture::Armhf => "armhf",
        }
    }
}
