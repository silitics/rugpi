#![no_std]

//! Helper crate to work with _bit, byte, and block sizes_.
//!
//! This crate provides three dedicated types, [`NumBits`], [`NumBytes`], and
//! [`NumBlocks`], to represent numbers of bits, bytes, and blocks, respectively. It
//! implements the usual traits for numeric operators such that calculations on them can
//! be carried out with succinct syntax. All operations will panic on errors, such as
//! over- or underflows. This is an intentional design decision to prevent subtly
//! incorrect results and behavior. In addition, this crate provides formatting and
//! parsing for byte sizes.
//!
//! This crate is `no_std`-compatible.
//!
//!
//! ## Conversions
//!
//! The provided types support convenient conversions to each other:
//!
//! ```rust
//! # use byte_calc::{NumBits, NumBytes, NumBlocks};
//! assert_eq!(NumBits::new(15).to_bytes_ceil(), NumBytes::bytes(2));
//! assert_eq!(NumBits::new(15).to_bytes_floor(), NumBytes::bytes(1));
//!
//! assert_eq!(NumBytes::bytes(2).to_bits(), NumBits::new(16));
//! assert_eq!(NumBits::from(NumBytes::bytes(2)), NumBits::new(16));
//!
//! const BLOCK_SIZE: NumBytes = NumBytes::kibibytes(4);
//! assert_eq!(NumBytes::bytes(8193).to_blocks_ceil(BLOCK_SIZE), NumBlocks::new(3));
//! assert_eq!(NumBytes::bytes(8193).to_blocks_floor(BLOCK_SIZE), NumBlocks::new(2));
//!
//! assert_eq!(NumBlocks::new(2).to_bytes(BLOCK_SIZE), NumBytes::kibibytes(8));
//! ```
//!
//!
//! ## Calculations
//!
//! Calculations can be performed with the types as well as with [`u64`] integers:
//!
//! ```rust
//! # use byte_calc::{NumBits, NumBytes, NumBlocks};
//! assert_eq!(NumBytes::bytes(10) / NumBytes::bytes(2), NumBytes::bytes(5));
//! assert_eq!(NumBytes::bytes(10) / 2, NumBytes::bytes(5));
//!
//! assert_eq!(NumBits::new(5) + NumBits::new(8), NumBits::new(13));
//! assert_eq!(NumBits::new(5) * 2, NumBits::new(10));
//!
//! assert_eq!(NumBlocks::new(10) + 2, NumBlocks::new(12));
//!
//! assert_eq!(NumBits::new(2) + NumBytes::bytes(1), NumBits::new(10));
//! ```
//!
//!
//! ## Comparisons
//!
//! Comparisons are supported on the types as well as with [`u64`] integers:
//!
//! ```rust
//! # use byte_calc::{NumBits, NumBytes, NumBlocks};
//! assert!(NumBytes::bytes(10) < 20);
//! assert!(NumBytes::bytes(10) != 0);
//!
//! assert_eq!(NumBits::new(5), 5);
//!
//! assert_eq!(NumBits::new(16), NumBytes::new(2));
//! assert!(NumBits::new(15) < NumBytes::new(2));
//! ```
//!
//!
//! ## Formatting
//!
//! Formatting of byte sizes maximizes the unit while minimizing the integer part towards
//! one. For example:
//!
//! ```rust
//! # use byte_calc::NumBytes;
//! assert_eq!(NumBytes::mebibytes(128).to_string(), "128MiB");
//! assert_eq!(NumBytes::gigabytes(1).to_string(), "1GB");
//! assert_eq!(NumBytes::bytes(1023).to_string(), "1.023kB");
//! assert_eq!(NumBytes::bytes(1000).to_string(), "1kB");
//! assert_eq!(NumBytes::bytes(999).to_string(), "999B");
//! assert_eq!(NumBytes::bytes(2560).to_string(), "2.5KiB");
//! ```
//!
//! The usual formatting syntax can be used to limit the precision:
//!
//! ```rust
//! # use byte_calc::NumBytes;
//! assert_eq!(format!("{:.2}", NumBytes::terabytes(2)), "1.81TiB");
//! ```
//!
//!
//! ## Parsing
//!
//! Byte sizes must follow the following syntax:
//!
//! ```plain
//! ⟨byte-size⟩  ::=  ⟨int⟩ [ '.' ⟨int⟩ ] [ ' '* ⟨unit⟩ ]
//! ⟨int⟩  ::=  [0-9_]+
//! ⟨unit⟩ ::=  'B' | 'K' … 'E' | 'kB' … 'EB' | 'KiB' … 'EiB' (case-insensitive)
//! ```
//!
//! The units (`K` ... `E`) are interpreted as binary units (`KiB` ...  `EiB`). Generally,
//! unit parsing is case-insensitive.
//!
//! ```rust
//! # use core::str::FromStr;
//! # use byte_calc::NumBytes;
//! assert_eq!(NumBytes::from_str("5").unwrap(), NumBytes::bytes(5));
//! assert_eq!(NumBytes::from_str("2.5KiB").unwrap(), NumBytes::bytes(2560));
//! assert_eq!(NumBytes::from_str("2_000kB").unwrap(), NumBytes::megabytes(2));
//! ```
//!
//! Parsing also works in `const` contexts using [`NumBytes::parse_str`] or
//! [`NumBytes::parse_ascii`]:
//!
//! ```rust
//! # use core::str::FromStr;
//! # use byte_calc::NumBytes;
//! const BLOCK_SIZE: NumBytes = match NumBytes::parse_str("4KiB") {
//!     Ok(value) => value,
//!     Err(_) => panic!("invalid format"),
//! };
//! ```
//!
//! The parser has been extensively fuzz-tested, ensuring that no input leads to panics.
//!
//!
//! ## Serialization and Deserialization
//!
//! By enabling the `serde` feature, [`NumBits`], [`NumBytes`], and [`NumBlocks`] can be
//! serialized and deserialized. All tree types always serialize as [`u64`] integers.
//! Deserialization of [`NumBytes`] is also supported from strings.

