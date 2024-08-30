//! *Structured Tag-Length-Value* (STLV) encoding.
//!
//! The STLV encoding specified and implemented here serves as the basis for Rugpi's
//! artifact format. It has been designed to facilitate streaming, random accesses, and
//! forward as well as backward compatibility of Rugpi artifacts.
//!
//!
//! # STLV Streams
//!
//! An STLV *stream* is a sequence of STLV *atoms*. Each atom consists of a *tag*, an
//! *indicator* byte, and optionally of a *length* and a *value*:
//!
//! ```plain
//! <atom>  ::=  <tag> <indicator> [<length>] [<value>]
//! ```
//!
//! We refer to the byte sequence of an atom without the value as the *atom head*.
//!
//! The tag has a fixed size of four bytes. Tags have an application-defined
//! interpretation and can be freely chosen. In particular, applications may encode
//! additional information into tags, e.g., whether an atom may be ignored.
//!
//! The first bit of the indicator byte indicates whether the atom is a *value* atom:
//!
//! ```plain
//!  7 6 5 4 3 2 1 0
//!  |
//!  \-- IS_VALUE
//! ```
//!
//! If the atom is a value atom, i.e., if the `IS_VALUE` bit is set, then the remaining
//! seven bits (partially) encode the length of the value. Otherwise, the atom is a
//! *segmentation* atom and the second bit indicates whether the atom *opens* or *closes*
//! a *segment*:
//!
//! ```plain
//!  7 6 5 4 3 2 1 0
//!    |
//!    \-- IS_OPEN
//! ```
//!
//! In this case, the remaining six bits must be zero.
//!
//! ### Value Atoms
//!
//! Value atoms are typically used to encode primitive values such as integers and
//! strings. The STLV encoding makes no assumptions about values and treats them as raw
//! byte sequences. It is up to the application to further interpret values.
//!
//! The length of the value in bytes is the sum of the length encoded in the indicator and
//! the subsequent optional length. If the length encoded in the indicator is 127
//! (`0b111_1111`), then the optional length must be present. Otherwise, the indicator
//! must be directly followed by the value. The optional length is encoded as a
//! variable-length integer using a typical base 128 encoding where the first bit of a
//! byte indicates whether a byte follows and the remaining seven bits are a base 128
//! digit.
//!
//! ### Segmentation Atoms
//!
//! Segmentation atoms are used to structure a stream. They neither have a length nor a
//! value. Every segmentation atom opening a segment should eventually be followed by a
//! segmentation atom, with the same tag, closing the segment. As a result, an STLV stream
//! describes a tree whose nodes are labeled with tags and whose leaves are values.
//!
//!
//! # API and Example
//!
//! This module provides low-level functions for reading and writing STLV streams.
//!
//! ```rust
//! # use rugpi_artifact::format::stlv::{
//! #   read_atom_head, skip, write_atom_head, write_open_segment,
//! #   write_close_segment, write_value, Tag, AtomHead, SkipSeek,
//! # };
//! #
//! use std::io::{self, Write, Read};
//!
//! const SEGMENT_TAG: Tag = Tag::from_bytes([0xAA, 0xBB, 0xCC, 0xDD]);
//! const VALUE_TAG: Tag = Tag::from_bytes([0x44, 0x33, 0x22, 0x11]);
//!
//! let mut buffer = io::Cursor::new(Vec::new());
//!
//! write_open_segment(&mut buffer, SEGMENT_TAG);
//! let value = "Hello World!";
//! write_value(&mut buffer, VALUE_TAG, value.as_bytes());
//! write_close_segment(&mut buffer, SEGMENT_TAG);
//!
//! buffer.set_position(0);
//!
//! assert_eq!(
//!     read_atom_head(&mut buffer).unwrap(),
//!     Some(AtomHead::open(SEGMENT_TAG))
//! );
//! let Some(AtomHead::Value {
//!     tag: VALUE_TAG,
//!     length,
//! }) = read_atom_head(&mut buffer).unwrap()
//! else {
//!     panic!("segment should contain a value");
//! };
//! let mut value_buffer = vec![0; length as usize];
//! buffer.read_exact(&mut value_buffer);
//! assert_eq!(value_buffer, value.as_bytes());
//! assert_eq!(
//!     read_atom_head(&mut buffer).unwrap(),
//!     Some(AtomHead::close(SEGMENT_TAG))
//! );
//! assert_eq!(
//!     read_atom_head(&mut buffer).unwrap(),
//!     None
//! );
//!
//! buffer.set_position(0);
//!
//! let head = read_atom_head(&mut buffer).unwrap().unwrap();
//! skip::<_, SkipSeek>(&mut buffer, head).unwrap();
//! assert_eq!(buffer.position(), buffer.get_ref().len() as u64);
//! ```
//!
//! ##### ❗️ Important Notes
//!
//! Readers and writers should be buffered as we are performing small reads and writes.
//!
//! #### Errors
//!
//! The functions defined here may return I/O errors ([`io::Error`]). In particular, the
//! following errors may be returned:
//!
//! - [`io::ErrorKind::InvalidData`]: Invalid data not adhering to the STLV specification.
//! - [`io::ErrorKind::UnexpectedEof`]: STLV stream has been truncated.
//!
//!
//! # Advantageous Properties
//!
//! STLV streams as specified here have several advantageous properties:
//!
//! - STLV streams can be parsed generically without knowing any of the tags and the
//!   interpretation of values.
//! - STLV streams can be streamed atom by atom and without knowing the size of segments
//!   beforehand.
//! - STLV streams are compositional; an STLV stream can be inserted between any two
//!   atoms.
//! - Unknown value atoms and segments can be skipped by readers enabling forward
//!   compatibility.
//! - Being binary, STLV streams can be more compact than text-based formats such as JSON
//!   or XML.
//! - STLV streams allow for detecting truncated and invalid inputs.

