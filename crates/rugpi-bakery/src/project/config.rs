//! Project configuration.

use core::fmt;
use std::{collections::HashMap, fs, path::Path, str::FromStr};

use anyhow::Context;
use clap::ValueEnum;
use rugpi_common::Anyhow;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{images::ImageConfig, repositories::Source};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BakeryConfig {
    /// The repositories to use.
    #[serde(default)]
    pub repositories: HashMap<String, Source>,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, Hash, ValueEnum)]
#[clap(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Architecture {
    #[default]
    Arm64,
    Armhf,
    Amd64,
}

#[derive(Debug, Error)]
#[error("invalid architecture")]
pub struct InvalidArchitectureError;

impl FromStr for Architecture {
    type Err = InvalidArchitectureError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "arm64" => Ok(Self::Arm64),
            "armhf" => Ok(Self::Armhf),
            "amd64" => Ok(Self::Amd64),
            _ => Err(InvalidArchitectureError),
        }
    }
}

impl Architecture {
    pub fn as_str(self) -> &'static str {
        match self {
            Architecture::Arm64 => "arm64",
            Architecture::Armhf => "armhf",
            Architecture::Amd64 => "amd64",
        }
    }
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
