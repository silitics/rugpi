use std::{ffi::CString, fs, io, path::Path, thread, time::Duration};

use anyhow::{bail, ensure};
use nix::mount::MntFlags;
use rugpi_common::{
    ctrl_config::{load_config, Config, Overlay, CTRL_CONFIG_PATH},
    disk::{
        blkpg::update_kernel_partitions,
        repart::{repart, PartitionSchema},
        PartitionTable,
    },
    partitions::{get_hot_partitions, mkfs_ext4, read_default_partitions, system_dev, Partitions},
    paths::{MOUNT_POINT_CONFIG, MOUNT_POINT_DATA, MOUNT_POINT_SYSTEM},
    Anyhow,
};
use xscript::{run, Run};

use crate::{
    state::{load_state_config, Persist, STATE_CONFIG_DIR},
    utils::{clear_flag, is_flag_set, is_init_process, reboot, DEFERRED_SPARE_REBOOT_FLAG},
};

pub fn main() -> Anyhow<()> {
    ensure!(is_init_process(), "process must be the init process");
    let result = init();
    match &result {
        Ok(_) => {
            eprintln!("initialization procedure terminated unexpectedly");
        }
        Err(error) => {
            eprintln!("error during initialization");
            eprintln!("{error}");
        }
    }
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
/// The `systemd-machine-id-setup` executable.
const SYSTEMD_MACHINE_ID_SETUP: &str = "/usr/bin/systemd-machine-id-setup";

const DEFAULT_STATE_DIR: &str = "/run/rugpi/mounts/data/state/default";

fn init() -> Anyhow<()> {
    println!(include_str!("../assets/BANNER.txt"));
    let config = load_config(CTRL_CONFIG_PATH)?;

    // Mount essential filesystems.
    mount_essential_filesystems()?;

    let partitions = Partitions::load(&config)?;

    // Ensure that the disks's partitions match the defined partition schema.
    let partition_schema = config
        .partition_schema
        .as_ref()
        .or(partitions.schema.as_ref());
    if let Some(partition_schema) = partition_schema {
        // If an update ships a schema that is incompatible with the existing schema,
        // then it is fine to reboot here and switch to the old version.
        repartition_disk(&config, &partitions.parent_dev, partition_schema)?;
    }

    let partitions = Partitions::load(&config)?;

    // 3️⃣ Check and mount the data partition.
    run!([FSCK, "-y", &partitions.data])?;
    fs::create_dir_all(MOUNT_POINT_DATA).ok();
    run!([MOUNT, "-o", "noatime", &partitions.data, MOUNT_POINT_DATA])?;

    // 4️⃣ Setup remaining mount points in `/run/rugpi/mounts`.
    let system_dev = system_dev()?;
    fs::create_dir_all(MOUNT_POINT_SYSTEM).ok();
    run!([MOUNT, "-o", "ro", system_dev, MOUNT_POINT_SYSTEM])?;
    fs::create_dir_all(MOUNT_POINT_CONFIG).ok();
    run!([MOUNT, "-o", "ro", &partitions.config, MOUNT_POINT_CONFIG])?;

    if let Err(error) = check_deferred_spare_reboot(&partitions) {
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
    run!([MOUNT, "--bind", &state_profile, STATE_DIR])?;

    // 7️⃣ Setup the root filesystem overlay.
    setup_root_overlay(&partitions, &config, state_profile)?;

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
fn mount_essential_filesystems() -> Anyhow<()> {
    // We ignore any errors. Errors likely mean that the filesystems have already been
    // mounted.
    run!([MOUNT, "-t", "proc", "proc", "/proc"]).ok();
    run!([MOUNT, "-t", "sysfs", "sys", "/sys"]).ok();
    run!([MOUNT, "-t", "tmpfs", "tmp", "/run"]).ok();
    Ok(())
}

/// Initializes the partitions and expands the partition table during the first boot.
fn repartition_disk(config: &Config, dev: &Path, schema: &PartitionSchema) -> Anyhow<()> {
    let old_table = PartitionTable::read(dev)?;
    if let Some(new_table) = repart(&old_table, schema)? {
        // Write new partition table to disk.
        new_table.write(dev)?;
        run!([SYNC])?;
        // Inform the kernel about new partitions.
        update_kernel_partitions(dev, &old_table, &new_table)?;
        let partitions = Partitions::load(config)?;
        mkfs_ext4(&partitions.data, "data")?;
        // We do not need to patch the partition ID in the configuration files as we
        // keep the id from the original image.
    }
    Ok(())
}

/// Sets up the overlay.
fn setup_root_overlay(
    partitions: &Partitions,
    config: &Config,
    state_profile: &Path,
) -> Anyhow<()> {
    let overlay_state = state_profile.join("overlay");
    let force_persist = state_profile.join(".rugpi/force-persist-overlay").exists();
    if !force_persist && !matches!(config.overlay, Overlay::Persist) {
        fs::remove_dir_all(&overlay_state).ok();
    }

    let hot_partitions = get_hot_partitions(partitions)?;
    let hot_overlay_state = overlay_state.join(hot_partitions.as_str());
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
    ])?;
    run!([MOUNT, "--rbind", "/run", overlay_root_dir().join("run")])?;
    if let Some(boot_dev) = hot_partitions.boot_dev(partitions) {
        run!([MOUNT, "-o", "ro", boot_dev, overlay_root_dir().join("boot")])?;
    }
    Ok(())
}

/// Sets up the bind mounts required for the persistent state.
fn setup_persistent_state(state_profile: &Path) -> Anyhow<()> {
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
                        run!([CP, "-a", &system_path, &state_path])?;
                    } else {
                        fs::create_dir_all(&state_path).ok();
                    }
                }
                if !system_path.is_dir() {
                    fs::create_dir_all(&system_path)?;
                }
                run!([MOUNT, "--bind", &state_path, &system_path])?;
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
                    create_parent_dir(&state_path)?;
                    if system_path.is_file() {
                        run!([CP, "-a", &system_path, &state_path])?;
                    } else {
                        fs::write(&state_path, default.as_deref().unwrap_or_default())?;
                    }
                }
                if !system_path.is_file() {
                    create_parent_dir(&system_path)?;
                    fs::write(&system_path, "")?;
                }
                run!([MOUNT, "--bind", &state_path, &system_path])?;
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
fn restore_machine_id() -> Anyhow<()> {
    let state_machine_id = state_dir().join("machine-id");
    let system_machine_id = overlay_root_dir().join("etc/machine-id");
    if !state_machine_id.exists() {
        // Ensure that the `machine-id` is valid.
        run!([SYSTEMD_MACHINE_ID_SETUP, "--root", OVERLAY_ROOT_DIR])?;
        fs::copy(system_machine_id, state_machine_id)?;
    } else {
        fs::copy(state_machine_id, system_machine_id)?;
    }
    Ok(())
}

