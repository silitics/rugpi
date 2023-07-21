use std::{fs, os::unix::prelude::FileTypeExt, path::Path, sync::OnceLock};

use anyhow::{anyhow, bail};
use camino::Utf8Path;
use xscript::{read_str, Run};

macro_rules! sd_card_dev_const {
    ($name:ident, $part:literal) => {
        pub const $name: &str = concat!("/dev/mmcblk0", $part);
    };
}

sd_card_dev_const!(SD_CARD, "");
sd_card_dev_const!(SD_PART_CONFIG, "p1");
sd_card_dev_const!(SD_PART_BOOT_A, "p2");
sd_card_dev_const!(SD_PART_BOOT_B, "p3");
sd_card_dev_const!(SD_PART_SYSTEM_A, "p5");
sd_card_dev_const!(SD_PART_SYSTEM_B, "p6");
sd_card_dev_const!(SD_PART_DATA, "p7");

pub fn is_block_dev(dev: impl AsRef<Path>) -> bool {
    let dev = dev.as_ref();
    dev.metadata()
        .map(|metadata| metadata.file_type().is_block_device())
        .unwrap_or(false)
}

pub fn is_dir(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_dir()
}

/// The `findmnt` executable.
const FINDMNT: &str = "/usr/bin/findmnt";

pub fn find_dev(path: impl AsRef<str>) -> anyhow::Result<String> {
    Ok(read_str!([
        FINDMNT, "-n", "-o", "SOURCE", "--target", path
    ])?)
}

pub fn system_dev() -> anyhow::Result<&'static Utf8Path> {
    static SYSTEM_DEV: OnceLock<anyhow::Result<String>> = OnceLock::new();
    SYSTEM_DEV
        .get_or_init(|| find_dev("/run/rugpi/mounts/system"))
        .as_ref()
        .map(|device| Utf8Path::new(device))
        .map_err(|error| anyhow!("error retrieving system device: {error}"))
}

pub fn hot_partition_set() -> anyhow::Result<PartitionSet> {
    let system_dev = system_dev()?.as_str();
    match system_dev {
        SD_PART_SYSTEM_A => Ok(PartitionSet::A),
        SD_PART_SYSTEM_B => Ok(PartitionSet::B),
        _ => bail!("unable to determine hot partition set, invalid device {system_dev}"),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AutobootSection {
    Unknown,
    All,
    Tryboot,
}

pub fn default_partition_set() -> anyhow::Result<PartitionSet> {
    let autoboot_txt = fs::read_to_string("/run/rugpi/mounts/config/autoboot.txt")?;
    let mut section = AutobootSection::Unknown;
    for line in autoboot_txt.lines() {
        if line.starts_with("[all]") {
            section = AutobootSection::All;
        } else if line.starts_with("[tryboot]") {
            section = AutobootSection::Tryboot;
        } else if line.starts_with("[") {
            section = AutobootSection::Unknown;
        } else if line.starts_with("boot_partition=2") {
            if section == AutobootSection::All {
                return Ok(PartitionSet::A);
            }
        } else if line.starts_with("boot_partition=3") {
            if section == AutobootSection::All {
                return Ok(PartitionSet::B);
            }
        }
    }
    bail!("Unable to determine default partition set.");
}

pub fn cold_partition_set() -> anyhow::Result<PartitionSet> {
    Ok(hot_partition_set()?.flipped())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartitionSet {
    A,
    B,
}

impl PartitionSet {
    pub fn as_str(self) -> &'static str {
        match self {
            PartitionSet::A => "a",
            PartitionSet::B => "b",
        }
    }

    pub fn system_dev(self) -> &'static Utf8Path {
        match self {
            PartitionSet::A => Utf8Path::new(SD_PART_SYSTEM_A),
            PartitionSet::B => Utf8Path::new(SD_PART_SYSTEM_B),
        }
    }

    pub fn boot_dev(self) -> &'static Utf8Path {
        match self {
            PartitionSet::A => Utf8Path::new(SD_PART_BOOT_A),
            PartitionSet::B => Utf8Path::new(SD_PART_BOOT_B),
        }
    }

    pub fn flipped(self) -> Self {
        match self {
            PartitionSet::A => Self::B,
            PartitionSet::B => Self::A,
        }
    }
}
