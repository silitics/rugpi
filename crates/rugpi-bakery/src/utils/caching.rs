//! Utilities for caching.

use std::{
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use rugpi_common::Anyhow;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use url::Url;
use xscript::{run, Run};

pub fn download(url: &str) -> Anyhow<PathBuf> {
    let url = url.parse::<Url>()?;
    let Some(file_name) = url.path_segments().and_then(|segments| segments.last()) else {
        anyhow::bail!("unable to obtain file name from URL");
    };
    let file_extension = file_name.split_once('.').map(|(_, extension)| extension);
    let mut url_hasher = Sha1::new();
    url_hasher.update(url.as_str().as_bytes());
    let url_hash = url_hasher.finalize();
    let mut cache_file_name = hex::encode(url_hash);
    if let Some(extension) = file_extension {
        cache_file_name.push('.');
        cache_file_name.push_str(extension);
    }
    let cache_file_path = Path::new(".rugpi/cache").join(cache_file_name);
    if !cache_file_path.exists() {
        std::fs::create_dir_all(".rugpi/cache")?;
        run!(["wget", "-O", &cache_file_path, url.as_str()])?;
    }
    Ok(cache_file_path)
}

pub fn sha1(string: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(string.as_bytes());
    hex::encode(hasher.finalize())
}

/// Modification time in seconds since the UNIX epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ModificationTime(u64);

impl ModificationTime {
    /// Extract the modification time from the provided filesystem metadata.
    fn from_metadata(metadata: &fs::Metadata) -> Result<Self, io::Error> {
        metadata.modified().map(|modified| {
            ModificationTime(
                modified
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            )
        })
    }
}

/// The modification time of the given path.
pub fn mtime(path: &Path) -> Result<ModificationTime, io::Error> {
    fs::metadata(path).and_then(|metadata| ModificationTime::from_metadata(&metadata))
}

/// Recursively scans a path and return the latest modification time.
pub fn mtime_recursive(path: &Path) -> Result<ModificationTime, io::Error> {
    let mut time = mtime(path)?;
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            time = time.max(if entry.file_type()?.is_dir() {
                mtime_recursive(&entry.path())?
            } else {
                ModificationTime::from_metadata(&entry.metadata()?)?
            });
        }
    }
    Ok(time)
}
