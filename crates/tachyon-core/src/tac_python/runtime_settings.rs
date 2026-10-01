use super::runtime_assets::RuntimeAssets;
use crate::Failure;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RuntimeSettings {
    pub(super) runtime: String,
    pub(super) profile: String,
}

impl RuntimeSettings {
    pub(super) fn read(value: &Value) -> Result<Self, Failure> {
        serde_json::from_value(value.clone()).map_err(|error| {
            RuntimeAssets::failure(
                "client/manifest.json",
                &format!("invalid tac.python settings: {error}"),
            )
        })
    }
}
