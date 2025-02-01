use std::collections::HashMap;

use crate::config::layers::LayerConfig;
use crate::config::systems::Architecture;
use crate::utils::caching::ModificationTime;

use super::repositories::RepositoryIdx;

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