pub mod errors;

use crate::errors::{ByteUnitParseError, NumBytesParseError};

/// Auxiliary macro as a replacement for `?` in `const` contexts.
macro_rules! const_try {
    ($expr:expr) => {
        match $expr {
            Ok(value) => value,
            Err(error) => return Err(error),
        }
    };
}

/// Auxiliary macro for the definition of [`u64`] wrapper types.
macro_rules! define_types {
    ($($name:ident, $type:literal;)*) => {
        $(
            #[doc = concat!("Represents a number of _", $type, "_.")]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $name {
                #[doc = concat!("Raw number of ", $type, ".")]
                pub raw: u64,
            }

            impl $name {
                #[doc = concat!(
                    "Construct [`", stringify!($name),
                    "`] from the provided raw number of ", $type, "."
                )]
                pub const fn new(raw: u64) -> Self {
                    Self { raw }
                }
            }

            impl core::ops::Add for $name {
                type Output = $name;

                fn add(self, rhs: Self) -> Self::Output {
                    self + rhs.raw
                }
            }

            impl core::ops::Add<u64> for $name {
                type Output = $name;

                fn add(self, rhs: u64) -> Self::Output {
                    Self::new(self.raw.checked_add(rhs).expect(concat!("overflow adding ", $type)))
                }
            }

            impl core::ops::AddAssign for $name {
                fn add_assign(&mut self, rhs: Self) {
                    *self = *self + rhs;
                }
            }

            impl core::ops::AddAssign<u64> for $name {
                fn add_assign(&mut self, rhs: u64) {
                    *self = *self + rhs;
                }
            }

            impl core::ops::Sub for $name {
                type Output = $name;

                fn sub(self, rhs: Self) -> Self::Output {
                    self - rhs.raw
                }
            }

            impl core::ops::Sub<u64> for $name {
                type Output = $name;

                fn sub(self, rhs: u64) -> Self::Output {
                    Self::new(self.raw.checked_sub(rhs).expect(concat!("underflow subtracting ", $type)))
                }
            }

            impl core::ops::SubAssign for $name {
                fn sub_assign(&mut self, rhs: Self) {
                    *self = *self - rhs;
                }
            }

            impl core::ops::SubAssign<u64> for $name {
                fn sub_assign(&mut self, rhs: u64) {
                    *self = *self - rhs;
                }
            }

            impl core::ops::Mul for $name {
                type Output = $name;

                fn mul(self, rhs: Self) -> Self::Output {
                    self * rhs.raw
                }
            }

            impl core::ops::Mul<u64> for $name {
                type Output = $name;

                fn mul(self, rhs: u64) -> Self::Output {
                    Self::new(self.raw.checked_mul(rhs).expect(concat!("overflow multiplying ", $type)))
                }
            }

            impl core::ops::MulAssign for $name {
                fn mul_assign(&mut self, rhs: Self) {
                    *self = *self * rhs;
                }
            }

            impl core::ops::MulAssign<u64> for $name {
                fn mul_assign(&mut self, rhs: u64) {
                    *self = *self * rhs;
                }
            }

            impl core::ops::Div for $name {
                type Output = $name;

                fn div(self, rhs: Self) -> Self::Output {
                    self / rhs.raw
                }
            }

            impl core::ops::Div<u64> for $name {
                type Output = $name;

                fn div(self, rhs: u64) -> Self::Output {
                    Self::new(self.raw.checked_div(rhs).expect("division by zero"))
                }
            }

            impl core::ops::DivAssign for $name {
                fn div_assign(&mut self, rhs: Self) {
                    *self = *self / rhs;
                }
            }

            impl core::ops::DivAssign<u64> for $name {
                fn div_assign(&mut self, rhs: u64) {
                    *self = *self / rhs;
                }
            }

            impl PartialEq<u64> for $name {
                fn eq(&self, other: &u64) -> bool {
                    self.raw == *other
                }
            }

            impl PartialEq<$name> for u64 {
                fn eq(&self, other: &$name) -> bool {
                    *self == other.raw
                }
            }

            impl PartialOrd<u64> for $name {
                fn partial_cmp(&self, other: &u64) -> Option<core::cmp::Ordering> {
                    self.raw.partial_cmp(other)
                }
            }

            impl PartialOrd<$name> for u64 {
                fn partial_cmp(&self, other: &$name) -> Option<core::cmp::Ordering> {
                    self.partial_cmp(&other.raw)
                }
            }
        )*
    };
}

