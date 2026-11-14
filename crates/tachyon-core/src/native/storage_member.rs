use super::blob_limits::BlobLimits;
use crate::Failure;
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StorageMember {
    #[serde(default)]
    blobs: Option<BlobLimits>,
}

impl StorageMember {
    pub(crate) fn blob_limits(self) -> Result<BlobLimits, Failure> {
        self.blobs.unwrap_or_default().checked()
    }
}
