//! Definition of the command line interface (CLI).

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::Child;

use rugix_bundle::manifest::ChunkerAlgorithm;
use rugix_bundle::reader::block_provider::StoredBlockProvider;
use rugix_bundle::reader::PayloadTarget;
use rugix_bundle::source::{BundleSource, ReaderSource, SkipRead};
use rugix_bundle::BUNDLE_MAGIC;
use rugix_hashes::{HashAlgorithm, HashDigest};
use rugix_hooks::HooksLoader;
use tracing::{error, info};

use crate::system::boot_groups::{BootGroup, BootGroupIdx};
use crate::system::slots::SlotKind;
use crate::system::{System, SystemResult};
use clap::{Parser, ValueEnum};
use reportify::{bail, whatever, ErrorExt, ResultExt};
use rugix_common::disk::stream::ImgStream;
use rugix_common::maybe_compressed::{MaybeCompressed, PeekReader};
use rugix_common::stream_hasher::StreamHasher;
use xscript::{vars, Vars};

use crate::http_source::HttpSource;
use crate::overlay::overlay_dir;
use crate::slot_db::{self, BlockProvider};
use crate::system_state;
use crate::utils::{clear_flag, reboot, set_flag, DEFERRED_SPARE_REBOOT_FLAG};

fn create_rugix_state_directory() -> SystemResult<()> {
    fs::create_dir_all("/run/rugix/state/.rugix")
        .whatever("unable to create `/run/rugix/state/.rugix`")
}

fn set_rugix_state_flag(name: &str) -> SystemResult<()> {
    fs::write(Path::new("/run/rugix/state/.rugix").join(name), "")
        .whatever("unable to write state flag")
        .with_info(|_| format!("name: {name}"))
}

fn clear_rugix_state_flag(name: &str) -> SystemResult<()> {
    let path = Path::new("/run/rugix/state/.rugix").join(name);
    fs::remove_file(&path).or_else(|error| match error.kind() {
        io::ErrorKind::NotFound => Ok(()),
        _ => Err(error
            .whatever("unable to clear state flag")
            .with_info(format!("name: {name}"))),
    })?;
    if path.exists() {
        return Err(whatever!("unable to clear state flag").with_info(format!("name: {name}")));
    }
    Ok(())
}

