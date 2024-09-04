//! Compatibility layers for other update solutions.
//!
//! Enables the migration from other solutions to Rugpi.

#[cfg(feature = "compat-mender")]
pub mod mender;
#[cfg(feature = "compat-rauc")]
pub mod rauc;
