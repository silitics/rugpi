//! Creates an image.

use std::fs;

use camino::Utf8Path;
use clap::Parser;
use rugpi_common::{
    loop_dev::LoopDevice,
    mount::Mounted,
    partitions::{get_disk_id, mkfs_ext4, mkfs_vfat, sfdisk_apply_layout, sfdisk_image_layout},
    patch_cmdline, Anyhow,
};
use tempdir::TempDir;
use xscript::{run, Run};

#[derive(Debug, Parser)]
pub struct BakeTask {
    /// The archive with the system files.
    archive: String,
    /// The output image.
    image: String,
}

pub fn run(task: &BakeTask) -> Anyhow<()> {
    let archive = Utf8Path::new(&task.archive);
    let image = Utf8Path::new(&task.image);
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
    let temp_dir = TempDir::new("rugpi")?;
    let temp_dir_path = Utf8Path::from_path(temp_dir.path()).unwrap();
    {
        let _mounted_root = Mounted::mount(loop_device.partition(5), temp_dir_path)?;
        let boot_dir = temp_dir_path.join("boot");
        fs::create_dir_all(&boot_dir)?;
        let _mounted_boot = Mounted::mount(loop_device.partition(2), &boot_dir)?;
        println!("Writing system files...");
        run!(["tar", "-x", "-f", &task.archive, "-C", temp_dir_path])?;
        println!("Patching `cmdline.txt`...");
        patch_cmdline(
            boot_dir.join("cmdline.txt"),
            format!("PARTUUID={disk_id}-05"),
        )?;
    }
    {
        let _mounted_config = Mounted::mount(loop_device.partition(1), temp_dir_path)?;
        run!(["cp", "-rTp", "/usr/share/rugpi/files/config", temp_dir_path])?;
        run!([
            "cp",
            "-f",
            "/usr/share/rugpi/rpi-eeprom/firmware/stable/pieeprom-2023-05-11.bin",
            temp_dir_path.join("pieeprom.upd")
        ])?;
        run!([
            "/usr/share/rugpi/rpi-eeprom/rpi-eeprom-digest",
            "-i",
            temp_dir_path.join("pieeprom.upd"),
            "-o",
            temp_dir_path.join("pieeprom.sig")
        ])?;
        run!([
            "cp",
            "-f",
            "/usr/share/rugpi/rpi-eeprom/firmware/stable/vl805-000138c0.bin",
            temp_dir_path.join("vl805.bin")
        ])?;
        run!([
            "/usr/share/rugpi/rpi-eeprom/rpi-eeprom-digest",
            "-i",
            temp_dir_path.join("vl805.bin"),
            "-o",
            temp_dir_path.join("vl805.sig")
        ])?;
        run!([
            "cp",
            "-f",
            "/usr/share/rugpi/rpi-eeprom/firmware/stable/recovery.bin",
            temp_dir_path.join("recovery.bin")
        ])?;
    }
    Ok(())
}

fn calculate_image_size(archive: &Utf8Path) -> Anyhow<u64> {
    let archive_bytes = fs::metadata(archive)?.len();
    let total_bytes = archive_bytes + (256 + 128 + 128) * 1024 * 1024;
    let total_blocks = (total_bytes / 4096) + 1;
    let actual_blocks = (1.2 * (total_blocks as f64)) as u64;
    Ok(actual_blocks * 4096)
}
