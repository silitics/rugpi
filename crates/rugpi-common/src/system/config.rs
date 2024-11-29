//! System configuration.

use std::{fs, path::Path};

use anyhow::Context;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::Anyhow;

/// Path of the system configuration file.
pub const SYSTEM_CONFIG_PATH: &str = "/etc/rugpi/system.toml";

/// Load the system configuration.
pub fn load_system_config() -> Anyhow<SystemConfig> {
    Ok(if Path::new(SYSTEM_CONFIG_PATH).exists() {
        toml::from_str(
            &fs::read_to_string(SYSTEM_CONFIG_PATH).context("reading system configuration file")?,
        )
        .context("parsing system configuration file")?
    } else {
        SystemConfig::default()
    })
}

/// System configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct SystemConfig {
    /// Configuration of the config partition.
    #[serde(default)]
    pub config_partition: PartitionConfig,
    /// Configuration of the data partition.
    #[serde(default)]
    pub data_partition: PartitionConfig,
    /// Configuration of the boot flow.
    pub boot_flow: Option<BootFlowConfig>,
    /// Configuration of the system's update slots.
    pub slots: Option<SlotsConfig>,
    /// Configuration of the system's boot entries.
    pub boot_groups: Option<BootGroupsConfig>,
}

/// Configuration of the system's update slots.
pub type SlotsConfig = IndexMap<String, SlotConfig>;

/// Configuration of the system's boot groups.
pub type BootGroupsConfig = IndexMap<String, BootGroupConfig>;

/// Partition configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct PartitionConfig {
    /// Indicates whether the partition has been disabled.
    #[serde(default)]
    pub disabled: bool,
    /// Path to the partition block device.
    pub device: Option<String>,
    /// Partition number of the root device.
    pub partition: Option<u32>,
    /// Path where the partition is or should be mounted.
    pub path: Option<String>,
    /// Indicates whether the partition is write-protected.
    pub protected: Option<bool>,
}

/// Boot flow configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, tag = "type", rename_all = "kebab-case")]
pub enum BootFlowConfig {
    /// Rugpi-native Tryboot boot flow.
    Tryboot,
    /// Rugpi-native Grub EFI boot flow.
    GrubEfi,
    /// Rugpi-native U-Boot boot flow.
    UBoot,
}

/// Slot configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SlotConfig {
    /// Kind of the slot configuration.
    #[serde(flatten)]
    pub kind: SlotConfigKind,
    /// Indicates whether the slot is protected.
    ///
    /// Protected slots cannot normally be upgraded.
    #[serde(default)]
    pub protected: bool,
}

/// Kind of a slot configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SlotConfigKind {
    /// Block slot.
    Block(BlockSlotConfig),
}

/// Block slot configuration.
///
/// A block slot is simply a block device where a any image can be installed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct BlockSlotConfig {
    /// Path to a block device.
    pub device: Option<String>,
    /// Partition number of the root device's parent.
    pub partition: Option<u32>,
}

/// Boot group configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct BootGroupConfig {
    /// Slots used by the boot group.
    ///
    /// The map introduces aliases for slots.
    pub slots: IndexMap<String, String>,
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::SystemConfig;

    #[test]
    fn test_from_toml() {
        toml::from_str::<SystemConfig>(indoc! {r#"
            [config-partition]
            disabled = false
            device = "/dev/sda1"

            [data-partition]
            disabled = false
            partition = 7

            [boot-flow]
            type = "u-boot"

            [slots.boot-a]
            type = "block"
            partition = 2

            [slots.boot-b]
            type = "block"
            device = "/dev/sda3"

            [slots.system-a]
            type = "block"
            device = "/dev/sda4"

            [slots.system-b]
            type = "block"
            device = "/dev/sda5"

            [slots.app-config]
            type = "block"
            device = "/dev/sda6"
            protected = true

            [boot-groups.a]
            slots = { boot = "boot-a", system = "system-a" }

            [boot-groups.b]
            slots = { boot = "boot-b", system = "system-b" }
        "#})
        .unwrap();
    }
}
