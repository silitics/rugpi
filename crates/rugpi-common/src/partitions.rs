use std::{
    fs,
    os::unix::prelude::FileTypeExt,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::{anyhow, bail};
use xscript::{read_str, run, Run};

use crate::{
    boot::{uboot::UBootEnv, BootFlow},
    Anyhow,
};

pub const MOUNT_POINT_SYSTEM: &str = "/run/rugpi/mounts/system";
pub const MOUNT_POINT_DATA: &str = "/run/rugpi/mounts/data";
pub const MOUNT_POINT_CONFIG: &str = "/run/rugpi/mounts/config";

/// The partitions used by Rugpi.
pub struct Partitions {
    pub parent_dev: PathBuf,
    pub config: PathBuf,
    pub boot_a: PathBuf,
    pub boot_b: PathBuf,
    pub system_a: PathBuf,
    pub system_b: PathBuf,
    pub data: PathBuf,
}

/// The `findmnt` executable.
const LSBLK: &str = "/usr/bin/lsblk";

impl Partitions {
    pub fn load() -> Anyhow<Self> {
        let system_dev = if Path::new(MOUNT_POINT_SYSTEM).exists() {
            find_dev(MOUNT_POINT_SYSTEM)?
        } else {
            find_dev("/")?
        };
        if !is_block_dev(&system_dev) {
            bail!("system device {system_dev:?} is not a block device");
        }
        let parent_dev_name = read_str!([LSBLK, "-no", "PKNAME", system_dev])?;
        let parent_dev_path = PathBuf::from(format!("/dev/{parent_dev_name}"));
        if !is_block_dev(&parent_dev_path) {
            bail!("system device parent {parent_dev_path:?} is not a block device");
        }
        let mut partition_dev_name = parent_dev_name.clone();
        if parent_dev_name.ends_with(|c: char| c.is_ascii_digit()) {
            partition_dev_name.push('p');
        }
        Ok(Self {
            parent_dev: parent_dev_path,
            config: PathBuf::from(format!("/dev/{partition_dev_name}1")),
            boot_a: PathBuf::from(format!("/dev/{partition_dev_name}2")),
            boot_b: PathBuf::from(format!("/dev/{partition_dev_name}3")),
            system_a: PathBuf::from(format!("/dev/{partition_dev_name}5")),
            system_b: PathBuf::from(format!("/dev/{partition_dev_name}6")),
            data: PathBuf::from(format!("/dev/{partition_dev_name}7")),
        })
    }
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

pub fn find_dev(path: impl AsRef<Path>) -> Anyhow<PathBuf> {
    Ok(read_str!([FINDMNT, "-n", "-o", "SOURCE", "--target", path.as_ref()])?.into())
}

pub fn system_dev() -> Anyhow<&'static Path> {
    static SYSTEM_DEV: OnceLock<Anyhow<PathBuf>> = OnceLock::new();
    SYSTEM_DEV
        .get_or_init(|| {
            if Path::new(MOUNT_POINT_SYSTEM).exists() {
                find_dev(MOUNT_POINT_SYSTEM)
            } else {
                find_dev("/")
            }
        })
        .as_ref()
        .map(AsRef::as_ref)
        .map_err(|error| anyhow!("error retrieving system device: {error}"))
}

pub fn get_hot_partitions(partitions: &Partitions) -> Anyhow<PartitionSet> {
    let system_dev = system_dev()?;
    if system_dev == partitions.system_a {
        Ok(PartitionSet::A)
    } else if system_dev == partitions.system_b {
        Ok(PartitionSet::B)
    } else {
        bail!("unable to determine hot partition set, invalid device {system_dev:?}")
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
        BootFlow::None => todo!(),
    }
    bail!("Unable to determine default partition set.");
}

pub fn cold_partition_set(partitions: &Partitions) -> Anyhow<PartitionSet> {
    Ok(get_hot_partitions(partitions)?.flipped())
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

    pub fn system_dev(self, partitions: &Partitions) -> &Path {
        match self {
            PartitionSet::A => &partitions.system_a,
            PartitionSet::B => &partitions.system_b,
        }
    }

    pub fn boot_dev(self, partitions: &Partitions) -> &Path {
        match self {
            PartitionSet::A => &partitions.boot_a,
            PartitionSet::B => &partitions.boot_b,
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
pub fn get_disk_id(path: impl AsRef<Path>) -> Anyhow<String> {
    fn _disk_id(path: &Path) -> Anyhow<String> {
        Ok(read_str!([SFDISK, "--disk-id", path])?
            .strip_prefix("0x")
            .ok_or_else(|| anyhow!("`sfdisk` returned invalid disk id"))?
            .to_owned())
    }
    _disk_id(path.as_ref())
}

/// Partitions an image or device with the provided layout.
pub fn sfdisk_apply_layout(path: impl AsRef<Path>, layout: impl AsRef<str>) -> Anyhow<()> {
    fn _sfdisk_apply_layout(path: &Path, layout: &str) -> Anyhow<()> {
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
pub fn mkfs_vfat(dev: impl AsRef<Path>, label: impl AsRef<str>) -> Anyhow<()> {
    run!([MKFS_VFAT, "-n", label.as_ref(), dev.as_ref()])?;
    Ok(())
}

/// Formats a system partition with EXT4.
pub fn mkfs_ext4(dev: impl AsRef<Path>, label: impl AsRef<str>) -> Anyhow<()> {
    run!([MKFS_ETX4, "-F", "-L", label.as_ref(), dev.as_ref()])?;
    Ok(())
}
