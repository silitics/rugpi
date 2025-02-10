use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{fs, io, thread};

use byte_calc::NumBytes;
use nix::mount::MntFlags;
use reportify::{bail, ensure, ErrorExt, ResultExt};
use rugix_common::disk::blkdev::BlockDevice;

use tracing::{info, warn};

use crate::config::bootstrapping::{BootstrappingConfig, DefaultLayoutConfig, SystemLayoutConfig};
use crate::config::state::{
    OverlayConfig, PersistConfig, PersistDirectoryConfig, PersistFileConfig, StateConfig,
};
use crate::config::system::PartitionConfig;
use crate::state::load_state_config;
use crate::system::config::load_system_config;
use crate::system::partitions::resolve_data_partition;
use crate::system::paths::{MOUNT_POINT_CONFIG, MOUNT_POINT_DATA, MOUNT_POINT_SYSTEM};
use crate::system::root::{find_system_device, SystemRoot};
use crate::system::{System, SystemError, SystemResult};
use rugix_common::disk::blkpg::update_kernel_partitions;
use rugix_common::disk::repart::{
    generic_efi_partition_schema, generic_mbr_partition_schema, repart, PartitionSchema,
    SchemaPartition,
};
use rugix_common::disk::PartitionTable;
use rugix_common::partitions::mkfs_ext4;
use rugix_hooks::HooksLoader;
use xscript::{run, Run, Vars};

use crate::utils::{clear_flag, is_flag_set, is_init_process, reboot, DEFERRED_SPARE_REBOOT_FLAG};

pub fn main() -> SystemResult<()> {
    ensure!(is_init_process(), "process must be the init process");
    let result = init();
    match &result {
        Ok(_) => {
            eprintln!("initialization procedure terminated unexpectedly");
        }
        Err(error) => {
            eprintln!("error during initialization");
            eprintln!("{error:?}");
        }
    }
    // nix::unistd::execv(c"/bin/bash", &[c"/bin/bash"]).whatever("unable to run bash")?;
    eprintln!("waiting for 30 seconds...");
    thread::sleep(Duration::from_secs(30));
    Ok(())
}

/// The `cp` executable.
const CP: &str = "/usr/bin/cp";
/// The `fsck` executable.
const FSCK: &str = "/usr/sbin/fsck";
/// The `mount` executable.
const MOUNT: &str = "/usr/bin/mount";
/// The `sync` executable.
const SYNC: &str = "/usr/bin/sync";

const DEFAULT_STATE_DIR: &str = "/run/rugix/mounts/data/state/default";

