use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use super::recipes::{ParameterValue, RecipeName};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayerConfig {
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
