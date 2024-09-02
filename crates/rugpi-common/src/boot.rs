use anyhow::bail;
use serde::{Deserialize, Serialize};

use crate::{
    system::{ConfigPartition, System},
    Anyhow,
};

pub mod grub;
pub mod tryboot;
pub mod uboot;

/// Rugpi boot flows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BootFlow {
    /// Use the `tryboot` feature for booting and partition switching.
    #[default]
    Tryboot,
    /// Use U-Boot for booting and partition switching.
    #[serde(rename = "u-boot")]
    UBoot,
    /// Use Grub (EFI) for booting and partition switching.
    #[serde(rename = "grub-efi")]
    GrubEfi,
}

impl BootFlow {
    /// The string representation of the boot flow.
    pub fn as_str(self) -> &'static str {
        match self {
            BootFlow::Tryboot => "tryboot",
            BootFlow::UBoot => "u-boot",
            BootFlow::GrubEfi => "grub-efi",
        }
    }
}

/// Dynamically detects the boot flow at runtime.
pub fn detect_boot_flow(config_partition: &ConfigPartition) -> Anyhow<BootFlow> {
    if config_partition.path().join("autoboot.txt").exists() {
        Ok(BootFlow::Tryboot)
    } else if config_partition
        .path()
        .join("bootpart.default.env")
        .exists()
    {
        Ok(BootFlow::UBoot)
    } else if config_partition
        .path()
        .join("rugpi/primary.grubenv")
        .exists()
        && config_partition.path().join("EFI").is_dir()
    {
        Ok(BootFlow::GrubEfi)
    } else {
        bail!("unable to detect boot flow");
    }
}

pub fn set_spare_flag(system: &System) -> Anyhow<()> {
    match system.boot_flow() {
        BootFlow::Tryboot => {
            tryboot::set_spare_flag()?;
        }
        BootFlow::UBoot => {
            uboot::set_spare_flag(system)?;
        }
        BootFlow::GrubEfi => {
            grub::set_spare_flag(system)?;
        }
    }
    Ok(())
}
