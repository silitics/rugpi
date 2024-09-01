//! Implementation of Rugpi's artifact format.
//!
//! The artifact format is based on the *STLV encoding* specified and implemented in
//! [`stlv`].

use std::{
    collections::HashMap,
    io::{self, BufRead, Write},
};

use bytes::Bytes;

use self::{
    decode::{Decode, DecodeError, SegmentDecoder},
    encode::Encode,
    macros::{define_enum, define_struct},
    stlv::{write_atom_head, AtomHead},
};

pub mod decode;
pub mod encode;
pub mod stlv;
pub mod tags;

mod macros;

define_struct! {
    pub struct ArtifactHeader {
        /// Metadata of the artifact.
        pub metadata[METADATA]: Metadata,
        /// Information about the artifact's fragments.
        pub fragments[FRAGMENT]: Vec<FragmentInfo>,
    }
}

define_struct! {
    pub struct FragmentInfo {
        /// Metadata of the fragment.
        pub metadata[METADATA]: Metadata,
        /// Filename of the fragment.
        pub filename[FRAGMENT_INFO_FILENAME]: String,
        /// Optional offset of the fragment.
        pub offset[FRAGMENT_INFO_OFFSET]: Option<u64>,
        /// Optional slot indicting where to install the fragment.
        pub slot[FRAGMENT_INFO_SLOT]: Option<String>,
        /// Hash of the fragment's header.
        pub header_hash[FRAGMENT_INFO_HEADER_HASH]: Hash,
        /// Hash of the fragment's payload.
        pub payload_hash[FRAGMENT_INFO_PAYLOAD_HASH]: Hash,
    }
}

define_struct! {
    pub struct Hash {
        /// Algorithm used for hashing.
        pub algorithm[HASH_ALGORITHM]: String,
        /// Hash digest.
        pub digest[HASH_DIGEST]: Bytes,
    }
}

define_struct! {
    pub struct FragmentHeader {
        /// Compression applied to the fragment's payload.
        pub compression[FRAGMENT_COMPRESSION]: Option<FragmentCompression>,
        /// Block index of the encoded payload.
        ///
        /// Is used to ensure the block-wise integrity of the payload.
        pub encoded_index[FRAGMENT_ENCODED_INDEX]: Option<BlockIndex>,
        /// Block index of the decoded payload.
        ///
        /// Is used to ensure the block-wise integrity of the payload.
        pub decoded_index[FRAGMENT_DECODED_INDEX]: Option<BlockIndex>,
    }
}

define_struct! {
    pub struct BlockIndex {
        pub block_size[BLOCK_INDEX_BLOCK_SIZE]: u64,
        pub hash_algorithm[BLOCK_INDEX_HASH_ALGORITHM]: String,
        pub hash_digests[BLOCK_INDEX_HASH_DIGESTS]: Bytes,
    }
}

define_struct! {
    pub struct DeltaCompression {
        pub method[COMPRESSION_METHOD]: String,
        pub sources[COMPRESSION_SOURCES]: Vec<Hash>,
    }
}

define_struct! {
    pub struct FullCompression {
        pub method[COMPRESSION_METHOD]: String,
    }
}

define_enum! {
    enum FragmentCompression {
        Full[COMPRESSION_FULL]: FullCompression,
        Delta[COMPRESSION_DELTA]: DeltaCompression,
    }
}

#[derive(Debug, Default, Clone)]
pub struct Metadata {
    pub map: HashMap<String, String>,
}

impl Decode for Metadata {
    fn decode_segment<'r, R: BufRead>(
        mut segment: SegmentDecoder<'r, R>,
    ) -> Result<Self, DecodeError> {
        let mut key = None;
        let mut map = HashMap::new();
        while let Some(decoder) = segment.next()? {
            match decoder.tag() {
                tags::METADATA_KEY => {
                    if key.is_some() {
                        todo!("duplicate key error");
                    }
                    key = decoder.decode()?;
                }
                tags::METADATA_VALUE => {
                    let Some(key) = key.take() else {
                        todo!("no key");
                    };
                    map.insert(key, decoder.decode()?);
                }
                tag if tags::is_optional(tag) => {
                    decoder.skip()?;
                }
                _ => {
                    todo!("handle unknown tag")
                }
            }
        }
        Ok(Self { map })
    }
}

impl Encode for Metadata {
    fn encode<W: Write>(&self, writer: &mut W, tag: stlv::Tag) -> io::Result<()> {
        write_atom_head(writer, AtomHead::Open { tag })?;
        for (key, value) in &self.map {
            key.encode(writer, tags::METADATA_KEY)?;
            value.encode(writer, tags::METADATA_VALUE)?;
        }
        write_atom_head(writer, AtomHead::Close { tag })?;
        Ok(())
    }
}