define_types! {
    NumBits, "bits";
    NumBytes, "bytes";
    NumBlocks, "blocks";
}

impl NumBits {
    /// Convert the number of bits to a number of bytes rounding down.
    pub const fn to_bytes_floor(self) -> NumBytes {
        NumBytes::new(self.raw / 8)
    }

    /// Convert the number of bits to a number of bytes rounding up.
    pub const fn to_bytes_ceil(self) -> NumBytes {
        NumBytes::new(self.raw.div_ceil(8))
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for NumBits {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u64(self.raw)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for NumBits {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self::new(u64::deserialize(deserializer)?))
    }
}

impl core::fmt::Display for NumBits {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}bits", self.raw)
    }
}

/// Auxiliary macro for implementing an operator for a combination of bits and bytes.
macro_rules! impl_bit_byte_op {
    ($($trait:ident, $func:ident, $trait_assign:ident, $func_assign:ident;)*) => {
        $(
            impl core::ops::$trait<NumBytes> for NumBits {
                type Output = NumBits;

                fn $func(self, rhs: NumBytes) -> Self::Output {
                    self.$func(rhs.to_bits())
                }
            }

            impl core::ops::$trait<NumBits> for NumBytes {
                type Output = NumBits;

                fn $func(self, rhs: NumBits) -> Self::Output {
                    self.to_bits().$func(rhs)
                }
            }

            impl core::ops::$trait_assign<NumBytes> for NumBits {
                fn $func_assign(&mut self, rhs: NumBytes) {
                    self.$func_assign(rhs.to_bits());
                }
            }
        )*
    };
}

