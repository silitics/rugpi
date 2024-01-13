use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use anyhow::Context;
use rugpi_common::Anyhow;
use serde::{Deserialize, Serialize};

use super::{
    config::Architecture,
    recipes::{ParameterValue, RecipeName},
    repositories::RepositoryIdx,
};
use crate::caching::ModificationTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerConfig {
    /// Optional human-readable name of the layer.
    pub name: Option<String>,
    /// An URL to fetch the layer from.
    pub url: Option<String>,
    pub parent: Option<String>,
    /// The recipes to include.
    #[serde(default)]
    pub recipes: HashSet<RecipeName>,
    /// The recipes to exclude.
    #[serde(default)]
    pub exclude: HashSet<RecipeName>,
    /// Parameters for the recipes.
    #[serde(default)]
    pub parameters: HashMap<RecipeName, HashMap<String, ParameterValue>>,
}

impl LayerConfig {
    pub fn load(path: &Path) -> Anyhow<Self> {
        toml::from_str(
            &fs::read_to_string(path)
                .with_context(|| format!("error reading layer config from `{path:?}`"))?,
        )
        .with_context(|| format!("error parsing layer config from path `{path:?}`"))
    }
}

#[derive(Debug)]
pub struct Layer {
    pub repo: RepositoryIdx,
    pub modified: ModificationTime,
    pub default_config: Option<LayerConfig>,
    pub arch_configs: HashMap<Architecture, LayerConfig>,
}

impl Layer {
    pub fn new(repo: RepositoryIdx, modified: ModificationTime) -> Self {
        Self {
            repo,
            modified,
            default_config: None,
            arch_configs: HashMap::new(),
        }
    }

    /// The layer configuration for the given architecture.
    pub fn config(&self, arch: Architecture) -> Option<&LayerConfig> {
        self.arch_configs
            .get(&arch)
            .or(self.default_config.as_ref())
    }
}
