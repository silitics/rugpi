//! Functionality for working with the data and config partition of a system.

use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::bail;
use tracing::{error, warn};
use xscript::{run, Run};

use super::{config::PartitionConfig, paths, root::SystemRoot};
use crate::{disk::blkdev::BlockDevice, Anyhow};

/// Resolve the data partition block device.
pub fn resolve_data_partition(
    root: Option<&SystemRoot>,
    config: &PartitionConfig,
) -> Option<BlockDevice> {
    resolve_partition(root, config, || {
        match root.map(|root| root.table.as_ref()).flatten() {
            Some(table) => Ok(if table.is_mbr() { 7 } else { 6 }),
            None => {
                bail!("no root device partition table")
            }
        }
    })
    .inspect_err(|error| error!("error resolving data partition: {error}"))
    .ok()
    .flatten()
}

/// Resolve the config partition block device.
pub fn resolve_config_partition(
    root: Option<&SystemRoot>,
    config: &PartitionConfig,
) -> Option<BlockDevice> {
    resolve_partition(root, config, || Ok(1))
        .inspect_err(|error| error!("error resolving config partition: {error}"))
        .ok()
        .flatten()
}

/// Resolve a partition block device based on the given config and default.
fn resolve_partition(
    root: Option<&SystemRoot>,
    config: &PartitionConfig,
    default: impl FnOnce() -> Anyhow<u32>,
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
        let partition = match config.partition {
            Some(partition) => partition,
            None => default()?,
        };
        if let Some(root) = root {
            match root.resolve_partition(partition) {
                Some(device) => device,
                None => bail!("unable to resolve partition {partition}: partition not found"),
            }
        } else {
            bail!("unable to resolve partition {partition}: no root device")
        }
    };
    Ok(Some(device.into()))
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
    writer_count: Mutex<u32>,
}

impl ConfigPartition {
    pub fn from_config(config: &PartitionConfig) -> Option<Self> {
        if config.disabled {
            None
        } else {
            Some(Self::new(
                config
                    .path
                    .as_deref()
                    .unwrap_or(paths::MOUNT_POINT_CONFIG)
                    .into(),
                config.protected.unwrap_or(true),
            ))
        }
    }

    /// Create an new config partition with the given path.
    fn new(path: PathBuf, protected: bool) -> Self {
        Self {
            path,
            protected,
            writer_count: Mutex::new(0),
        }
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
        *writer_count = writer_count
            .checked_add(1)
            .expect("writer count should never overflow");
        Ok(ConfigPartitionWriteGuard(self))
    }
}

/// Guard for making the config partition writable.
#[derive(Debug)]
struct ConfigPartitionWriteGuard<'p>(&'p ConfigPartition);

impl<'p> Drop for ConfigPartitionWriteGuard<'p> {
    fn drop(&mut self) {
        let mut writer_count = self.0.writer_count.lock().unwrap();
        if self.0.protected && *writer_count == 1 {
            let _ = run!(["mount", "-o", "remount,ro", &self.0.path]);
        }
        *writer_count -= 1;
    }
}
