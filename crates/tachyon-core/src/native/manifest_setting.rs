use serde_json::Value;
use std::path::Path;

pub(super) struct ManifestSetting;

impl ManifestSetting {
    const MANIFEST: &'static str = "client/manifest.json";

    pub(super) fn read(snapshot_root: &Path, pointer: &str) -> Option<Value> {
        let bytes = std::fs::read(snapshot_root.join(Self::MANIFEST)).ok()?;
        let manifest: Value = serde_json::from_slice(&bytes).ok()?;
        manifest.pointer(pointer).cloned()
    }
}
