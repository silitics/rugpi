//! Creates an image.

use std::{
    fs,
    path::{Path, PathBuf},
};

use rugpi_common::{
    boot::{uboot::UBootEnv, BootFlow},
    ctrl_config::load_config,
    loop_dev::LoopDevice,
    mount::{MountStack, Mounted},
    partitions::{get_disk_id, mkfs_ext4, mkfs_vfat, sfdisk_apply_layout},
    patch_boot, patch_config, Anyhow,
};
use tempfile::tempdir;
use xscript::{run, Run};

use crate::{
    project::{
        config::{Architecture, IncludeFirmware},
        images::{self, pi_image_layout, ImageConfig},
    },
    utils::prelude::*,
};

pub fn make_image(image_config: &ImageConfig, src: &Path, image: &Path) -> Anyhow<()> {
    let size = calculate_image_size(src)?;
    let system_size = calculate_system_size(src)?;
    fs::remove_file(image).ok();
    if let Some(parent) = image.parent() {
        fs::create_dir_all(parent).ok();
    }
    info!("creating image (size: {} bytes)", size);
    if let Some(size) = &image_config.size {
        println!("Allocate image of size {size}.");
        run!(["fallocate", "-l", "{size}", image])?;
    } else {
        run!(["fallocate", "-l", "{size}", image])?;
    }
    let layout = image_config.layout.clone().unwrap_or_else(pi_image_layout);
    sfdisk_apply_layout(image, layout.sfdisk_render())?;
    let disk_id = get_disk_id(image)?;
    let loop_device = LoopDevice::attach(image)?;
    info!("creating filesystems");
    for (idx, partition) in layout.partitions.iter().enumerate() {
        let part = idx + 1;
        let Some(fs) = partition.filesystem else {
            continue;
        };
        match fs {
            images::Filesystem::Ext4 => {
                mkfs_ext4(
                    loop_device.partition(part),
                    partition.label.as_deref().unwrap(),
                )?;
            }
            images::Filesystem::Fat32 => {
                mkfs_vfat(
                    loop_device.partition(part),
                    &partition.label.as_deref().unwrap().to_uppercase(),
                )?;
            }
        }
    }
    let root_dir = tempdir()?;
    let root_dir_path = root_dir.path();
    {
        let mut system_partitions = layout
            .partitions
            .iter()
            .enumerate()
            .filter_map(|(idx, partition)| partition.path.as_deref().map(|path| (path, idx + 1)))
            .collect::<Vec<_>>();
        system_partitions.sort();

        let mut mount_stack = MountStack::new();

        for (path, part) in &system_partitions {
            let full_path = if path.is_empty() {
                root_dir_path.to_owned()
            } else {
                root_dir_path.join(path)
            };
            fs::create_dir_all(&full_path).ok();
            mount_stack.push(Mounted::mount(loop_device.partition(*part), &full_path)?);
        }

        run!(["tar", "-x", "-f", src, "-C", root_dir_path])?;

        info!("checking filesystem size");
        let config = load_config(&root_dir_path.join("etc/rugpi/ctrl.toml"))?;
        // This is an over approximation.
        if config.system_size_bytes()? < system_size {
            bail!("system size configured in `ctrl.toml` not large enough")
        }

        if let Some(boot_flow) = image_config.boot_flow {
            let config_dir = tempdir()?;
            let config_dir_path = config_dir.path();
            let mounted_config = Mounted::mount(loop_device.partition(1), config_dir_path)?;
            let ctx = BakeCtx {
                config: image_config,
                boot_path: root_dir_path.join("boot"),
                mounted_config,
                loop_device: loop_device.path(),
            };
            if matches!(boot_flow, BootFlow::Tryboot | BootFlow::UBoot) {
                info!("patching boot configuration");
                patch_boot(&ctx.boot_path, format!("PARTUUID={disk_id}-05"))?;
                info!("patching `config.txt`");
                patch_config(ctx.boot_path.join("config.txt"))?;
            }

            match boot_flow {
                BootFlow::Tryboot => setup_tryboot_boot_flow(&ctx)?,
                BootFlow::UBoot => setup_uboot_boot_flow(&ctx)?,
                BootFlow::GrubEfi => setup_grub_boot_flow(&ctx)?,
            }

            std::fs::copy(
                "/usr/share/rugpi/boot/u-boot/bin/second.scr",
                ctx.boot_path.join("second.scr"),
            )?;
            std::fs::copy(
                "/usr/share/rugpi/boot/grub/second.grub.cfg",
                ctx.boot_path.join("grub/second.grub.cfg"),
            )?;

            if let Some(include_firmware) = &image_config.include_firmware {
                match include_firmware {
                    IncludeFirmware::Pi4 => include_pi4_firmware(ctx.mounted_config.path())?,
                    IncludeFirmware::Pi5 => include_pi5_firmware(ctx.mounted_config.path())?,
                }
            }
        }
    }
    Ok(())
}

