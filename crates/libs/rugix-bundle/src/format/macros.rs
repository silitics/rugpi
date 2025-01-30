//! Auxiliary macros for defining STLV structs and enums.

macro_rules! define_struct {
    (
        $(#[$struct_meta:meta])*
        $vis:vis struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                $field_vis:vis $field_name:ident [$field_tag:ident] : $field_type:ty,
            )*
        }
    ) => {
        $(#[$struct_meta])*
        #[derive(Debug, Clone)]
        #[non_exhaustive]
        $vis struct $name {
            $(
                $(#[$field_meta])*
                $field_vis $field_name: $field_type,
            )*
        }

        impl $crate::format::decode::Decode for $name {
            fn decode<S: BundleSource>(decoder: &mut Decoder<S>, atom: AtomHead) -> BundleResult<Self> {
                let AtomHead::Start { tag } = atom else {
                    bail!("expected segment, found {atom:?}");
                };
                $(
                    let mut $field_name = <$field_type as $crate::format::decode::Decode>::initial_value();
                )*
                loop {
                    let atom = decoder.next_atom_head()?;
                    match atom {
                        AtomHead::Start { tag } | AtomHead::Value { tag, .. } => match tag {
                            $(
                                tags::$field_tag => {
                                    match &mut $field_name {
                                        Some(value) => {
                                            value.continue_decode(decoder, atom)?;
                                        }
                                        None => $field_name = Some(<$field_type>::decode(decoder, atom)?),
                                    }
                                }
                            )*
                            tag if tags::is_optional(tag) => {
                                decoder.skip(atom)?;
                            }
                            tag => bail!("unknown tag {tag} found while decoding {}", std::any::type_name::<Self>())
                        }
                        AtomHead::End { tag: end_tag } if end_tag == tag => break,
                        AtomHead::End { tag } => bail!("unbalanced segment end with tag {tag}"),
                    }
                }
                Ok(Self {
                    $(
                        $field_name: match $field_name {
                            Some(value) => value,
                            None => bail!("missing value for field {}", stringify!($field_name))
                        },
                    )*
                })
            }
        }

        impl Encode for $name {
            fn encode(&self, writer: &mut dyn Write, tag: Tag) -> io::Result<()> {
                write_atom_head(writer, AtomHead::Start { tag })?;
                $(
                    self.$field_name.encode(writer, tags::$field_tag)?;
                )*
                write_atom_head(writer, AtomHead::End { tag })?;
                Ok(())
            }
        }
    };
}

#[expect(unused_macros, reason = "we may need this later")]
macro_rules! define_enum {
    (
        $(#[$struct_meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant_name:ident [$variant_tag:ident] : $variant_type:ty,
            )*
        }
    ) => {
        $(#[$struct_meta])*
        #[derive(Debug, Clone)]
        #[non_exhaustive]
        $vis enum $name {
            $(
                $(#[$variant_meta])*
                $variant_name($variant_type),
            )*
        }

        impl $crate::format::decode::Decode for $name {
            fn decode<S: BundleSource>(decoder: &mut Decoder<S>, atom: AtomHead) -> BundleResult<Self> {
                let AtomHead::Start { tag } = atom else {
                    bail!("expected segment, found {atom:?}");
                };
                let mut value: Option<Self> = None;
                loop {
                    let atom = decoder.next_atom_head()?;
                    match atom {
                        AtomHead::Start { tag } | AtomHead::Value { tag, .. } => match tag {
                            $(
                                tags::$variant_tag => {
                                    match &mut value {
                                        Some(value) => {
                                            value.continue_decode(decoder, atom)?;
                                        }
                                        None => value = Some(Self::$variant_name(<$variant_type>::decode(decoder, atom)?)),
                                    }
                                }
                            )*
                            tag if tags::is_optional(tag) => {
                                decoder.skip(atom)?;
                            }
                            tag => bail!("unknown tag {tag} found while decoding {}", std::any::type_name::<Self>())
                        }
                        AtomHead::End { tag: end_tag } if end_tag == tag => break,
                        AtomHead::End { tag } => bail!("unbalanced segment end with tag {tag}"),
                    }
                }
                match value {
                    Some(value) => Ok(value),
                    None => {
                        bail!("no variant found")
                    }
                }
            }

            fn continue_decode<S: BundleSource>(
                &mut self,
                decoder: &mut Decoder<S>,
                atom: AtomHead,
            ) -> BundleResult<()> {
                match self {
                    $(
                        Self::$variant_name(value) => value.continue_decode(decoder, atom),
                    )*
                }
            }
        }

        impl Encode for $name {
            fn encode(&self, writer: &mut dyn Write, tag: Tag) -> io::Result<()> {
                write_atom_head(writer, AtomHead::Start { tag })?;
                match self {
                    $(
                        Self::$variant_name(value) => {
                            value.encode(writer, tags::$variant_tag)?;
                        }
                    )*
                }
                write_atom_head(writer, AtomHead::End { tag })?;
                Ok(())
            }
        }
    };
}

pub(crate) use define_struct;
