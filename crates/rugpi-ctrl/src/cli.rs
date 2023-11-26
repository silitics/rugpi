//! Definition of the command line interface (CLI).

use std::{
    fs::{self, File},
    io,
    path::Path,
};

use anyhow::{bail, Context};
use camino::Utf8Path;
use clap::{Parser, ValueEnum};
use rugpi_common::{
    boot::{
        tryboot::{AUTOBOOT_A, AUTOBOOT_B},
        uboot::UBootEnv,
        BootFlow,
    },
    img_stream::ImgStream,
    loop_dev::LoopDevice,
    mount::Mounted,
    partitions::{
        devices::SD_CARD, get_boot_flow, get_default_partitions, get_disk_id, get_hot_partitions,
        mkfs_ext4, mkfs_vfat, PartitionSet,
    },
    patch_boot, Anyhow, DropGuard,
};
use tempdir::TempDir;
use xscript::{run, Run};

use crate::overlay::overlay_dir;

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
                OverlayCommand::ForcePersist { persist } => match persist {
                    Boolean::True => {
                        fs::create_dir_all("/run/rugpi/state/.rugpi")?;
                        fs::write("/run/rugpi/state/.rugpi/force-persist-overlay", "")?;
                    }
                    Boolean::False => {
                        fs::remove_file("/run/rugpi/state/.rugpi/force-persist-overlay").ok();
                        if Path::new("/run/rugpi/state/.rugpi/force-persist-overlay").exists() {
                            bail!("Unable to unset `overlay-persist`.");
                        }
                    }
                },
            },
        },
        Command::Update(update_cmd) => match update_cmd {
            UpdateCommand::Install {
                image,
                no_reboot,
                keep_overlay,
                stream,
            } => {
                let hot_partitions = get_hot_partitions()?;
                let default_partitions = get_default_partitions()?;
                let spare_partitions = default_partitions.flipped();
                if hot_partitions != default_partitions {
                    bail!("Hot partitions are not the default!");
                }
                if !keep_overlay {
                    let spare_overlay_dir = overlay_dir(spare_partitions);
                    fs::remove_dir_all(spare_overlay_dir).ok();
                }

                if *stream {
                    install_update_stream(image)?;
                } else {
                    install_update_cp(image)?;
                }

                if !*no_reboot {
                    reboot(true)?;
                }
            }
        },
        Command::System(sys_cmd) => match sys_cmd {
            SystemCommand::Info => {
                let boot_flow = get_boot_flow().context("loading boot flow")?;
                println!("Boot Flow: {}", boot_flow.as_str());
                let hot_partitions = get_hot_partitions().context("loading hot partitions")?;
                let default_partitions =
                    get_default_partitions().context("loading default partitions")?;
                println!("Hot: {}", hot_partitions.as_str());
                println!("Cold: {}", hot_partitions.flipped().as_str());
                println!("Default: {}", default_partitions.as_str());
                println!("Spare: {}", default_partitions.flipped().as_str());
            }
            SystemCommand::Commit => {
                let boot_flow = get_boot_flow()?;
                let hot_partitions = get_hot_partitions()?;
                let default_partitions = get_default_partitions()?;
                if hot_partitions != default_partitions {
                    run!(["mount", "-o", "remount,rw", "/run/rugpi/mounts/config"])?;
                    let _remount_guard = DropGuard::new(|| {
                        run!(["mount", "-o", "remount,ro", "/run/rugpi/mounts/config"]).ok();
                    });
                    match boot_flow {
                        BootFlow::Tryboot => {
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
                        }
                        BootFlow::UBoot => {
                            let mut bootpart_env = UBootEnv::new();
                            match hot_partitions {
                                PartitionSet::A => bootpart_env.set("bootpart", "2"),
                                PartitionSet::B => bootpart_env.set("bootpart", "3"),
                            }
                            bootpart_env
                                .save("/run/rugpi/mounts/config/bootpart.default.env.new")?;
                            let autoboot_new_file =
                                File::open("/run/rugpi/mounts/config/bootpart.default.env.new")?;
                            autoboot_new_file.sync_all()?;
                            fs::rename(
                                "/run/rugpi/mounts/config/bootpart.default.env.new",
                                "/run/rugpi/mounts/config/bootpart.default.env",
                            )?;
                        }
                    }
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
    match get_boot_flow()? {
        BootFlow::Tryboot => {
            if spare {
                run!(["reboot", "0 tryboot"])?;
            } else {
                run!(["reboot"])?;
            }
        }
        BootFlow::UBoot => {
            if spare {
                let mut boot_spare_env = UBootEnv::new();
                boot_spare_env.set("boot_spare", "1");
                run!(["mount", "-o", "remount,rw", "/run/rugpi/mounts/config"])?;
                let _remount_guard = DropGuard::new(|| {
                    run!(["mount", "-o", "remount,ro", "/run/rugpi/mounts/config"]).ok();
                });
                // It is safe to directly write to the file here. If the file is corrupt,
                // the system will simply boot from the default partition set.
                boot_spare_env.save("/run/rugpi/mounts/config/boot_spare.env")?;
            }
            run!(["reboot"])?;
        }
    }

    Ok(())
}

fn install_update_cp(image: &String) -> Anyhow<()> {
    let default_partitions = get_default_partitions()?;
    let spare_partitions = default_partitions.flipped();
    let loop_device = LoopDevice::attach(image)?;
    println!("Formatting partitions...");
    let boot_label = format!("BOOT-{}", spare_partitions.as_str().to_uppercase());
    let system_label = format!("system-{}", spare_partitions.as_str());
    mkfs_vfat(spare_partitions.boot_dev(), boot_label)?;
    mkfs_ext4(spare_partitions.system_dev(), system_label)?;
    let temp_dir_image = TempDir::new("rugpi-image")?;
    let temp_dir_image = Utf8Path::from_path(temp_dir_image.path()).unwrap();
    let temp_dir_spare = TempDir::new("rugpi-spare")?;
    let temp_dir_spare = Utf8Path::from_path(temp_dir_spare.path()).unwrap();
    // 1️⃣ Copy `/`.
    {
        let _mounted_image = Mounted::mount(loop_device.partition(5), temp_dir_image)?;
        let _mounted_spare = Mounted::mount(spare_partitions.system_dev(), temp_dir_spare)?;
        run!(["cp", "-arTp", temp_dir_image, temp_dir_spare])?;
    }
    // 2️⃣ Copy `/boot`.
    {
        let _mounted_image = Mounted::mount(loop_device.partition(2), temp_dir_image)?;
        let _mounted_spare = Mounted::mount(spare_partitions.boot_dev(), temp_dir_spare)?;
        run!(["cp", "-arTp", temp_dir_image, temp_dir_spare])?;
        // Patch cmdline.txt.
        let disk_id = get_disk_id(SD_CARD)?;
        let root = match spare_partitions {
            PartitionSet::A => format!("PARTUUID={disk_id}-05"),
            PartitionSet::B => format!("PARTUUID={disk_id}-06"),
        };
        patch_boot(temp_dir_spare, root)?;
    }
    Ok(())
}

fn install_update_stream(image: &String) -> Anyhow<()> {
    let default_partitions = get_default_partitions()?;
    let spare_partitions = default_partitions.flipped();
    let reader: Box<dyn io::Read> = if image == "-" {
        Box::new(io::stdin())
    } else {
        Box::new(File::open(image)?)
    };
    println!("Copying partitions...");
    let boot_label = format!("BOOT-{}", spare_partitions.as_str().to_uppercase());
    let system_label = format!("system-{}", spare_partitions.as_str());
    let mut img_stream = ImgStream::new(reader)?;
    let mut partition_idx = 0;
    while let Some(mut partition) = img_stream.next_partition()? {
        let partition_name = match partition_idx {
            0 => "CONFIG",
            1 => "BOOT-A",
            2 => "BOOT-B",
            3 => "system-a",
            4 => "system-b",
            5 => "data",
            _ => "<unknown>",
        };
        println!(
            "Found {partition_idx},{partition_name} {}",
            partition.entry()
        );
        match partition_idx {
            1 => {
                io::copy(
                    &mut partition,
                    &mut fs::File::create(spare_partitions.boot_dev())?,
                )?;
                run!(["fatlabel", spare_partitions.boot_dev(), &boot_label])?;
            }
            3 => {
                io::copy(
                    &mut partition,
                    &mut fs::File::create(spare_partitions.system_dev())?,
                )?;
                run!(["e2label", spare_partitions.system_dev(), &system_label])?;
            }
            _ => { /* Nothing to do! */ }
        }

        partition_idx += 1;
    }

    let temp_dir_spare = TempDir::new("rugpi-spare")?;
    let temp_dir_spare = Utf8Path::from_path(temp_dir_spare.path()).unwrap();

    // Path `/boot`.
    {
        let _mounted_spare = Mounted::mount(spare_partitions.boot_dev(), temp_dir_spare)?;
        let disk_id = get_disk_id(SD_CARD)?;
        let root = match spare_partitions {
            PartitionSet::A => format!("PARTUUID={disk_id}-05"),
            PartitionSet::B => format!("PARTUUID={disk_id}-06"),
        };
        patch_boot(temp_dir_spare, root)?;
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
    ForcePersist { persist: Boolean },
}

#[derive(Debug, Parser)]
pub enum UpdateCommand {
    /// Install an update.
    Install {
        /// Path to the image.
        image: String,
        /// Prevent Rugpi from rebooting the system.
        #[clap(long)]
        no_reboot: bool,
        /// Do not delete an existing overlay.
        #[clap(long)]
        keep_overlay: bool,
        /// Use the experimental streaming update mechanism.
        #[clap(long)]
        stream: bool,
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
