//! Definition of the command line interface (CLI).

use std::{
    fs::{self, File},
    path::Path,
};

use anyhow::bail;
use camino::Utf8Path;
use clap::{Parser, ValueEnum};
use rugpi_common::{
    autoboot::{AUTOBOOT_A, AUTOBOOT_B},
    loop_dev::LoopDevice,
    mount::Mounted,
    partitions::{
        devices::SD_CARD, get_default_partitions, get_disk_id, get_hot_partitions, mkfs_ext4,
        mkfs_vfat, PartitionSet,
    },
    patch_cmdline, Anyhow, DropGuard,
};
use tempdir::TempDir;
use xscript::{run, Run};

pub fn main() -> Anyhow<()> {
    let args = Args::parse();
    match &args.command {
        Command::State(state_cmd) => match state_cmd {
            StateCommand::Reset => {
                fs::create_dir_all("/run/rugpi/state/.rugpi")?;
                fs::write("/run/rugpi/state/.rugpi/reset-state", "")?;
                reboot(false)?;
            }
            StateCommand::Overlay(overlay_cmd) => match overlay_cmd {
                OverlayCommand::SetPersist { persist } => match persist {
                    Boolean::True => {
                        fs::create_dir_all("/run/rugpi/state/.rugpi")?;
                        fs::write("/run/rugpi/state/.rugpi/persist-overlay", "")?;
                    }
                    Boolean::False => {
                        fs::remove_file("/run/rugpi/state/.rugpi/persist-overlay").ok();
                        if Path::new("/run/rugpi/state/.rugpi/persist-overlay").exists() {
                            bail!("Unable to unset `overlay-persist`.");
                        }
                    }
                },
            },
        },
        Command::Update(update_cmd) => match update_cmd {
            UpdateCommand::Install { image, no_reboot } => {
                let hot_partitions = get_hot_partitions()?;
                let default_partitions = get_default_partitions()?;
                let spare_partitions = default_partitions.flipped();
                if hot_partitions != default_partitions {
                    bail!("Hot partitions are not the default!");
                }
                let loop_device = LoopDevice::attach(&image)?;
                println!("Formatting partitions...");
                let boot_label = format!("BOOT-{}", spare_partitions.as_str().to_uppercase());
                let system_label = format!("system-{}", spare_partitions.as_str());
                mkfs_vfat(spare_partitions.boot_dev(), &boot_label)?;
                mkfs_ext4(spare_partitions.system_dev(), &system_label)?;
                let temp_dir_image = TempDir::new("rugpi-image")?;
                let temp_dir_image = Utf8Path::from_path(temp_dir_image.path()).unwrap();
                let temp_dir_spare = TempDir::new("rugpi-spare")?;
                let temp_dir_spare = Utf8Path::from_path(temp_dir_spare.path()).unwrap();
                // 1️⃣ Copy `/`.
                {
                    let _mounted_image = Mounted::mount(loop_device.partition(5), temp_dir_image)?;
                    let _mounted_spare =
                        Mounted::mount(spare_partitions.system_dev(), temp_dir_spare)?;
                    run!(["cp", "-arTp", temp_dir_image, temp_dir_spare])?;
                }
                // 2️⃣ Copy `/boot`.
                {
                    let _mounted_image = Mounted::mount(loop_device.partition(2), temp_dir_image)?;
                    let _mounted_spare =
                        Mounted::mount(spare_partitions.boot_dev(), temp_dir_spare)?;
                    run!(["cp", "-arTp", temp_dir_image, temp_dir_spare])?;
                    // Patch cmdline.txt.
                    let disk_id = get_disk_id(SD_CARD)?;
                    let root = match spare_partitions {
                        PartitionSet::A => format!("PARTUUID={disk_id}-05"),
                        PartitionSet::B => format!("PARTUUID={disk_id}-06"),
                    };
                    patch_cmdline(temp_dir_spare.join("cmdline.txt"), root)?;
                }
                if !*no_reboot {
                    reboot(true)?;
                }
            }
        },
        Command::System(sys_cmd) => match sys_cmd {
            SystemCommand::Info => {
                let hot_partitions = get_hot_partitions()?;
                let default_partitions = get_default_partitions()?;
                println!("Hot: {}", hot_partitions.as_str());
                println!("Cold: {}", hot_partitions.flipped().as_str());
                println!("Default: {}", default_partitions.as_str());
                println!("Spare: {}", default_partitions.flipped().as_str());
            }
            SystemCommand::Commit => {
                let hot_partitions = get_hot_partitions()?;
                let default_partitions = get_default_partitions()?;
                if hot_partitions != default_partitions {
                    run!(["mount", "-o", "remount,rw", "/run/rugpi/mounts/config"])?;
                    let _remount_guard = DropGuard::new(|| {
                        run!(["mount", "-o", "remount,ro", "/run/rugpi/mounts/config"]).ok();
                    });

                    let autoboot_txt = match hot_partitions {
                        PartitionSet::A => AUTOBOOT_A,
                        PartitionSet::B => AUTOBOOT_B,
                    };
                    fs::write("/run/rugpi/mounts/config/autoboot.txt.new", autoboot_txt)?;
                    let autoboot_new_file =
                        File::open("/run/rugpi/mounts/config/autoboot.txt.new")?;
                    autoboot_new_file.sync_all()?;
                    fs::rename(
                        "/run/rugpi/mounts/config/autoboot.txt.new",
                        "/run/rugpi/mounts/config/autoboot.txt",
                    )?;
                } else {
                    println!("Hot partition is already the default!");
                }
            }
            SystemCommand::Reboot { spare } => {
                reboot(*spare)?;
            }
        },
    }
    Ok(())
}

pub fn reboot(spare: bool) -> Anyhow<()> {
    if spare {
        run!(["reboot", "0 tryboot"])?;
    } else {
        run!(["reboot"])?;
    }
    Ok(())
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Boolean {
    True,
    False,
}

#[derive(Debug, Parser)]
#[clap(author, about)]
pub struct Args {
    /// The command.
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Parser)]
pub enum Command {
    /// Manage the persistent state of the system.
    #[clap(subcommand)]
    State(StateCommand),
    /// Install and inspect over-the-air updates.
    #[clap(subcommand)]
    Update(UpdateCommand),
    /// Manage the system.
    #[clap(subcommand)]
    System(SystemCommand),
}

#[derive(Debug, Parser)]
pub enum StateCommand {
    /// Perform a factory reset of the system.
    Reset,
    /// Configure the root filesystem overlay.
    #[clap(subcommand)]
    Overlay(OverlayCommand),
}

#[derive(Debug, Parser)]
pub enum OverlayCommand {
    /// Set the persistency of the overlay.
    SetPersist { persist: Boolean },
}

#[derive(Debug, Parser)]
pub enum UpdateCommand {
    /// Install an update.
    Install {
        /// Path to the image.
        image: String,
        #[clap(long)]
        no_reboot: bool,
    },
}

#[derive(Debug, Parser)]
pub enum SystemCommand {
    Info,
    /// Make the hot system the default.
    Commit,
    /// Reboot the system.
    Reboot {
        /// Reboot into the spare system.
        #[clap(long)]
        spare: bool,
    },
}