fn init() -> SystemResult<()> {
    println!(include_str!("../assets/BANNER.txt"));

    rugix_cli::CliBuilder::new().init();

    // Mount essential filesystems.
    mount_essential_filesystems()?;

    let system_config = load_system_config()?;
    let Some(system_device) = find_system_device() else {
        bail!("unable to determine system device")
    };
    let Some(root) = SystemRoot::from_system_device(&system_device) else {
        bail!("unable to determine system root");
    };

    let Some(config_partition) = (match system_config.config_partition {
        Some(partition) => {
            if let Some(partition) = partition.partition {
                root.resolve_partition(partition)
            } else if let Some(device) = partition.device {
                Some(BlockDevice::new(device).whatever("unable to find config partition device")?)
            } else {
                None
            }
        }
        None => root.resolve_partition(1),
    }) else {
        bail!("bootstrapping requires a config partition");
    };

    fs::create_dir_all(MOUNT_POINT_CONFIG).ok();
    run!([
        MOUNT,
        "-o",
        "ro",
        config_partition.path(),
        MOUNT_POINT_CONFIG
    ])
    .whatever("unable to mount config partition")?;

    if Path::new(MOUNT_POINT_CONFIG)
        .join(".rugix/bootstrap")
        .exists()
    {
        bootstrap(&root)?;
        run!([MOUNT, "-o", "remount,rw", MOUNT_POINT_CONFIG])
            .whatever("unable to mount config partition as read-write")?;
        std::fs::remove_file(Path::new(MOUNT_POINT_CONFIG).join(".rugix/bootstrap"))
            .whatever("unable to remove bootstrap marker")?;
        run!([MOUNT, "-o", "remount,ro", MOUNT_POINT_CONFIG])
            .whatever("unable to mount config partition as readonly")?;
        info!("Done bootstrapping")
    }

    let Some(data_partition) = resolve_data_partition(
        Some(&root),
        system_config
            .data_partition
            .as_ref()
            .unwrap_or(&PartitionConfig::new()),
    ) else {
        bail!("Rugix pre-init requires a data partition");
    };

    // 3️⃣ Check and mount the data partition.
    run!([FSCK, "-y", data_partition.path()]).whatever("unable to check filesystem")?;
    fs::create_dir_all(MOUNT_POINT_DATA).ok();
    run!([
        MOUNT,
        "-o",
        "noatime",
        data_partition.path(),
        MOUNT_POINT_DATA
    ])
    .whatever("unable to mount data partition")?;

    let state_config = load_state_config()?;

    if !matches!(state_config.overlay, Some(OverlayConfig::Disabled)) {
        // 4️⃣ Setup remaining mount points in `/run/rugix/mounts`.
        fs::create_dir_all(MOUNT_POINT_SYSTEM).ok();
        run!([MOUNT, "-o", "ro", system_device.path(), MOUNT_POINT_SYSTEM])
            .whatever("unable to mount system partition")?;
    }

    let system = System::initialize()?;

    if let Err(error) = check_deferred_spare_reboot(&system) {
        println!("Warning: Error executing deferred reboot.");
        println!("{:?}", error);
    }

    // 6️⃣ Setup state in `/run/rugix/state`.
    let state_profile = Path::new(DEFAULT_STATE_DIR);
    if state_profile.join(".rugix/reset-state").exists() {
        let reset_hooks = HooksLoader::default()
            .load_hooks("state-reset")
            .whatever("unable to load `state-reset` hooks")?;

        reset_hooks
            .run_hooks("pre-reset", Vars::new())
            .whatever("unable to run `pre-reset` hooks")?;
        // The existence of the file indicates that the state shall be reset.
        fs::remove_dir_all(state_profile).ok();
        reset_hooks
            .run_hooks("pre-reset", Vars::new())
            .whatever("unable to run `post-reset` hooks")?;
    }
    fs::create_dir_all(state_profile).ok();
    fs::create_dir_all(STATE_DIR).ok();
    run!([MOUNT, "--bind", &state_profile, STATE_DIR])
        .whatever("unable to bind mount state profile")?;

    // 7️⃣ Setup the root filesystem overlay.
    let root_dir = setup_root_overlay(&system, &state_config, state_profile)?;

    // 8️⃣ Setup the bind mounts for the persistent state.
    setup_persistent_state(&root_dir, state_profile, &state_config)?;

    // 9️⃣ Restore the machine id and hand off to Systemd.
    exec_chroot_init(&root_dir)?;

    Ok(())
}

const STATE_DIR: &str = "/run/rugix/state";

pub fn state_dir() -> &'static Path {
    Path::new(STATE_DIR)
}

const BOOTSTRAP_CONFIG_PATH: &str = "/etc/rugix/bootstrapping.toml";

fn load_bootstrap_config() -> SystemResult<BootstrappingConfig> {
    Ok(if Path::new(BOOTSTRAP_CONFIG_PATH).exists() {
        toml::from_str(
            &fs::read_to_string(BOOTSTRAP_CONFIG_PATH)
                .whatever("unable to read system configuration file")?,
        )
        .whatever("unable to parse system configuration file")?
    } else {
        BootstrappingConfig::default()
    })
}

