use serde::{Deserialize, Serialize};

/// Structure of the Rugpi Ctrl configuration file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The size of the system partition(s).
    pub system_size: Option<String>,
    /// Indicates what to do with the overlay.
    #[serde(default)]
    pub overlay: Overlay,
}

impl Config {
    /// The size of the system partition(s) (defaults to `4G`).
    pub fn system_size(&self) -> &str {
        self.system_size.as_deref().unwrap_or("4G")
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Overlay {
    Persist,
    #[default]
    Discard,
}
