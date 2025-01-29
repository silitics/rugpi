//! Cryptographic hash functionality.
//!
//! ```rust
//! # use std::str::FromStr;
//! # use rugix_hashes::{HashAlgorithm, HashDigest};
//! #
//! let expected = "sha256:dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f";
//!
//! // Parse the string representation of the expected hash.
//! let digest = HashDigest::from_str(expected).unwrap();
//! assert_eq!(digest.algorithm(), HashAlgorithm::Sha256);
//! assert_eq!(digest.to_string(), expected);
//!
//! // Compute a digest.
//! let mut hasher = digest.algorithm().hasher();
//! hasher.update(b"Hello, World!");
//! assert_eq!(hasher.finalize(), digest);
//! ```

use std::fmt::Write;
use std::str::FromStr;

use errors::InvalidDigestError;
use sha2::Digest;

#[cfg(feature = "serde")]
mod serde;

/// Error types.
pub mod errors {
    /// Invalid hash algorithm.
    #[derive(Debug)]
    pub struct InvalidAlgorithmError;

    impl std::fmt::Display for InvalidAlgorithmError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("invalid hash algorithm")
        }
    }

    impl std::error::Error for InvalidAlgorithmError {}

    /// Invalid hash digest.
    #[derive(Debug)]
    pub struct InvalidDigestError(pub(crate) &'static str);

    impl std::fmt::Display for InvalidDigestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(self.0)
        }
    }

    impl std::error::Error for InvalidDigestError {}

    impl From<InvalidAlgorithmError> for InvalidDigestError {
        fn from(_: InvalidAlgorithmError) -> Self {
            InvalidDigestError("unknown algorithm")
        }
    }
}

macro_rules! define_hashes {
    ($($variant:ident, $name:literal, $hasher:ty;)*) => {
        /// Hash algorithms supported by Rugix.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[non_exhaustive]
        pub enum HashAlgorithm {
            $(
                $variant,
            )*
        }

        impl HashAlgorithm {
            /// Name of the algorithm.
            pub fn name(self) -> &'static str {
                match self {
                    $(
                        Self::$variant => $name,
                    )*
                }
            }

            /// Create a fresh hasher.
            pub fn hasher(self) -> Hasher {
                match self {
                    $(
                        Self::$variant => Hasher {
                            algorithm: self,
                            inner: HasherInner::$variant(<$hasher>::new())
                        },
                    )*
                }
            }

            /// Size of the hash.
            pub fn hash_size(self) -> usize {
                match self {
                    $(
                        Self::$variant => <$hasher>::output_size(),
                    )*
                }
            }
        }

        impl FromStr for HashAlgorithm {
            type Err = errors::InvalidAlgorithmError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $(
                        $name => Ok(Self::$variant),
                    )*
                    _ => Err(errors::InvalidAlgorithmError),
                }
            }
        }

        /// Internal hasher representation.
        #[derive(Debug, Clone)]
        enum HasherInner {
            $(
                $variant($hasher),
            )*
        }

        impl HasherInner {
            /// Update the hash with the given bytes.
            fn update(&mut self, bytes: &[u8]) {
                match self {
                    $(
                        HasherInner::$variant(hasher) => hasher.update(bytes),
                    )*
                }
            }

            /// Finalize the hash.
            fn finalize(self) -> Box<[u8]> {
                match self {
                    $(
                        HasherInner::$variant(hasher) => hasher.finalize().as_slice().into(),
                    )*
                }
            }
        }
    };
}

define_hashes! {
    Sha256, "sha256", sha2::Sha256;
    Sha512_256, "sha512-256", sha2::Sha512_256;
    Sha512, "sha512", sha2::Sha512;
}

impl HashAlgorithm {
    /// Hash the given bytes.
    pub fn hash(self, bytes: &[u8]) -> HashDigest {
        let mut hasher = self.hasher();
        hasher.update(bytes);
        hasher.finalize()
    }
}

/// Hasher for computing hashes.
#[derive(Debug, Clone)]
pub struct Hasher {
    algorithm: HashAlgorithm,
    inner: HasherInner,
}

impl Hasher {
    /// Algorithm of the hasher.
    pub fn algorithm(&self) -> HashAlgorithm {
        self.algorithm
    }

    /// Update the hasher with the given bytes.
    pub fn update(&mut self, bytes: &[u8]) {
        self.inner.update(bytes);
    }

    /// Finalize the hasher and return the digest.
    pub fn finalize(self) -> HashDigest {
        HashDigest {
            algorithm: self.algorithm,
            raw: self.inner.finalize(),
        }
    }
}

/// Hash digest.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HashDigest {
    /// Algorithm used to compute the digest.
    algorithm: HashAlgorithm,
    /// Raw digest.
    raw: Box<[u8]>,
}

impl HashDigest {
    /// Create [`HashDigest`] from the provided algorithm and raw digest.
    pub fn new(algorithm: HashAlgorithm, raw: &[u8]) -> Result<Self, InvalidDigestError> {
        if raw.len() != algorithm.hash_size() {
            return Err(InvalidDigestError("invalid digest size"));
        }
        Ok(Self::new_unchecked(algorithm, raw))
    }

    /// Create [`HashDigest`] from the provided algorithm and raw digest without checking
    /// the digest's length.
    pub fn new_unchecked(algorithm: HashAlgorithm, raw: &[u8]) -> Self {
        Self {
            algorithm,
            raw: raw.into(),
        }
    }

    /// Algorithm used to compute the digest.
    pub fn algorithm(&self) -> HashAlgorithm {
        self.algorithm
    }

    /// Raw digest.
    pub fn raw(&self) -> &[u8] {
        &self.raw
    }
}

impl std::fmt::Display for HashDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.algorithm.name())?;
        f.write_char(':')?;
        f.write_str(&hex::encode(&self.raw))?;
        Ok(())
    }
}

impl FromStr for HashDigest {
    type Err = errors::InvalidDigestError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let Some((algorithm, digest)) = s.split_once(':') else {
            return Err(errors::InvalidDigestError("missing `:` delimiter"));
        };
        let algorithm = HashAlgorithm::from_str(algorithm)?;
        let Ok(digest) = hex::decode(digest) else {
            return Err(errors::InvalidDigestError("digest is not a hex string"));
        };
        if digest.len() != algorithm.hash_size() {
            return Err(errors::InvalidDigestError("invalid digest size"));
        };
        Ok(Self {
            algorithm,
            raw: digest.into(),
        })
    }
}
