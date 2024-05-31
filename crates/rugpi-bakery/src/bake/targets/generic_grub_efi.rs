use std::path::Path;

use anyhow::bail;
use rugpi_common::{boot::grub::grub_write_defaults, fsutils::copy_recursive, Anyhow};

use crate::project::{config::Architecture, images::ImageConfig};

pub fn initialize_grub(config: &ImageConfig, config_dir: &Path) -> Anyhow<()> {
    std::fs::create_dir_all(config_dir.join("EFI/BOOT")).ok();
    std::fs::create_dir_all(config_dir.join("rugpi")).ok();
    copy_recursive(
        "/usr/share/rugpi/boot/grub/cfg/first.grub.cfg",
        config_dir.join("rugpi/grub.cfg"),
    )?;
    grub_write_defaults(config_dir)?;
    match config.architecture {
        Architecture::Arm64 => {
            copy_recursive(
                "/usr/share/rugpi/boot/grub/bin/BOOTAA64.efi",
                config_dir.join("EFI/BOOT/BOOTAA64.efi"),
            )?;
        }
        Architecture::Amd64 => {
            copy_recursive(
                "/usr/share/rugpi/boot/grub/bin/BOOTX64.efi",
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
