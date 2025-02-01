#![forbid(unsafe_code)]

//! Implementation of Rugix Ctrl's update bundle format.

use std::io::BufReader;
use std::path::Path;

use byte_calc::NumBytes;
use format::decode::decode_slice;
use format::BundleHeader;
use reader::{expect_start, read_into_vec};
use reportify::{Report, ResultExt};
use rugix_hashes::HashDigest;
use source::FileSource;

pub mod block_encoding;
pub mod builder;
pub mod format;
pub mod manifest;
pub mod reader;
pub mod source;

/// Start sequence of an update bundle.
pub const BUNDLE_MAGIC: &[u8] = &[
    0x6b, 0x50, 0x74, 0x1c, 0x40, // Start bundle.
    0x49, 0xaf, 0x64, 0x33, 0x40, // Start bundle header.
];

reportify::new_whatever_type! {
    /// Error reading or writing a bundle.
    BundleError
}

/// Result with [`BundleError`] as error type.
pub type BundleResult<T> = Result<T, Report<BundleError>>;

const BUNDLE_HEADER_SIZE_LIMIT: NumBytes = NumBytes::kibibytes(64);
// We need a large limit here as the payload header may contain a block index.
const PAYLOAD_HEADER_SIZE_LIMIT: NumBytes = NumBytes::mebibytes(16);

/// Compute and return the hash for the given bundle.
pub fn bundle_hash(bundle: &Path) -> BundleResult<HashDigest> {
    let bundle_file =
        BufReader::new(std::fs::File::open(bundle).whatever("unable to open bundle file")?);
    let mut source = FileSource::new(bundle_file);
    let _ = expect_start(&mut source, format::tags::BUNDLE)?;
    let mut header_bytes = Vec::new();
    let start = expect_start(&mut source, format::tags::BUNDLE_HEADER)?;
    read_into_vec(
        &mut source,
        &mut header_bytes,
        start,
        BUNDLE_HEADER_SIZE_LIMIT,
    )?;
    let bundle_header = decode_slice::<BundleHeader>(&header_bytes)?;
    let hash_algorithm = bundle_header.hash_algorithm;
    Ok(hash_algorithm.hash(&header_bytes))
}
