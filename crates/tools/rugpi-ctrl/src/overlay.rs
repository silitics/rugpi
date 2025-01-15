use std::path::{Path, PathBuf};

use rugpi_common::system::boot_groups::BootGroup;

/// Get the overlay directory for the given partition set.
pub fn overlay_dir(entry: &BootGroup) -> PathBuf {
    Path::new("/run/rugpi/state/overlay").join(entry.name())
}
