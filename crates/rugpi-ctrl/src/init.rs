use std::{ffi::CString, fs, io, path::Path, process, sync::OnceLock, thread, time::Duration};

use anyhow::{anyhow, bail, ensure};
use camino::Utf8Path;
use indoc::formatdoc;
use rugpi_common::patch_cmdline;
use xscript::{read_str, run, Out, Run};

use crate::{
    config::Config,
    partitions::{
        is_block_dev, SD_CARD, SD_PART_BOOT_A, SD_PART_BOOT_B, SD_PART_DATA, SD_PART_SYSTEM_A,
        SD_PART_SYSTEM_B,
    },
    state::{load_state_config, Persist},
};

pub fn main() -> anyhow::Result<()> {
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

/// The `chroot` executable.
const CHROOT: &str = "/usr/sbin/chroot";
/// The `cp` executable.
const CP: &str = "/usr/bin/cp";
/// The `fdisk` executable.
const FDISK: &str = "/usr/sbin/fdisk";
/// The `findmnt` executable.
const FINDMNT: &str = "/usr/bin/findmnt";
/// The `fsck` executable.
const FSCK: &str = "/usr/sbin/fsck";
/// The `mkfs.ext4` executable.
const MKFS_ETX4: &str = "/usr/sbin/mkfs.ext4";
/// The `mount` executable.
const MOUNT: &str = "/usr/bin/mount";
/// The `partprobe` executable.
const PARTPROBE: &str = "/usr/sbin/partprobe";
/// The `reboot` executable.
const REBOOT: &str = "/usr/sbin/reboot";
/// The `sfdisk` executable.
const SFDISK: &str = "/usr/sbin/sfdisk";
/// The `sync` executable.
const SYNC: &str = "/usr/bin/sync";
/// The `systemd-machine-id-setup` executable.
const SYSTEMD_MACHINE_ID_SETUP: &str = "/usr/bin/systemd-machine-id-setup";

const DEFAULT_STATE_DIR: &str = "/run/rugpi/data/state/default";

pub fn find_dev(path: impl AsRef<str>) -> anyhow::Result<String> {
    Ok(read_str!([
        FINDMNT, "-n", "-o", "SOURCE", "--target", path
    ])?)
}

pub fn system_dev() -> anyhow::Result<&'static Utf8Path> {
    static SYSTEM_DEV: OnceLock<anyhow::Result<String>> = OnceLock::new();
    SYSTEM_DEV
        .get_or_init(|| find_dev("/run/rugpi/system"))
        .as_ref()
        .map(|device| Utf8Path::new(device))
        .map_err(|error| anyhow!("error retrieving system device: {error}"))
}

pub fn hot_partition_set() -> anyhow::Result<PartitionSet> {
    let system_dev = system_dev()?.as_str();
    match system_dev {
        SD_PART_SYSTEM_A => Ok(PartitionSet::A),
        SD_PART_SYSTEM_B => Ok(PartitionSet::B),
        _ => bail!("unable to determine hot partition set, invalid device {system_dev}"),
    }
}

pub fn spare_partition_set() -> anyhow::Result<PartitionSet> {
    Ok(hot_partition_set()?.flipped())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartitionSet {
    A,
    B,
}

impl PartitionSet {
    pub fn as_str(self) -> &'static str {
        match self {
            PartitionSet::A => "a",
            PartitionSet::B => "b",
        }
    }

    pub fn system_dev(self) -> &'static Utf8Path {
        match self {
            PartitionSet::A => Utf8Path::new(SD_PART_SYSTEM_A),
            PartitionSet::B => Utf8Path::new(SD_PART_SYSTEM_B),
        }
    }

    pub fn boot_dev(self) -> &'static Utf8Path {
        match self {
            PartitionSet::A => Utf8Path::new(SD_PART_BOOT_A),
            PartitionSet::B => Utf8Path::new(SD_PART_BOOT_B),
        }
    }

    pub fn flipped(self) -> Self {
        match self {
            PartitionSet::A => Self::B,
            PartitionSet::B => Self::A,
        }
    }
}

pub fn overlay_dir() -> &'static Utf8Path {
    Utf8Path::new("/run/rugpi/data/overlay")
}

pub fn overlay_root_dir() -> &'static Utf8Path {
    Utf8Path::new("/run/rugpi/data/overlay/root")
}

