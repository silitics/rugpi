//! Low-level implementation of the bundle format and its structures.
//!
//! The bundle format is based on the *STLV encoding* specified and implemented in
//! [`stlv`].

use std::io::{self, Write};

use reportify::bail;

use crate::source::BundleSource;
use crate::BundleResult;

use self::decode::{Decode, Decoder};
use self::encode::Encode;
use self::macros::define_struct;
use self::stlv::{write_atom_head, write_value, AtomHead, Tag};

mod macros;

pub mod decode;
pub mod encode;
pub mod stlv;
pub mod tags;

define_struct! {
    /// Bundle header.
    pub struct BundleHeader {
        /// Bundle manifest (JSON-encoded).
        pub manifest[BUNDLE_MANIFEST]: String,
        /// Payload index.
        pub payload_index[PAYLOAD_ENTRY]: Vec<PayloadEntry>,
    }
}

define_struct! {
    /// Entry in the payload index.
    pub struct PayloadEntry {
        /// Slot where the payload should be installed to.
        pub slot[PAYLOAD_ENTRY_SLOT]: Option<String>,
        /// Hash of the payload header.
        pub header_hash[PAYLOAD_ENTRY_HEADER_HASH]: Bytes,
        /// Hash of the payload file.
        pub file_hash[PAYLOAD_ENTRY_FILE_HASH]: Bytes,
    }
}

define_struct! {
    /// Header of a payload.
    pub struct PayloadHeader {
        /// Block encoding data.
        pub block_encoding[BLOCK_ENCODING]: Option<BlockEncoding>,
    }
}

define_struct! {
    /// Payload block encoding.
    pub struct BlockEncoding {
        /// Block index.
        pub block_index[BLOCK_ENCODING_INDEX]: Bytes,
        /// Block sizes.
        pub block_sizes[BLOCK_ENCODING_SIZES]: Option<Bytes>,
    }
}

/// Encodable and decodable bytes.
#[derive(Debug, Clone)]
pub struct Bytes {
    /// Raw byte vector.
    pub raw: Vec<u8>,
}

impl Encode for Bytes {
    fn encode(&self, writer: &mut dyn Write, tag: Tag) -> io::Result<()> {
        write_value(writer, tag, &self.raw)
    }
}

impl Decode for Bytes {
    fn decode<S: BundleSource>(decoder: &mut Decoder<S>, atom: AtomHead) -> BundleResult<Self> {
        if !atom.is_value() {
            bail!("cannot decode `Bytes` from segment");
        }
        Ok(Self {
            raw: decoder.read_value()?,
        })
    }
}
