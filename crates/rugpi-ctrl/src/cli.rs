//! Definition of the command line interface (CLI).

use std::{
    fs::{self, File},
    path::Path,
};

use anyhow::{anyhow, bail};
use camino::Utf8Path;
use clap::{Parser, ValueEnum};
use rugpi_common::{
    autoboot::{AUTOBOOT_A, AUTOBOOT_B},
    loop_dev::LoopDevice,
    mkfs,
    mount::Mounted,
    patch_cmdline,
};
use tempdir::TempDir;
use xscript::{run, Run};

use crate::{
    init::{cold_partition_set, default_partition_set, hot_partition_set},
    partitions::SD_CARD,
};

pub fn main() -> anyhow::Result<()> {
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
                let hot_partitions = hot_partition_set()?;
                let default_partitions = default_partition_set()?;
                let spare_partitions = default_partitions.flipped();
                if hot_partitions != default_partitions {
                    bail!("Hot partitions are not the default!");
                }
                let loop_device = LoopDevice::attach(&image)?;
                println!("Formatting partitions...");
                let boot_dev_label = format!("BOOT-{}", spare_partitions.as_str().to_uppercase());
                let sys_dev_label = format!("system-{}", spare_partitions.as_str());
                mkfs::make_boot_fs(spare_partitions.boot_dev(), &boot_dev_label)?;
                mkfs::make_system_fs(spare_partitions.system_dev(), &sys_dev_label)?;
                let temp_dir_image = TempDir::new("rugpi-image")?;
                let temp_dir_image = Utf8Path::from_path(temp_dir_image.path()).unwrap();
                let temp_dir_spare = TempDir::new("rugpi-spare")?;
                let temp_dir_spare = Utf8Path::from_path(temp_dir_spare.path()).unwrap();
                // 1️⃣ Copy system.
                {
                    let _mounted_image = Mounted::mount(loop_device.partition(5), temp_dir_image)?;
                    let _mounted_spare =
                        Mounted::mount(spare_partitions.system_dev(), temp_dir_spare)?;
                    run!(["cp", "-arTp", temp_dir_image, temp_dir_spare])?;
                }
                // 1️⃣ Copy boot.
                {
                    let _mounted_image = Mounted::mount(loop_device.partition(2), temp_dir_image)?;
                    let _mounted_spare =
                        Mounted::mount(spare_partitions.boot_dev(), temp_dir_spare)?;
                    run!(["cp", "-arTp", temp_dir_image, temp_dir_spare])?;
                    // Path cmdline.txt.
                    let disk_id = xscript::read_str!(["sfdisk", "--disk-id", SD_CARD])?
                        .strip_prefix("0x")
                        .ok_or_else(|| anyhow!("`sfdisk` returned invalid disk id"))?
                        .to_owned();
                    let root = match spare_partitions {
                        crate::init::PartitionSet::A => format!("PARTUUID={disk_id}-05"),
                        crate::init::PartitionSet::B => format!("PARTUUID={disk_id}-06"),
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
                let hot_partitions = hot_partition_set()?;
                let cold_partitions = cold_partition_set()?;
                let default_partitions = default_partition_set()?;
                let spare_partitions = default_partitions.flipped();
                println!("Hot: {}", hot_partitions.as_str());
                println!("Cold: {}", cold_partitions.as_str());
                println!("Default: {}", default_partitions.as_str());
                println!("Spare: {}", spare_partitions.as_str());
            }
            SystemCommand::Commit => {
                let hot_partitions = hot_partition_set()?;
                let default_partitions = default_partition_set()?;
                if hot_partitions != default_partitions {
                    run!(["mount", "-o", "remount,rw", "/run/rugpi/mounts/config"])?;
                    let autoboot_txt = match hot_partitions {
                        crate::init::PartitionSet::A => AUTOBOOT_A,
                        crate::init::PartitionSet::B => AUTOBOOT_B,
                    };
                    fs::write("/run/rugpi/mounts/config/autoboot.txt.new", autoboot_txt)?;
                    let autoboot_new_file =
                        File::open("/run/rugpi/mounts/config/autoboot.txt.new")?;
                    autoboot_new_file.sync_all()?;
                    fs::rename(
                        "/run/rugpi/mounts/config/autoboot.txt.new",
                        "/run/rugpi/mounts/config/autoboot.txt",
                    )?;
                    run!(["mount", "-o", "remount,ro", "/run/rugpi/mounts/config"])?;
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

pub fn reboot(spare: bool) -> anyhow::Result<()> {
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
