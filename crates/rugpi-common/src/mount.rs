use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use reportify::{Report, ResultExt};
use tracing::info;
use xscript::{run, Run};

reportify::new_whatever_type! {
    MountError
}

pub struct Mounted {
    path: PathBuf,
}

impl Mounted {
    pub fn mount(dev: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<Self, Report<MountError>> {
        let dst = dst.as_ref();
        let dev = dev.as_ref();
        info!("Mounting {dev:?} to {dst:?}.");
        // FIXME: The `mount` command works without specifying the filesystem type,
        // which is not the case for `nix::mount::mount`.
        run!(["/usr/bin/mount", dev, dst])
            .whatever("unable to mount filesystem")
            .with_info(|_| format!("dev: {dev:?}"))
            .with_info(|_| format!("dst: {dst:?}"))?;
        Ok(Mounted { path: dst.into() })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn mount_fs(
        fstype: impl AsRef<str>,
        src: impl AsRef<Path>,
        dst: impl AsRef<Path>,
    ) -> Result<Self, Report<MountError>> {
        let dst = dst.as_ref();
        let src = src.as_ref();
        let fstype = fstype.as_ref();
        info!("Mounting {src:?} with {fstype:?} to {dst:?}.");
        nix::mount::mount(
            Some(src),
            dst,
            Some(fstype),
            nix::mount::MsFlags::empty(),
            None as Option<&OsStr>,
        )
        .whatever("unable to mount filesystem")
        .with_info(|_| format!("src: {src:?}"))
        .with_info(|_| format!("dst: {dst:?}"))
        .with_info(|_| format!("fstype: {fstype}"))?;
        Ok(Mounted { path: dst.into() })
    }

    pub fn bind(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Result<Self, Report<MountError>> {
        let dst = dst.as_ref();
        let src = src.as_ref();
        info!("Mounting {src:?} to {dst:?}.");
        nix::mount::mount(
            Some(src),
            dst,
            None as Option<&OsStr>,
            nix::mount::MsFlags::MS_BIND,
            None as Option<&OsStr>,
        )
        .whatever("unable to bind mount")
        .with_info(|_| format!("src: {src:?}"))
        .with_info(|_| format!("dst: {dst:?}"))?;
        Ok(Mounted { path: dst.into() })
    }
}

impl Drop for Mounted {
    fn drop(&mut self) {
        if let Err(error) = nix::mount::umount(&self.path) {
            eprintln!("Error unmounting {:?}: {:?}", self.path, error)
        }
    }
}

pub struct MountStack(Vec<Mounted>);

impl MountStack {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn push(&mut self, mounted: Mounted) {
        self.0.push(mounted);
    }

    pub fn unmount_all(&mut self) {
        while let Some(top) = self.0.pop() {
            drop(top);
        }
    }
}

impl Drop for MountStack {
    fn drop(&mut self) {
        self.unmount_all()
    }
}
