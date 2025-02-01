//! Data structures and other functionality for the various configuration files.

use std::path::Path;
use std::str::FromStr;
use std::{fmt, fs};

use images::{Filesystem, ImageConfig, PartitionTableType};
use projects::ProjectConfig;
use rugix_tasks::check_canceled;
use serde::Deserialize;

use reportify::{whatever, ResultExt};

use crate::BakeryResult;

use self::recipes::ParameterValue;
use self::systems::Architecture;

sidex::include_bundle! {
    #[doc(hidden)]
    rugix_bakery as generated
}
// Re-export the generated data structures.
pub use generated::*;

pub mod errors {
    //! Error types.

    use thiserror::Error;

    #[derive(Debug, Error)]
    #[error("invalid architecture")]
    pub struct InvalidArchitectureError;
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

impl FromStr for Architecture {
    type Err = errors::InvalidArchitectureError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "amd64" => Ok(Self::Amd64),
            "arm64" => Ok(Self::Arm64),
            "armv7" => Ok(Self::Armv7),
            "armhf" => Ok(Self::Armhf),
            "arm" => Ok(Self::Arm),
            _ => Err(errors::InvalidArchitectureError),
        }
    }
}

impl fmt::Display for ParameterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterValue::String(value) => value.fmt(f),
            ParameterValue::Boolean(value) => value.fmt(f),
            ParameterValue::Integer(value) => value.fmt(f),
            ParameterValue::Float(value) => value.fmt(f),
        }
    }
}

impl ProjectConfig {
    /// Retrieve the configuration of the image with the provided name.
    pub fn get_image_config(&self, name: &str) -> Option<&ImageConfig> {
        self.images.as_ref().and_then(|images| images.get(name))
    }

    /// Resolve the name of an image.
    pub fn resolve_image_config(&self, name: &str) -> BakeryResult<&ImageConfig> {
        self.get_image_config(name)
            .ok_or_else(|| whatever!("unable to to find image {name:?}"))
    }
}

impl Filesystem {
    /// Name of the filesystem.
    pub fn name(&self) -> &'static str {
        match self {
            Filesystem::Ext4 => "ext4",
            Filesystem::Fat32 => "fat32",
        }
    }
}

impl PartialEq<rugpi_common::disk::PartitionTableType> for PartitionTableType {
    fn eq(&self, other: &rugpi_common::disk::PartitionTableType) -> bool {
        match (self, other) {
            (PartitionTableType::Mbr, rugpi_common::disk::PartitionTableType::Mbr) => true,
            (PartitionTableType::Gpt, rugpi_common::disk::PartitionTableType::Gpt) => true,
            _ => false,
        }
    }
}

/// Parse a configuration of type `T` from the provided string.
pub fn parse_config<T>(config: &str) -> BakeryResult<T>
where
    T: 'static + for<'de> Deserialize<'de>,
{
    toml::from_str(&config).whatever("unable to parse configuration file")
}

/// Load a configuration of type `T` from the provided path.
pub fn load_config<T>(path: &Path) -> BakeryResult<T>
where
    T: 'static + for<'de> Deserialize<'de>,
{
    check_canceled();
    parse_config(&fs::read_to_string(path).whatever("unable to read configuration file")?)
        .with_info(|_| format!("loading configuration from {path:?}"))
}
