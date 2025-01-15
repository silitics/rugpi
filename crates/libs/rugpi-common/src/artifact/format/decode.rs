//! High-level decoding API.

use std::io::{self, BufRead, Seek};
use std::ops::Range;

use bytes::Bytes;
use thiserror::Error;

use super::stlv::{self, read_atom_head, AtomHead, SkipRead, Tag};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DecodeError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("unexpected tag {tag}")]
    UnexpectedTag { tag: Tag },
    #[error("invalid length (got: {length}, expected: [{},{}])", expected.start, expected.end - 1)]
    InvalidLength { length: u64, expected: Range<usize> },
    #[error("invalid atom head {head:?}")]
    InvalidAtom { head: AtomHead },
}

/// Data structure that can be decoded from an STLV stream.
pub trait Decode: Sized {
    /// Try to decode the structure from a segment.
    #[allow(unused_variables)]
    fn decode_segment<R: BufRead>(segment: SegmentDecoder<'_, R>) -> Result<Self, DecodeError> {
        todo!("cannot be decoded from segment")
    }

    /// Try to decode the structure from a value.
    #[allow(unused_variables)]
    fn decode_value<R: BufRead>(value: ValueDecoder<'_, R>) -> Result<Self, DecodeError> {
        todo!("cannot be decoded from value")
    }

    /// Try to decode the structure from either a segment or a value.
    fn decode<R: BufRead>(decoder: Decoder<'_, R>) -> Result<Self, DecodeError> {
        match decoder {
            Decoder::Segment(segment) => Self::decode_segment(segment),
            Decoder::Value(value) => Self::decode_value(value),
        }
    }

    #[allow(unused_variables)]
    fn decode_extension<R: BufRead>(&mut self, decoder: Decoder<'_, R>) -> Result<(), DecodeError> {
        todo!(
            "cannot decode extension of {}",
            std::any::type_name::<Self>()
        )
    }

    fn initial_value() -> Option<Self> {
        None
    }
}

impl<T: Decode> Decode for Option<T> {
    fn decode<R: BufRead>(decoder: Decoder<'_, R>) -> Result<Self, DecodeError> {
        Ok(Some(decoder.decode()?))
    }

    fn decode_extension<R: BufRead>(&mut self, decoder: Decoder<'_, R>) -> Result<(), DecodeError> {
        match self {
            Some(inner) => inner.decode_extension(decoder),
            None => {
                *self = Some(decoder.decode()?);
                Ok(())
            }
        }
    }

    fn initial_value() -> Option<Self> {
        Some(T::initial_value())
    }
}

impl<T: Decode> Decode for Vec<T> {
    fn decode<R: BufRead>(decoder: Decoder<'_, R>) -> Result<Self, DecodeError> {
        Ok(vec![decoder.decode()?])
    }

    fn decode_extension<R: BufRead>(&mut self, decoder: Decoder<'_, R>) -> Result<(), DecodeError> {
        self.push(decoder.decode()?);
        Ok(())
    }

    fn initial_value() -> Option<Self> {
        Some(Vec::new())
    }
}

impl Decode for String {
    fn decode_value<R: BufRead>(mut value: ValueDecoder<'_, R>) -> Result<Self, DecodeError> {
        String::from_utf8(value.consume_bytes()?).map_err(|_| todo!("handle invalid UTF-8"))
    }
}

impl Decode for Bytes {
    fn decode_value<R: BufRead>(mut value: ValueDecoder<'_, R>) -> Result<Self, DecodeError> {
        Ok(value.consume_bytes()?.into())
    }
}

impl Decode for bool {
    fn decode_value<R: BufRead>(mut value: ValueDecoder<'_, R>) -> Result<Self, DecodeError> {
        Ok(value.consume_array::<1>()?[0] != 0)
    }
}

macro_rules! impl_decode_for_int {
    ($($int:ty),*) => {
        $(
            impl Decode for $int {
                fn decode_value<R: BufRead>(mut value: ValueDecoder<'_, R>) -> Result<Self, DecodeError> {
                    Ok(Self::from_be_bytes(value.consume_array()?))
                }
            }
        )*
    };
}

impl_decode_for_int! { i8, u8, i16, u16, i32, u32, i64, u64, i128, u128 }

