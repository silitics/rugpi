use std::path::Path;

use reportify::{bail, ResultExt};
use rugpi_common::{boot::grub::grub_write_defaults, fsutils::copy_recursive};

use crate::{
    project::{config::Architecture, images::ImageConfig},
    BakeryResult,
};

pub fn initialize_grub(config: &ImageConfig, config_dir: &Path) -> BakeryResult<()> {
    std::fs::create_dir_all(config_dir.join("EFI/BOOT")).ok();
    std::fs::create_dir_all(config_dir.join("rugpi")).ok();
    copy_recursive(
        "/usr/share/rugpi/boot/grub/cfg/first.grub.cfg",
        config_dir.join("rugpi/grub.cfg"),
    )
    .whatever("unable to copy first stage boot script")?;
    grub_write_defaults(config_dir).whatever("unable to write Grub default environment")?;
    match config.architecture {
        Architecture::Arm64 => {
            copy_recursive(
                "/usr/share/rugpi/boot/grub/bin/BOOTAA64.efi",
                config_dir.join("EFI/BOOT/BOOTAA64.efi"),
            )
            .whatever("unable to copy Grub binary")?;
        }
        Architecture::Amd64 => {
            copy_recursive(
                "/usr/share/rugpi/boot/grub/bin/BOOTX64.efi",
                config_dir.join("EFI/BOOT/BOOTX64.efi"),
            )
            .whatever("unable to copy Grub binary")?;
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
