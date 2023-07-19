//! Loop device interface.

use camino::{Utf8Path, Utf8PathBuf};
use sha1::{Digest, Sha1};
use url::Url;
use xscript::{read_str, run, Run};

/// A loop device with an attached image.
#[derive(Debug)]
pub struct LoopDevice {
    path: Utf8PathBuf,
}

impl LoopDevice {
    /// Attaches an image to the next free loop device.
    pub fn attach(image: impl AsRef<str>) -> anyhow::Result<Self> {
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

pub fn download(url: &str) -> anyhow::Result<Utf8PathBuf> {
    let url = url.parse::<Url>()?;
    let Some(file_name) = url
        .path_segments()
        .and_then(|segments| segments.last()) else {
            anyhow::bail!("unable to obtain file name from URL");
        };
    let file_extension = file_name.split_once(".").map(|(_, extension)| extension);
    let mut url_hasher = Sha1::new();
    url_hasher.update(url.as_str().as_bytes());
    let url_hash = url_hasher.finalize();
    let mut cache_file_name = hex::encode(url_hash);
    if let Some(extension) = file_extension {
        cache_file_name.push('.');
        cache_file_name.push_str(extension);
    }
    let cache_file_path = Utf8Path::new(".rugpi/cache").join(cache_file_name);
    if !cache_file_path.exists() {
        std::fs::create_dir_all(".rugpi/cache")?;
        run!(["wget", "-O", &cache_file_path, url.as_str()])?;
    }
    Ok(cache_file_path)
}
