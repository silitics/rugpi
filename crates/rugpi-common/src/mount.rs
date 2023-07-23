use camino::Utf8PathBuf;
use xscript::{cmd, run, ParentEnv, Run};

use crate::Anyhow;

/// The `mount` executable.
const MOUNT: &str = "/usr/bin/mount";
/// The `umount` executable.
const UMOUNT: &str = "/usr/bin/umount";

pub struct Mounted {
    path: Utf8PathBuf,
}

impl Mounted {
    pub fn mount(dev: impl AsRef<str>, dst: impl AsRef<str>) -> Anyhow<Self> {
        let dst = dst.as_ref();
        run!([MOUNT, dev, dst])?;
        Ok(Mounted { path: dst.into() })
    }

    pub fn mount_fs(
        fstype: impl AsRef<str>,
        src: impl AsRef<str>,
        dst: impl AsRef<str>,
    ) -> Anyhow<Self> {
        let dst = dst.as_ref();
        run!([MOUNT, "-t", fstype, src, dst])?;
        Ok(Mounted { path: dst.into() })
    }

    pub fn bind(src: impl AsRef<str>, dst: impl AsRef<str>) -> Anyhow<Self> {
        let dst = dst.as_ref();
        run!([MOUNT, "--bind", src, dst])?;
        Ok(Mounted { path: dst.into() })
    }
}

impl Drop for Mounted {
    fn drop(&mut self) {
        run!([UMOUNT, &self.path]).ok();
    }
}
