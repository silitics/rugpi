//! Definition of the command line interface (CLI).

use std::{
    fs::{self, File},
    io::{self, Read},
    path::Path,
};

use anyhow::{anyhow, bail, Context};
use clap::{Parser, ValueEnum};
use rugpi_common::{
    boot::{detect_boot_flow, grub, tryboot, uboot, BootFlow},
    ctrl_config::{load_config, CTRL_CONFIG_PATH},
    disk::{stream::ImgStream, PartitionTable},
    grub_patch_env,
    maybe_compressed::MaybeCompressed,
    mount::Mounted,
    partitions::{
        get_disk_id, get_hot_partitions, read_default_partitions, PartitionSet, Partitions,
    },
    rpi_patch_boot,
    stream_hasher::StreamHasher,
    utils::ascii_numbers,
    Anyhow,
};
use tempfile::tempdir;

use crate::{
    overlay::overlay_dir,
    utils::{clear_flag, reboot, set_flag, DEFERRED_SPARE_REBOOT_FLAG},
};

pub fn main() -> Anyhow<()> {
    let args = Args::parse();
    let config = load_config(CTRL_CONFIG_PATH)?;
    let partitions = Partitions::load(&config)?;
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
        Command::Update(update_cmd) => {
            match update_cmd {
                UpdateCommand::Install {
                    image,
                    no_reboot,
                    reboot: reboot_type,
                    keep_overlay,
                    check_hash,
                    stream,
                } => {
                    if reboot_type.is_some() && *no_reboot {
                        bail!("--no-reboot and --reboot are incompatible");
                    }

                    let check_hash = check_hash.as_deref()
                        .map(|encoded_hash| -> Anyhow<ImageHash> {
                            let (algorithm, hash) = encoded_hash
                                .split_once(':')
                                .ok_or_else(||
                                    anyhow!("Invalid format of hash. Format must be `sha256:<HEX-ENCODED-HASH>`.")
                                )?;
                            if algorithm != "sha256" {
                                bail!("Algorithm must be SHA256.");
                            }
                            let decoded_hash = hex::decode(hash)?;
                            Ok(ImageHash::Sha256(decoded_hash))
                    }).transpose()?;

                    let hot_partitions = get_hot_partitions(&partitions)?;
                    let default_partitions = read_default_partitions()?;
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

                    install_update_stream(&partitions, image, check_hash)?;

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
            }
        }
        Command::System(sys_cmd) => match sys_cmd {
            SystemCommand::Info => {
                let boot_flow = detect_boot_flow().context("loading boot flow")?;
                println!("Boot Flow: {}", boot_flow.as_str());
                let hot_partitions =
                    get_hot_partitions(&partitions).context("loading hot partitions")?;
                let default_partitions =
                    read_default_partitions().context("loading default partitions")?;
                println!("Hot: {}", hot_partitions.as_str());
                println!("Cold: {}", hot_partitions.flipped().as_str());
                println!("Default: {}", default_partitions.as_str());
                println!("Spare: {}", default_partitions.flipped().as_str());
            }
            SystemCommand::Commit => {
                let boot_flow = detect_boot_flow()?;
                let hot_partitions = get_hot_partitions(&partitions)?;
                let default_partitions = read_default_partitions()?;
                if hot_partitions != default_partitions {
                    match boot_flow {
                        BootFlow::Tryboot => tryboot::commit(hot_partitions)?,
                        BootFlow::UBoot => uboot::commit(hot_partitions)?,
                        BootFlow::GrubEfi => grub::commit(hot_partitions)?,
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
            UnstableCommand::SetDeferredSpareReboot { value } => match value {
                Boolean::True => set_flag(DEFERRED_SPARE_REBOOT_FLAG)?,
                Boolean::False => clear_flag(DEFERRED_SPARE_REBOOT_FLAG)?,
            },
        },
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub enum ImageHash {
    Sha256(Vec<u8>),
}

pub enum MaybeStreamHasher<R> {
    NoHash {
        reader: R,
    },
    Sha256 {
        hasher: StreamHasher<R, sha2::Sha256>,
        expected: Vec<u8>,
    },
}

impl<R> MaybeStreamHasher<R> {
    pub fn verify(self) -> Anyhow<()> {
        match self {
            MaybeStreamHasher::NoHash { .. } => Ok(()),
            MaybeStreamHasher::Sha256 { hasher, expected } => {
                let found = hasher.finalize();
                if expected.as_slice() != found.as_slice() {
                    bail!(indoc::formatdoc! {
                        r#"
                            **Image Hash Mismatch:**
                            Expected: {}
                            Found: {}
                        "#,
                        hex::encode(expected),
                        hex::encode(found)
                    })
                }
                Ok(())
            }
        }
    }
}

impl<R: Read> Read for MaybeStreamHasher<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            MaybeStreamHasher::NoHash { reader } => reader.read(buf),
            MaybeStreamHasher::Sha256 { hasher, .. } => hasher.read(buf),
        }
    }
}

fn install_update_stream(
    partitions: &Partitions,
    image: &String,
    check_hash: Option<ImageHash>,
) -> Anyhow<()> {
    let default_partitions = read_default_partitions()?;
    let spare_partitions = default_partitions.flipped();
    let reader: &mut dyn io::Read = if image == "-" {
        &mut io::stdin()
    } else {
        &mut File::open(image)?
    };
    let reader = match check_hash {
        Some(ImageHash::Sha256(expected)) => MaybeStreamHasher::Sha256 {
            hasher: StreamHasher::new(reader),
            expected,
        },
        None => MaybeStreamHasher::NoHash { reader },
    };
    println!("Copying partitions...");
    // let boot_label = format!("BOOT-{}", spare_partitions.as_str().to_uppercase());
    // let system_label = format!("system-{}", spare_partitions.as_str());
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
                    &mut fs::File::create(spare_partitions.boot_dev(partitions).unwrap())?,
                )?;
                // run!([
                //     "fatlabel",
                //     spare_partitions.boot_dev(partitions).unwrap(),
                //     &boot_label
                // ])?;
            }
            3 => {
                io::copy(
                    &mut partition,
                    &mut fs::File::create(spare_partitions.system_dev(partitions))?,
                )?;
                // run!([
                //     "e2label",
                //     spare_partitions.system_dev(partitions),
                //     &system_label
                // ])?;
            }
            _ => { /* Nothing to do! */ }
        }

        partition_idx += 1;
    }

    let mut hashed_stream = img_stream.into_inner().into_inner();
    // Make sure that the entire stream has been consumed. Otherwise, the hash
    // may not be match the file.
    loop {
        let mut buffer = vec![0; 4096];
        if hashed_stream.read_to_end(&mut buffer)? == 0 {
            break;
        }
    }
    hashed_stream.verify()?;

    let temp_dir_spare = tempdir()?;
    let temp_dir_spare = temp_dir_spare.path();

    // Path `/boot`.
    let boot_flow = detect_boot_flow()?;
    if matches!(boot_flow, BootFlow::Tryboot | BootFlow::UBoot) {
        let _mounted_spare = Mounted::mount(
            spare_partitions.boot_dev(partitions).unwrap(),
            temp_dir_spare,
        )?;
        let disk_id = get_disk_id(&partitions.parent_dev)?;
        let root = match spare_partitions {
            PartitionSet::A => format!("PARTUUID={disk_id}-05"),
            PartitionSet::B => format!("PARTUUID={disk_id}-06"),
        };
        rpi_patch_boot(temp_dir_spare, root)?;
    }
    if matches!(boot_flow, BootFlow::GrubEfi) {
        let _mounted_spare = Mounted::mount(
            spare_partitions.boot_dev(partitions).unwrap(),
            temp_dir_spare,
        )?;
        let table = PartitionTable::read(&partitions.parent_dev)?;
        let root_part = match spare_partitions {
            PartitionSet::A => &table.partitions[3],
            PartitionSet::B => &table.partitions[4],
        };
        let part_uuid = root_part
            .gpt_id
            .unwrap()
            .to_hex_str(ascii_numbers::Case::Lower);
        grub_patch_env(temp_dir_spare, part_uuid)?;
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
        /// Check whether the (streamed) image matches the given hash.
        #[clap(long)]
        check_hash: Option<String>,
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
    /// Set deferred spare reboot flag.
    SetDeferredSpareReboot { value: Boolean },
}
