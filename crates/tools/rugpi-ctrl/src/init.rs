use std::ffi::CString;
use std::path::Path;
use std::time::Duration;
use std::{fs, io, thread};

use nix::mount::MntFlags;
use reportify::{bail, ensure, ErrorExt, ResultExt};
use rugix_hooks::HooksLoader;
use rugpi_common::ctrl_config::{load_config, Config, Overlay, CTRL_CONFIG_PATH};
use rugpi_common::disk::blkpg::update_kernel_partitions;
use rugpi_common::disk::repart::{
    generic_efi_partition_schema, generic_mbr_partition_schema, repart, PartitionSchema,
};
use rugpi_common::disk::PartitionTable;
use rugpi_common::partitions::mkfs_ext4;
use rugpi_common::system::config::{load_system_config, PartitionConfig};
use rugpi_common::system::partitions::{resolve_config_partition, resolve_data_partition};
use rugpi_common::system::paths::{MOUNT_POINT_CONFIG, MOUNT_POINT_DATA, MOUNT_POINT_SYSTEM};
use rugpi_common::system::root::{find_system_device, SystemRoot};
use rugpi_common::system::slots::SlotKind;
use rugpi_common::system::{System, SystemError, SystemResult};
use xscript::{run, Run};

use crate::state::{load_state_config, Persist, STATE_CONFIG_DIR};
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

const DEFAULT_STATE_DIR: &str = "/run/rugpi/mounts/data/state/default";

fn init() -> SystemResult<()> {
    println!(include_str!("../assets/BANNER.txt"));

    rugpi_cli::CliBuilder::new().init();

    let config = load_config(CTRL_CONFIG_PATH)?;

    // Mount essential filesystems.
    mount_essential_filesystems()?;

    let system_config = load_system_config()?;
    let Some(system_device) = find_system_device() else {
        bail!("unable to determine system device")
    };
    let Some(root) = SystemRoot::from_system_device(&system_device) else {
        bail!("unable to determine system root");
    };

    let has_data_partition = if let Some(device) = &system_config.data_partition.device {
        if system_config.data_partition.partition.is_some() {
            eprintln!("ignoring `partition` because `device` is set");
        }
        Path::new(device).exists()
    } else {
        let partition = match system_config.config_partition.partition {
            Some(partition) => partition,
            None => {
                // The default depends on the partition table of the parent.
                let Some(table) = &root.table else {
                    bail!("unable to determine default data partition: no partition table");
                };
                if table.is_mbr() {
                    7
                } else {
                    6
                }
            }
        };
        root.resolve_partition(partition).is_some()
    };

    let bootstrap_hooks = HooksLoader::default()
        .load_hooks("bootstrap")
        .whatever("unable to load bootstrap hooks")?;

    if !has_data_partition {
        bootstrap_hooks
            .run_hooks("prepare")
            .whatever("unable to run `bootstrap/prepare` hooks")?;

        bootstrap_hooks
            .run_hooks("pre-layout")
            .whatever("unable to run `bootstrap/pre-layout` hooks")?;

        // If the data partitions already exists, we do not repartition the disk or
        // create any filesystems on it.
        let mut partition_schema = config.partition_schema.clone();
        if partition_schema.is_none() {
            let Some(table) = &root.table else {
                bail!("no root partition table");
            };
            if table.is_mbr() {
                partition_schema = Some(generic_mbr_partition_schema(config.system_size_bytes()?));
            } else {
                partition_schema = Some(generic_efi_partition_schema(config.system_size_bytes()?));
            }
        }
        if let Some(partition_schema) = partition_schema {
            bootstrap_partitions(
                root.device.as_ref(),
                &partition_schema,
                &root,
                &system_config.data_partition,
            )?;
        }
        bootstrap_hooks
            .run_hooks("post-layout")
            .whatever("unable to run `bootstrap/post-layout` hooks")?;
    }

    let Some(config_partition) =
        resolve_config_partition(Some(&root), &system_config.config_partition)
    else {
        bail!("Rugpi pre-init requires a config partition");
    };
    let Some(data_partition) = resolve_data_partition(Some(&root), &system_config.data_partition)
    else {
        bail!("Rugpi pre-init requires a data partition");
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

    // 4️⃣ Setup remaining mount points in `/run/rugpi/mounts`.
    fs::create_dir_all(MOUNT_POINT_SYSTEM).ok();
    run!([MOUNT, "-o", "ro", system_device.path(), MOUNT_POINT_SYSTEM])
        .whatever("unable to mount system partition")?;
    fs::create_dir_all(MOUNT_POINT_CONFIG).ok();
    run!([
        MOUNT,
        "-o",
        "ro",
        config_partition.path(),
        MOUNT_POINT_CONFIG
    ])
    .whatever("unable to mount config partition")?;

    let system = System::initialize()?;

    if let Err(error) = check_deferred_spare_reboot(&system) {
        println!("Warning: Error executing deferred reboot.");
        println!("{:?}", error);
    }

    // 6️⃣ Setup state in `/run/rugpi/state`.
    let state_profile = Path::new(DEFAULT_STATE_DIR);
    if state_profile.join(".rugpi/reset-state").exists() {
        // The existence of the file indicates that the state shall be reset.
        fs::remove_dir_all(state_profile).ok();
    }
    fs::create_dir_all(state_profile).ok();
    fs::create_dir_all(STATE_DIR).ok();
    run!([MOUNT, "--bind", &state_profile, STATE_DIR])
        .whatever("unable to bind mount state profile")?;

    // 7️⃣ Setup the root filesystem overlay.
    setup_root_overlay(&system, &config, state_profile)?;

    // 8️⃣ Setup the bind mounts for the persistent state.
    setup_persistent_state(state_profile)?;

    // 9️⃣ Restore the machine id and hand off to Systemd.
    restore_machine_id()?;
    exec_chroot_init()?;

    Ok(())
}

const STATE_DIR: &str = "/run/rugpi/state";

pub fn state_dir() -> &'static Path {
    Path::new(STATE_DIR)
}