impl_bit_byte_op! {
    Add, add, AddAssign, add_assign;
    Sub, sub, SubAssign, sub_assign;
}

impl From<NumBytes> for NumBits {
    fn from(value: NumBytes) -> Self {
        value.to_bits()
    }
}

impl PartialEq<NumBits> for NumBytes {
    fn eq(&self, other: &NumBits) -> bool {
        self.to_bits() == *other
    }
}

impl PartialEq<NumBytes> for NumBits {
    fn eq(&self, other: &NumBytes) -> bool {
        *self == other.to_bits()
    }
}

impl PartialOrd<NumBits> for NumBytes {
    fn partial_cmp(&self, other: &NumBits) -> Option<core::cmp::Ordering> {
        self.to_bits().partial_cmp(other)
    }
}

impl PartialOrd<NumBytes> for NumBits {
    fn partial_cmp(&self, other: &NumBytes) -> Option<core::cmp::Ordering> {
        self.partial_cmp(&other.to_bits())
    }
}

impl NumBytes {
    /// Construct [`NumBytes`] from the provided raw number of bytes.
    pub const fn from_usize(n: usize) -> Self {
        if usize::BITS > u64::BITS && n > (u64::MAX as usize) {
            panic!("unable to convert `usize` to `NumBytes`, number exceeds 64 bits")
        } else {
            // We just made sure that the following conversion never truncates.
            Self::new(n as u64)
        }
    }

    /// Convert the number of bytes to a number of bits.
    pub const fn to_bits(self) -> NumBits {
        NumBits::new(
            self.raw
                .checked_mul(8)
                .expect("overflow converting bytes to bits"),
        )
    }

    /// Compute a number of blocks rounding down.
    pub const fn to_blocks_floor(self, block_size: NumBytes) -> NumBlocks {
        NumBlocks::new(
            self.raw
                .checked_div(block_size.raw)
                .expect("division by zero, block size is zero"),
        )
    }

    /// Compute a number of blocks rounding up.
    pub const fn to_blocks_ceil(self, block_size: NumBytes) -> NumBlocks {
        if block_size.raw == 0 {
            panic!("division by zero, block size is zero")
        }
        NumBlocks::new(self.raw.div_ceil(block_size.raw))
    }

    /// Align the number of bytes to the next multiple of the block size rounding down.
    pub const fn align_blocks_floor(self, block_size: NumBytes) -> NumBytes {
        self.to_blocks_floor(block_size).to_bytes(block_size)
    }

    /// Align the number of bytes to the next multiple of the block size rounding up.
    pub const fn align_blocks_ceil(self, block_size: NumBytes) -> NumBytes {
        self.to_blocks_ceil(block_size).to_bytes(block_size)
    }

    /// Splits the number into a whole and a fractional part based on the provided unit.
    pub const fn split_fractional(self, unit: ByteUnit) -> (u64, u64) {
        let whole = self.raw / unit.num_bytes().raw;
        let fractional = self.raw % unit.num_bytes().raw;
        (whole, fractional)
    }

    /// Unit to use when displaying the number.
    pub const fn display_unit(self) -> ByteUnit {
        let mut unit_idx = ByteUnit::UNITS.len() - 1;
        while unit_idx > 0 {
            if ByteUnit::UNITS[unit_idx].num_bytes().raw <= self.raw {
                break;
            }
            unit_idx -= 1;
        }
        ByteUnit::UNITS[unit_idx]
    }

    /// Parse a byte size string.
    pub const fn parse_str(s: &str) -> Result<NumBytes, NumBytesParseError> {
        Self::parse_ascii(s.as_bytes())
    }