use std::{
    fmt,
    io::{self, BufRead, Seek, Write},
};

use console::style;

/// Tag size (4 bytes).
pub const TAG_SIZE: usize = 4;

/// Bit mask for the `IS_VALUE` bit.
const IS_VALUE_MASK: u8 = 1 << 7;
/// Bit mask for the `IS_OPEN` bit.
const IS_OPEN_MASK: u8 = 1 << 6;

/// Tag.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tag {
    /// Bytes of the tag.
    bytes: [u8; TAG_SIZE],
}

impl Tag {
    /// Construct a tag from the given bytes.
    pub const fn from_bytes(bytes: [u8; TAG_SIZE]) -> Self {
        Self { bytes }
    }

    /// Returns the [`u32`] representation of the tag.
    pub const fn as_u32(self) -> u32 {
        u32::from_be_bytes(self.bytes)
    }

    /// Returns the byte representation of the tag.
    pub const fn as_bytes(self) -> [u8; TAG_SIZE] {
        self.bytes
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{:08x}", self.as_u32()))
    }
}

impl fmt::Debug for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("Tag({:08x})", self.as_u32()))
    }
}

impl AsRef<[u8]> for Tag {
    fn as_ref(&self) -> &[u8] {
        &self.bytes
    }
}

impl AsRef<[u8; TAG_SIZE]> for Tag {
    fn as_ref(&self) -> &[u8; TAG_SIZE] {
        &self.bytes
    }
}

/// In-memory representation of an atom head.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum AtomHead {
    /// Head of a value atom.
    Value {
        /// Tag of the atom.
        tag: Tag,
        /// Total length of the value.
        length: u64,
    },
    /// Head of an opening segmentation atom.
    Open {
        /// Tag of the atom.
        tag: Tag,
    },
    /// Head of a closing segmentation atom.
    Close {
        /// Tag of the atom.
        tag: Tag,
    },
}

impl AtomHead {
    /// Create an atom head for a value atom.
    pub const fn value(tag: Tag, length: u64) -> Self {
        Self::Value { tag, length }
    }

    /// Create an atom head for an opening segmentation atom.
    pub const fn open(tag: Tag) -> Self {
        Self::Open { tag }
    }

    /// Create an atom head for a closing segmentation atom.
    pub const fn close(tag: Tag) -> Self {
        Self::Close { tag }
    }

    /// Returns whether the atom is a value atom.
    pub const fn is_value(self) -> bool {
        matches!(self, AtomHead::Value { .. })
    }

    /// Returns whether the atom is a segmentation atom.
    pub const fn is_segmentation(self) -> bool {
        !self.is_value()
    }

    /// Returns whether the atom is an opening segmentation atom.
    pub const fn is_open(self) -> bool {
        matches!(self, AtomHead::Open { .. })
    }

    /// Returns whether the atom is a closing segmentation atom.
    pub const fn is_close(self) -> bool {
        matches!(self, AtomHead::Close { .. })
    }

