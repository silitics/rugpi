use std::path::{Path, PathBuf};

use xscript::{run, Run};

use crate::Anyhow;

/// The `mount` executable.
const MOUNT: &str = "/usr/bin/mount";
/// The `umount` executable.
const UMOUNT: &str = "/usr/bin/umount";

pub struct Mounted {
    path: PathBuf,
}

impl Mounted {
    pub fn mount(dev: impl AsRef<Path>, dst: impl AsRef<Path>) -> Anyhow<Self> {
        let dst = dst.as_ref();
        run!([MOUNT, dev.as_ref(), dst])?;
        Ok(Mounted { path: dst.into() })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn mount_fs(
        fstype: impl AsRef<str>,
        src: impl AsRef<Path>,
        dst: impl AsRef<Path>,
    ) -> Anyhow<Self> {
        let dst = dst.as_ref();
        run!([MOUNT, "-t", fstype.as_ref(), src.as_ref(), dst])?;
        Ok(Mounted { path: dst.into() })
    }

    pub fn bind(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> Anyhow<Self> {
        let dst = dst.as_ref();
        run!([MOUNT, "--bind", src.as_ref(), dst])?;
        Ok(Mounted { path: dst.into() })
    }
}

impl Drop for Mounted {
    fn drop(&mut self) {
        run!([UMOUNT, &self.path]).ok();
    }
}
