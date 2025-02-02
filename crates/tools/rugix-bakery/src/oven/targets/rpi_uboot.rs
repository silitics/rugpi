use std::path::Path;

use reportify::{bail, ResultExt};

use rugix_common::boot::uboot::UBootEnv;
use rugix_common::fsutils::copy_recursive;

use crate::config::systems::{Architecture, SystemConfig};
use crate::BakeryResult;

pub fn initialize_uboot(config: &SystemConfig, config_dir: &Path) -> BakeryResult<()> {
    copy_recursive("/usr/share/rugix/pi/firmware", &config_dir)
        .whatever("unable to copy RPi firmware")?;
    match config.architecture {
        Architecture::Arm64 => {
            copy_recursive(
                "/usr/share/rugix/boot/u-boot/arm64_config.txt",
                config_dir.join("config.txt"),
            )
            .whatever("unable to copy `config.txt`")?;
            copy_recursive(
                "/usr/share/rugix/boot/u-boot/bin/u-boot-arm64.bin",
                config_dir.join("u-boot-arm64.bin"),
            )
            .whatever("unable to copy U-Boot binary")?;
        }
        Architecture::Armhf => {
            copy_recursive(
                "/usr/share/rugix/boot/u-boot/armhf_config.txt",
                config_dir.join("config.txt"),
            )
            .whatever("unable to copy `config.txt`")?;
            for model in ["zerow", "pi1", "pi2", "pi3"] {
                copy_recursive(
                    format!("/usr/share/rugix/boot/u-boot/bin/u-boot-armhf-{model}.bin"),
                    config_dir.join(format!("u-boot-armhf-{model}.bin")),
                )
                .whatever("unable to copy U-Boot binary")?;
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
        "/usr/share/rugix/boot/u-boot/bin/boot.scr",
        config_dir.join("boot.scr"),
    )
    .whatever("unable to copy first stage boot script")?;
    std::fs::write(config_dir.join("cmdline.txt"), "").whatever("unable to write `cmdline.txt`")?;

    let mut env = UBootEnv::new();
    env.set("bootpart", "2");
    env.save(config_dir.join("bootpart.default.env"))
        .whatever("unable to create default U-Boot environment")?;

    let mut env = UBootEnv::new();
    env.set("boot_spare", "0");
    env.save(config_dir.join("boot_spare.disabled.env"))
        .whatever("unable to write U-Boot environment")?;
    env.save(config_dir.join("boot_spare.env"))
        .whatever("unable to write U-Boot environment")?;

    let mut env = UBootEnv::new();
    env.set("boot_spare", "1");
    env.save(config_dir.join("boot_spare.enabled.env"))
        .whatever("unable to write U-Boot environment")?;

    Ok(())
}
