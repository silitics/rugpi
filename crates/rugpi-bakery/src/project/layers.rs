use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use reportify::ResultExt;
use serde::{Deserialize, Serialize};

use super::config::Architecture;
use super::recipes::{ParameterValue, RecipeName};
use super::repositories::RepositoryIdx;
use crate::utils::caching::ModificationTime;
use crate::BakeryResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerConfig {
    /// Optional human-readable name of the layer.
    pub name: Option<String>,
    /// An URL to fetch the layer from.
    pub url: Option<String>,
    pub parent: Option<String>,
    #[serde(default)]
    pub root: bool,
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
    pub fn load(path: &Path) -> BakeryResult<Self> {
        toml::from_str(
            &fs::read_to_string(path)
                .whatever_with(|_| format!("unable to read layer file from {path:?}"))?,
        )
        .whatever_with(|_| format!("unable to parse layer file from {path:?}"))
    }
}

#[derive(Debug)]
pub struct Layer {
    pub name: String,
    pub repo: RepositoryIdx,
    pub modified: ModificationTime,
    pub default_config: Option<LayerConfig>,
    pub arch_configs: HashMap<Architecture, LayerConfig>,
}

impl Layer {
    pub fn new(name: String, repo: RepositoryIdx, modified: ModificationTime) -> Self {
        Self {
            name,
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
