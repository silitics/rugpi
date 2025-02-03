//! Functionality related to finding a system's root device.

use std::path::Path;

use tracing::error;

use super::paths;
use rugix_common::disk::blkdev::{find_block_device, BlockDevice};
use rugix_common::disk::PartitionTable;

/// Find the system block device.
pub fn find_system_device() -> Option<BlockDevice> {
    find_block_device(if Path::new(paths::MOUNT_POINT_SYSTEM).exists() {
        paths::MOUNT_POINT_SYSTEM
    } else {
        "/"
    })
    .inspect_err(|error| error!("error determining system block device: {error}"))
    .ok()
    .flatten()
}

/// System root device.
#[derive(Debug, Clone)]
pub struct SystemRoot {
    /// Root device.
    pub device: BlockDevice,
    /// Partition table of the root device.
    pub table: Option<PartitionTable>,
}

impl SystemRoot {
    /// Obtain the system root device from the provided system device.
    pub fn from_system_device(system_device: &BlockDevice) -> Option<Self> {
        system_device
            .find_parent()
            .inspect_err(|error| error!("error determining system device's parent: {error}"))
            .ok()
            .flatten()
            .map(|root_device| {
                let table = PartitionTable::read(&root_device)
                    .inspect_err(|error| {
                        error!("error reading partition table from root device: {error:?}")
                    })
                    .ok();
                SystemRoot {
                    device: root_device,
                    table,
                }
            })
    }

    /// Resolve a partition.
    pub fn resolve_partition(&self, partition: u32) -> Option<BlockDevice> {
        self.device
            .get_partition(partition)
            .inspect_err(|error| {
                error!("error resoling partition {partition} of root device: {error}")
            })
            .ok()
            .flatten()
    }
}
