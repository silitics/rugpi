use camino::Utf8PathBuf;
use xscript::{run, Run};

pub struct Mounted {
    path: Utf8PathBuf,
}

impl Mounted {
    pub fn mount(dev: impl AsRef<str>, dst: impl AsRef<str>) -> anyhow::Result<Self> {
        let dst = dst.as_ref();
        run!(["mount", dev, dst])?;
        Ok(Mounted { path: dst.into() })
    }

    pub fn mount_fs(
        fstype: impl AsRef<str>,
        src: impl AsRef<str>,
        dst: impl AsRef<str>,
    ) -> anyhow::Result<Self> {
        let dst = dst.as_ref();
        run!(["mount", "-t", fstype, src, dst])?;
        Ok(Mounted { path: dst.into() })
    }

    pub fn bind(src: impl AsRef<str>, dst: impl AsRef<str>) -> anyhow::Result<Self> {
        let dst = dst.as_ref();
        run!(["mount", "--bind", src, dst])?;
        Ok(Mounted { path: dst.into() })
    }
}

impl Drop for Mounted {
    fn drop(&mut self) {
        run!(["umount", &self.path]).ok();
    }
}