    /// Parse a byte size ASCII string.
    pub const fn parse_ascii(mut buffer: &[u8]) -> Result<NumBytes, NumBytesParseError> {
        const fn expect_int(
            buffer: &mut &[u8],
            truncate: bool,
        ) -> Result<(u64, u128), NumBytesParseError> {
            let mut value = 0u64;
            let mut base = 1;
            while let Some((head, tail)) = buffer.split_first() {
                match *head {
                    b'0'..=b'9' => {
                        match value.checked_mul(10) {
                            Some(shifted) => {
                                let Some(new_value) = shifted.checked_add((*head - b'0') as u64)
                                else {
                                    if !truncate {
                                        return Err(NumBytesParseError::Overflow);
                                    } else {
                                        continue;
                                    }
                                };
                                value = new_value;
                                if value != 0 {
                                    base *= 10;
                                }
                            }
                            None if !truncate => {
                                return Err(NumBytesParseError::Overflow);
                            }
                            _ => { /* truncate */ }
                        };
                    }
                    b'_' => { /* skip underscores */ }
                    _ => break,
                }
                *buffer = tail;
            }
            if base > 1 {
                Ok((value, base))
            } else {
                // We expected an integer but there was none.
                Err(NumBytesParseError::Format)
            }
        }
        let (whole, _) = const_try!(expect_int(&mut buffer, false));
        let mut fractional = (0, 1);
        if let Some((b'.', tail)) = buffer.split_first() {
            buffer = tail;
            fractional = const_try!(expect_int(&mut buffer, true));
        }
        while let Some((b' ', tail)) = buffer.split_first() {
            buffer = tail;
        }
        let unit = if buffer.is_empty() {
            ByteUnit::Byte
        } else {
            match ByteUnit::parse_ascii(buffer) {
                Ok(unit) => unit,
                Err(_) => return Err(NumBytesParseError::Format),
            }
        };
        let Some(value) = whole.checked_mul(unit.num_bytes().raw) else {
            return Err(NumBytesParseError::Overflow);
        };
        let (mut fractional_value, mut fractional_base) = fractional;
        if fractional_value != 0 && !matches!(unit, ByteUnit::Byte) {
            let unit_divisor = unit.base10_fractional_divisor() as u128;
            // Strip insignificant digits.
            while fractional_base >= unit_divisor {
                fractional_value /= 10;
                fractional_base /= 10;
            }
            // We carry out the following operations with 128-bit integers to prevent
            // overflows in intermediate values of the calculation. We need to do those
            // calculations as `.5` means `1/2` of the unit's value.
            let fractional_value = fractional_value as u128 * unit_divisor / fractional_base;
            let unit_value = unit.num_bytes().raw as u128;
            let fractional_part = fractional_value * unit_value / unit_divisor;
            if let Some(value) = value.checked_add(fractional_part as u64) {
                return Ok(NumBytes::new(value));
            } else {
                return Err(NumBytesParseError::Overflow);
            }
        }
        Ok(NumBytes::new(value))
    }
}

impl core::fmt::Display for NumBytes {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use core::fmt::Write;
        if f.alternate() {
            return self.raw.fmt(f);
        }
        let unit = self.display_unit();
        let (whole, fractional) = self.split_fractional(unit);
        write!(f, "{}", whole)?;
        // Compute the desired precision limited by the unit.
        let precision = f
            .precision()
            .map(|p| p as u32)
            .unwrap_or(u32::MAX)
            .min(unit.base10_fractional_digits());
        let mut fractional_base = 10u64.pow(precision);
        // Convert the fractional part to base 10 (required for binary units). We carry
        // out the computation with 128-bit integers to prevent overflows. This will also
        // truncate the fractional part to the specified precision.
        let mut fractional_value = ((fractional as u128) * (fractional_base as u128)
            / (unit.num_bytes().raw as u128)) as u64;
        if f.precision().is_some() {
            f.write_char('.')?;
            write!(f, "{fractional_value:0p$}", p = precision as usize)?;
        } else if fractional_value != 0 {
            f.write_char('.')?;
            fractional_base /= 10;
            while fractional_base > 0 && fractional_value > 0 {
                let digit = fractional_value / fractional_base;
                write!(f, "{digit}")?;
                fractional_value %= fractional_base;
                fractional_base /= 10;
            }
        }
        f.write_str(unit.as_str())
    }
}