pub fn overlay_work_dir() -> &'static Utf8Path {
    Utf8Path::new("/run/rugpi/data/overlay/work")
}

pub fn create_parent_dir(path: impl AsRef<Path>) -> io::Result<()> {
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

fn init() -> anyhow::Result<()> {
    println!(include_str!("../assets/BANNER.txt"));
    let config: Config = fs::read_to_string("/etc/rugpi/ctrl.toml")
        .map_err(|error| anyhow::Error::from(error))
        .and_then(|config_str| Ok(toml::from_str(&config_str)?))
        .unwrap_or_default();
    // 1️⃣ Mount essential filesystems.
    run!([MOUNT, "-t", "proc", "proc", "/proc"])?;
    run!([MOUNT, "-t", "sysfs", "sys", "/sys"])?;
    run!([MOUNT, "-t", "tmpfs", "tmp", "/run"])?;
    // 2️⃣ Expand partition table, if data partition does not exist.
    if !is_block_dev(SD_PART_DATA) {
        expand_partition_table(&config)?;
    }
    // Bind system partition to `/run/rugpi/system`.
    fs::create_dir_all("/run/rugpi/system").ok();
    run!([MOUNT, "--bind", "/", "/run/rugpi/system"])?;

    let hot_partition_set = hot_partition_set()?;

    // 3️⃣ Check and mount data partition.
    run!([FSCK, "-y", SD_PART_DATA])?;
    fs::create_dir_all("/run/rugpi/data").ok();
    run!([MOUNT, "-o", "noatime", SD_PART_DATA, "/run/rugpi/data"])?;

    let state_dir = Utf8Path::new(DEFAULT_STATE_DIR);
    fs::create_dir_all(state_dir).ok();
    fs::create_dir_all("/run/rugpi/state").ok();
    run!([MOUNT, "--bind", &state_dir, "/run/rugpi/state"])?;

    let overlay_state = state_dir.join("overlay").join(hot_partition_set.as_str());
    fs::remove_dir_all(state_dir.join("overlay")).ok();
    fs::create_dir_all(&overlay_state).ok();

    // 4️⃣ Cleanup temporary data (ignoring any errors).
    fs::remove_dir_all(overlay_dir()).ok();
    fs::create_dir_all(overlay_work_dir()).ok();
    fs::create_dir_all(overlay_root_dir()).ok();

    run!([
        MOUNT,
        "-t",
        "overlay",
        "overlay",
        "-o",
        "noatime,lowerdir=/,upperdir={overlay_state},workdir=/run/rugpi/data/overlay/work",
        "/run/rugpi/data/overlay/root"
    ])?;

    run!([MOUNT, "--rbind", "/run", "/run/rugpi/data/overlay/root/run"])?;

    run!([
        MOUNT,
        "-o",
        "ro",
        hot_partition_set.boot_dev(),
        overlay_root_dir().join("boot")
    ])
    .ok();

    let new_root = overlay_root_dir();

    let state_persist_dir = state_dir.join("persist");
    fs::create_dir_all(&state_dir).ok();

    let state_config = load_state_config();
    for persist in &state_config.persist {
        match persist {
            Persist::Directory { directory } => {
                let mut path = Utf8Path::new(directory);
                if let Ok(stripped) = path.strip_prefix("/") {
                    path = stripped;
                }
                eprintln!("Setting up bind mounds for directory `{path}`...");
                let system_path = new_root.join(path);
                let state_path = state_persist_dir.join(path);
                if system_path.exists() && !system_path.is_dir() {
                    eprintln!("Error persisting `{directory}`, not a directory!");
                    continue;
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
                let mut path = Utf8Path::new(file);
                if let Ok(stripped) = path.strip_prefix("/") {
                    path = stripped;
                }
                eprintln!("Setting up bind mounds for file `{path}`...");
                let system_path = new_root.join(path);
                let state_path = state_persist_dir.join(path);
                if system_path.exists() && !system_path.is_file() {
                    eprintln!("Error persisting `{file}`, not a file!");
                    continue;
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
                if let Err(error) = run!([MOUNT, "--bind", &state_path, &system_path]) {
                    eprintln!("{state_path}");
                    eprintln!("{system_path}");
                    eprintln!("{error}");
                    thread::sleep(Duration::from_secs(60));
                }
            }
        }
    }

    let state_machine_id = state_dir.join("machine-id");
    let system_machine_id = new_root.join("etc/machine-id");
    if !state_machine_id.exists() {
        // Ensure that the `machine-id` is valid.
        run!([SYSTEMD_MACHINE_ID_SETUP, "--root", new_root])?;
        fs::copy(system_machine_id, state_machine_id)?;
    } else {
        fs::copy(state_machine_id, new_root.join("etc/machine-id"))?;
    }

    // 5️⃣ Create state directory, if it does not exist.
    // if !is_dir(DATA_STATE_DIR) {
    //     fs::create_dir_all(DATA_STATE_DIR)?;
    // }
    nix::unistd::execv(
        &CString::new(CHROOT).unwrap(),
        &[
            CString::new("chroot").unwrap(),
            CString::new("/run/rugpi/data/overlay/root").unwrap(),
            CString::new("/sbin/init").unwrap(),
        ],
    )?;
    Ok(())
}

fn expand_partition_table(config: &Config) -> anyhow::Result<()> {
    let system_size = config.system_size();
    eprintln!("Creating system partitions... DO NOT TURN OFF!");
    run!([MOUNT, "-o", "remount,rw", "/"])?;
    run!([MOUNT, SD_PART_BOOT_A, "/boot"])?;
    run!([SFDISK, "--no-reread", SD_CARD].with_stdin(formatdoc! {"
        label: dos
        unit: sectors
        grain: 4M
        
        config   : type=0c, size=256M
        boot-a   : type=0c, size=128M
        boot-b   : type=0c, size=128M
        
        extended : type=05
        
        system-a : type=83, size={system_size}
        system-b : type=83, size={system_size}
        data     : type=83
    "}))?;
    run!([FDISK, "-l", SD_CARD].with_stdout(Out::Inherit))?;
    run!([PARTPROBE])?;
    // run!([MKFS_BRTFS, "-F", "-L", "system-b", SD_PART_SYSTEM_B])?;
    run!([MKFS_ETX4, "-F", "-L", "data", SD_PART_DATA])?;
    let disk_id = xscript::read_str!([SFDISK, "--disk-id", SD_CARD])?
        .strip_prefix("0x")
        .ok_or_else(|| anyhow!("`sfdisk` returned invalid disk id"))?
        .to_owned();
    // patch_fstab(Utf8Path::new("/etc/fstab"), &disk_id)?;
    patch_cmdline("/boot/cmdline.txt", format!("PARTUUID={disk_id}-05"))?;
    // patch_cmdline(Utf8Path::new("/boot/cmdline.txt"), &disk_id)?;
    reboot()?;
    Ok(())
}

pub fn reboot() -> anyhow::Result<()> {
    run!([REBOOT, "-f"])?;
    eprint!("Rebooting in 5 seconds...");
    run!([SYNC])?;
    thread::sleep(Duration::from_secs(5));
    process::exit(0)
}

// pub fn patch_cmdline(path: &Utf8Path, disk_id: &str) -> anyhow::Result<()> {
//     let cmdline = fs::read_to_string(path)?;
//     let mut parts = cmdline
//         .split_ascii_whitespace()
//         .filter(|part| !part.starts_with("root=") && !part.starts_with("init=") &&
// *part != "quiet")         .map(str::to_owned)
//         .collect::<Vec<_>>();
//     parts.push(format!("root=PARTUUID={disk_id}-05"));
//     parts.push("init=/usr/bin/rugpi-ctrl".to_owned());
//     fs::write(path, parts.join(" "))?;
//     Ok(())
// }

// pub fn patch_fstab(path: &Utf8Path, disk_id: &str) -> anyhow::Result<()> {
//     let fstab = fs::read_to_string(path)?;
//     let lines = fstab
//         .lines()
//         .map(|line| {
//             if line.starts_with("PARTUUID=") {
//                 let (_, tail) = line.split_once("-").unwrap();
//                 format!("PARTUUID={disk_id}-{tail}")
//             } else {
//                 line.to_owned()
//             }
//         })
//         .collect::<Vec<_>>();
//     fs::write(path, lines.join("\n"))?;
//     Ok(())
// }

/// Indicates whether the process is the init process.
pub fn is_init_process() -> bool {
    std::process::id() == 1
}
