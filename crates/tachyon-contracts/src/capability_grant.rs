use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityGrant {
    pub name: String,
    pub scope: Vec<String>,
    pub reason: String,
    pub targets: Vec<String>,
}
