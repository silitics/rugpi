use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::{anyhow, bail};
use boot_entries::{BootEntries, BootEntry, BootEntryIdx};
use boot_flow::BootFlow;
use config::{load_system_config, PartitionConfig, SystemConfig};
use slots::{SlotKind, SystemSlots};
use tracing::warn;
use xscript::{run, Run};

use crate::{
    disk::{
        blkdev::{find_block_device, BlockDevice},
        PartitionTable,
    },
    paths::{MOUNT_POINT_CONFIG, MOUNT_POINT_SYSTEM},
    Anyhow,
};

pub mod boot_entries;
pub mod boot_flow;
pub mod compat;
pub mod config;
pub mod slots;

#[derive(Debug)]
pub struct SystemRoot {
    pub device: Option<BlockDevice>,
    pub parent: Option<BlockDevice>,
    pub table: Option<PartitionTable>,
}

impl SystemRoot {
    pub fn detect() -> Self {
        let device = find_block_device(if Path::new(MOUNT_POINT_SYSTEM).exists() {
            MOUNT_POINT_SYSTEM
        } else {
            "/"
        })
        .inspect_err(|error| warn!("error determining root block device: {error}"))
        .ok()
        .flatten();
        let parent = device.as_ref().and_then(|device| {
            device
                .find_parent()
                .inspect_err(|error| warn!("error determining root device's parent: {error}"))
                .ok()
                .flatten()
        });
        let table = parent.as_ref().and_then(|parent| {
            PartitionTable::read(parent)
                .inspect_err(|error| {
                    warn!("error reading partition table from root device's parent: {error}")
                })
                .ok()
        });
        Self {
            device,
            parent,
            table,
        }
    }

    pub fn resolve_partition(&self, partition: u32) -> Anyhow<Option<BlockDevice>> {
        let Some(parent) = &self.parent else {
            bail!("unable to resolve partition: no parent device");
        };
        Ok(parent.get_partition(partition)?)
    }
}

pub fn detect_config_partition(
    root: &SystemRoot,
    config: &PartitionConfig,
) -> Anyhow<Option<BlockDevice>> {
    if config.disabled {
        return Ok(None);
    }
    let device = if let Some(device) = &config.device {
        if config.partition.is_some() {
            warn!("ignoring `partition` because `device` is set");
        }
        BlockDevice::new(device)?
    } else {
        // By default, the first partition is the config partition.
        let partition = config.partition.unwrap_or(1);
        match root.resolve_partition(partition)? {
            Some(device) => device,
            None => bail!("unable to load config partition: partition not found"),
        }
    };
    Ok(Some(device.into()))
}

pub fn detect_data_partition(
    root: &SystemRoot,
    config: &PartitionConfig,
) -> Anyhow<Option<BlockDevice>> {
    let device = if let Some(device) = &config.device {
        if config.partition.is_some() {
            warn!("ignoring `partition` because `device` is set");
        }
        BlockDevice::new(device)?
    } else {
        let partition = match config.partition {
            Some(partition) => partition,
            None => {
                // The default depends on the partition table of the parent.
                let Some(table) = &root.table else {
                    bail!("unable to determine default data partition: no partition table");
                };
                if table.is_mbr() {
                    7
                } else {
                    6
                }
            }
        };
        match root.resolve_partition(partition)? {
            Some(device) => device,
            None => bail!("unable to load data partition: partition not found"),
        }
    };
    Ok(Some(device.into()))
}

pub struct System {
    config: SystemConfig,
    root: SystemRoot,
    slots: SystemSlots,
    boot_entries: BootEntries,
    active_boot_entry: Option<BootEntryIdx>,
    boot_flow: Box<dyn BootFlow>,
    config_partition: Option<ConfigPartition>,
}

impl System {
    pub fn initialize() -> Anyhow<Self> {
        let system_config = load_system_config()?;
        let system_root = SystemRoot::detect();
        let config_partition = ConfigPartition::from_config(&system_config.config_partition);
        let Some(config_partition) = config_partition else {
            bail!("config partition cannot currently be disabled");
        };
        let slots = SystemSlots::from_config(&system_root, system_config.slots.as_ref())?;
        let boot_entries = BootEntries::from_config(&slots, system_config.boot_entries.as_ref())?;
        // Mark boot entries and slots active.
        let mut active_boot_entry = None;
        for (idx, entry) in boot_entries.iter() {
            for (_, slot) in entry.slots() {
                let SlotKind::Raw(raw) = &slots[slot].kind();
                if Some(raw.device()) == system_root.device.as_ref() {
                    entry.mark_active();
                    break;
                }
                /* TODO: Also look at `/proc/cmdline` to allow setting the active boot
                entry explicitly via a flag `rugpi.boot-entry=...`. For compatibility
                with RAUC, it makes sense to also look at `rauc.slot=...`. This holds
                the name of a RAUC slot, which we could directly map to a Rugpi slot
                assuming that the configuration preserves these names, e.g.:

                [slots."rootfs.0"]
                partition = 2

                [slots."rootfs.1"]
                partition = 3

                [boot-entries.A]
                slots = { rootfs = "rootfs.0" }

                [boot-entries.B]
                slots = { rootfs = "rootfs.1" }
                */
            }
            if entry.active() {
                active_boot_entry = Some(idx);
                // If the entry is active, then so are all its slots.
                for (_, slot) in entry.slots() {
                    slots[slot].mark_active();
                }
                break;
            }
        }
        if active_boot_entry.is_none() {
            warn!("unable to determine active boot entry");
        }
        let boot_flow = boot_flow::from_config(
            system_config.boot_flow.as_ref(),
            &config_partition,
            &boot_entries,
        )?;
        Ok(Self {
            config: system_config,
            root: system_root,
            slots,
            boot_entries,
            active_boot_entry,
            boot_flow,
            config_partition: Some(config_partition),
        })
    }

    pub fn root(&self) -> &SystemRoot {
        &self.root
    }

    pub fn config(&self) -> &SystemConfig {
        &self.config
    }

    pub fn slots(&self) -> &SystemSlots {
        &self.slots
    }

    pub fn boot_entries(&self) -> &BootEntries {
        &self.boot_entries
    }

    pub fn active_boot_entry(&self) -> Option<BootEntryIdx> {
        self.active_boot_entry
    }

    /// First entry that is not the default.
    pub fn spare_entry(&self) -> Anyhow<Option<(BootEntryIdx, &BootEntry)>> {
        let default = self.boot_flow.get_default(self)?;
        Ok(self.boot_entries().iter().find(|(idx, _)| *idx != default))
    }

    pub fn needs_commit(&self) -> Anyhow<bool> {
        Ok(self.active_boot_entry != Some(self.boot_flow.get_default(self)?))
    }

    pub fn boot_flow(&self) -> &dyn BootFlow {
        &*self.boot_flow
    }

    pub fn config_partition(&self) -> Option<&ConfigPartition> {
        self.config_partition.as_ref()
    }

    pub fn require_config_partition(&self) -> Anyhow<&ConfigPartition> {
        self.config_partition()
            .ok_or_else(|| anyhow!("config partition is required"))
    }

    pub fn commit(&self) -> Anyhow<()> {
        self.boot_flow.commit(self)
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
    pub fn from_config(config: &PartitionConfig) -> Option<Self> {
        if config.disabled {
            None
        } else {
            Some(Self::new(
                config.path.as_deref().unwrap_or(MOUNT_POINT_CONFIG).into(),
            ))
        }
    }

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
        let _guard = self.acquire_write_guard()?;
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
