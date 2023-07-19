//! Data structures for representing a Rugpi Bakery configuration.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::recipes::{ParameterValue, RecipeName};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BakeryConfig {
    /// The recipes to include.
    #[serde(default)]
    pub recipes: HashSet<RecipeName>,
    /// The recipes to exclude.
    #[serde(default)]
    pub exclude: HashSet<RecipeName>,
    /// The image configuration.
    #[serde(default)]
    pub system: SystemConfig,
    /// Parameters for the recipes.
    #[serde(default)]
    pub parameters: HashMap<RecipeName, HashMap<String, ParameterValue>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SystemConfig {
    /// The size of the system partition, "4G" by default.
    pub system_size: Option<String>,
}