    /// The tag of the atom.
    pub const fn tag(self) -> Tag {
        match self {
            AtomHead::Value { tag, .. } => tag,
            AtomHead::Open { tag } => tag,
            AtomHead::Close { tag } => tag,
        }
    }

    /// Compute the size of the atom head in bytes.
    pub const fn head_size(self) -> u64 {
        let mut size = TAG_SIZE as u64;
        size += 1;
        match self {
            AtomHead::Value { length, .. } => {
                if length >= 127 {
                    size += compute_varint_size(length - 127) as u64;
                }
            }
            _ => { /* nothing to do */ }
        }
        size
    }

    /// Compute the size of the entire atom in bytes.
    pub const fn atom_size(self) -> u64 {
        let mut size = self.head_size();
        match self {
            AtomHead::Value { length, .. } => {
                size += length;
            }
            _ => { /* nothing to do */ }
        }
        size
    }
}

/// Write the head of an atom to the provided writer.
///
/// ❗️ The provided writer should be buffered as we are performing small writes.
pub fn write_atom_head<W: Write>(writer: &mut W, head: AtomHead) -> io::Result<()> {
    match head {
        AtomHead::Value { tag, length } => {
            writer.write_tag(tag)?;
            writer.write_byte(IS_VALUE_MASK | (length.min(127) as u8))?;
            if length >= 127 {
                writer.write_varint(length - 127)?;
            }
        }
        AtomHead::Open { tag } => {
            writer.write_tag(tag)?;
            writer.write_byte(IS_OPEN_MASK)?;
        }
        AtomHead::Close { tag } => {
            writer.write_tag(tag)?;
            writer.write_byte(0)?;
        }
    }
    Ok(())
}

/// Write a value with the given tag to the given writer.
pub fn write_value<W: Write>(writer: &mut W, tag: Tag, value: &[u8]) -> io::Result<()> {
    let length = u64::try_from(value.len()).expect("should fit");
    write_atom_head(writer, AtomHead::value(tag, length))?;
    writer.write_all(value)
}

/// Write an opening segmentation atom to the given writer.
pub fn write_open_segment<W: Write>(writer: &mut W, tag: Tag) -> io::Result<()> {
    write_atom_head(writer, AtomHead::open(tag))
}

/// Write a closing segmentation atom to the given writer.
pub fn write_close_segment<W: Write>(writer: &mut W, tag: Tag) -> io::Result<()> {
    write_atom_head(writer, AtomHead::close(tag))
}

/// Read the head of the next atom from the provided reader, if there is any.
///
/// Returns [`None`] in case the end of the input has been reached.
pub fn read_atom_head<R: BufRead>(reader: &mut R) -> io::Result<Option<AtomHead>> {
    let Some(tag) = reader.read_tag()? else {
        return Ok(None);
    };
    let indicator = reader.read_byte()?;
    if indicator & IS_VALUE_MASK != 0 {
        let mut length = u64::from(indicator & !IS_VALUE_MASK);
        if length == 127 {
            length += reader.read_varint()?;
        }
        Ok(Some(AtomHead::Value { tag, length }))
    } else {
        if indicator & !IS_VALUE_MASK & !IS_OPEN_MASK != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "non-zero segmentation atom indicator bits",
            ));
        }
        if indicator & IS_OPEN_MASK != 0 {
            Ok(Some(AtomHead::Open { tag }))
        } else {
            Ok(Some(AtomHead::Close { tag }))
        }
    }
}

/// Trait for skipping bytes from a reader.
pub trait Skip<R> {
    /// Skip the given number of bytes.
    fn skip(reader: &mut R, skip: u64) -> io::Result<()>;
}

/// Skip bytes by reading.
pub struct SkipRead(());

impl<R: BufRead> Skip<R> for SkipRead {
    fn skip(reader: &mut R, mut skip: u64) -> io::Result<()> {
        while skip > 0 {
            let buffer = reader.fill_buf()?;
            let consume = u64::try_from(buffer.len()).expect("should fit").min(skip);
            reader.consume(usize::try_from(consume).expect("must fit"));
            skip -= consume;
        }
        Ok(())
    }
}

/// Skip bytes by seeking.
pub struct SkipSeek(());

impl<R: Seek> Skip<R> for SkipSeek {
    fn skip(reader: &mut R, skip: u64) -> io::Result<()> {
        let skip = i64::try_from(skip).expect("should fit");
        reader.seek_relative(skip)
    }
}

