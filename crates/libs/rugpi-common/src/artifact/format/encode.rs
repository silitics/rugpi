//! High-level encoding API.

use std::io::{self, Write};

use bytes::Bytes;

use super::stlv::{write_value, Tag};

pub trait Encode {
    fn encode<W: Write>(&self, writer: &mut W, tag: Tag) -> io::Result<()>;
}

impl Encode for String {
    fn encode<W: Write>(&self, writer: &mut W, tag: Tag) -> io::Result<()> {
        write_value(writer, tag, self.as_bytes())
    }
}

impl<T: Encode> Encode for Vec<T> {
    fn encode<W: Write>(&self, writer: &mut W, tag: Tag) -> io::Result<()> {
        for item in self {
            item.encode(writer, tag)?;
        }
        Ok(())
    }
}

impl<T: Encode> Encode for Option<T> {
    fn encode<W: Write>(&self, writer: &mut W, tag: Tag) -> io::Result<()> {
        match self {
            Some(value) => value.encode(writer, tag)?,
            None => { /* nothing to do */ }
        }
        Ok(())
    }
}

impl Encode for Bytes {
    fn encode<W: Write>(&self, writer: &mut W, tag: Tag) -> io::Result<()> {
        write_value(writer, tag, self)
    }
}

impl Encode for bool {
    fn encode<W: Write>(&self, writer: &mut W, tag: Tag) -> io::Result<()> {
        write_value(writer, tag, if *self { &[1] } else { &[0] })
    }
}

macro_rules! impl_encode_for_int {
    ($($int:ty),*) => {
        $(
            impl Encode for $int {
                fn encode<W: Write>(&self, writer: &mut W, tag: Tag) -> io::Result<()> {
                    write_value(writer, tag, &self.to_be_bytes())
                }
            }
        )*
    };
}

impl_encode_for_int! { i8, u8, i16, u16, i32, u32, i64, u64, i128, u128 }

pub fn to_vec<T: Encode>(value: T, tag: Tag) -> Vec<u8> {
    let mut buffer = io::Cursor::new(Vec::new());
    value.encode(&mut buffer, tag).expect("should not fail");
    buffer.into_inner()
}
