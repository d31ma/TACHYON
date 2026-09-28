use crate::CapabilityGrant;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityManifest {
    pub contract_version: u8,
    pub application_id: String,
    pub default_policy: String,
    pub remote_content_bridge: bool,
    pub capabilities: Vec<CapabilityGrant>,
}

impl CapabilityManifest {
    #[must_use]
    pub fn deny_all(application_id: String) -> Self {
        Self {
            contract_version: 1,
            application_id,
            default_policy: String::from("deny"),
            remote_content_bridge: false,
            capabilities: Vec::new(),
        }
    }
}
