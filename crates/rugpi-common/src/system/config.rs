use serde::{Deserialize, Serialize};

/// System configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SystemConfig {
    /// Configuration of the config partition.
    #[serde(default)]
    pub config_partition: PartitionConfig,
    /// Configuration of the data partition.
    #[serde(default)]
    pub data_partition: PartitionConfig,
}

/// Partition configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PartitionConfig {
    /// Indicates whether the partition has been disabled.
    #[serde(default)]
    pub disabled: bool,
    /// Path to the partition block device.
    pub device: Option<String>,
    /// Partition number of the root device's parent.
    pub partition: Option<u32>,
    /// Path where the partition should be mounted.
    pub path: Option<String>,
}
