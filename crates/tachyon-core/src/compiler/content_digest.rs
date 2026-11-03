use sha2::{Digest, Sha256};

pub(crate) struct ContentDigest;

impl ContentDigest {
    const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

    pub(super) fn sha256_bytes(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Self::hex(hasher.finalize())
    }

    pub(crate) fn hex(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .flat_map(|byte| {
                [
                    Self::HEX_DIGITS[usize::from(byte >> 4)],
                    Self::HEX_DIGITS[usize::from(byte & 0x0f)],
                ]
            })
            .map(char::from)
            .collect()
    }

    pub(super) fn route_input_sha(
        source: &[u8],
        component_digest: &str,
        dependency_digest: &str,
    ) -> String {
        let mut hasher = Sha256::new();
        hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
        hasher.update([0]);
        hasher.update(component_digest.as_bytes());
        hasher.update([0]);
        hasher.update(dependency_digest.as_bytes());
        hasher.update([0]);
        hasher.update(source);
        Self::hex(hasher.finalize())
    }
}