/// Skip a value atom or segment given an atom head.
///
/// The atom head must indicate a value atom or open a segment.
pub fn skip<R: BufRead, S: Skip<R>>(reader: &mut R, head: AtomHead) -> io::Result<()> {
    match head {
        AtomHead::Value { length, .. } => {
            S::skip(reader, length)?;
            Ok(())
        }
        AtomHead::Open { tag: open_tag } => {
            while let Some(head) = read_atom_head(reader)? {
                match head {
                    AtomHead::Close { tag: close_tag } if open_tag == close_tag => {
                        return Ok(());
                    }
                    _ => skip::<R, S>(reader, head)?,
                }
            }
            Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!("unexpected end of input while skipping segment with tag {open_tag}"),
            ))
        }
        AtomHead::Close { tag } => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("cannot skip unbalanced closing segment atom with tag {tag}"),
        )),
    }
}

/// Resolver for tag names used for pretty printing.
pub trait TagNameResolver {
    /// Resolve the name of the given tag.
    fn resolve(&self, tag: Tag) -> Option<&str>;
}

/// Read and pretty print the structure of an STLV stream to stderr.
pub fn pretty_print<R: BufRead, S: Skip<R>>(
    reader: &mut R,
    resolver: Option<&dyn TagNameResolver>,
) -> io::Result<()> {
    let mut indent = 0;
    while let Some(head) = read_atom_head(reader)? {
        match head {
            AtomHead::Value { tag, length } => {
                // Size is at most 64 and should fit into usize.
                let buffer = &mut [0; 64][..length.min(64) as usize];
                reader.read_exact(buffer)?;
                S::skip(reader, length.saturating_sub(64))?;
                eprintln!(
                    "{:indent$}{} [{}] = {}",
                    "",
                    style(DisplayTag { resolver, tag }).for_stderr().green(),
                    style(length).for_stderr().blue(),
                    style(DisplayBytes {
                        bytes: buffer,
                        limit: 64,
                        truncated: length > 64,
                    })
                    .for_stderr()
                    .black()
                );
            }
            AtomHead::Open { tag } => {
                eprintln!(
                    "{:indent$}<{}",
                    "",
                    style(DisplayTag { resolver, tag }).for_stderr().cyan()
                );
                indent += 2;
            }
            AtomHead::Close { tag } => {
                indent = indent.saturating_sub(2);
                eprintln!(
                    "{:indent$}{}>",
                    "",
                    style(DisplayTag { resolver, tag }).for_stderr().cyan()
                );
            }
        }
    }
    Ok(())
}

/// Auxiliary struct for displaying tags.
struct DisplayTag<'r> {
    /// Tag to display.
    tag: Tag,
    /// Optional resolver for tag names.
    resolver: Option<&'r dyn TagNameResolver>,
}

impl<'r> fmt::Display for DisplayTag<'r> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.resolver {
            Some(resolver) => fmt::Write::write_fmt(
                f,
                format_args!(
                    "{} ({})",
                    resolver.resolve(self.tag).unwrap_or("UNKNOWN"),
                    self.tag
                ),
            ),
            None => fmt::Display::fmt(&self.tag, f),
        }
    }
}

/// Auxiliary struct for displaying bytes.
#[derive(Debug)]
struct DisplayBytes<T> {
    /// Bytes to display.
    bytes: T,
    /// Soft character limit.
    limit: usize,
    /// Whether the bytes have already been truncated.
    truncated: bool,
}

impl<T: AsRef<[u8]>> fmt::Display for DisplayBytes<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut remaining = self.limit;
        let mut truncated = self.truncated;
        for &byte in self.bytes.as_ref() {
            if remaining == 0 {
                truncated = true;
                break;
            }
            for c in std::ascii::escape_default(byte) {
                fmt::Write::write_char(f, c as char)?;
                remaining = remaining.saturating_sub(1);
            }
        }
        if truncated {
            fmt::Write::write_str(f, " ...")?;
        }
        Ok(())
    }
}

