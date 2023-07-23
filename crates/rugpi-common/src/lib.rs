//! Common functionality shared between Rugpi Bakery and Rugpi Ctrl.

use std::{fs, io, path::Path};

pub mod autoboot;
pub mod loop_dev;
pub mod mount;
pub mod partitions;

/// The [`anyhow`] result type.
pub type Anyhow<T> = anyhow::Result<T>;

/// Patches `cmdline.txt` to use the given root device and `rugpi-ctrl` as init process.
pub fn patch_cmdline(path: impl AsRef<Path>, root: impl AsRef<str>) -> io::Result<()> {
    fn _patch_cmdline(path: &Path, root: &str) -> io::Result<()> {
        let cmdline = fs::read_to_string(path)?;
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
        fs::write(path, parts.join(" "))?;
        Ok(())
    }
    _patch_cmdline(path.as_ref(), root.as_ref())
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
