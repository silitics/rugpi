//! Definition of the command line interface (CLI).

use std::{
    fs::{self, File},
    io,
    path::Path,
};

use anyhow::{bail, Context};
use clap::{Parser, ValueEnum};
use rugpi_common::{
    boot::{
        tryboot::{AUTOBOOT_A, AUTOBOOT_B},
        uboot::UBootEnv,
        BootFlow,
    },
    img_stream::ImgStream,
    maybe_compressed::MaybeCompressed,
    mount::Mounted,
    partitions::{
        get_boot_flow, get_default_partitions, get_disk_id, get_hot_partitions, PartitionSet,
        Partitions,
    },
    patch_boot, Anyhow, DropGuard,
};
use tempfile::tempdir;
use xscript::{run, Run};

use crate::{
    overlay::overlay_dir,
    utils::{clear_flag, reboot, reboot_syscall, set_flag, DEFERRED_SPARE_REBOOT_FLAG},
};

pub fn main() -> Anyhow<()> {
    let args = Args::parse();
    let partitions = Partitions::load()?;
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
                reboot: reboot_type,
                keep_overlay,
                stream,
            } => {
                if reboot_type.is_some() && *no_reboot {
                    bail!("--no-reboot and --reboot are incompatible");
                }

                let hot_partitions = get_hot_partitions(&partitions)?;
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
                    indoc::eprintdoc! {"
                        **Deprecation Warning:**
                        The option `--stream` has been deprecated and will be removed in future versions.
                        Streaming updates are the default now.
                    "}
                }

                install_update_stream(&partitions, image)?;

                let reboot_type = reboot_type.clone().unwrap_or(if *no_reboot {
                    UpdateRebootType::No
                } else {
                    UpdateRebootType::Yes
                });

                match reboot_type {
                    UpdateRebootType::Yes => reboot(true)?,
                    UpdateRebootType::No => { /* nothing to do */ }
                    UpdateRebootType::Deferred => {
                        set_flag(DEFERRED_SPARE_REBOOT_FLAG)?;
                    }
                }
            }
        },
        Command::System(sys_cmd) => match sys_cmd {
            SystemCommand::Info => {
                let boot_flow = get_boot_flow().context("loading boot flow")?;
                println!("Boot Flow: {}", boot_flow.as_str());
                let hot_partitions =
                    get_hot_partitions(&partitions).context("loading hot partitions")?;
                let default_partitions =
                    get_default_partitions().context("loading default partitions")?;
                println!("Hot: {}", hot_partitions.as_str());
                println!("Cold: {}", hot_partitions.flipped().as_str());
                println!("Default: {}", default_partitions.as_str());
                println!("Spare: {}", default_partitions.flipped().as_str());
            }
            SystemCommand::Commit => {
                let boot_flow = get_boot_flow()?;
                let hot_partitions = get_hot_partitions(&partitions)?;
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
                        BootFlow::None => todo!(),
                    }
                } else {
                    println!("Hot partition is already the default!");
                }
            }
            SystemCommand::Reboot { spare } => {
                reboot(*spare)?;
            }
        },
        Command::Unstable(command) => match command {
            UnstableCommand::Tryboot => reboot_syscall(true)?,
            UnstableCommand::SetDeferredSpareReboot { value } => match value {
                Boolean::True => set_flag(DEFERRED_SPARE_REBOOT_FLAG)?,
                Boolean::False => clear_flag(DEFERRED_SPARE_REBOOT_FLAG)?,
            },
        },
    }
    Ok(())
}

fn install_update_stream(partitions: &Partitions, image: &String) -> Anyhow<()> {
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
    let mut img_stream = ImgStream::new(MaybeCompressed::new(reader)?)?;
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
                    &mut fs::File::create(spare_partitions.boot_dev(partitions))?,
                )?;
                run!([
                    "fatlabel",
                    spare_partitions.boot_dev(partitions),
                    &boot_label
                ])?;
            }
            3 => {
                io::copy(
                    &mut partition,
                    &mut fs::File::create(spare_partitions.system_dev(partitions))?,
                )?;
                run!([
                    "e2label",
                    spare_partitions.system_dev(partitions),
                    &system_label
                ])?;
            }
            _ => { /* Nothing to do! */ }
        }

        partition_idx += 1;
    }

    let temp_dir_spare = tempdir()?;
    let temp_dir_spare = temp_dir_spare.path();

    // Path `/boot`.
    {
        let _mounted_spare = Mounted::mount(spare_partitions.boot_dev(partitions), temp_dir_spare)?;
        let disk_id = get_disk_id(&partitions.parent_dev)?;
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
    /// Unstable experimental commands.
    #[clap(subcommand)]
    Unstable(UnstableCommand),
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
        #[clap(long)]
        reboot: Option<UpdateRebootType>,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum UpdateRebootType {
    Yes,
    No,
    Deferred,
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

#[derive(Debug, Parser)]
pub enum UnstableCommand {
    /// Directly reboot with tryboot using a syscall.
    Tryboot,
    /// Set deferred spare reboot flag.
    SetDeferredSpareReboot { value: Boolean },
}
