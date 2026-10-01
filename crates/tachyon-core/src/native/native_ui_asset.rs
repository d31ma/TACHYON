use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct NativeUiAsset {
    pub(super) path: String,
    pub(super) size: u64,
    pub(super) sha256: String,
}

impl NativeUiAsset {
    pub(super) fn is_stylesheet(&self) -> bool {
        self.path.ends_with(".css")
    }
}