struct BakeCtx<'p> {
    config: &'p ImageConfig,
    boot_path: PathBuf,
    mounted_config: Mounted,
    #[allow(dead_code)]
    loop_device: &'p Path,
}

fn calculate_system_size(archive: &Path) -> Anyhow<u64> {
    let archive_bytes = fs::metadata(archive)?.len();
    let total_bytes = archive_bytes;
    let total_blocks = (total_bytes / 4096) + 1;
    let actual_blocks = (1.2 * (total_blocks as f64)) as u64;
    Ok(actual_blocks * 4096)
}

fn calculate_image_size(archive: &Path) -> Anyhow<u64> {
    let archive_bytes = fs::metadata(archive)?.len();
    let total_bytes = archive_bytes + (256 + 128 + 128) * 1024 * 1024;
    let total_blocks = (total_bytes / 4096) + 1;
    let actual_blocks = (1.2 * (total_blocks as f64)) as u64;
    Ok(actual_blocks * 4096)
}

fn setup_grub_boot_flow(ctx: &BakeCtx) -> Anyhow<()> {
    std::fs::create_dir_all(ctx.mounted_config.path().join("EFI/BOOT")).ok();
    std::fs::create_dir_all(ctx.mounted_config.path().join("EFI/BOOT")).ok();
    std::fs::copy(
        "/usr/share/rugpi/boot/grub/first.grub.cfg",
        ctx.mounted_config.path().join("EFI/BOOT/grub.cfg"),
    )?;
    std::fs::create_dir_all(ctx.mounted_config.path().join("rugpi")).ok();
    std::fs::copy(
        "/usr/share/rugpi/boot/grub/bootpart.default.grubenv",
        ctx.mounted_config
            .path()
            .join("rugpi/bootpart.default.grubenv"),
    )?;
    std::fs::copy(
        "/usr/share/rugpi/boot/grub/boot_spare.grubenv",
        ctx.mounted_config.path().join("rugpi/boot_spare.grubenv"),
    )?;
    match ctx.config.architecture {
        Architecture::Arm64 => {
            std::fs::copy(
                "/usr/lib/grub/arm64-efi/monolithic/grubaa64.efi",
                ctx.mounted_config.path().join("EFI/BOOT/BOOTAA64.efi"),
            )?;
        }
        Architecture::Armhf => {
            bail!("unable to install Grub for `armhf`");
        }
        Architecture::Amd64 => {
            std::fs::copy(
                "/usr/lib/grub/x86_64-efi/monolithic/grubx64.efi",
                ctx.mounted_config.path().join("EFI/BOOT/BOOTX64.efi"),
            )?;
        }
    }
    Ok(())
}

fn setup_tryboot_boot_flow(ctx: &BakeCtx) -> Anyhow<()> {
    run!([
        "cp",
        "-rTp",
        "/usr/share/rugpi/boot/tryboot",
        ctx.mounted_config.path()
    ])?;
    Ok(())
}

