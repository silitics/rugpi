//! Error types.

/// Unable to parse byte unit.
#[derive(Debug)]
pub struct ByteUnitParseError;

impl core::fmt::Display for ByteUnitParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("unable to parse byte unit, the provided unit is invalid")
    }
}

impl core::error::Error for ByteUnitParseError {}

/// Unable to parse number of bytes.
#[derive(Debug)]
pub enum NumBytesParseError {
    Format,
    Overflow,
}

impl core::fmt::Display for NumBytesParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NumBytesParseError::Format => {
                f.write_str("unable to parse number of bytes, invalid format")
            }
            NumBytesParseError::Overflow => {
                f.write_str("unable to parse number of bytes, number does not fit into `u64`")
            }
        }
    }
}

impl core::error::Error for NumBytesParseError {}
