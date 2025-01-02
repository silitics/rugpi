//! Utilities for caching.

use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    time::SystemTime,
};

use indicatif::{ProgressBar, ProgressStyle};
use reportify::{bail, ResultExt};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use tracing::info;
use url::Url;

use crate::{utils::prelude::*, BakeryResult};

pub fn download(url: &Url) -> BakeryResult<PathBuf> {
    let Some(file_name) = url.path_segments().and_then(|segments| segments.last()) else {
        bail!("unable to obtain file name from URL");
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
        info!("downloading `{url}`");
        std::fs::create_dir_all(".rugpi/cache").whatever("error creating cache directory")?;
        let mut response = reqwest::blocking::get(url.clone()).whatever("error retrieving URL")?;
        if response.status().is_success() {
            let Some(size) = response.content_length() else {
                bail!("server did not send `Content-Length` header");
            };
            let progress = ProgressBar::new(size).with_style(
                ProgressStyle::with_template(
                    "[{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} [{bytes_per_sec}] {msg}",
                )
                .unwrap(),
            );
            let mut file =
                fs::File::create(&cache_file_path).whatever("error creating hash file")?;
            let mut buffer = vec![0u8; 8096];
            loop {
                let chunk_size = response
                    .read(&mut buffer)
                    .whatever("error reading from response")?;
                if chunk_size > 0 {
                    file.write_all(&buffer[..chunk_size])
                        .whatever("error writing to cache file")?;
                    progress.inc(chunk_size as u64);
                } else {
                    break;
                }
            }
        } else {
            bail!("error downloading file: {}", response.status());
        }
    }
    Ok(cache_file_path)
}

#[derive(Debug, Default)]
pub struct Hasher {
    hasher: Sha1,
}

impl Hasher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, tag: &str, value: impl AsRef<[u8]>) {
        self.hasher.update(tag.as_bytes());
        self.hasher.update(b":");
        self.hasher.update(value.as_ref());
        self.hasher.update(b"\n");
    }

    pub fn finalize(self) -> String {
        hex::encode(self.hasher.finalize())
    }
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
