//! Project configuration.

use core::fmt;
use std::{collections::HashMap, fs, path::Path, str::FromStr};

use clap::ValueEnum;
use reportify::ResultExt;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::{images::ImageConfig, repositories::Source};
use crate::BakeryResult;

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
    pub fn load(path: &Path) -> BakeryResult<Self> {
        toml::from_str(
            &fs::read_to_string(path)
                .whatever_with(|_| format!("reading configuration file from {path:?}"))?,
        )
        .whatever_with(|_| format!("loading configuration file from {path:?}"))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, ValueEnum)]
#[clap(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Architecture {
    Amd64,
    Arm64,
    Armv7,
    Armhf,
    Arm,
}

#[derive(Debug, Error)]
#[error("invalid architecture")]
pub struct InvalidArchitectureError;

impl FromStr for Architecture {
    type Err = InvalidArchitectureError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "amd64" => Ok(Self::Amd64),
            "arm64" => Ok(Self::Arm64),
            "armv7" => Ok(Self::Armv7),
            "armhf" => Ok(Self::Armhf),
            "arm" => Ok(Self::Arm),
            _ => Err(InvalidArchitectureError),
        }
    }
}

impl Architecture {
    pub fn as_str(self) -> &'static str {
        match self {
            Architecture::Amd64 => "amd64",
            Architecture::Arm64 => "arm64",
            Architecture::Armv7 => "armv7",
            Architecture::Armhf => "armhf",
            Architecture::Arm => "arm",
        }
    }
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
