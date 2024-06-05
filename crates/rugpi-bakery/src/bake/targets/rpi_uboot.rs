use std::path::Path;

use anyhow::bail;
use rugpi_common::{boot::uboot::UBootEnv, fsutils::copy_recursive, Anyhow};

use crate::project::{config::Architecture, images::ImageConfig};

pub fn initialize_uboot(config: &ImageConfig, config_dir: &Path) -> Anyhow<()> {
    copy_recursive("/usr/share/rugpi/pi/firmware", &config_dir)?;
    match config.architecture {
        Architecture::Arm64 => {
            copy_recursive(
                "/usr/share/rugpi/boot/u-boot/arm64_config.txt",
                config_dir.join("config.txt"),
            )?;
            copy_recursive(
                "/usr/share/rugpi/boot/u-boot/bin/u-boot-arm64.bin",
                config_dir.join("u-boot-arm64.bin"),
            )?;
        }
        Architecture::Armhf => {
            copy_recursive(
                "/usr/share/rugpi/boot/u-boot/armhf_config.txt",
                config_dir.join("config.txt"),
            )?;
            for model in ["zerow", "pi1", "pi2", "pi3"] {
                copy_recursive(
                    format!("/usr/share/rugpi/boot/u-boot/bin/u-boot-armhf-{model}.bin"),
                    config_dir.join(format!("u-boot-armhf-{model}.bin")),
                )?;
            }
        }
        _ => {
            bail!(
                "no U-Boot support for architecture `{}`",
                config.architecture.as_str()
            );
        }
    }
    copy_recursive(
        "/usr/share/rugpi/boot/u-boot/bin/boot.scr",
        config_dir.join("boot.scr"),
    )?;
    std::fs::write(config_dir.join("cmdline.txt"), "")?;

    let mut env = UBootEnv::new();
    env.set("bootpart", "2");
    env.save(config_dir.join("bootpart.default.env"))?;

    let mut env = UBootEnv::new();
    env.set("boot_spare", "0");
    env.save(config_dir.join("boot_spare.disabled.env"))?;
    env.save(config_dir.join("boot_spare.env"))?;

    let mut env = UBootEnv::new();
    env.set("boot_spare", "1");
    env.save(config_dir.join("boot_spare.enabled.env"))?;

    Ok(())
}
