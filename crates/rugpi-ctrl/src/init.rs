use std::{ffi::CString, fs, io, path::Path, thread, time::Duration};

use anyhow::{bail, ensure};
use rugpi_common::{
    partitions::{
        devices::{SD_CARD, SD_PART_BOOT_A, SD_PART_CONFIG, SD_PART_DATA},
        get_disk_id, get_hot_partitions, is_block_dev, mkfs_ext4, sfdisk_apply_layout,
        sfdisk_system_layout,
    },
    patch_boot, Anyhow,
};
use xscript::{run, Run};

use crate::{
    config::{Config, Overlay},
    state::{load_state_config, Persist, STATE_CONFIG_DIR},
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

/// Indicates whether the process is the init process.
pub fn is_init_process() -> bool {
    std::process::id() == 1
}

/// The `cp` executable.
const CP: &str = "/usr/bin/cp";
/// The `fsck` executable.
const FSCK: &str = "/usr/sbin/fsck";
/// The `mount` executable.
const MOUNT: &str = "/usr/bin/mount";
/// The `umount` executable.
const UMOUNT: &str = "/usr/bin/umount";
/// The `sync` executable.
const SYNC: &str = "/usr/bin/sync";
/// The `systemd-machine-id-setup` executable.
const SYSTEMD_MACHINE_ID_SETUP: &str = "/usr/bin/systemd-machine-id-setup";

const DEFAULT_STATE_DIR: &str = "/run/rugpi/mounts/data/state/default";

fn init() -> Anyhow<()> {
    println!(include_str!("../assets/BANNER.txt"));
    let config = load_config()?;

    // 1️⃣ Mount essential filesystems.
    mount_essential_filesystems()?;

    // 2️⃣ Initialize partitions during the first boot.
    if !is_block_dev(SD_PART_DATA) {
        initialize_partitions(&config)?;
    }

    // 3️⃣ Check and mount the data partition.
    run!([FSCK, "-y", SD_PART_DATA])?;
    fs::create_dir_all(MOUNT_POINT_DATA).ok();
    run!([MOUNT, "-o", "noatime", SD_PART_DATA, MOUNT_POINT_DATA])?;

    // 4️⃣ Setup remaining mount points in `/run/rugpi/mounts`.
    fs::create_dir_all(MOUNT_POINT_SYSTEM).ok();
    run!([MOUNT, "--bind", "/", MOUNT_POINT_SYSTEM])?;
    fs::create_dir_all(MOUNT_POINT_CONFIG).ok();
    run!([MOUNT, "-o", "ro", SD_PART_CONFIG, MOUNT_POINT_CONFIG])?;

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
    setup_root_overlay(&config, state_profile)?;

    // 8️⃣ Setup the bind mounts for the persistent state.
    setup_persistent_state(state_profile)?;

    // 9️⃣ Restore the machine id and hand off to Systemd.
    restore_machine_id()?;
    exec_chroot_init()?;

    Ok(())
}

const MOUNT_POINT_SYSTEM: &str = "/run/rugpi/mounts/system";
const MOUNT_POINT_DATA: &str = "/run/rugpi/mounts/data";
const MOUNT_POINT_CONFIG: &str = "/run/rugpi/mounts/config";

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

const CTRL_CONFIG_PATH: &str = "/etc/rugpi/ctrl.toml";

pub fn config_path() -> &'static Path {
    Path::new(CTRL_CONFIG_PATH)
}

/// Loads the Rugpi Ctrl configuration.
fn load_config() -> Anyhow<Config> {
    if config_path().exists() {
        Ok(toml::from_str(&fs::read_to_string(config_path())?)?)
    } else {
        Ok(Config::default())
    }
}

/// Mounts the essential filesystems `/proc`, `/sys`, and `/run`.
fn mount_essential_filesystems() -> Anyhow<()> {
    run!([MOUNT, "-t", "proc", "proc", "/proc"])?;
    run!([MOUNT, "-t", "sysfs", "sys", "/sys"])?;
    run!([MOUNT, "-t", "tmpfs", "tmp", "/run"])?;
    Ok(())
}

/// Initializes the partitions and expands the partition table during the first boot.
fn initialize_partitions(config: &Config) -> Anyhow<()> {
    eprintln!("Creating system partitions... DO NOT TURN OFF!");
    let system_size = config.system_size();

    // 1️⃣ Apply the system layout to the SD card and reread the partition table.
    sfdisk_apply_layout(SD_CARD, sfdisk_system_layout(system_size))?;

    // 2️⃣ Patch the `cmdline.txt` with the new disk id.
    let disk_id = get_disk_id(SD_CARD)?;
    run!([MOUNT, SD_PART_BOOT_A, "/boot"])?;
    patch_boot("/boot", format!("PARTUUID={disk_id}-05"))?;
    run!([UMOUNT, "/boot"])?;

    // 3️⃣ Create a file system on the data partition.
    mkfs_ext4(SD_PART_DATA, "data")?;

    // 4️⃣ Make sure everything is written to disk.
    run!([SYNC])?;

    Ok(())
}

/// Sets up the overlay.
fn setup_root_overlay(config: &Config, state_profile: &Path) -> Anyhow<()> {
    let overlay_state = state_profile.join("overlay");
    let force_persist = state_profile.join(".rugpi/force-persist-overlay").exists();
    if !force_persist && !matches!(config.overlay, Overlay::Persist) {
        fs::remove_dir_all(&overlay_state).ok();
    }

    let hot_partitions = get_hot_partitions()?;
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
        "noatime,lowerdir=/,upperdir={hot_overlay_state},workdir={OVERLAY_WORK_DIR}",
        OVERLAY_ROOT_DIR
    ])?;
    run!([MOUNT, "--rbind", "/run", overlay_root_dir().join("run")])?;
    run!([
        MOUNT,
        "-o",
        "ro",
        hot_partitions.boot_dev(),
        overlay_root_dir().join("boot")
    ])?;
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

/// The `chroot` executable.
const CHROOT: &str = "/usr/sbin/chroot";

/// Changes the root directory and hands off to Systemd.
fn exec_chroot_init() -> Anyhow<()> {
    let chroot_prog = &CString::new(CHROOT).unwrap();
    let new_root = &CString::new(OVERLAY_ROOT_DIR).unwrap();
    let systemd_init = &CString::new("/sbin/init").unwrap();
    nix::unistd::execv(chroot_prog, &[chroot_prog, new_root, systemd_init])?;
    Ok(())
}
