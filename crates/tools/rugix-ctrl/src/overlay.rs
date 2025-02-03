use std::path::{Path, PathBuf};

use crate::system::boot_groups::BootGroup;

/// Get the overlay directory for the given partition set.
pub fn overlay_dir(entry: &BootGroup) -> PathBuf {
    Path::new("/run/rugix/state/overlay").join(entry.name())
}
