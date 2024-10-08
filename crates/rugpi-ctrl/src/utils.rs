use std::{fs, path::Path};

use rugpi_common::Anyhow;
use xscript::{run, Run};

pub static DEFERRED_SPARE_REBOOT_FLAG: &str = "/run/rugpi/mounts/data/.rugpi/deferred-reboot-spare";

/// Indicates whether the process is the init process.
pub fn is_init_process() -> bool {
    std::process::id() == 1
}

/// Reboot the system.
pub fn reboot() -> Anyhow<()> {
    if is_init_process() {
        // Make sure that no data is lost.
        nix::unistd::sync();
        unsafe {
            // SAFETY: The provided arguments are proper `\0`-terminated strings.
            nix::libc::syscall(
                nix::libc::SYS_reboot,
                nix::libc::LINUX_REBOOT_MAGIC1,
                nix::libc::LINUX_REBOOT_MAGIC2,
                nix::libc::LINUX_REBOOT_CMD_RESTART2,
                b"\0".as_ptr(),
            );
        }
    } else {
        run!(["reboot"])?;
    };
    Ok(())
}

pub fn set_flag(path: impl AsRef<Path>) -> Anyhow<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, "")?;
    Ok(())
}

pub fn clear_flag(path: impl AsRef<Path>) -> Anyhow<()> {
    let path = path.as_ref();
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn is_flag_set(path: impl AsRef<Path>) -> bool {
    path.as_ref().exists()
}
