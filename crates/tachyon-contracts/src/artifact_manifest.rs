use crate::{ArtifactContractVersions, ArtifactOutput, ArtifactTarget, ArtifactToolchain};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactManifest {
    pub contract_version: u8,
    pub release_version: String,
    pub commit: String,
    pub source_date_epoch: u64,
    pub target: ArtifactTarget,
    pub toolchains: Vec<ArtifactToolchain>,
    pub contracts: ArtifactContractVersions,
    pub outputs: Vec<ArtifactOutput>,
}
