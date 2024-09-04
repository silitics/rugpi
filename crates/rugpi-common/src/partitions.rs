use std::path::Path;

use xscript::{read_str, run, Run};

use crate::Anyhow;

pub fn is_dir(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_dir()
}

/// The `sfdisk` executable.
const SFDISK: &str = "/usr/sbin/sfdisk";

/// Returns the disk id of the provided image or device.
pub fn get_disk_id(path: impl AsRef<Path>) -> Anyhow<String> {
    fn _disk_id(path: &Path) -> Anyhow<String> {
        let disk_id = read_str!([SFDISK, "--disk-id", path])?;
        if let Some(dos_id) = disk_id.strip_prefix("0x") {
            Ok(dos_id.to_owned())
        } else {
            Ok(disk_id)
        }
    }
    _disk_id(path.as_ref())
}

/// The `mkfs.ext4` executable.
const MKFS_ETX4: &str = "/usr/sbin/mkfs.ext4";
/// The `mkfs.vfat` executable.
const MKFS_VFAT: &str = "/usr/sbin/mkfs.vfat";

/// Formats a boot partition with FAT32.
pub fn mkfs_vfat(dev: impl AsRef<Path>, label: impl AsRef<str>) -> Anyhow<()> {
    run!([MKFS_VFAT, "-n", label.as_ref(), dev.as_ref()])?;
    Ok(())
}

/// Formats a system partition with EXT4.
pub fn mkfs_ext4(dev: impl AsRef<Path>, label: impl AsRef<str>) -> Anyhow<()> {
    run!([MKFS_ETX4, "-F", "-L", label.as_ref(), dev.as_ref()])?;
    Ok(())
}