const OVERLAY_DIR: &str = "/run/rugpi/mounts/data/overlay";
const OVERLAY_ROOT_DIR: &str = "/run/rugpi/mounts/data/overlay/root";
const OVERLAY_WORK_DIR: &str = "/run/rugpi/mounts/data/overlay/work";

pub fn overlay_dir() -> &'static Path {
    Path::new(OVERLAY_DIR)
}

pub fn overlay_root_dir() -> &'static Path {
    Path::new(OVERLAY_ROOT_DIR)
}

pub fn overlay_work_dir() -> &'static Path {
    Path::new(OVERLAY_WORK_DIR)
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
    dev: &Path,
    schema: &PartitionSchema,
    root: &SystemRoot,
    data_partition_config: &PartitionConfig,
) -> SystemResult<()> {
    let old_table = PartitionTable::read(dev).whatever("unable to read partition table")?;
    if let Some(new_table) =
        repart(&old_table, schema).whatever("unable to compute new partition table")?
    {
        // Write new partition table to disk.
        new_table
            .write(dev)
            .whatever("unable to write new partition table")?;
        run!([SYNC]).whatever("unable to synchronize file systems")?;
        // Inform the kernel about new partitions.
        update_kernel_partitions(dev, &old_table, &new_table)
            .whatever("unable to update partitions in the kernel")?;
        if let Some(data_partition) = resolve_data_partition(Some(root), data_partition_config) {
            mkfs_ext4(data_partition, "data").whatever("unable to create EXT4 filesystem")?;
        }
        // We do not need to patch the partition ID in the configuration files as we
        // keep the id from the original image.
    }
    Ok(())
}

/// Sets up the overlay.
fn setup_root_overlay(system: &System, config: &Config, state_profile: &Path) -> SystemResult<()> {
    let overlay_state = state_profile.join("overlay");
    let force_persist = state_profile.join(".rugpi/force-persist-overlay").exists();
    if !force_persist && !matches!(config.overlay, Overlay::Persist) {
        fs::remove_dir_all(&overlay_state).ok();
    }

    let active_boot_entry = &system.boot_entries()[system.active_boot_entry().unwrap()];
    let hot_overlay_state = overlay_state.join(active_boot_entry.name());
    fs::create_dir_all(&hot_overlay_state).ok();

    // Reinitialize `work` and `root` directories.
    fs::remove_dir_all(overlay_dir()).ok();
    fs::create_dir_all(overlay_work_dir()).ok();
    fs::create_dir_all(overlay_root_dir()).ok();

    let hot_overlay_state = hot_overlay_state.to_string_lossy();
    run!([
        MOUNT,
        "-t",
        "overlay",
        "overlay",
        "-o",
        "noatime,lowerdir={MOUNT_POINT_SYSTEM},upperdir={hot_overlay_state},workdir={OVERLAY_WORK_DIR}",
        OVERLAY_ROOT_DIR
    ]).whatever("unable to setup system overlay mounts")?;
    run!([MOUNT, "--rbind", "/run", overlay_root_dir().join("run")])
        .whatever("unable to rbind /run")?;
    if let Some(boot_slot) = active_boot_entry.get_slot("boot") {
        let SlotKind::Block(boot_slot) = system.slots()[boot_slot].kind();
        run!([
            MOUNT,
            "-o",
            "ro",
            boot_slot.device().path(),
            overlay_root_dir().join("boot")
        ])
        .whatever("unable to mount boot partition")?;
    }
    Ok(())
}

/// Sets up the bind mounts required for the persistent state.
fn setup_persistent_state(state_profile: &Path) -> SystemResult<()> {
    let root_dir = overlay_root_dir();

    let persist_dir = state_profile.join("persist");
    fs::create_dir_all(state_profile).ok();

    let state_config = load_state_config(STATE_CONFIG_DIR);

    for persist in &state_config.persist {
        match persist {
            Persist::Directory { directory } => {
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
            Persist::File { file, default } => {
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
fn restore_machine_id() -> SystemResult<()> {
    let state_machine_id = state_dir().join("machine-id");
    let system_machine_id = overlay_root_dir().join("etc/machine-id");
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
fn exec_chroot_init() -> SystemResult<()> {
    println!("Changing current working directory to overlay root directory.");
    nix::unistd::chdir(OVERLAY_ROOT_DIR).whatever("unable to switch to overlay directory")?;
    println!("Pivoting root mount point to current working directory.");
    nix::unistd::pivot_root(".", ".").whatever("unable to pivot root directory")?;
    println!("Unmounting the previous root filesystem.");
    nix::mount::umount2(".", MntFlags::MNT_DETACH)
        .whatever("unable to unmount old root directory")?;
    println!("Starting system init process.");
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
                    .whatever("unable to set next boot entry")?;
                reboot()?;
            }
        }
    }
    Ok(())
}