impl core::str::FromStr for NumBytes {
    type Err = NumBytesParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        NumBytes::parse_str(s)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for NumBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_u64(self.raw)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for NumBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = NumBytes;

            fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.write_str("byte size")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                NumBytes::parse_str(v)
                    .map_err(|_| E::invalid_value(serde::de::Unexpected::Str(v), &"byte size"))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(NumBytes::new(v))
            }
        }

        deserializer.deserialize_u64(Visitor)
    }
}

impl NumBlocks {
    /// Convert the number of blocks to a number of bytes.
    pub const fn to_bytes(self, block_size: NumBytes) -> NumBytes {
        NumBytes::new(
            self.raw
                .checked_mul(block_size.raw)
                .expect("overflow converting blocks to bytes"),
        )
    }
}

impl core::fmt::Display for NumBlocks {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}blocks", self.raw)
    }
}

/// Auxiliary macro for the definition of byte units.
macro_rules! define_units {
    ($($name:ident, $name_lower:ident, $suffix:literal, $suffix_lower:literal, $value:expr;)*) => {
        /// A _byte unit_ like Megabyte (MB) or Kibibyte (KiB).
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum ByteUnit {
            /// Base unit of one byte.
            Byte,
            $(
                #[doc = concat!("1 ", $suffix, " is `", stringify!($value), "` bytes.")]
                $name,
            )*
        }

        impl ByteUnit {
            /// Slice of all units.
            pub const UNITS: &[Self] = &[Self::Byte, $(Self::$name),*];

            /// String representation of the unit.
            pub const fn as_str(self) -> &'static str {
                match self {
                    Self::Byte => "B",
                    $(
                        Self::$name => $suffix,
                    )*
                }
            }

            /// Number of bytes the unit corresponds to.
            pub const fn num_bytes(self) -> NumBytes {
                NumBytes::new(match self {
                    Self::Byte => 1,
                    $(
                        Self::$name => $value,
                    )*
                })
            }

            /// Parse the string representation of the unit (lowercase).
            const fn parse_ascii_lowercase(ascii: &[u8]) -> Result<Self, ByteUnitParseError> {
                #![expect(non_upper_case_globals)]
                $(
                    const $name: &[u8] = $suffix_lower.as_bytes();
                )*
                match ascii {
                    b"b" => Ok(Self::Byte),
                    b"k" => Ok(Self::Kibibyte),
                    b"m" => Ok(Self::Mebibyte),
                    b"g" => Ok(Self::Gibibyte),
                    b"t" => Ok(Self::Tebibyte),
                    b"p" => Ok(Self::Pebibyte),
                    b"e" => Ok(Self::Exbibyte),
                    $(
                        $name => Ok(Self::$name),
                    )*
                    _ => Err(ByteUnitParseError)
                }
            }

            /// The maximal number of digits of the fractional part of the unit.
            ///
            /// For instance, the maximal number of digits of `kB` is three. Everything
            /// beyond three digits represents fractional bytes.
            const fn base10_fractional_digits(self) -> u32 {
                match self {
                    Self::Byte => 0,
                    $(
                        Self::$name => (Self::$name.num_bytes().raw * 10 - 1).ilog10(),
                    )*
                }
            }

            /// The base-10 divisor of the fractional part.
            const fn base10_fractional_divisor(self) -> u64 {
                10u64.pow(self.base10_fractional_digits())
            }
        }

        impl NumBytes {
            /// Construct [`NumBytes`] from the given number `n` of bytes.
            pub const fn bytes(n: u64) -> Self {
                Self::new(n)
            }

            $(
                #[doc = concat!("Construct [`NumBytes`] from the given number `n` of ", stringify!($name), "s.")]
                pub const fn $name_lower(n: u64) -> NumBytes {
                    NumBytes::new(n * $value)
                }
            )*
        }
    };
}

