//! Utilities for block devices.

use std::{
    fs::File,
    io,
    os::{fd::AsRawFd, unix::fs::FileTypeExt},
    path::Path,
};

/// Check whether the provided path is a block device.
pub fn is_block_device(dev: impl AsRef<Path>) -> bool {
    dev.as_ref()
        .metadata()
        .map(|metadata| metadata.file_type().is_block_device())
        .unwrap_or(false)
}

/// Get the size of a block device in bytes.
pub fn block_device_get_size(dev: impl AsRef<Path>) -> io::Result<u64> {
    use nix::{ioctl_read, libc::c_ulonglong};

    let file = File::open(dev)?;

    ioctl_read! {
        /// Get the size of the block device in bytes.
        ioctl_get_size, 0x12, 114, c_ulonglong
    }

    let mut size = 0;
    unsafe { ioctl_get_size(file.as_raw_fd(), &mut size) }?;
    Ok(size)
}