/// Changes the root directory and hands off to the system init process.
///
/// We follow the example from the manpage of the `pivot_root` system call here.
///
/// We are not using `chroot` as this lead to problems with Docker.
fn exec_chroot_init() -> Anyhow<()> {
    println!("Changing current working directory to overlay root directory.");
    nix::unistd::chdir(OVERLAY_ROOT_DIR)?;
    println!("Pivoting root mount point to current working directory.");
    nix::unistd::pivot_root(".", ".")?;
    println!("Unmounting the previous root filesystem.");
    nix::mount::umount2(".", MntFlags::MNT_DETACH)?;
    println!("Starting system init process.");
    let systemd_init = &CString::new("/sbin/init").unwrap();
    nix::unistd::execv(systemd_init, &[systemd_init])?;
    Ok(())
}

/// Reboot the system to the spare partitions if the deferred spare reboot flag is set.
fn check_deferred_spare_reboot(partitions: &Partitions) -> Anyhow<()> {
    if is_flag_set(DEFERRED_SPARE_REBOOT_FLAG) {
        println!("Executing deferred reboot to spare partitions.");
        // Remove file and make sure that changes are synced to disk.
        clear_flag(DEFERRED_SPARE_REBOOT_FLAG)?;
        nix::unistd::sync();
        let default_partitions = read_default_partitions()?;
        let hot_partitions = get_hot_partitions(partitions)?;
        if default_partitions == hot_partitions {
            // Reboot to the spare partitions.
            reboot(true)?;
        }
    }
    Ok(())
}