define_units! {
    Kilobyte, kilobytes, "kB", "kb", 10u64.pow(3);
    Kibibyte, kibibytes, "KiB", "kib", 1 << 10;
    Megabyte, megabytes, "MB", "mb", 10u64.pow(6);
    Mebibyte, mebibytes, "MiB", "mib", 1 << 20;
    Gigabyte, gigabytes, "GB", "gb", 10u64.pow(9);
    Gibibyte, gibibytes, "GiB", "gib", 1 << 30;
    Terabyte, terabytes, "TB", "tb", 10u64.pow(12);
    Tebibyte, tebibytes, "TiB", "tib", 1 << 40;
    Petabyte, petabytes, "PB", "pb", 10u64.pow(15);
    Pebibyte, pebibytes, "PiB", "pib", 1 << 50;
    Exabyte, exabytes, "EB", "eb", 10u64.pow(18);
    Exbibyte, exbibytes, "EiB", "eib", 1 << 60;
}

impl ByteUnit {
    /// Parse the string representation of a unit (case insensitive).
    pub const fn parse_ascii(ascii: &[u8]) -> Result<Self, ByteUnitParseError> {
        // We exploit the fact that we know the maximal length to make this work in `const`.
        match ascii {
            [b1] => Self::parse_ascii_lowercase(&[b1.to_ascii_lowercase()]),
            [b1, b2] => {
                Self::parse_ascii_lowercase(&[b1.to_ascii_lowercase(), b2.to_ascii_lowercase()])
            }
            [b1, b2, b3] => Self::parse_ascii_lowercase(&[
                b1.to_ascii_lowercase(),
                b2.to_ascii_lowercase(),
                b3.to_ascii_lowercase(),
            ]),
            _ => Err(ByteUnitParseError),
        }
    }

    /// Parse the string representation of the unit (case insensitive).
    pub const fn parse_str(string: &str) -> Result<Self, ByteUnitParseError> {
        Self::parse_ascii(string.as_bytes())
    }
}

impl core::str::FromStr for ByteUnit {
    type Err = ByteUnitParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s)
    }
}

impl From<ByteUnit> for NumBytes {
    fn from(value: ByteUnit) -> Self {
        value.num_bytes()
    }
}

/// Type which has a well-defined length in bytes.
pub trait ByteLen {
    /// Length in bytes of the value.
    fn byte_len(&self) -> NumBytes;
}

impl ByteLen for str {
    fn byte_len(&self) -> NumBytes {
        NumBytes::from_usize(self.len())
    }
}

impl ByteLen for [u8] {
    fn byte_len(&self) -> NumBytes {
        NumBytes::from_usize(self.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_byte_unit_order() {
        let mut unit_iter = ByteUnit::UNITS.iter().peekable();
        while let Some(this) = unit_iter.next() {
            if let Some(next) = unit_iter.peek() {
                assert!(this.num_bytes() < next.num_bytes())
            }
        }
    }

    #[test]
    pub fn test_byte_display_unit() {
        assert_eq!(NumBytes::new(0).display_unit(), ByteUnit::Byte);
        for unit in ByteUnit::UNITS {
            assert_eq!(unit.num_bytes().display_unit(), *unit);
        }
    }

    #[test]
    pub fn test_base10_fractional_digits() {
        assert_eq!(ByteUnit::Byte.base10_fractional_digits(), 0);
        assert_eq!(ByteUnit::Kilobyte.base10_fractional_digits(), 3);
        assert_eq!(ByteUnit::Kibibyte.base10_fractional_digits(), 4);
        assert_eq!(ByteUnit::Exabyte.base10_fractional_digits(), 18);
        assert_eq!(ByteUnit::Exbibyte.base10_fractional_digits(), 19);
    }
}
