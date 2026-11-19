//! Content hashes: SHA-256, written as lowercase hex.
//!
//! Structured values are hashed over their RFC 8785 canonical JSON, so the hash does not depend on
//! key order or number formatting. Files are hashed over their bytes.

use std::{fmt, fs, io, path::Path};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, thiserror::Error)]
pub enum HashError {
    #[error("{0:?} is not a SHA-256 digest in lowercase hex")]
    Malformed(String),
    #[error("value has no canonical JSON form: {0}")]
    Canonical(#[from] serde_json::Error),
}

/// A value's RFC 8785 canonical JSON. A file holding exactly these bytes has the hash
/// [`Sha256Hex::of_json`] gives the value.
pub fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>, HashError> {
    Ok(serde_json_canonicalizer::to_vec(value)?)
}

/// A SHA-256 digest in lowercase hex.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Sha256Hex(String);

impl Sha256Hex {
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(hex::encode(Sha256::digest(bytes)))
    }

    pub fn of_file(path: &Path) -> io::Result<Self> {
        Ok(Self::of_bytes(&fs::read(path)?))
    }

    /// The hash of a value's RFC 8785 canonical JSON.
    pub fn of_json<T: Serialize>(value: &T) -> Result<Self, HashError> {
        Ok(Self::of_bytes(&canonical_json(value)?))
    }

    pub fn parse(text: &str) -> Result<Self, HashError> {
        let hex_digit = |c: char| c.is_ascii_digit() || ('a'..='f').contains(&c);
        if text.len() == 64 && text.chars().all(hex_digit) {
            Ok(Self(text.to_owned()))
        } else {
            Err(HashError::Malformed(text.to_owned()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Sha256Hex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Sha256Hex {
    type Error = HashError;
    fn try_from(text: String) -> Result<Self, HashError> {
        Self::parse(&text)
    }
}

impl From<Sha256Hex> for String {
    fn from(hash: Sha256Hex) -> String {
        hash.0
    }
}

#[cfg(test)]
mod tests {
    use crate::hash::*;

    #[test]
    fn bytes_hash_to_the_known_digest() {
        assert_eq!(
            Sha256Hex::of_bytes(b"abc").as_str(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn json_hash_ignores_key_order() {
        let a: serde_json::Value = serde_json::from_str(r#"{"b": 1, "a": [1.0, 2]}"#).unwrap();
        let b: serde_json::Value = serde_json::from_str(r#"{"a": [1, 2.0], "b": 1}"#).unwrap();
        assert_eq!(Sha256Hex::of_json(&a).unwrap(), Sha256Hex::of_json(&b).unwrap());
        let c: serde_json::Value = serde_json::from_str(r#"{"a": [1, 2], "b": 2}"#).unwrap();
        assert_ne!(Sha256Hex::of_json(&a).unwrap(), Sha256Hex::of_json(&c).unwrap());
    }

    #[test]
    fn a_file_of_canonical_json_hashes_as_its_value() {
        let value: serde_json::Value = serde_json::from_str(r#"{"b": 1, "a": [1.5, 2]}"#).unwrap();
        let bytes = canonical_json(&value).unwrap();
        assert_eq!(bytes, br#"{"a":[1.5,2],"b":1}"#);
        assert_eq!(Sha256Hex::of_bytes(&bytes), Sha256Hex::of_json(&value).unwrap());
    }

    #[test]
    fn malformed_digests_are_rejected() {
        assert!(Sha256Hex::parse("abc").is_err());
        assert!(Sha256Hex::parse(&"A".repeat(64)).is_err());
    }
}