fn setup_uboot_boot_flow(ctx: &BakeCtx) -> Anyhow<()> {
    run!(["cp", "-rTp", &ctx.boot_path, ctx.mounted_config.path()])?;
    std::fs::remove_file(ctx.mounted_config.path().join("kernel8.img"))?;
    match ctx.config.architecture {
        Architecture::Arm64 => {
            std::fs::copy(
                "/usr/share/rugpi/boot/u-boot/arm64_config.txt",
                ctx.mounted_config.path().join("config.txt"),
            )?;
            std::fs::copy(
                "/usr/share/rugpi/boot/u-boot/bin/u-boot-arm64.bin",
                ctx.mounted_config.path().join("u-boot-arm64.bin"),
            )?;
        }
        Architecture::Armhf => {
            std::fs::copy(
                "/usr/share/rugpi/boot/u-boot/armhf_config.txt",
                ctx.mounted_config.path().join("config.txt"),
            )?;
            for model in ["zerow", "pi1", "pi2", "pi3"] {
                std::fs::copy(
                    format!("/usr/share/rugpi/boot/u-boot/bin/u-boot-armhf-{model}.bin"),
                    ctx.mounted_config
                        .path()
                        .join(format!("u-boot-armhf-{model}.bin")),
                )?;
            }
        }
        Architecture::Amd64 => {
            eprintln!("No bootloader support.");
        }
    }

    std::fs::copy(
        "/usr/share/rugpi/boot/u-boot/bin/boot.scr",
        ctx.mounted_config.path().join("boot.scr"),
    )?;
    std::fs::write(ctx.mounted_config.path().join("cmdline.txt"), "")?;

    let mut env = UBootEnv::new();
    env.set("bootpart", "2");
    env.save(ctx.mounted_config.path().join("bootpart.default.env"))?;

    let mut env = UBootEnv::new();
    env.set("boot_spare", "0");
    env.save(ctx.mounted_config.path().join("boot_spare.disabled.env"))?;
    env.save(ctx.mounted_config.path().join("boot_spare.env"))?;

    let mut env = UBootEnv::new();
    env.set("boot_spare", "1");
    env.save(ctx.mounted_config.path().join("boot_spare.enabled.env"))?;

    Ok(())
}

fn include_pi4_firmware(autoboot_path: &Path) -> Anyhow<()> {
    run!([
        "cp",
        "-f",
        "/usr/share/rugpi/rpi-eeprom/firmware-2711/stable/pieeprom-2023-05-11.bin",
        autoboot_path.join("pieeprom.upd")
    ])?;
    run!([
        "/usr/share/rugpi/rpi-eeprom/rpi-eeprom-digest",
        "-i",
        autoboot_path.join("pieeprom.upd"),
        "-o",
        autoboot_path.join("pieeprom.sig")
    ])?;
    run!([
        "cp",
        "-f",
        "/usr/share/rugpi/rpi-eeprom/firmware-2711/stable/vl805-000138c0.bin",
        autoboot_path.join("vl805.bin")
    ])?;
    run!([
        "/usr/share/rugpi/rpi-eeprom/rpi-eeprom-digest",
        "-i",
        autoboot_path.join("vl805.bin"),
        "-o",
        autoboot_path.join("vl805.sig")
    ])?;
    run!([
        "cp",
        "-f",
        "/usr/share/rugpi/rpi-eeprom/firmware-2711/stable/recovery.bin",
        autoboot_path.join("recovery.bin")
    ])?;
    Ok(())
}

fn include_pi5_firmware(autoboot_path: &Path) -> Anyhow<()> {
    run!([
        "cp",
        "-f",
        "/usr/share/rugpi/rpi-eeprom/firmware-2712/stable/pieeprom-2023-10-30.bin",
        autoboot_path.join("pieeprom.upd")
    ])?;
    run!([
        "/usr/share/rugpi/rpi-eeprom/rpi-eeprom-digest",
        "-i",
        autoboot_path.join("pieeprom.upd"),
        "-o",
        autoboot_path.join("pieeprom.sig")
    ])?;
    run!([
        "cp",
        "-f",
        "/usr/share/rugpi/rpi-eeprom/firmware-2712/stable/recovery.bin",
        autoboot_path.join("recovery.bin")
    ])?;
    Ok(())
}
