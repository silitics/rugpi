use std::{
    os::unix::prelude::FileTypeExt,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::{anyhow, bail};
use xscript::{read_str, run, Run};

use crate::{
    boot::{detect_boot_flow, grub, tryboot, uboot, BootFlow},
    ctrl_config::Config,
    disk::{
        repart::{generic_efi_partition_schema, generic_mbr_partition_schema, PartitionSchema},
        PartitionTable,
    },
    paths::MOUNT_POINT_SYSTEM,
    Anyhow,
};

/// The partitions used by Rugpi.
pub struct Partitions {
    pub parent_dev: PathBuf,
    pub config: PathBuf,
    pub boot_a: Option<PathBuf>,
    pub boot_b: Option<PathBuf>,
    pub system_a: PathBuf,
    pub system_b: PathBuf,
    pub data: PathBuf,
    pub schema: Option<PartitionSchema>,
}

/// The `findmnt` executable.
const LSBLK: &str = "/usr/bin/lsblk";

impl Partitions {
    pub fn load(config: &Config) -> Anyhow<Self> {
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
        let table = PartitionTable::read(&parent_dev_path)?;
        if table.is_mbr() {
            Ok(Self {
                parent_dev: parent_dev_path,
                config: PathBuf::from(format!("/dev/{partition_dev_name}1")),
                boot_a: Some(PathBuf::from(format!("/dev/{partition_dev_name}2"))),
                boot_b: Some(PathBuf::from(format!("/dev/{partition_dev_name}3"))),
                system_a: PathBuf::from(format!("/dev/{partition_dev_name}5")),
                system_b: PathBuf::from(format!("/dev/{partition_dev_name}6")),
                data: PathBuf::from(format!("/dev/{partition_dev_name}7")),
                schema: Some(generic_mbr_partition_schema(config.system_size_bytes()?)),
            })
        } else {
            Ok(Self {
                parent_dev: parent_dev_path,
                config: PathBuf::from(format!("/dev/{partition_dev_name}1")),
                boot_a: Some(PathBuf::from(format!("/dev/{partition_dev_name}2"))),
                boot_b: Some(PathBuf::from(format!("/dev/{partition_dev_name}3"))),
                system_a: PathBuf::from(format!("/dev/{partition_dev_name}4")),
                system_b: PathBuf::from(format!("/dev/{partition_dev_name}5")),
                data: PathBuf::from(format!("/dev/{partition_dev_name}6")),
                schema: Some(generic_efi_partition_schema(config.system_size_bytes()?)),
            })
        }
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

pub fn read_default_partitions() -> Anyhow<PartitionSet> {
    match detect_boot_flow()? {
        BootFlow::Tryboot => tryboot::read_default_partitions(),
        BootFlow::UBoot => uboot::read_default_partitions(),
        BootFlow::GrubEfi => grub::read_default_partitions(),
    }
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

    pub fn boot_dev(self, partitions: &Partitions) -> Option<&Path> {
        match self {
            PartitionSet::A => partitions.boot_a.as_deref(),
            PartitionSet::B => partitions.boot_b.as_deref(),
        }
    }

    pub fn flipped(self) -> Self {
        match self {
            PartitionSet::A => Self::B,
            PartitionSet::B => Self::A,
        }
    }
}

/// The `sfdisk` executable.
const SFDISK: &str = "/usr/sbin/sfdisk";
/// The `partprobe` executable.
const PARTPROBE: &str = "/usr/sbin/partprobe";

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

pub struct WritableConfig(());

impl Drop for WritableConfig {
    fn drop(&mut self) {
        run!(["mount", "-o", "remount,ro", "/run/rugpi/mounts/config"]).ok();
    }
}

pub fn make_config_writeable() -> Anyhow<WritableConfig> {
    run!(["mount", "-o", "remount,rw", "/run/rugpi/mounts/config"])?;
    Ok(WritableConfig(()))
}
