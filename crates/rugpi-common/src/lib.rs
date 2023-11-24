//! Common functionality shared between Rugpi Bakery and Rugpi Ctrl.

use std::{fs, io, path::Path};

use crate::boot::uboot::UBootEnv;

pub mod autoboot;
pub mod boot;
pub mod loop_dev;
pub mod mount;
pub mod partitions;

/// The [`anyhow`] result type.
pub type Anyhow<T> = anyhow::Result<T>;

/// Patches `cmdline.txt` to use the given root device and `rugpi-ctrl` as init process.
pub fn patch_boot(path: impl AsRef<Path>, root: impl AsRef<str>) -> Anyhow<()> {
    fn _patch_cmdline(path: &Path, root: &str) -> Anyhow<()> {
        let cmdline_path = path.join("cmdline.txt");
        let cmdline = fs::read_to_string(&cmdline_path)?;
        let mut parts = cmdline
            .split_ascii_whitespace()
            .filter(|part| {
                !part.starts_with("root=")
                    && !part.starts_with("init=")
                    && !part.starts_with("panic")
                    && *part != "quiet"
            })
            .map(str::to_owned)
            .collect::<Vec<_>>();
        parts.push("panic=60".to_owned());
        parts.push(format!("root={root}"));
        parts.push("init=/usr/bin/rugpi-ctrl".to_owned());
        let cmdline_value = parts.join(" ");
        fs::write(&cmdline_path, &cmdline_value)?;
        let boot_env_path = path.join("boot.env");
        let mut env = if boot_env_path.exists() {
            UBootEnv::load(&boot_env_path)?
        } else {
            UBootEnv::new()
        };
        env.set("bootargs", &cmdline_value);
        env.save(boot_env_path)?;
        Ok(())
    }
    _patch_cmdline(path.as_ref(), root.as_ref())
}

/// Patches `config.txt` to not use `initramfs`.
pub fn patch_config(path: impl AsRef<Path>) -> io::Result<()> {
    fn _patch_config(path: &Path) -> io::Result<()> {
        let config = fs::read_to_string(path)?;
        let lines = config
            .lines()
            .filter(|line| !line.trim_start().starts_with("auto_initramfs"))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        fs::write(path, lines.join("\n"))?;
        Ok(())
    }
    _patch_config(path.as_ref())
}

/// Runs a closure on drop.
pub struct DropGuard<F: FnOnce()>(Option<F>);

impl<F: FnOnce()> DropGuard<F> {
    /// Construct a new [`DropGuard`] with the given closure.
    pub fn new(closure: F) -> Self {
        Self(Some(closure))
    }

    /// Do not run the closure on drop.
    pub fn disable(&mut self) {
        self.0.take();
    }
}

impl<F: FnOnce()> Drop for DropGuard<F> {
    fn drop(&mut self) {
        if let Some(closure) = self.0.take() {
            closure()
        }
    }
}