pub fn main() -> SystemResult<()> {
    rugix_cli::CliBuilder::new().init();

    let args = Args::parse();
    let system = System::initialize()?;
    match &args.command {
        Command::State(state_cmd) => match state_cmd {
            StateCommand::Reset => {
                let reset_hooks = HooksLoader::default()
                    .load_hooks("state-reset")
                    .whatever("unable to load `state-reset` hooks")?;

                reset_hooks
                    .run_hooks("prepare", Vars::new())
                    .whatever("unable to run `state-reset/prepare` hooks")?;
                create_rugix_state_directory()?;
                set_rugix_state_flag("reset-state")?;
                reboot()?;
            }
            StateCommand::Overlay(overlay_cmd) => match overlay_cmd {
                OverlayCommand::ForcePersist { persist } => match persist {
                    Boolean::True => {
                        create_rugix_state_directory()?;
                        set_rugix_state_flag("force-persist-overlay")?;
                    }
                    Boolean::False => {
                        clear_rugix_state_flag("force-persist-overlay")?;
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
                    verify_bundle,
                    boot_entry,
                    without_boot_flow,
                } => {
                    if reboot_type.is_some() && *no_reboot {
                        bail!("--no-reboot and --reboot are incompatible");
                    }

                    let check_hash = check_hash.as_deref()
                        .map(|encoded_hash| -> SystemResult<ImageHash> {
                            let (algorithm, hash) = encoded_hash
                                .split_once(':')
                                .ok_or_else(||
                                    whatever!("Invalid format of hash. Format must be `sha256:<HEX-ENCODED-HASH>`.")
                                )?;
                            if algorithm != "sha256" {
                                bail!("Algorithm must be SHA256.");
                            }
                            let decoded_hash = hex::decode(hash).whatever("unable to decode image hash")?;
                            Ok(ImageHash::Sha256(decoded_hash))
                    }).transpose()?;

                    if system.needs_commit()? {
                        bail!("System needs to be committed before installing an update.");
                    }

                    // Find the entry where we are going to install the update to.
                    let (entry_idx, boot_group) = match boot_entry {
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
                                .find(|(_, entry)| !entry.active())
                            else {
                                bail!("unable to find an entry");
                            };
                            entry
                        }
                    };
                    if boot_group.active() {
                        bail!("selected entry {} is active", boot_group.name());
                    }

                    let hooks = HooksLoader::default()
                        .load_hooks("update-install")
                        .whatever("unable to load `update-install` hooks")?;

                    let hook_vars = vars! {
                        RUGIX_BOOT_GROUP = boot_group.name(),
                    };

                    hooks
                        .run_hooks("pre-update", hook_vars.clone())
                        .whatever("error running `pre-update` hooks")?;

                    if !keep_overlay {
                        let spare_overlay_dir = overlay_dir(boot_group);
                        fs::remove_dir_all(spare_overlay_dir).ok();
                    }

                    install_update_stream(
                        &system,
                        image,
                        check_hash,
                        verify_bundle,
                        entry_idx,
                        boot_group,
                        *without_boot_flow,
                    )?;

                    hooks
                        .run_hooks("post-update", hook_vars.clone())
                        .whatever("error running `post-update` hooks")?;

                    let reboot_type = reboot_type.clone().unwrap_or(if *no_reboot {
                        UpdateRebootType::No
                    } else {
                        UpdateRebootType::Yes
                    });

                    match reboot_type {
                        UpdateRebootType::Yes => {
                            system
                                .boot_flow()
                                .set_try_next(&system, entry_idx)
                                .whatever("unable to set next boot entry")?;
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
            SystemCommand::Info { json } => {
                let output = system_state::state_from_system(&system);
                if let Some(boot) = &output.boot {
                    eprintln!("Boot Flow: {}", boot.boot_flow);
                    eprintln!(
                        "Active Boot Group: {}",
                        boot.active_group.as_deref().unwrap_or("<unknown>")
                    );
                    eprintln!(
                        "Default Boot Group: {}",
                        boot.default_group.as_deref().unwrap_or("<unknown>")
                    );
                }
                for (name, info) in &output.slots {
                    eprintln!(
                        "Slot {name:?}: {}",
                        if let Some(active) = info.active {
                            if active {
                                "active"
                            } else {
                                "inactive"
                            }
                        } else {
                            "<unknown>"
                        }
                    );
                }
                if !rugix_cli::is_attended() || *json {
                    serde_json::to_writer(std::io::stdout(), &output)
                        .whatever("unable to write system info to stdout")?;
                }
            }
            SystemCommand::Commit => {
                if system.needs_commit()? {
                    let hooks = HooksLoader::default()
                        .load_hooks("system-commit")
                        .whatever("unable to load `system-commit` hooks")?;
                    hooks
                        .run_hooks("pre-commit", Vars::new())
                        .whatever("unable to run `pre-commit` hooks")?;
                    system.commit()?;
                    hooks
                        .run_hooks("post-commit", Vars::new())
                        .whatever("unable to run `post-commit` hooks")?;
                } else {
                    println!("Hot partition is already the default!");
                }
            }
            SystemCommand::Reboot { spare } => {
                if *spare {
                    if let Some((spare, _)) = system.spare_entry()? {
                        system
                            .boot_flow()
                            .set_try_next(&system, spare)
                            .whatever("unable to set next boot group")?;
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
        Command::Slots(slots_command) => match slots_command {
            SlotsCommand::Inspect { slot } => {
                let indices = slot_db::get_stored_indices(slot)?;
                if indices.is_empty() {
                    eprintln!("No indices for slot {slot}")
                } else {
                    for index in &indices {
                        eprintln!("Found index {:?}", &index.index_file);
                    }
                }
            }
            SlotsCommand::CreateIndex {
                slot,
                chunker: chunker_algorithm,
                hash_algorithm,
            } => {
                let Some((_, slot)) = system.slots().find_by_name(slot) else {
                    bail!("slot {slot} not found")
                };
                match slot.kind() {
                    SlotKind::Block(block_slot) => {
                        slot_db::add_index(
                            slot.name(),
                            block_slot.device().path(),
                            chunker_algorithm,
                            hash_algorithm,
                        )?;
                    }
                    SlotKind::File { path } => {
                        slot_db::add_index(slot.name(), path, chunker_algorithm, hash_algorithm)?;
                    }
                    SlotKind::Custom { .. } => {
                        bail!("cannot create indices on custom slots");
                    }
                }
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
    pub fn verify(self) -> SystemResult<()> {
        match self {
            MaybeStreamHasher::NoHash { .. } => Ok(()),
            MaybeStreamHasher::Sha256 { hasher, expected } => {
                let found = hasher.finalize();
                if expected.as_slice() != found.as_slice() {
                    return Err(whatever(indoc::formatdoc! {
                        r#"
                            **Image Hash Mismatch:**
                            Expected: {}
                            Found: {}
                        "#,
                        hex::encode(expected),
                        hex::encode(found)
                    }));
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
    verify_bundle: &Option<HashDigest>,
    entry_idx: BootGroupIdx,
    entry: &BootGroup,
    without_boot_flow: bool,
) -> SystemResult<()> {
    if image.starts_with("http") {
        if check_hash.is_some() {
            bail!("--check-hash is not supported for update bundles, use --verify-bundle");
        }
        let mut bundle_source = HttpSource::new(image)?;
        install_update_bundle(
            system,
            &mut bundle_source,
            verify_bundle,
            entry_idx,
            entry,
            without_boot_flow,
        )?;
        info!(
            "downloaded {:.1}% of the full bundle",
            bundle_source.get_download_ratio() * 100.0
        );
        return Ok(());
    }
    let reader: &mut dyn io::Read = if image == "-" {
        &mut io::stdin()
    } else {
        &mut File::open(image).whatever("error opening image")?
    };
    let reader = match check_hash.clone() {
        Some(ImageHash::Sha256(expected)) => MaybeStreamHasher::Sha256 {
            hasher: StreamHasher::new(reader),
            expected,
        },
        None => MaybeStreamHasher::NoHash { reader },
    };
    let mut update_stream = PeekReader::new(reader);

    let magic = update_stream
        .peek(BUNDLE_MAGIC.len())
        .whatever("error reading bundle magic")?;

    if magic == BUNDLE_MAGIC {
        if check_hash.is_some() {
            bail!("--check-hash is not supported for update bundles, use --verify-bundle");
        }
        let bundle_source = ReaderSource::<_, SkipRead>::from_unbuffered(update_stream);
        return install_update_bundle(
            system,
            bundle_source,
            verify_bundle,
            entry_idx,
            entry,
            without_boot_flow,
        );
    }
    if verify_bundle.is_some() {
        bail!("--verify-bundle is not supported on images, use --check-hash");
    }

    let update_stream =
        MaybeCompressed::new(update_stream).whatever("error decompressing stream")?;

    system
        .boot_flow()
        .pre_install(system, entry_idx)
        .whatever("error executing pre-install step")?;

    let boot_slot = entry.get_slot("boot").unwrap();
    let system_slot = entry.get_slot("system").unwrap();

    let SlotKind::Block(raw_boot_slot) = system.slots()[boot_slot].kind() else {
        bail!("boot slot must be a block device");
    };
    let SlotKind::Block(raw_system_slot) = system.slots()[system_slot].kind() else {
        bail!("system slot must be a block device");
    };

    let mut img_stream =
        ImgStream::new(update_stream).whatever("error reading image partitions")?;
    let mut partition_idx = 0;
    while let Some(mut partition) = img_stream
        .next_partition()
        .whatever("error reading next partition")?
    {
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
                    &mut fs::File::create(raw_boot_slot.device())
                        .whatever("error opening boot partition file")?,
                )
                .whatever("error copying boot partition")?;
            }
            3 => {
                io::copy(
                    &mut partition,
                    &mut fs::File::create(raw_system_slot.device())
                        .whatever("error opening system partition file")?,
                )
                .whatever("error copying system partition")?;
            }
            _ => { /* Nothing to do! */ }
        }

        partition_idx += 1;
    }

    let mut hashed_stream = img_stream.into_inner().into_inner().into_inner();
    // Make sure that the entire stream has been consumed. Otherwise, the hash
    // may not be match the file.
    loop {
        let mut buffer = vec![0; 4096];
        if hashed_stream
            .read_to_end(&mut buffer)
            .whatever("error reading image")?
            == 0
        {
            break;
        }
    }

    if let Err(error) = hashed_stream.verify() {
        error!("hash verification failed");
        if let Err(error) =
            rugix_fs::File::open_write(raw_boot_slot.device().path()).and_then(|mut device| {
                device.write_zeros(
                    byte_calc::NumBytes::new(0),
                    byte_calc::NumBytes::mebibytes(1),
                )
            })
        {
            error!("error overwriting boot partition: {error:?}");
        }
        if let Err(error) =
            rugix_fs::File::open_write(raw_system_slot.device().path()).and_then(|mut device| {
                device.write_zeros(
                    byte_calc::NumBytes::new(0),
                    byte_calc::NumBytes::mebibytes(1),
                )
            })
        {
            error!("error overwriting system partition: {error:?}");
        }
        return Err(error);
    }

    system
        .boot_flow()
        .post_install(system, entry_idx)
        .whatever("error running post-install step")?;
    Ok(())
}

fn install_update_bundle<R: BundleSource>(
    system: &System,
    bundle_source: R,
    verify_bundle: &Option<HashDigest>,
    entry_idx: BootGroupIdx,
    entry: &BootGroup,
    without_boot_flow: bool,
) -> SystemResult<()> {
    if !without_boot_flow {
        system
            .boot_flow()
            .pre_install(system, entry_idx)
            .whatever("error executing pre-install step")?;
    }

    let mut bundle_reader =
        rugix_bundle::reader::BundleReader::start(bundle_source, verify_bundle.clone())
            .whatever("unable to read bundle")?;

    while let Some(payload) = bundle_reader
        .next_payload()
        .whatever("unable to read payload")?
    {
        let payload_entry = payload.entry();
        if let Some(slot_type) = &payload_entry.type_slot {
            let slot = entry
                .get_slot(&slot_type.slot)
                .or_else(|| system.slots().find_by_name(&slot_type.slot).map(|e| e.0));
            if let Some(slot) = slot {
                let slot = &system.slots()[slot];
                eprintln!(
                    "Installing bundle payload {} to slot {}",
                    payload.idx(),
                    slot.name()
                );
                slot_db::erase(slot.name())?;
                let mut block_provider = None;
                if let Some(block_encoding) = &payload.header().block_encoding {
                    let mut provider = BlockProvider::new(
                        block_encoding.chunker.clone(),
                        block_encoding.hash_algorithm,
                    );
                    for (_, slot) in system.slots().iter() {
                        // Since we erased all the indices of the target slot, it
                        // is fine to also add the target slot here.
                        match slot.kind() {
                            SlotKind::Block(block_slot) => {
                                provider.add_slot(
                                    slot.name(),
                                    block_slot.device().path().to_path_buf(),
                                )?;
                            }
                            SlotKind::File { path } => {
                                provider.add_slot(slot.name(), path.to_path_buf())?;
                            }
                            SlotKind::Custom { .. } => { /* nothing to do */ }
                        }
                    }
                    block_provider = Some(provider);
                }
                match slot.kind() {
                    SlotKind::Block(block_slot) => {
                        let target = std::fs::OpenOptions::new()
                            .read(true)
                            .write(true)
                            .open(block_slot.device())
                            .whatever("unable to open payload target")?;
                        payload
                            .decode_into(
                                target,
                                block_provider
                                    .as_ref()
                                    .map(|p| p as &dyn StoredBlockProvider),
                            )
                            .whatever("unable to decode payload")?;
                    }
                    SlotKind::File { path } => {
                        let target = std::fs::OpenOptions::new()
                            .read(true)
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(path)
                            .whatever("unable to open payload target")?;
                        payload
                            .decode_into(
                                target,
                                block_provider
                                    .as_ref()
                                    .map(|p| p as &dyn StoredBlockProvider),
                            )
                            .whatever("unable to decode payload")?;
                    }
                    SlotKind::Custom { handler } => {
                        let target = CustomTarget::new(handler.iter().map(|arg| arg.as_str()))?;
                        payload
                            .decode_into(
                                target,
                                block_provider
                                    .as_ref()
                                    .map(|p| p as &dyn StoredBlockProvider),
                            )
                            .whatever("unable to decode payload")?;
                    }
                }
                continue;
            } else {
                error!(
                    "slot {:?} for bundle payload {} not found",
                    slot_type.slot,
                    payload.idx()
                );
            }
        } else if let Some(type_execute) = &payload_entry.type_execute {
            eprintln!("executing update payload {}", payload.idx(),);
            let target = CustomTarget::new(type_execute.handler.iter().map(|arg| arg.as_str()))?;
            payload
                .decode_into(target, None)
                .whatever("unable to decode payload")?;
            continue;
        }
        payload.skip().whatever("unable to skip payload")?;
    }

    if !without_boot_flow {
        system
            .boot_flow()
            .post_install(system, entry_idx)
            .whatever("error running post-install step")?;
    }
    Ok(())
}

#[derive(Debug)]
pub struct CustomTarget {
    child: Child,
}

impl CustomTarget {
    pub fn new<'arg>(mut command: impl Iterator<Item = &'arg str>) -> SystemResult<Self> {
        let Some(prog) = command.next() else {
            bail!("custom update handler cannot be an empty sequence");
        };
        let child = std::process::Command::new(prog)
            .args(command)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .whatever("unable to spawn custom update handler")?;
        Ok(Self { child })
    }
}

impl PayloadTarget for CustomTarget {
    fn write(&mut self, bytes: &[u8]) -> rugix_bundle::BundleResult<()> {
        self.child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(bytes)
            .whatever("unable to write payload to custom handler")
    }

    fn finalize(mut self) -> rugix_bundle::BundleResult<()> {
        info!("waiting on custom update handler to finalize");
        // Flush all bytes and close stdin.
        drop(self.child.stdin.take().unwrap());
        let status = self
            .child
            .wait()
            .whatever("error waiting for update handler")?;
        if !status.success() {
            bail!(
                "error running custom update handler, code {:?}",
                status.code()
            )
        }
        Ok(())
    }
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
    /// Manage the update slots of the system.
    #[clap(subcommand)]
    Slots(SlotsCommand),
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
pub enum SlotsCommand {
    /// Query the state of a slot.
    Inspect { slot: String },
    /// Add an index to a slot.
    CreateIndex {
        slot: String,
        chunker: ChunkerAlgorithm,
        hash_algorithm: HashAlgorithm,
    },
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
        /// Verify a bundle.
        #[clap(long)]
        verify_bundle: Option<HashDigest>,
        /// Prevent Rugix from rebooting the system.
        #[clap(long)]
        no_reboot: bool,
        /// Do not involve the boot flow in the update.
        #[clap(long, hide(true))]
        without_boot_flow: bool,
        /// Do not delete an existing overlay.
        #[clap(long)]
        keep_overlay: bool,
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
    Info {
        /// Output system information as JSON.
        #[clap(long)]
        json: bool,
    },
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
