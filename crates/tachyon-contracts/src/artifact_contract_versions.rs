use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactContractVersions {
    pub artifact_manifest: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_manifest: Option<u8>,
    pub diagnostics: u8,
    pub handler_protocol: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_ui: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_host: Option<u8>,
    pub route_manifest: u8,
    pub view_ir: u8,
}

impl Default for ArtifactContractVersions {
    fn default() -> Self {
        Self {
            artifact_manifest: 1,
            capability_manifest: None,
            diagnostics: 1,
            handler_protocol: 1,
            native_ui: None,
            native_host: Some(3),
            route_manifest: 1,
            view_ir: 1,
        }
    }
}
