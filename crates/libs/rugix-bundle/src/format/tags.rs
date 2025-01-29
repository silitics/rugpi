/* cspell:ignore randint, clike, unportable */
//! STLV tags of the bundle format.
//!
//! The first bit of a tag indicates whether a value or segment with the tag may be
//! skipped by an older reader not supporting the tag. We call such tags *optional*. This
//! allows future versions to extend the format in a forward compatible way. Instead of
//! skipping *all* tags which are not supported by the reader, encoding this information
//! explicitly has the advantage that we can also make tags *required* in cases where an
//! extension must be processed by a reader. For instance, when extending a hash with a
//! salt, the reader must take the salt into account; otherwise, it may compute an
//! incorrect hash which can lead to hard to diagnose bugs down the line. In this case, if
//! the reader does not take the salt into account, the result would be indistinguishable
//! from an incorrect hash leading to confusing error messages. We want to fail early and
//! tell the user that the format is newer, not that the hash does not match.
//!
//! We use globally unique tags such that we can identify segments and values without any
//! context.
//!
//!
//! # Tag Generation
//!
//! The tags defined here have been randomly generated with the following Python snippets.
//!
//! To generate a random required tag:
//!
//! ```python
//! f"0x{random.randint(0, 2**31 - 1):08x}"
//! ```
//!
//! To generate a random optional tag:
//!
//! ```python
//! f"0x{random.randint(2**31, 2**32 - 1):08x}"
//! ```

use super::stlv::{self, Tag};

/// Bit mask to determine whether the handling of a tag is optional or required.
const IS_OPTIONAL_MASK: u8 = 0b1000_0000;

/// Returns whether handling of the tag is optional.
pub const fn is_optional(tag: Tag) -> bool {
    (tag.as_bytes()[0] & IS_OPTIONAL_MASK) != 0
}

/// Returns whether handling of the tag is required.
pub const fn is_required(tag: Tag) -> bool {
    !is_optional(tag)
}

/// Auxiliary macro for defining tags.
macro_rules! define_tags {
    (@define { }) => {};
    (@define {
        $(#[$meta:meta])*
        $name:ident = $tag:literal
        $($tail:tt)*
    }) => {
        $(#[$meta])*
        pub const $name: Tag = Tag::from_bytes(($tag as u32).to_be_bytes());
        define_tags! { @define $name { $($tail)* }}
    };
    (@define $name:ident { , $($tail:tt)* }) => {
        // Compile time check that the tag is indeed required.
        const _: () = {
            if is_optional($name) {
                panic!("tag is required but marked as optional");
            }
        };
        define_tags! { @define { $($tail)* }}
    };
    (@define $name:ident { ?, $($tail:tt)* }) => {
        // Compile time check that the tag is indeed optional.
        const _: () = {
            if is_required($name) {
                panic!("tag is optional but marked as required");
            }
        };
        define_tags! { @define { $($tail)* }}
    };
    (@impl {
        $(
            $(#[$meta:meta])*
            $name:ident = $tag:literal$(?)?,
        )*
    }) => {
        // Compile time check that all tags are unique.
        #[cfg(target_pointer_width = "64")]
        const _: () = {
            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            #[allow(clippy::enum_clike_unportable_variant)]
            #[allow(clippy::upper_case_acronyms)]
            enum Tags {
                $(
                    $name = $tag,
                )*
            }
        };

        /// Tag name resolver for pretty printing.
        #[derive(Debug, Clone, Copy)]
        pub struct TagNameResolver;

        impl stlv::TagNameResolver for TagNameResolver {
            fn resolve(&self, tag: Tag) -> Option<&str> {
                match tag {
                    $(
                        $name => Some(stringify!($name)),
                    )*
                    _ => None,
                }
            }
        }

        /// Returns whether the tag is known.
        pub const fn is_know(tag: Tag) -> bool {
            match tag {
                $(
                    $name => true,
                )*
                _ => false,
            }
        }
    };
    ($($tail:tt)*) => {
        define_tags! { @define { $($tail)* }}
        define_tags! { @impl { $($tail)* }}
    };
}

define_tags! {
    /// Bundle root segment.
    BUNDLE = 0x6b50741c,
    /// Bundle header segment.
    BUNDLE_HEADER = 0x49af6433,
    /// Bundle manifest.
    BUNDLE_MANIFEST = 0x5cb80dd6,

    /// Entry in the payload index.
    PAYLOAD_ENTRY = 0x13737992,
    /// Slot where the payload should be installed to.
    PAYLOAD_ENTRY_SLOT = 0x45ca7e7e,
    /// Hash of the payload's header.
    PAYLOAD_ENTRY_HEADER_HASH = 0x5f6a60b1,
    /// Hash of the payload's file.
    PAYLOAD_ENTRY_FILE_HASH = 0x0c8d1fd0,

    /// Payloads segment of the bundle.
    PAYLOADS = 0x1f38fba,

    /// Payload segment.
    PAYLOAD = 0x490cafaf,
    /// Payload header segment.
    PAYLOAD_HEADER = 0x0959ca75,
    /// Data of the payload.
    PAYLOAD_DATA = 0x42fd641a,

    /// Payload block encoding.
    BLOCK_ENCODING = 0x40ed9314,
    /// Block index.
    BLOCK_ENCODING_INDEX = 0x76b3d7a0,
    /// Block sizes.
    BLOCK_ENCODING_SIZES = 0x27e5d3f2,
}
