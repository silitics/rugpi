//! Constants for paths.

use std::path::{Path, PathBuf};

/// Path where the system's root partition is mounted.
pub const MOUNT_POINT_SYSTEM: &str = "/run/rugpi/mounts/system";

/// Path where the data partition is mounted.
pub const MOUNT_POINT_DATA: &str = "/run/rugpi/mounts/data";

/// Path where the config partition is mounted.
pub const MOUNT_POINT_CONFIG: &str = "/run/rugpi/mounts/config";

pub fn config_partition_path(path: impl AsRef<Path>) -> PathBuf {
    Path::new(MOUNT_POINT_CONFIG).join(path)
}
