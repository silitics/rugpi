use std::{fs::File, io::Write};

use anyhow::bail;

use crate::{
    devices,
    partitions::PartitionSet,
    system::{ConfigPartition, System},
    Anyhow,
};

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
enum AutobootSection {
    Unknown,
    All,
    Tryboot,
}

pub fn parse_autoboot(autoboot_txt: &str) -> Anyhow<PartitionSet> {
    let mut section = AutobootSection::Unknown;
    for line in autoboot_txt.lines() {
        if line.starts_with("[all]") {
            section = AutobootSection::All;
        } else if line.starts_with("[tryboot]") {
            section = AutobootSection::Tryboot;
        } else if line.starts_with('[') {
            section = AutobootSection::Unknown;
        } else if line.starts_with("boot_partition=2") && section == AutobootSection::All {
            return Ok(PartitionSet::A);
        } else if line.starts_with("boot_partition=3") && section == AutobootSection::All {
            return Ok(PartitionSet::B);
        }
    }
    bail!("unable to determine partition set from `autoboot.txt`");
}

pub fn read_default_partitions(config_partition: &ConfigPartition) -> Anyhow<PartitionSet> {
    parse_autoboot(&std::fs::read_to_string(
        config_partition.path().join("autoboot.txt"),
    )?)
}

pub fn commit(system: &System) -> Anyhow<()> {
    let config_partition = system.require_config_partition()?;
    config_partition.ensure_writable(|| {
        let autoboot_new_path = config_partition.path().join("autoboot.txt.new");
        let mut autoboot_new = File::create(&autoboot_new_path)?;
        autoboot_new.write_all(
            match system.hot_partitions() {
                PartitionSet::A => AUTOBOOT_A,
                PartitionSet::B => AUTOBOOT_B,
            }
            .as_bytes(),
        )?;
        autoboot_new.flush()?;
        autoboot_new.sync_all()?;
        drop(autoboot_new);
        std::fs::rename(
            autoboot_new_path,
            config_partition.path().join("autoboot.txt"),
        )?;
        Ok(())
    })?
}

pub fn set_spare_flag() -> Anyhow<()> {
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
