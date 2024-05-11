//! Utilities for working with GUID partition tables.

use std::ops::Deref;

use thiserror::Error;

use super::NumBlocks;
use crate::utils::{
    ascii_numbers::{self, byte_to_ascii_hex, parse_ascii_hex_byte},
    const_helpers::const_for,
};

/// Number of blocks used by a GPT partition table.
pub const GPT_TABLE_BLOCKS: NumBlocks = NumBlocks::from_value(33);

/// Length of the GUID string encoding.
pub const GUID_STRING_LENGTH: usize = 36;

/// GUID as defined in Appendix A of the UEFI standard.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Guid {
    bytes: [u8; 16],
}

impl Guid {
    /// Create a GUID from the given bytes.
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self { bytes }
    }

    /// Create a GUID from the standard string encoding.
    pub const fn from_hex_str(hex_str: &str) -> Result<Self, InvalidGuid> {
        let hex_bytes = hex_str.as_bytes();
        if hex_bytes.len() != GUID_STRING_LENGTH {
            return Err(InvalidGuid::InvalidLength {
                length: hex_bytes.len(),
            });
        }
        let mut bytes = [0; 16];
        const_for!(pos in [8, 13, 18, 23] {
            if hex_bytes[pos] != b'-' {
                return Err(InvalidGuid::InvalidByte { pos });
            }
        });
        const_for!(idx, pos in [6, 4, 2, 0, 11, 9, 16, 14, 19, 21, 24, 26, 28, 30, 32, 34] {
            let hex = [hex_bytes[pos], hex_bytes[pos + 1]];
            bytes[idx] = match parse_ascii_hex_byte(hex, pos, None) {
                Ok(byte) => byte,
                Err(error) => return Err(InvalidGuid::InvalidByte { pos: error.position() }),
            }
        });
        Ok(Self::from_bytes(bytes))
    }

    /// Convert the GUID to the standard string encoding.
    pub const fn to_hex_str(&self) -> GuidString {
        let mut hex_bytes = [0; GUID_STRING_LENGTH];
        const_for!(idx in [8, 13, 18, 23] {
            hex_bytes[idx] = b'-';
        });
        const_for!(byte, idx in [6, 4, 2, 0, 11, 9, 16, 14, 19, 21, 24, 26, 28, 30, 32, 34] {
            let hex = byte_to_ascii_hex(self.bytes[byte], ascii_numbers::Case::Upper);
            hex_bytes[idx] = hex[0];
            hex_bytes[idx + 1] = hex[1];
        });
        GuidString { hex_bytes }
    }
}

impl std::fmt::Display for Guid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex_str())
    }
}

impl std::fmt::Debug for Guid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Guid({})", &self.to_hex_str()))
    }
}

impl std::str::FromStr for Guid {
    type Err = InvalidGuid;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Guid::from_hex_str(s)
    }
}

impl serde::Serialize for Guid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_hex_str())
    }
}

impl<'de> serde::Deserialize<'de> for Guid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        Guid::from_hex_str(&string).map_err(|_| {
            <D::Error as serde::de::Error>::invalid_value(
                serde::de::Unexpected::Str(&string),
                &"a size",
            )
        })
    }
}

/// GPT partition types.
pub mod gpt_types {
    use super::Guid;
    use crate::{disk::PartitionType, utils::const_helpers::const_unwrap_result};

    /// EFI GPT partition type.
    pub const EFI: PartitionType = PartitionType::Gpt(const_unwrap_result!(Guid::from_hex_str(
        "C12A7328-F81F-11D2-BA4B-00A0C93EC93B"
    )));
    /// Linux GPT partition type.
    pub const LINUX: PartitionType = PartitionType::Gpt(const_unwrap_result!(Guid::from_hex_str(
        "0FC63DAF-8483-4772-8E79-3D69D8477DE4"
    )));
}

/// GUID string representation.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct GuidString {
    hex_bytes: [u8; GUID_STRING_LENGTH],
}

impl std::fmt::Display for GuidString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self)
    }
}

impl std::fmt::Debug for GuidString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("GuidString({:?})", self.deref()))
    }
}

impl std::ops::Deref for GuidString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        std::str::from_utf8(&self.hex_bytes).expect("ASCII encoding is valid UTF-8")
    }
}

/// Error indicating an invalid GUID string representation.
#[derive(Debug, Clone, Error)]
pub enum InvalidGuid {
    /// Invalid length.
    #[error("invalid length {length} of GUID, should be 36")]
    InvalidLength { length: usize },
    /// Invalid byte.
    #[error("invalid byte at position {pos}")]
    InvalidByte { pos: usize },
}

#[cfg(test)]
pub mod tests {
    use std::ops::Deref;

    use crate::disk::gpt::Guid;

    #[test]
    pub fn test_guid_roundtrip() {
        const EFI: &str = "C12A7328-F81F-11D2-BA4B-00A0C93EC93B";
        assert_eq!(Guid::from_hex_str(EFI).unwrap().to_hex_str().deref(), EFI);
        const LINUX: &str = "0FC63DAF-8483-4772-8E79-3D69D8477DE4";
        assert_eq!(
            Guid::from_hex_str(LINUX).unwrap().to_hex_str().deref(),
            LINUX
        );
    }
}
