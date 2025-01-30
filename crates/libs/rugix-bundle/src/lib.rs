#![forbid(unsafe_code)]

//! Implementation of Rugix Ctrl's update bundle format.

use std::io::BufReader;
use std::path::Path;

use byte_calc::{ByteLen, NumBytes};
use format::decode::Decoder;
use format::stlv::{read_atom_head, write_atom_head, AtomHead, Tag};
use format::BundleHeader;
use reportify::{bail, whatever, Report, ResultExt};
use rugix_hashes::HashDigest;
use source::{BundleSource, FileSource, ReaderSource, SkipRead};

pub mod block_encoding;
pub mod builder;
pub mod format;
pub mod manifest;
pub mod source;

reportify::new_whatever_type! {
    /// Error reading or writing a bundle.
    BundleError
}

/// Result with [`BundleError`] as error type.
pub type BundleResult<T> = Result<T, Report<BundleError>>;

/// Compute and return the hash for the given bundle.
pub fn bundle_hash(bundle: &Path) -> BundleResult<HashDigest> {
    let bundle_file =
        BufReader::new(std::fs::File::open(bundle).whatever("unable to open bundle file")?);
    let mut source = FileSource::new(bundle_file);
    let _ = expect_start(&mut source, format::tags::BUNDLE)?;
    let mut header_bytes = Vec::new();
    let start = expect_start(&mut source, format::tags::BUNDLE_HEADER)?;
    read_into_vec(&mut source, &mut header_bytes, start)?;
    let header_source = ReaderSource::<_, SkipRead>::new(header_bytes.as_slice());
    let mut decoder = Decoder::with_default_limits(header_source);
    let bundle_header = decoder.decode::<BundleHeader>()?;
    let hash_algorithm = bundle_header.hash_algorithm;
    Ok(hash_algorithm.hash(&header_bytes))
}

/// Read next segment or value into vector.
pub fn read_into_vec(
    source: &mut dyn BundleSource,
    output: &mut Vec<u8>,
    head: AtomHead,
) -> BundleResult<()> {
    write_atom_head(output, head).unwrap();
    match head {
        AtomHead::Value { length, .. } => {
            if output.byte_len() + length < NumBytes::kibibytes(64) {
                let offset = output.len();
                output.resize(offset + length.raw as usize, 0);
                source
                    .read_exact(&mut output[offset..])
                    .whatever("unable to read value")?;
            } else {
                bail!("value too long");
            }
        }
        AtomHead::Start { tag: start_tag } => loop {
            let inner = expect_atom_head(source)?;
            match inner {
                atom @ AtomHead::End { tag } if tag == start_tag => {
                    write_atom_head(output, atom).unwrap();
                    break;
                }
                atom => {
                    read_into_vec(source, output, atom)?;
                }
            }
        },
        AtomHead::End { tag } => {
            bail!("unbalanced segment end with tag {tag}");
        }
    }
    Ok(())
}

/// Expect a segment start.
#[track_caller]
fn expect_start(source: &mut dyn BundleSource, tag: Tag) -> BundleResult<AtomHead> {
    match expect_atom_head(source)? {
        atom @ AtomHead::Start { tag: start_tag, .. } if start_tag == tag => Ok(atom),
        atom => bail!("expected start of {tag}, found {atom:?}"),
    }
}

/// Expect the head of an atom.
#[track_caller]
fn expect_atom_head(source: &mut dyn BundleSource) -> BundleResult<AtomHead> {
    read_atom_head(source)
        .and_then(|head| head.ok_or_else(|| whatever!("unexpected end of bundle, expected atom")))
}