fn bootstrap(root: &SystemRoot) -> SystemResult<()> {
    let bootstrap_hooks = HooksLoader::default()
        .load_hooks("bootstrap")
        .whatever("unable to load bootstrap hooks")?;

    bootstrap_hooks
        .run_hooks("prepare", Vars::new())
        .whatever("unable to run `bootstrap/prepare` hooks")?;

    let bootstrap_config = load_bootstrap_config()?;

    if bootstrap_config.disabled.unwrap_or(false) {
        warn!("Found bootstrapping marker but bootstrapping is disabled. Skip bootstrapping");
        return Ok(());
    }

    info!("Found bootstrapping marker. Begin bootstrapping");
    let layout = bootstrap_config.layout.unwrap_or_else(|| {
        SystemLayoutConfig::Default(DefaultLayoutConfig::new(NumBytes::gibibytes(4)))
    });

    let ty = root.table.as_ref().unwrap().ty();

    let schema = match &layout {
        SystemLayoutConfig::Mbr(partition_layout_config)
        | SystemLayoutConfig::Gpt(partition_layout_config) => Some(PartitionSchema {
            ty,
            partitions: partition_layout_config
                .partitions
                .iter()
                .map(|part| SchemaPartition {
                    number: None,
                    name: part.name.clone(),
                    size: part.size.map(|s| s.raw.into()),
                    ty: part.ty,
                })
                .collect(),
        }),
        SystemLayoutConfig::Default(default_layout_config) => match ty {
            rugix_common::disk::PartitionTableType::Gpt => Some(generic_efi_partition_schema(
                default_layout_config.system_size.raw.into(),
            )),
            rugix_common::disk::PartitionTableType::Mbr => Some(generic_mbr_partition_schema(
                default_layout_config.system_size.raw.into(),
            )),
        },
        SystemLayoutConfig::None => None,
    };

    if let Some(schema) = schema {
        bootstrap_hooks
            .run_hooks("pre-layout", Vars::new())
            .whatever("unable to run `bootstrap/pre-layout` hooks")?;
        if let Some((old_table, _)) = bootstrap_partitions(&schema, root)? {
            // Partition is new, let's see whether we need to create a filesystem.
            match &layout {
                SystemLayoutConfig::Mbr(partition_layout_config)
                | SystemLayoutConfig::Gpt(partition_layout_config) => {
                    for (idx, config) in partition_layout_config.partitions.iter().enumerate() {
                        let Some(filesystem) = &config.filesystem else {
                            continue;
                        };
                        if idx < old_table.partitions.len() {
                            warn!(
                                "refuse to create filesystems on already existing partition {}",
                                idx + 1
                            );
                            continue;
                        }
                        let block_device = root.resolve_partition((idx + 1) as u32).unwrap();
                        match filesystem {
                            crate::config::bootstrapping::Filesystem::Ext4(ext4_filesystem) => {
                                mkfs_ext4(
                                    block_device,
                                    ext4_filesystem.label.as_deref().unwrap_or(""),
                                )
                                .whatever("unable to create filesystem on data partition")?;
                            }
                        }
                    }
                }
                SystemLayoutConfig::Default(_) => {
                    let data_partition_idx = if ty.is_mbr() { 7 } else { 6 };
                    if data_partition_idx as usize >= old_table.partitions.len() {
                        // Create Ext4 filesystem on data partition.
                        mkfs_ext4(root.resolve_partition(data_partition_idx).unwrap(), "data")
                            .whatever("unable to create filesystem on data partition")?;
                    }
                }
                SystemLayoutConfig::None => unreachable!(),
            }
        }
        bootstrap_hooks
            .run_hooks("post-layout", Vars::new())
            .whatever("unable to run `bootstrap/post-layout` hooks")?;
    }

    Ok(())
}

/// Mounts the essential filesystems `/proc`, `/sys`, and `/run`.
fn mount_essential_filesystems() -> SystemResult<()> {
    // We ignore any errors. Errors likely mean that the filesystems have already been
    // mounted.
    if let Err(error) = run!([MOUNT, "-t", "proc", "proc", "/proc"]) {
        eprintln!(
            "{:?}",
            error.whatever::<SystemError, _>("error mounting /proc"),
        );
    }
    if let Err(error) = run!([MOUNT, "-t", "sysfs", "sys", "/sys"]) {
        eprintln!(
            "{:?}",
            error.whatever::<SystemError, _>("error mounting /sys"),
        );
    }
    if let Err(error) = run!([MOUNT, "-t", "tmpfs", "tmp", "/run"]) {
        eprintln!(
            "{:?}",
            error.whatever::<SystemError, _>("error mounting /tmp"),
        );
    }
    Ok(())
}

/// Initializes the partitions and expands the partition table during the first boot.
fn bootstrap_partitions(
    schema: &PartitionSchema,
    root: &SystemRoot,
) -> SystemResult<Option<(PartitionTable, PartitionTable)>> {
    let old_table =
        PartitionTable::read(root.device.path()).whatever("unable to read partition table")?;
    if let Some(new_table) =
        repart(&old_table, schema).whatever("unable to compute new partition table")?
    {
        // Write new partition table to disk.
        new_table
            .write(root.device.path())
            .whatever("unable to write new partition table")?;
        run!([SYNC]).whatever("unable to synchronize file systems")?;
        // Inform the kernel about new partitions.
        update_kernel_partitions(root.device.path(), &old_table, &new_table)
            .whatever("unable to update partitions in the kernel")?;
        Ok(Some((old_table, new_table)))
    } else {
        Ok(None)
    }
}

