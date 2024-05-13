use std::{fs::File, io::Write};

use anyhow::bail;

use crate::{
    partitions::{make_config_writeable, PartitionSet},
    paths::config_partition_path,
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

pub fn read_default_partitions() -> Anyhow<PartitionSet> {
    parse_autoboot(&std::fs::read_to_string(config_partition_path(
        "autoboot.txt",
    ))?)
}

pub fn commit(hot_partitions: PartitionSet) -> Anyhow<()> {
    let _writable_config = make_config_writeable()?;
    let autoboot_new_path = config_partition_path("autoboot.txt.new");
    let mut autoboot_new = File::create(&autoboot_new_path)?;
    autoboot_new.write_all(
        match hot_partitions {
            PartitionSet::A => AUTOBOOT_A,
            PartitionSet::B => AUTOBOOT_B,
        }
        .as_bytes(),
    )?;
    autoboot_new.flush()?;
    autoboot_new.sync_all()?;
    std::fs::rename(autoboot_new_path, config_partition_path("autoboot.txt"))?;
    Ok(())
}
