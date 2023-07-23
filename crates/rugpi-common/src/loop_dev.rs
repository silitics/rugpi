use camino::Utf8PathBuf;
use xscript::{read_str, run, Run};

use crate::Anyhow;

/// A loop device with an attached image.
#[derive(Debug)]
pub struct LoopDevice {
    path: Utf8PathBuf,
}

impl LoopDevice {
    /// Attaches an image to the next free loop device.
    pub fn attach(image: impl AsRef<str>) -> Anyhow<Self> {
        let path = read_str!(["losetup", "-f"])?;
        run!(["losetup", "-P", &path, image])?;
        Ok(LoopDevice { path: path.into() })
    }

    pub fn partition(&self, part: usize) -> String {
        format!("{}p{}", self.path, part)
    }
}

impl Drop for LoopDevice {
    fn drop(&mut self) {
        // Detach the loop device and ignore any errors.
        run!(["losetup", "-d", &self.path]).ok();
    }
}