/// Sets up the overlay.
fn setup_root_overlay(
    system: &System,
    config: &StateConfig,
    state_profile: &Path,
) -> SystemResult<PathBuf> {
    let overlay_state = state_profile.join("overlay");
    let force_persist = state_profile.join(".rugix/force-persist-overlay").exists();
    let overlay_config = config.overlay.clone().unwrap_or(OverlayConfig::Discard);

    if !force_persist && !matches!(overlay_config, OverlayConfig::Persist) {
        fs::remove_dir_all(&overlay_state).ok();
    }

    let (overlay_dir, overlay_root_dir, overlay_work_dir, upper) = match overlay_config {
        OverlayConfig::Persist | OverlayConfig::Discard => {
            let active_boot_entry = &system.boot_entries()[system.active_boot_entry().unwrap()];
            let hot_overlay_state = overlay_state.join(active_boot_entry.name());
            const OVERLAY_DIR: &str = "/run/rugix/mounts/data/overlay";
            const OVERLAY_ROOT_DIR: &str = "/run/rugix/mounts/data/overlay/root";
            const OVERLAY_WORK_DIR: &str = "/run/rugix/mounts/data/overlay/work";
            (
                OVERLAY_DIR,
                OVERLAY_ROOT_DIR,
                OVERLAY_WORK_DIR,
                hot_overlay_state,
            )
        }
        OverlayConfig::InMemory => {
            const TEMP_OVERLAY_DIR: &str = "/run/rugix/overlay";
            const TEMP_OVERLAY_ROOT_DIR: &str = "/run/rugix/overlay/root";
            const TEMP_OVERLAY_WORK_DIR: &str = "/run/rugix/overlay/work";
            (
                TEMP_OVERLAY_DIR,
                TEMP_OVERLAY_ROOT_DIR,
                TEMP_OVERLAY_WORK_DIR,
                PathBuf::from("/run/rugix/overlay/upper"),
            )
        }
        OverlayConfig::Disabled => return Ok(PathBuf::from("/")),
    };

    // Reinitialize `work` and `root` directories.
    fs::remove_dir_all(overlay_dir).ok();
    fs::create_dir_all(overlay_work_dir).ok();
    fs::create_dir_all(overlay_root_dir).ok();
    fs::create_dir_all(&upper).ok();

    let upper = upper.to_string_lossy();
    run!([
        MOUNT,
        "-t",
        "overlay",
        "overlay",
        "-o",
        "noatime,lowerdir={MOUNT_POINT_SYSTEM},upperdir={upper},workdir={overlay_work_dir}",
        overlay_root_dir
    ])
    .whatever("unable to setup system overlay mounts")?;
    let overlay_root_dir = Path::new(overlay_root_dir);
    run!([MOUNT, "--rbind", "/run", overlay_root_dir.join("run")])
        .whatever("unable to rbind /run")?;
    Ok(overlay_root_dir.to_path_buf())
}