/// Extension trait for [`BufRead`] with some utility methods.
trait BufReadExt: BufRead {
    /// Read a single byte.
    #[inline]
    fn read_byte(&mut self) -> io::Result<u8> {
        let mut buf = [0; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    /// Read a [`Tag`].
    #[inline]
    fn read_tag(&mut self) -> io::Result<Option<Tag>> {
        let mut buf = [0; TAG_SIZE];
        let mut tail = buf.as_mut_slice();
        while !tail.is_empty() {
            let bytes_read = self.read(tail)?;
            if bytes_read == 0 {
                if tail.len() != TAG_SIZE {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "expected end of file while reading tag",
                    ));
                }
                return Ok(None);
            }
            tail = &mut tail[bytes_read..];
        }
        Ok(Some(Tag::from_bytes(buf)))
    }

    /// Read a variable-length integer.
    fn read_varint(&mut self) -> io::Result<u64> {
        let mut integer = 0u64;
        loop {
            let byte = self.read_byte()?;
            match integer.checked_shl(7) {
                Some(shifted) => {
                    integer = shifted | u64::from(byte & 0b0111_1111);
                    if byte & 0b1000_0000 == 0 {
                        break Ok(integer);
                    }
                    if integer == 0 {
                        break Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "invalid zero digit in variable-length integer",
                        ));
                    }
                }
                None => {
                    break Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "variable-length integer exceeds 64 bits",
                    ))
                }
            }
        }
    }
}

impl<R: BufRead> BufReadExt for R {}

/// Extension trait for [`Write`] with some utility methods.
trait WriteExt: Write {
    /// Write a single byte.
    #[inline]
    fn write_byte(&mut self, byte: u8) -> io::Result<()> {
        self.write_all(&[byte])
    }

    /// Write a [`Tag`].
    #[inline]
    fn write_tag(&mut self, tag: Tag) -> io::Result<()> {
        self.write_all(tag.as_ref())
    }

    /// Write a variable-length integer.
    fn write_varint(&mut self, integer: u64) -> io::Result<()> {
        let mut shift = (compute_varint_size(integer) - 1) * 7;
        loop {
            // After masking, the integer fits into `u8`.
            let digit = ((integer >> shift) & 0b0111_1111) as u8;
            if shift > 0 {
                shift -= 7;
                self.write_byte(digit | 0b1000_0000)?;
            } else {
                break self.write_byte(digit);
            }
        }
    }
}

impl<W: Write> WriteExt for W {}

/// Compute the size of a variable-length integer in bytes.
const fn compute_varint_size(integer: u64) -> usize {
    // This is at most 64 and should therefore fit into `usize`.
    let bits = (64 - integer.leading_zeros()) as usize;
    if bits == 0 {
        1
    } else {
        (bits + 6) / 7
    }
}

#[cfg(test)]
mod tests {
    use std::{io, u64};

    use super::{read_atom_head, write_atom_head, AtomHead, BufReadExt, Tag, WriteExt};

    #[test]
    pub fn test_varint_encoding() {
        fn test_roundtrip(integer: u64) {
            let mut buffer = io::Cursor::new(vec![]);
            buffer.write_varint(integer).unwrap();
            buffer.set_position(0);
            assert_eq!(buffer.read_varint().unwrap(), integer);
        }
        for integer in [0, 1, 0x7f - 1, 0x7f, 0x7f + 1, u64::MAX] {
            test_roundtrip(integer);
        }
        // Invalid zero digit.
        assert!(io::Cursor::new(vec![128]).read_varint().is_err());
        // Overflow.
        assert!(io::Cursor::new(vec![0xff; 10]).read_varint().is_err());
    }

    #[test]
    pub fn test_atom_head_encoding() {
        fn test_roundtrip(head: AtomHead) {
            let mut buffer = io::Cursor::new(vec![]);
            write_atom_head(&mut buffer, head).unwrap();
            buffer.set_position(0);
            assert_eq!(read_atom_head(&mut buffer).unwrap().unwrap(), head);
        }
        test_roundtrip(AtomHead::open(Tag::from_bytes([0x99, 0x88, 0x77, 0x66])));
        test_roundtrip(AtomHead::close(Tag::from_bytes([0x99, 0x88, 0x77, 0x66])));
        for length in [0, 1, 0x7f - 1, 0x7f, 0x7f + 1, u64::MAX] {
            test_roundtrip(AtomHead::Value {
                tag: Tag::from_bytes([0x99, 0x88, 0x77, 0x66]),
                length,
            })
        }
        // Empty buffer.
        assert_eq!(read_atom_head(&mut io::Cursor::new(vec![])).unwrap(), None);
        // Truncated buffer.
        assert!(read_atom_head(&mut io::Cursor::new(vec![0x99, 0x88])).is_err());
    }
}