pub fn start_decoder<R: BufRead>(reader: &mut R) -> Result<Option<Decoder<'_, R>>, DecodeError> {
    let Some(head) = read_atom_head(reader)? else {
        return Ok(None);
    };
    match head {
        AtomHead::Value { length, .. } => Ok(Some(Decoder::Value(ValueDecoder {
            reader,
            head,
            remaining: length,
        }))),
        AtomHead::Open { .. } => Ok(Some(Decoder::Segment(SegmentDecoder {
            reader,
            head,
            completed: false,
        }))),
        AtomHead::Close { .. } => todo!("cannot start decoder on closing segment"),
    }
}

#[must_use]
pub struct SegmentDecoder<'r, R> {
    reader: &'r mut R,
    head: AtomHead,
    completed: bool,
}

impl<'r, R: BufRead> SegmentDecoder<'r, R> {
    pub fn start(reader: &'r mut R) -> Result<Self, DecodeError> {
        let Some(head) = read_atom_head(reader)? else {
            todo!("no segment to read");
        };
        match head {
            AtomHead::Open { .. } => Ok(Self {
                reader,
                head,
                completed: false,
            }),
            _ => {
                todo!("no opening segmentation atom")
            }
        }
    }

    pub fn tag(&self) -> Tag {
        self.head.tag()
    }

    pub fn skip(&mut self) -> Result<(), DecodeError> {
        if !self.completed {
            stlv::skip::<_, SkipRead>(self.reader, self.head)?;
        }
        self.completed = true;
        Ok(())
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<Decoder<'_, R>>, DecodeError> {
        if self.completed {
            return Ok(None);
        }
        let Some(head) = read_atom_head(self.reader)? else {
            todo!("unexpected eof");
        };
        match head {
            AtomHead::Value { length, .. } => Ok(Some(Decoder::Value(ValueDecoder {
                reader: self.reader,
                head,
                remaining: length,
            }))),
            AtomHead::Open { .. } => Ok(Some(Decoder::Segment(SegmentDecoder {
                reader: self.reader,
                head,
                completed: false,
            }))),
            AtomHead::Close { tag } => {
                if tag == self.tag() {
                    self.completed = true;
                    Ok(None)
                } else {
                    todo!("wrong closing guard");
                }
            }
        }
    }
}

#[must_use]
pub enum Decoder<'r, R> {
    Segment(SegmentDecoder<'r, R>),
    Value(ValueDecoder<'r, R>),
}

impl<R: BufRead> Decoder<'_, R> {
    pub fn skip(self) -> Result<(), DecodeError> {
        match self {
            Decoder::Segment(mut segment) => segment.skip(),
            Decoder::Value(mut value) => value.skip(),
        }
    }

    pub fn tag(&self) -> Tag {
        match self {
            Decoder::Segment(segment) => segment.head.tag(),
            Decoder::Value(value) => value.head.tag(),
        }
    }

    pub fn decode<T: Decode>(self) -> Result<T, DecodeError> {
        T::decode(self)
    }
}

#[must_use]
pub struct ValueDecoder<'r, R> {
    reader: &'r mut R,
    head: AtomHead,
    remaining: u64,
}

impl<R: BufRead> ValueDecoder<'_, R> {
    pub fn tag(&self) -> Tag {
        self.head.tag()
    }

    pub fn skip(&mut self) -> Result<(), DecodeError> {
        todo!()
    }

    pub fn consume_bytes(&mut self) -> Result<Vec<u8>, DecodeError> {
        let mut buffer = vec![0; self.remaining_as_usize()?];
        self.reader.read_exact(&mut buffer)?;
        self.remaining = 0;
        Ok(buffer)
    }

    pub fn consume_array<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        if self.remaining_as_usize()? != N {
            todo!("invalid length");
        }
        let mut buffer = [0; N];
        self.reader.read_exact(&mut buffer)?;
        self.remaining = 0;
        Ok(buffer)
    }

    pub fn remaining_as_usize(&self) -> Result<usize, DecodeError> {
        usize::try_from(self.remaining).map_err(|_| todo!("overflow"))
    }
}

impl<R: Seek> ValueDecoder<'_, R> {
    pub fn skip_seek(&mut self) -> io::Result<()> {
        self.reader.seek_relative(self.remaining as i64)?;
        self.remaining = 0;
        Ok(())
    }
}