/// Sets up the bind mounts required for the persistent state.
fn setup_persistent_state(
    root_dir: &Path,
    state_profile: &Path,
    state_config: &StateConfig,
) -> SystemResult<()> {
    let persist_dir = state_profile.join("persist");
    fs::create_dir_all(state_profile).ok();

    let Some(persist) = &state_config.persist else {
        return Ok(());
    };

    for persist in persist {
        match persist {
            PersistConfig::Directory(PersistDirectoryConfig { directory }) => {
                let directory = path_strip_root(directory.as_ref());
                eprintln!(
                    "Setting up bind mounds for directory `{}`...",
                    directory.to_string_lossy()
                );
                let system_path = root_dir.join(directory);
                let state_path = persist_dir.join(directory);
                if system_path.exists() && !system_path.is_dir() {
                    bail!(
                        "Error persisting `{}`, not a directory!",
                        directory.to_string_lossy()
                    );
                }
                if !state_path.is_dir() {
                    fs::remove_dir_all(&state_path).ok();
                    create_parent_dir(&state_path).ok();
                    if system_path.is_dir() {
                        run!([CP, "-a", &system_path, &state_path])
                            .whatever("unable to copy system files from root partition to state")?;
                    } else {
                        fs::create_dir_all(&state_path).ok();
                    }
                }
                if !system_path.is_dir() {
                    fs::create_dir_all(&system_path)
                        .whatever("unable to create system directory")?;
                }
                run!([MOUNT, "--bind", &state_path, &system_path])
                    .whatever("unable to bind-mount persistent directory")?;
            }
            PersistConfig::File(PersistFileConfig { file, default }) => {
                let file = path_strip_root(file.as_ref());
                eprintln!(
                    "Setting up bind mounds for file `{}`...",
                    file.to_string_lossy()
                );
                let system_path = root_dir.join(file);
                let state_path = persist_dir.join(file);
                if system_path.exists() && !system_path.is_file() {
                    bail!("Error persisting `{}`, not a file!", file.to_string_lossy());
                }
                if !state_path.is_file() {
                    fs::remove_dir_all(&state_path).ok();
                    create_parent_dir(&state_path)
                        .whatever("unable to create parent directory of persistent file")?;
                    if system_path.is_file() {
                        run!([CP, "-a", &system_path, &state_path])
                            .whatever("unable to copy persistent file from system")?;
                    } else {
                        fs::write(&state_path, default.as_deref().unwrap_or_default())
                            .whatever("unable to write default")?;
                    }
                }
                if !system_path.is_file() {
                    create_parent_dir(&system_path)
                        .whatever("unable to create system parent directory")?;
                    fs::write(&system_path, "").whatever("unable to initialize file")?;
                }
                run!([MOUNT, "--bind", &state_path, &system_path])
                    .whatever("unable to bind mount file")?;
            }
        }
    }

    Ok(())
}

/// Strips the root `/` from a path.
fn path_strip_root(path: &Path) -> &Path {
    if let Ok(stripped) = path.strip_prefix("/") {
        stripped
    } else {
        path
    }
}

/// Creates the parent directories of a path.
fn create_parent_dir(path: impl AsRef<Path>) -> io::Result<()> {
    fn _create_parent_dir(path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                format!("path `{path:?}` has no parent"),
            ))
        }
    }
    _create_parent_dir(path.as_ref())
}

/// Makes sure `/etc/machine-id` has been restored/initialized.
fn restore_machine_id(root_dir: &Path) -> SystemResult<()> {
    let state_machine_id = state_dir().join("machine-id");
    let system_machine_id = root_dir.join("etc/machine-id");
    if !state_machine_id.exists() {
        let machine_id = format!("{}", uuid::Uuid::new_v4().simple());
        fs::write(&system_machine_id, machine_id.as_bytes())
            .whatever("unable to write machine-id")?;
    }
    fs::copy(system_machine_id, state_machine_id)
        .whatever("unable to copy machine id into state")?;
    Ok(())
}

/// Changes the root directory and hands off to the system init process.
///
/// We follow the example from the manpage of the `pivot_root` system call here.
///
/// We are not using `chroot` as this lead to problems with Docker.
fn exec_chroot_init(root_dir: &Path) -> SystemResult<()> {
    if root_dir != Path::new("/") {
        restore_machine_id(root_dir)?;
        println!("Changing current working directory to overlay root directory.");
        nix::unistd::chdir(root_dir).whatever("unable to switch to overlay directory")?;
        println!("Pivoting root mount point to current working directory.");
        nix::unistd::pivot_root(".", ".").whatever("unable to pivot root directory")?;
        println!("Unmounting the previous root filesystem.");
        nix::mount::umount2(".", MntFlags::MNT_DETACH)
            .whatever("unable to unmount old root directory")?;
        println!("Starting system init process.");
    }
    let systemd_init = &CString::new("/sbin/init").unwrap();
    nix::unistd::execv(systemd_init, &[systemd_init]).whatever("unable to run system init")?;
    Ok(())
}

/// Reboot the system to the spare partitions if the deferred spare reboot flag is set.
fn check_deferred_spare_reboot(system: &System) -> SystemResult<()> {
    if is_flag_set(DEFERRED_SPARE_REBOOT_FLAG) {
        println!("Executing deferred reboot to spare partitions.");
        // Remove file and make sure that changes are synced to disk.
        clear_flag(DEFERRED_SPARE_REBOOT_FLAG)?;
        nix::unistd::sync();
        if !system.needs_commit()? {
            // Reboot to the spare partitions.
            if let Some((spare, _)) = system.spare_entry()? {
                system
                    .boot_flow()
                    .set_try_next(system, spare)
                    .whatever("unable to set next boot group")?;
                reboot()?;
            }
        }
    }
    Ok(())
}
