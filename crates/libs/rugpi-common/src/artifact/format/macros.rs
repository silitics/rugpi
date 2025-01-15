//! Auxiliary macros for defining STLV structs and enums.

macro_rules! define_struct {
    (
        $vis:vis struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                $field_vis:vis $field_name:ident [$field_tag:ident] : $field_type:ty,
            )*
        }
    ) => {
        #[derive(Debug, Clone)]
        $vis struct $name {
            $(
                $(#[$field_meta])*
                $field_vis $field_name: $field_type,
            )*
        }

        impl $crate::artifact::format::decode::Decode for $name {
            fn decode_segment<R: std::io::BufRead>(
                mut segment: $crate::artifact::format::decode::SegmentDecoder<'_, R>,
            ) -> Result<Self, $crate::artifact::format::decode::DecodeError> {
                $(
                    let mut $field_name = <$field_type>::initial_value();
                )*
                while let Some(decoder) = segment.next()? {
                    match decoder.tag() {
                        $(
                            tags::$field_tag => {
                                $field_name.decode_extension(decoder)?;
                            }
                        )*
                        tag if tags::is_optional(tag) => {
                            decoder.skip()?;
                        }
                        _ => {
                            todo!("handle unknown non-optional tag");
                        }
                    }
                }
                Ok(Self { $($field_name: $field_name.unwrap(),)* })
            }
        }

        impl Encode for $name {
            fn encode<W: Write>(&self, writer: &mut W, tag: $crate::artifact::format::stlv::Tag) -> io::Result<()> {
                write_atom_head(writer, AtomHead::Open { tag })?;
                $(
                    self.$field_name.encode(writer, tags::$field_tag)?;
                )*
                write_atom_head(writer, AtomHead::Close { tag })?;
                Ok(())
            }
        }
    };
}

macro_rules! define_enum {
    (
        enum $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant_name:ident [$variant_tag:ident] : $variant_type:ty,
            )*
        }
    ) => {
        #[derive(Debug, Clone)]
        pub enum $name {
            $(
                $(#[$variant_meta])*
                $variant_name($variant_type),
            )*
        }

        impl Decode for $name {
            fn decode_segment<R: BufRead>(
                mut segment: SegmentDecoder<'_, R>,
            ) -> Result<Self, DecodeError> {
                let mut variant = None;
                while let Some(decoder) = segment.next()? {
                    match decoder.tag() {
                        $(
                            tags::$variant_tag => {
                                if variant.is_some() {
                                    todo!("duplicate variant");
                                }
                                variant = Some(Self::$variant_name(decoder.decode()?));
                            }
                        )*
                        tag if tags::is_optional(tag) => {
                            decoder.skip()?;
                        }
                        _ => {
                            todo!("handle unknown compression");
                        }
                    }
                }
                variant.ok_or_else(|| todo!("error no variant"))
            }
        }

        impl Encode for $name {
            fn encode<W: Write>(&self, writer: &mut W, tag: $crate::artifact::format::stlv::Tag) -> io::Result<()> {
                write_atom_head(writer, AtomHead::Open { tag })?;
                match self {
                    $(
                        Self::$variant_name(inner) => {
                            inner.encode(writer, tags::$variant_tag)?
                        }
                    )*
                }
                write_atom_head(writer, AtomHead::Close { tag })?;
                Ok(())
            }
        }
    };
}

pub(crate) use {define_enum, define_struct};
