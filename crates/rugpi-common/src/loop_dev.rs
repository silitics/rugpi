use std::path::{Path, PathBuf};

use reportify::{Report, ResultExt};
use xscript::{read_str, run, Run};

reportify::new_whatever_type! {
    LoopDeviceError
}

/// A loop device with an attached image.
#[derive(Debug)]
pub struct LoopDevice {
    path: PathBuf,
}

impl LoopDevice {
    /// Attaches an image to the next free loop device.
    pub fn attach(image: impl AsRef<Path>) -> Result<Self, Report<LoopDeviceError>> {
        let image = image.as_ref();
        let path = read_str!(["losetup", "-f"]).whatever("failed to find free loop device")?;
        run!(["losetup", "-P", &path, image]).whatever("failed to bind image to loop device")?;
        Ok(LoopDevice { path: path.into() })
    }

    /// Path to the partition device.
    pub fn partition(&self, part: usize) -> PathBuf {
        let mut path = self.path.as_os_str().to_owned();
        path.push(&format!("p{}", part));
        path.into()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for LoopDevice {
    fn drop(&mut self) {
        // Detach the loop device and ignore any errors.
        run!(["losetup", "-d", &self.path]).ok();
    }
}
