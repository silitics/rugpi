use std::collections::HashMap;
use std::io;
use std::path::Path;

use reportify::{Report, ResultExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::system::boot_flows::BootFlowError;
use crate::system::System;

/// A U-Boot environment.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct UBootEnv {
    #[serde(flatten)]
    environ: HashMap<String, String>,
}

impl UBootEnv {
    /// Create an empty environment.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load an environment from a file.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, UBootEnvLoadError> {
        Self::from_bytes(&std::fs::read(path)?)
    }

    /// Decode an environment from U-Boot's binary representation.
    pub fn from_bytes(data: &[u8]) -> Result<Self, UBootEnvLoadError> {
        if data.len() < 5 {
            return Err(UBootEnvLoadError::InvalidSize(data.len()));
        }
        let checksum = crc32(&data[4..]);
        if data[..4] != checksum {
            return Err(UBootEnvLoadError::InvalidChecksum {
                found: data[..4].try_into().unwrap(),
                expected: checksum,
            });
        }
        let environ = data[4..]
            .split(|byte| *byte == 0)
            .filter(|entry| !entry.is_empty())
            .map(|entry| {
                std::str::from_utf8(entry)
                    .map_err(|err| {
                        eprintln!("invalid UTF-8 in entry: {entry:?}");
                        UBootEnvLoadError::InvalidUtf8(err)
                    })
                    .and_then(|entry| entry.split_once('=').ok_or(UBootEnvLoadError::InvalidEntry))
                    .map(|(key, value)| (key.to_owned(), value.to_owned()))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(Self { environ })
    }

    /// Set a value of the environment.
    pub fn set(&mut self, key: &str, value: impl AsRef<str>) {
        self.environ
            .insert(key.to_owned(), value.as_ref().to_owned());
    }

    /// Get a value from the environment.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.environ.get(key).map(String::as_str)
    }

    /// Remove a value from the environment.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.environ.remove(key)
    }

    /// Encode the environment in U-Boot's binary representation.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut data = vec![0, 0, 0, 0];
        for (idx, (key, value)) in self.environ.iter().enumerate() {
            if idx > 0 {
                data.push(0);
            }
            data.extend(key.as_bytes());
            data.push(b'=');
            data.extend(value.as_bytes());
        }
        data.push(0);
        let checksum = crc32(&data[4..]);
        data[..4].copy_from_slice(&checksum);
        data
    }

    /// Save the environment to a file.
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), io::Error> {
        std::fs::write(path, self.to_bytes())
    }
}

/// Error loading an U-Boot environment.
#[derive(Debug, Error)]
pub enum UBootEnvLoadError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("invalid size of environment file ({0} bytes)")]
    InvalidSize(usize),
    #[error("invalid CRC32 checksum (found: {found:?}, expected: {expected:?}")]
    InvalidChecksum { found: [u8; 4], expected: [u8; 4] },
    #[error("invalid UTF-8 encoding in entry")]
    InvalidUtf8(#[from] std::str::Utf8Error),
    #[error("invalid entry without `=`")]
    InvalidEntry,
}

/// Compute the CRC32 checksum of the given data.
fn crc32(data: &[u8]) -> [u8; 4] {
    crc32fast::hash(data).to_le_bytes()
}

pub fn set_spare_flag(system: &System) -> Result<(), Report<BootFlowError>> {
    let mut boot_spare_env = UBootEnv::new();
    boot_spare_env.set("boot_spare", "1");
    let config_partition = system
        .require_config_partition()
        .whatever("unable to get config partition")?;
    config_partition
        .ensure_writable(|| -> Result<(), Report<BootFlowError>> {
            // It is safe to directly write to the file here. If the file is corrupt,
            // the system will simply boot from the default partition set.
            boot_spare_env
                .save(config_partition.path().join("boot_spare.env"))
                .whatever("unable to save uboot environment")?;
            Ok(())
        })
        .whatever("unable to make config partition writable")??;
    Ok(())
}

pub fn clear_spare_flag(system: &System) -> Result<(), Report<BootFlowError>> {
    let mut boot_spare_env = UBootEnv::new();
    boot_spare_env.set("boot_spare", "0");
    let config_partition = system
        .require_config_partition()
        .whatever("unable to get config partition")?;
    config_partition
        .ensure_writable(|| -> Result<(), Report<BootFlowError>> {
            // It is safe to directly write to the file here. If the file is corrupt,
            // the system will simply boot from the default partition set.
            boot_spare_env
                .save(config_partition.path().join("boot_spare.env"))
                .whatever("unable to save uboot environment")?;
            Ok(())
        })
        .whatever("unable to make config partition writable")??;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_env() {
        UBootEnv::from_bytes(include_bytes!("../../assets/bootpart.a.env")).unwrap();
        UBootEnv::from_bytes(include_bytes!("../../assets/bootpart.b.env")).unwrap();
    }
}
