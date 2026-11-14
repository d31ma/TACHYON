use super::config::config_failure;
use crate::Failure;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct BlobLimits {
    #[serde(default)]
    max_bytes: Option<u64>,
    #[serde(default)]
    max_blob_bytes: Option<u64>,
}

impl BlobLimits {
    const DEFAULT_MAX_BYTES: u64 = 536_870_912;
    const DEFAULT_MAX_BLOB_BYTES: u64 = 67_108_864;
    const MOST_BYTES: u64 = 1_125_899_906_842_624;
    const REFUSAL: &'static str = "tac.storage.blobs needs maxBlobBytes from 1 to maxBytes, and maxBytes at most 1125899906842624 (1 PiB). Both count bytes (ADR 0134).";

    pub(crate) fn checked(self) -> Result<Self, Failure> {
        let ordered = [1, self.max_blob_bytes(), self.max_bytes(), Self::MOST_BYTES].is_sorted();
        ordered
            .then_some(self)
            .ok_or_else(|| config_failure(Self::REFUSAL))
    }

    fn max_bytes(self) -> u64 {
        self.max_bytes.unwrap_or(Self::DEFAULT_MAX_BYTES)
    }

    fn max_blob_bytes(self) -> u64 {
        let fallback = Self::DEFAULT_MAX_BLOB_BYTES.min(self.max_bytes());
        self.max_blob_bytes.unwrap_or(fallback)
    }

    pub(crate) fn content(self) -> String {
        format!("{} {}", self.max_bytes(), self.max_blob_bytes())
    }
}
