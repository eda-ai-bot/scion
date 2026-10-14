//! Content-addressed identifiers. All are 32-byte BLAKE3 digests.

use std::fmt;
use std::str::FromStr;

/// Error returned when parsing an id from hex.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdParseError {
    #[error("expected 64 hex characters, got {0}")]
    Length(usize),
    #[error("invalid hex: {0}")]
    Hex(String),
}

macro_rules! define_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(transparent))]
        pub struct $name([u8; 32]);

        impl $name {
            /// Length of the id in bytes.
            pub const LEN: usize = 32;

            /// Construct from raw digest bytes.
            #[must_use]
            pub const fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            /// Hash arbitrary bytes with BLAKE3.
            #[must_use]
            pub fn hash(data: &[u8]) -> Self {
                Self(*blake3::hash(data).as_bytes())
            }

            /// The raw digest bytes.
            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }

            /// Lowercase hex representation.
            #[must_use]
            pub fn to_hex(&self) -> String {
                hex::encode(self.0)
            }

            /// Parse from a lowercase or uppercase hex string.
            pub fn from_hex(s: &str) -> Result<Self, IdParseError> {
                if s.len() != 64 {
                    return Err(IdParseError::Length(s.len()));
                }
                let mut out = [0u8; 32];
                hex::decode_to_slice(s, &mut out).map_err(|e| IdParseError::Hex(e.to_string()))?;
                Ok(Self(out))
            }

            /// Try to construct from a byte slice of exactly 32 bytes.
            pub fn try_from_slice(bytes: &[u8]) -> Result<Self, IdParseError> {
                let arr: [u8; 32] =
                    bytes.try_into().map_err(|_| IdParseError::Length(bytes.len()))?;
                Ok(Self(arr))
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), &self.to_hex()[..12])
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.to_hex())
            }
        }

        impl FromStr for $name {
            type Err = IdParseError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::from_hex(s)
            }
        }

        impl AsRef<[u8]> for $name {
            fn as_ref(&self) -> &[u8] {
                &self.0
            }
        }

        impl From<[u8; 32]> for $name {
            fn from(b: [u8; 32]) -> Self {
                Self(b)
            }
        }
    };
}

define_id! {
    /// Identifier of a changeset: `blake3(canonical(changeset))`.
    ChangesetId
}
define_id! {
    /// Identifier of a tree: `blake3(canonical(tree))`.
    TreeId
}
define_id! {
    /// Identifier of a blob: `blake3(raw bytes)`.
    BlobId
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let id = BlobId::hash(b"hello");
        let parsed: BlobId = id.to_hex().parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn blake3_known_vector() {
        // blake3("") — from the BLAKE3 reference test vectors.
        assert_eq!(
            BlobId::hash(b"").to_hex(),
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }

    #[test]
    fn rejects_bad_length() {
        assert_eq!(BlobId::from_hex("abc"), Err(IdParseError::Length(3)));
    }
}
