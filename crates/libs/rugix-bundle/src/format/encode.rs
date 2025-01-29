//! High-level data structure encoding API.

use std::io::{self, Write};

use byte_calc::NumBytes;

use super::stlv::{write_value, Tag};

/// A data structure that can be encoded into an STLV stream.
pub trait Encode {
    /// Encode `self` with the given tag into the writer.
    fn encode(&self, writer: &mut dyn Write, tag: Tag) -> io::Result<()>;
}

impl Encode for String {
    fn encode(&self, writer: &mut dyn Write, tag: Tag) -> io::Result<()> {
        write_value(writer, tag, self.as_bytes())
    }
}

impl<T: Encode> Encode for Vec<T> {
    fn encode(&self, writer: &mut dyn Write, tag: Tag) -> io::Result<()> {
        for item in self {
            item.encode(writer, tag)?;
        }
        Ok(())
    }
}

impl<T: Encode> Encode for Option<T> {
    fn encode(&self, writer: &mut dyn Write, tag: Tag) -> io::Result<()> {
        match self {
            Some(value) => value.encode(writer, tag)?,
            None => { /* nothing to do */ }
        }
        Ok(())
    }
}

impl Encode for bool {
    fn encode(&self, writer: &mut dyn Write, tag: Tag) -> io::Result<()> {
        write_value(writer, tag, if *self { &[1] } else { &[0] })
    }
}

impl Encode for NumBytes {
    fn encode(&self, writer: &mut dyn Write, tag: Tag) -> io::Result<()> {
        self.raw.encode(writer, tag)
    }
}

/// Auxiliary macro for implementing [`Encode`] on integer types.
macro_rules! impl_encode_for_int {
    ($($int:ty),*) => {
        $(
            impl Encode for $int {
                fn encode(&self, writer: &mut dyn Write, tag: Tag) -> io::Result<()> {
                    write_value(writer, tag, &self.to_be_bytes())
                }
            }
        )*
    };
}

impl_encode_for_int! { i8, u8, i16, u16, i32, u32, i64, u64, i128, u128 }

/// Encode a data structure as a byte vector.
pub fn to_vec<T: Encode>(value: &T, tag: Tag) -> Vec<u8> {
    let mut buffer = io::Cursor::new(Vec::new());
    value.encode(&mut buffer, tag).expect("should not fail");
    buffer.into_inner()
}
