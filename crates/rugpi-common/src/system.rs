use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};
use xscript::{run, Run};

use crate::{
    boot::{detect_boot_flow, grub, tryboot, uboot, BootFlow},
    ctrl_config::Config,
    partitions::{get_hot_partitions, read_default_partitions, PartitionSet, Partitions},
    paths::MOUNT_POINT_CONFIG,
    Anyhow,
};

pub struct System {
    boot_flow: BootFlow,
    hot_partitions: PartitionSet,
    default_partitions: PartitionSet,
    /// Configuration partition of the system.
    config_partition: Option<ConfigPartition>,
}

impl System {
    pub fn initialize(config: &Config) -> Anyhow<Self> {
        let partitions = Partitions::load(config).context("loading partitions")?;
        let config_partition = ConfigPartition::new(MOUNT_POINT_CONFIG.into());
        let boot_flow = detect_boot_flow(&config_partition).context("detecting boot flow")?;
        let hot_partitions =
            get_hot_partitions(&partitions).context("determining hot partitions")?;
        let default_partitions =
            read_default_partitions(&config_partition).context("reading default partitions")?;
        Ok(Self {
            boot_flow,
            hot_partitions,
            default_partitions,
            config_partition: Some(config_partition),
        })
    }

    pub fn hot_partitions(&self) -> PartitionSet {
        self.hot_partitions
    }

    pub fn spare_partitions(&self) -> PartitionSet {
        self.hot_partitions.flipped()
    }

    pub fn default_partitions(&self) -> PartitionSet {
        self.default_partitions
    }

    pub fn needs_commit(&self) -> bool {
        self.hot_partitions != self.default_partitions
    }

    pub fn boot_flow(&self) -> BootFlow {
        self.boot_flow
    }

    pub fn config_partition(&self) -> Option<&ConfigPartition> {
        self.config_partition.as_ref()
    }

    pub fn require_config_partition(&self) -> Anyhow<&ConfigPartition> {
        self.config_partition()
            .ok_or_else(|| anyhow!("config partition is required"))
    }

    pub fn commit(&self) -> Anyhow<()> {
        match self.boot_flow {
            BootFlow::Tryboot => tryboot::commit(self),
            BootFlow::UBoot => uboot::commit(self),
            BootFlow::GrubEfi => grub::commit(self),
        }
    }
}

/// Config partition of the system.
#[derive(Debug)]
pub struct ConfigPartition {
    /// Path where the config partition is mounted.
    path: PathBuf,
    /// Indicates whether the config partition is write-protected.
    ///
    /// If set, the config partition must be mounted writable prior to any modifications.
    protected: bool,
    /// Count of currently active writers.
    ///
    /// This is used to keep track of when to mount the partition read-only again.
    writer_count: Mutex<usize>,
}

impl ConfigPartition {
    /// Create an new config partition with the given path.
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            protected: true,
            writer_count: Mutex::new(0),
        }
    }

    /// Set whether the config partition is write-protected.
    pub fn with_protected(mut self, protected: bool) -> Self {
        self.protected = protected;
        self
    }

    /// Path of the config partition.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Ensure that the partition is writable while the closure runs.
    pub fn ensure_writable<U, F: FnOnce() -> U>(&self, closure: F) -> Anyhow<U> {
        let _ = self.acquire_write_guard()?;
        Ok(closure())
    }

    /// Make the partition writable and return a guard.
    ///
    /// When the guard is dropped, the partition may become read-only again.
    fn acquire_write_guard(&self) -> Anyhow<ConfigPartitionWriteGuard> {
        let mut writer_count = self.writer_count.lock().unwrap();
        if self.protected && *writer_count == 0 {
            run!(["mount", "-o", "remount,rw", &self.path])?;
        }
        *writer_count += 1;
        Ok(ConfigPartitionWriteGuard(self))
    }
}

/// Guard for making the config partition writable.
#[derive(Debug)]
struct ConfigPartitionWriteGuard<'c>(&'c ConfigPartition);

impl<'c> Drop for ConfigPartitionWriteGuard<'c> {
    fn drop(&mut self) {
        let mut writer_count = self.0.writer_count.lock().unwrap();
        if self.0.protected && *writer_count == 1 {
            let _ = run!(["mount", "-o", "remount,ro", &self.0.path]);
        }
        *writer_count -= 1;
    }
}

/// Configuration of the config partition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigPartitionConfig {
    /// Path where the config partition is mounted.
    pub path: Option<String>,
    /// Block device of the config partition.
    pub device: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SystemConfig {
    config_device: Option<String>,
    data_device: Option<String>,
    boot_flow: Option<String>,
    slots: Option<HashMap<String, SlotConfig>>,
    boot_entries: Option<HashMap<String, BootEntryConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum BootFlowConfig {
    GrubEfi,
    Tryboot,
    UBoot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SlotConfig {
    Partition(PartitionSlotConfig),
    Directory(DirectorySlotConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionSlotConfig {
    device: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectorySlotConfig {
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootEntryConfig {
    slots: HashMap<String, String>,
}
