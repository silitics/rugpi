use std::path::{Path, PathBuf};

use rugpi_common::partitions::PartitionSet;

/// Get the overlay directory for the given partition set.
pub fn overlay_dir(partitions: PartitionSet) -> PathBuf {
    Path::new("/run/rugpi/state/overlay").join(partitions.as_str())
}
