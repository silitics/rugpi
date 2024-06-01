use anyhow::bail;
use serde::{Deserialize, Serialize};

use crate::{paths::config_partition_path, Anyhow};

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
pub fn detect_boot_flow() -> Anyhow<BootFlow> {
    if config_partition_path("autoboot.txt").exists() {
        Ok(BootFlow::Tryboot)
    } else if config_partition_path("bootpart.default.env").exists() {
        Ok(BootFlow::UBoot)
    } else if config_partition_path("rugpi/primary.grubenv").exists()
        && config_partition_path("EFI").is_dir()
    {
        Ok(BootFlow::GrubEfi)
    } else {
        bail!("unable to detect boot flow");
    }
}
