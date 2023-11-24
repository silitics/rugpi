use std::{fs, os::unix::prelude::FileTypeExt, path::Path, sync::OnceLock};

use anyhow::{anyhow, bail};
use camino::Utf8Path;
use xscript::{read_str, run, Run};

use self::devices::{SD_PART_SYSTEM_A, SD_PART_SYSTEM_B};
use crate::{
    boot::{uboot::UBootEnv, BootFlow},
    Anyhow,
};

pub mod devices {
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
}

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

pub fn find_dev(path: impl AsRef<str>) -> Anyhow<String> {
    Ok(read_str!([
        FINDMNT, "-n", "-o", "SOURCE", "--target", path
    ])?)
}

pub fn system_dev() -> Anyhow<&'static Utf8Path> {
    static SYSTEM_DEV: OnceLock<Anyhow<String>> = OnceLock::new();
    SYSTEM_DEV
        .get_or_init(|| find_dev("/run/rugpi/mounts/system"))
        .as_ref()
        .map(Utf8Path::new)
        .map_err(|error| anyhow!("error retrieving system device: {error}"))
}

pub fn get_hot_partitions() -> Anyhow<PartitionSet> {
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

pub fn get_boot_flow() -> Anyhow<BootFlow> {
    if Path::new("/run/rugpi/mounts/config/autoboot.txt").exists() {
        Ok(BootFlow::Tryboot)
    } else if Path::new("/run/rugpi/mounts/config/boot.scr").exists() {
        Ok(BootFlow::UBoot)
    } else {
        bail!("Unable to determine boot flow.");
    }
}

pub fn get_default_partitions() -> Anyhow<PartitionSet> {
    match get_boot_flow()? {
        BootFlow::Tryboot => {
            let autoboot_txt = fs::read_to_string("/run/rugpi/mounts/config/autoboot.txt")?;
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
        }
        BootFlow::UBoot => {
            let bootpart_env = UBootEnv::load("/run/rugpi/mounts/config/bootpart.default.env")?;
            let Some(bootpart) = bootpart_env.get("bootpart") else {
                bail!("Invalid bootpart environment.");
            };
            if bootpart == "2" {
                return Ok(PartitionSet::A);
            } else if bootpart == "3" {
                return Ok(PartitionSet::B);
            }
        }
    }
    bail!("Unable to determine default partition set.");
}

pub fn cold_partition_set() -> Anyhow<PartitionSet> {
    Ok(get_hot_partitions()?.flipped())
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
            PartitionSet::A => Utf8Path::new(devices::SD_PART_SYSTEM_A),
            PartitionSet::B => Utf8Path::new(devices::SD_PART_SYSTEM_B),
        }
    }

    pub fn boot_dev(self) -> &'static Utf8Path {
        match self {
            PartitionSet::A => Utf8Path::new(devices::SD_PART_BOOT_A),
            PartitionSet::B => Utf8Path::new(devices::SD_PART_BOOT_B),
        }
    }

    pub fn flipped(self) -> Self {
        match self {
            PartitionSet::A => Self::B,
            PartitionSet::B => Self::A,
        }
    }
}

/// The size of the config partition.
const CONFIG_PART_SIZE: &str = "256M";
/// The size of the boot partitions.
const BOOT_PART_SIZE: &str = "128M";

/// The `sfdisk` partition layout for images.
pub fn sfdisk_image_layout() -> String {
    indoc::formatdoc! { r#"
        label: dos
        unit: sectors
        grain: 4M
        
        config   : type=0c, size={CONFIG_PART_SIZE}
        boot-a   : type=0c, size={BOOT_PART_SIZE}
        boot-b   : type=0c, size={BOOT_PART_SIZE}
        
        extended : type=05
        
        system-a : type=83 
    "# }
}

/// The `sfdisk` partition layout for a Rugpi system.
pub fn sfdisk_system_layout(system_size: &str) -> String {
    indoc::formatdoc! { r#"
        label: dos
        unit: sectors
        grain: 4M
        
        config   : type=0c, size={CONFIG_PART_SIZE}
        boot-a   : type=0c, size={BOOT_PART_SIZE}
        boot-b   : type=0c, size={BOOT_PART_SIZE}
        
        extended : type=05
        
        system-a : type=83, size={system_size}
        system-b : type=83, size={system_size}
        data     : type=83
    "# }
}

/// The `sfdisk` executable.
const SFDISK: &str = "/usr/sbin/sfdisk";
/// The `partprobe` executable.
const PARTPROBE: &str = "/usr/sbin/partprobe";

/// Returns the disk id of the provided image or device.
pub fn get_disk_id(path: impl AsRef<str>) -> Anyhow<String> {
    fn _disk_id(path: &str) -> Anyhow<String> {
        Ok(read_str!([SFDISK, "--disk-id", path])?
            .strip_prefix("0x")
            .ok_or_else(|| anyhow!("`sfdisk` returned invalid disk id"))?
            .to_owned())
    }
    _disk_id(path.as_ref())
}

/// Partitions an image or device with the provided layout.
pub fn sfdisk_apply_layout(path: impl AsRef<str>, layout: impl AsRef<str>) -> Anyhow<()> {
    fn _sfdisk_apply_layout(path: &str, layout: &str) -> Anyhow<()> {
        run!([SFDISK, "--no-reread", path].with_stdin(layout))?;
        if is_block_dev(path) {
            run!([PARTPROBE, path])?;
        }
        Ok(())
    }
    _sfdisk_apply_layout(path.as_ref(), layout.as_ref())
}

/// The `mkfs.ext4` executable.
const MKFS_ETX4: &str = "/usr/sbin/mkfs.ext4";
/// The `mkfs.vfat` executable.
const MKFS_VFAT: &str = "/usr/sbin/mkfs.vfat";

/// Formats a boot partition with FAT32.
pub fn mkfs_vfat(dev: impl AsRef<str>, label: impl AsRef<str>) -> Anyhow<()> {
    run!([MKFS_VFAT, "-n", label, dev])?;
    Ok(())
}

/// Formats a system partition with EXT4.
pub fn mkfs_ext4(dev: impl AsRef<str>, label: impl AsRef<str>) -> Anyhow<()> {
    run!([MKFS_ETX4, "-F", "-L", label, dev])?;
    Ok(())
}
