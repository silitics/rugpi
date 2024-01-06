//! Creates an image.

use std::{fs, path::Path};

use clap::Parser;
use rugpi_common::{
    boot::{uboot::UBootEnv, BootFlow},
    loop_dev::LoopDevice,
    mount::Mounted,
    partitions::{get_disk_id, mkfs_ext4, mkfs_vfat, sfdisk_apply_layout, sfdisk_image_layout},
    patch_boot, patch_config, Anyhow,
};
use tempfile::tempdir;
use xscript::{run, Run};

use crate::project::{
    config::{Architecture, IncludeFirmware},
    Project,
};

#[derive(Debug, Parser)]
pub struct BakeTask {
    /// The archive with the system files.
    archive: String,
    /// The output image.
    image: String,
}

pub fn run(project: &Project, task: &BakeTask) -> Anyhow<()> {
    let archive = Path::new(&task.archive);
    let image = Path::new(&task.image);
    let size = calculate_image_size(archive)?;
    println!("Size: {} bytes", size);
    fs::remove_file(image).ok();
    println!("Creating image...");
    run!(["fallocate", "-l", "{size}", image])?;
    sfdisk_apply_layout(image, sfdisk_image_layout())?;
    let disk_id = get_disk_id(image)?;
    let loop_device = LoopDevice::attach(image)?;
    println!("Creating file systems...");
    mkfs_vfat(loop_device.partition(1), "CONFIG")?;
    mkfs_vfat(loop_device.partition(2), "BOOT-A")?;
    mkfs_ext4(loop_device.partition(5), "system-a")?;
    let root_dir = tempdir()?;
    let root_dir_path = root_dir.path();
    {
        let mounted_root = Mounted::mount(loop_device.partition(5), root_dir_path)?;
        let mut boot_dir = root_dir_path.join("boot");
        let boot_firmware_dir = boot_dir.join("firmware");
        if boot_firmware_dir.exists() {
            boot_dir = boot_firmware_dir;
        }
        fs::create_dir_all(&boot_dir)?;
        let mounted_boot = Mounted::mount(loop_device.partition(2), &boot_dir)?;
        let config_dir = tempdir()?;
        let config_dir_path = config_dir.path();
        let mounted_config = Mounted::mount(loop_device.partition(1), config_dir_path)?;
        let ctx = BakeCtx {
            project,
            mounted_boot,
            mounted_root,
            mounted_config,
        };

        run!(["tar", "-x", "-f", &task.archive, "-C", root_dir_path])?;
        println!("Patching boot configuration...");
        patch_boot(ctx.mounted_boot.path(), format!("PARTUUID={disk_id}-05"))?;
        println!("Patching `config.txt`...");
        patch_config(boot_dir.join("config.txt"))?;

        match project.config.boot_flow {
            BootFlow::Tryboot => setup_tryboot_boot_flow(&ctx)?,
            BootFlow::UBoot => setup_uboot_boot_flow(&ctx)?,
        }

        std::fs::copy(
            "/usr/share/rugpi/boot/u-boot/bin/second.scr",
            ctx.mounted_boot.path().join("second.scr"),
        )?;

        match project.config.include_firmware {
            IncludeFirmware::None => { /* Do not include any firmware. */ }
            IncludeFirmware::Pi4 => include_pi4_firmware(ctx.mounted_config.path())?,
            IncludeFirmware::Pi5 => include_pi5_firmware(ctx.mounted_config.path())?,
        }
    }
    // Shrink root filesystem.
    // run!(["resize2fs", "-M", loop_device.partition(5)])?;
    // run!(["dumpe2fs", "-h", loop_device.partition(5)])?;
    Ok(())
}

struct BakeCtx<'p> {
    project: &'p Project,
    mounted_boot: Mounted,
    #[allow(unused)]
    mounted_root: Mounted,
    mounted_config: Mounted,
}

fn calculate_image_size(archive: &Path) -> Anyhow<u64> {
    let archive_bytes = fs::metadata(archive)?.len();
    let total_bytes = archive_bytes + (256 + 128 + 128) * 1024 * 1024;
    let total_blocks = (total_bytes / 4096) + 1;
    let actual_blocks = (1.2 * (total_blocks as f64)) as u64;
    Ok(actual_blocks * 4096)
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
    run!([
        "cp",
        "-rTp",
        ctx.mounted_boot.path(),
        ctx.mounted_config.path()
    ])?;
    std::fs::remove_file(ctx.mounted_config.path().join("kernel8.img"))?;
    match ctx.project.config.architecture {
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
