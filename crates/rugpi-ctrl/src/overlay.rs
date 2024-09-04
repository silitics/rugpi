use std::path::{Path, PathBuf};

use rugpi_common::system::boot_entries::BootEntry;

/// Get the overlay directory for the given partition set.
pub fn overlay_dir(entry: &BootEntry) -> PathBuf {
    Path::new("/run/rugpi/state/overlay").join(entry.name())
}
