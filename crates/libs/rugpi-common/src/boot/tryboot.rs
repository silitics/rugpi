use std::io;

use crate::devices;

/// The autoboot configuration for system `A`.
pub const AUTOBOOT_A: &str = "[all]
tryboot_a_b=1
boot_partition=2
[tryboot]
boot_partition=3";

/// The autoboot configuration for system `B`.
pub const AUTOBOOT_B: &str = "[all]
tryboot_a_b=1
boot_partition=3
[tryboot]
boot_partition=2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AutobootSection {
    Unknown,
    All,
    Tryboot,
}

pub fn set_spare_flag() -> Result<(), io::Error> {
    // Instead of rebooting with `reboot "0 tryboot"`, we directly set the
    // required flag via Raspberry Pi's firmware interface. By default,
    // `reboot` should not set any reboot flags, hence, our flags wil not
    // be overwritten. Using `reboot "0 tryboot"` requires support by the
    // kernel and a `reboot` binary that actually passes down the flags to
    // the kernel. This cannot be assumed on all systems. In particular, on
    // Alpine Linux, the `reboot`` binary does not pass down flags.
    devices::rpi::set_tryboot_flag(true)?;
    Ok(())
}

pub fn clear_spare_flag() -> Result<(), io::Error> {
    if devices::rpi::get_tryboot_flag()? {
        devices::rpi::set_tryboot_flag(false)?;
    }
    Ok(())
}
