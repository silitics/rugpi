use std::path::{Path, PathBuf};

use xscript::{read_str, run, Run};

use crate::Anyhow;

/// A loop device with an attached image.
#[derive(Debug)]
pub struct LoopDevice {
    path: PathBuf,
}

impl LoopDevice {
    /// Attaches an image to the next free loop device.
    pub fn attach(image: impl AsRef<Path>) -> Anyhow<Self> {
        let image = image.as_ref();
        let path = read_str!(["losetup", "-f"])?;
        run!(["losetup", "-P", &path, image])?;
        Ok(LoopDevice { path: path.into() })
    }

    /// Path to the partition device.
    pub fn partition(&self, part: usize) -> PathBuf {
        let mut path = self.path.as_os_str().to_owned();
        path.push(&format!("p{}", part));
        path.into()
    }
}

impl Drop for LoopDevice {
    fn drop(&mut self) {
        // Detach the loop device and ignore any errors.
        run!(["losetup", "-d", &self.path]).ok();
    }
}
