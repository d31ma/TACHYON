use crate::Failure;
use crate::failure::{diagnostic, source_span};
use serde::Deserialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct VendorProvenance {
    package: String,
    version: String,
    license: String,
    sha256: String,
}

impl VendorProvenance {
    const MAX_LABEL_BYTES: usize = 128;
    const SHA256_HEX_BYTES: usize = 64;
    const VERSION_PUNCTUATION: &'static [u8] = b".-_+";

    pub(crate) fn verify(&self, path: &str, bytes: &[u8]) -> Result<(), Failure> {
        self.validate_labels(path)?;
        self.validate_version(path)?;
        self.validate_hash(path)?;
        self.compare_hash(path, bytes)
    }

    fn validate_labels(&self, path: &str) -> Result<(), Failure> {
        let valid = [&self.package, &self.version, &self.license]
            .into_iter()
            .all(|value| Self::valid_label(value));
        Self::require(
            valid,
            path,
            "Vendor package, version and license must contain 1 to 128 printable bytes.",
        )
    }

    fn valid_label(value: &str) -> bool {
        !value.trim().is_empty()
            && value.len() <= Self::MAX_LABEL_BYTES
            && !value.chars().any(char::is_control)
    }

    fn validate_version(&self, path: &str) -> Result<(), Failure> {
        let version = self.version.strip_prefix('v').unwrap_or(&self.version);
        let valid = version.as_bytes().first().is_some_and(u8::is_ascii_digit)
            & version.bytes().all(Self::version_character);
        Self::require(
            valid,
            path,
            "Vendor version must be an exact pinned version without ranges, wildcards or latest tags.",
        )
    }

    fn version_character(byte: u8) -> bool {
        byte.is_ascii_alphanumeric() | Self::VERSION_PUNCTUATION.contains(&byte)
    }

    fn validate_hash(&self, path: &str) -> Result<(), Failure> {
        let valid = (self.sha256.len() == Self::SHA256_HEX_BYTES)
            & self.sha256.bytes().all(Self::hash_character);
        Self::require(
            valid,
            path,
            "Vendor sha256 must be exactly 64 lowercase hexadecimal characters.",
        )
    }

    fn hash_character(byte: u8) -> bool {
        byte.is_ascii_digit() | (b'a'..=b'f').contains(&byte)
    }

    fn compare_hash(&self, path: &str, bytes: &[u8]) -> Result<(), Failure> {
        let actual = format!("{:x}", Sha256::digest(bytes));
        Self::require(
            actual == self.sha256,
            path,
            "Captured vendor bytes do not match the declared sha256.",
        )
    }

    fn require(valid: bool, path: &str, message: &str) -> Result<(), Failure> {
        if !valid {
            return Err(Failure::one(diagnostic(
                1403,
                message,
                Some(String::from(
                    "Pin the exact vendor package, version, license and source-file SHA-256 in client/manifest.json.",
                )),
                source_span(path, 0, 0),
            )));
        }
        Ok(())
    }
}
