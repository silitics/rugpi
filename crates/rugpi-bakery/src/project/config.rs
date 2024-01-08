//! Project configuration.

use std::{collections::HashMap, fs, path::Path};

use anyhow::Context;
use clap::ValueEnum;
use rugpi_common::Anyhow;
use serde::{Deserialize, Serialize};

use super::{images::ImageConfig, layers::LayerConfig, repositories::Source};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BakeryConfig {
    /// The repositories to use.
    #[serde(default)]
    pub repositories: HashMap<String, Source>,
    /// The layers of the project.
    #[serde(default)]
    pub layers: HashMap<String, LayerConfig>,
    /// The images of the project.
    #[serde(default)]
    pub images: HashMap<String, ImageConfig>,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, ValueEnum)]
#[clap(rename_all = "lowercase")]
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
