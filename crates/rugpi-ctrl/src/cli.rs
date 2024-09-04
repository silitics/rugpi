//! Definition of the command line interface (CLI).

use std::{
    fs::{self, File},
    io::{self, Read},
    path::Path,
};

use anyhow::{anyhow, bail};
use clap::{Parser, ValueEnum};
use rugpi_common::{
    disk::stream::ImgStream,
    maybe_compressed::MaybeCompressed,
    stream_hasher::StreamHasher,
    system::{
        boot_entries::{BootEntry, BootEntryIdx},
        slots::SlotKind,
        System,
    },
    Anyhow,
};

use crate::{
    overlay::overlay_dir,
    utils::{clear_flag, reboot, set_flag, DEFERRED_SPARE_REBOOT_FLAG},
};

pub fn main() -> Anyhow<()> {
    let args = Args::parse();
    let system = System::initialize()?;
    match &args.command {
        Command::State(state_cmd) => match state_cmd {
            StateCommand::Reset => {
                fs::create_dir_all("/run/rugpi/state/.rugpi")?;
                fs::write("/run/rugpi/state/.rugpi/reset-state", "")?;
                reboot()?;
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
                    boot_entry,
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

                    if system.needs_commit()? {
                        bail!("System needs to be committed before installing an update.");
                    }

                    // Find the entry where we are going to install the update to.
                    let (entry_idx, entry) = match boot_entry {
                        Some(entry_name) => {
                            let Some(entry) = system.boot_entries().find_by_name(entry_name) else {
                                bail!("unable to find entry {entry_name}")
                            };
                            entry
                        }
                        None => {
                            let Some(entry) = system
                                .boot_entries()
                                .iter()
                                .filter(|(_, entry)| !entry.active())
                                .next()
                            else {
                                bail!("unable to find an entry");
                            };
                            entry
                        }
                    };
                    if entry.active() {
                        bail!("selected entry {} is active", entry.name());
                    }

                    if !keep_overlay {
                        let spare_overlay_dir = overlay_dir(entry);
                        fs::remove_dir_all(spare_overlay_dir).ok();
                    }

                    if *stream {
                        indoc::eprintdoc! {"
                        **Deprecation Warning:**
                        The option `--stream` has been deprecated and will be removed in future versions.
                        Streaming updates are the default now.
                    "}
                    }

                    install_update_stream(&system, image, check_hash, entry_idx, entry)?;

                    let reboot_type = reboot_type.clone().unwrap_or(if *no_reboot {
                        UpdateRebootType::No
                    } else {
                        UpdateRebootType::Yes
                    });

                    match reboot_type {
                        UpdateRebootType::Yes => {
                            system.boot_flow().set_try_next(&system, entry_idx)?;
                            reboot()?;
                        }
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
                println!("Boot Flow: {}", system.boot_flow().name());
                let hot = system.active_boot_entry().unwrap();
                let default = system.boot_flow().get_default(&system)?;
                let spare = system.spare_entry()?.unwrap().0;
                let cold = if hot == default { spare } else { default };
                let entries = system.boot_entries();
                println!("Hot: {}", entries[hot].name());
                println!("Cold: {}", entries[cold].name());
                println!("Default: {}", entries[default].name());
                println!("Spare: {}", entries[spare].name());
            }
            SystemCommand::Commit => {
                if system.needs_commit()? {
                    system.commit()?;
                } else {
                    println!("Hot partition is already the default!");
                }
            }
            SystemCommand::Reboot { spare } => {
                if *spare {
                    if let Some((spare, _)) = system.spare_entry()? {
                        system.boot_flow().set_try_next(&system, spare)?;
                    }
                }
                reboot()?;
            }
        },
        Command::Unstable(command) => match command {
            UnstableCommand::SetDeferredSpareReboot { value } => match value {
                Boolean::True => set_flag(DEFERRED_SPARE_REBOOT_FLAG)?,
                Boolean::False => clear_flag(DEFERRED_SPARE_REBOOT_FLAG)?,
            },
            UnstableCommand::PrintSystemInfo => {
                println!("Config:");
                println!("{:#?}", system.config());
                println!("Root:");
                println!("{:#?}", system.root());
                println!("Slots:");
                for (_, slot) in system.slots().iter() {
                    println!("{:#?}", slot)
                }
                println!("Boot Entries");
                println!("{:#?}", system.boot_entries());
            }
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
    system: &System,
    image: &String,
    check_hash: Option<ImageHash>,
    entry_idx: BootEntryIdx,
    entry: &BootEntry,
) -> Anyhow<()> {
    system.boot_flow().pre_install(system, entry_idx)?;

    let boot_slot = entry.get_slot("boot").unwrap();
    let system_slot = entry.get_slot("system").unwrap();

    let SlotKind::Raw(raw_boot_slot) = system.slots()[boot_slot].kind();
    let SlotKind::Raw(raw_system_slot) = system.slots()[system_slot].kind();

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
                    &mut fs::File::create(raw_boot_slot.device())?,
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
                    &mut fs::File::create(raw_system_slot.device())?,
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

    system.boot_flow().post_install(system, entry_idx)?;
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
        /// Boot entry to install the update to.
        #[clap(long)]
        boot_entry: Option<String>,
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
    SetDeferredSpareReboot {
        value: Boolean,
    },
    PrintSystemInfo,
}
