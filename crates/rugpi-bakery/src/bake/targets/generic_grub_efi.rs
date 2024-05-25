use std::path::Path;

use anyhow::bail;
use rugpi_common::{fsutils::copy_recursive, Anyhow};

use crate::project::{config::Architecture, images::ImageConfig};

pub fn initialize_grub(config: &ImageConfig, config_dir: &Path) -> Anyhow<()> {
    std::fs::create_dir_all(config_dir.join("EFI/BOOT")).ok();
    copy_recursive(
        "/usr/share/rugpi/boot/grub/first.grub.cfg",
        config_dir.join("EFI/BOOT/grub.cfg"),
    )?;
    std::fs::create_dir_all(config_dir.join("rugpi")).ok();
    copy_recursive(
        "/usr/share/rugpi/boot/grub/bootpart.default.grubenv",
        config_dir.join("rugpi/bootpart.default.grubenv"),
    )?;
    copy_recursive(
        "/usr/share/rugpi/boot/grub/boot_spare.grubenv",
        config_dir.join("rugpi/boot_spare.grubenv"),
    )?;
    match config.architecture {
        Architecture::Arm64 => {
            copy_recursive(
                "/usr/lib/grub/arm64-efi/monolithic/grubaa64.efi",
                config_dir.join("EFI/BOOT/BOOTAA64.efi"),
            )?;
        }
        Architecture::Amd64 => {
            copy_recursive(
                "/usr/lib/grub/x86_64-efi/monolithic/grubx64.efi",
                config_dir.join("EFI/BOOT/BOOTX64.efi"),
            )?;
        }
        _ => {
            bail!(
                "no Grub support for architecture `{}`",
                config.architecture.as_str()
            );
        }
    }
    Ok(())
}
