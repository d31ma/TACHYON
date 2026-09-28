use crate::HotUpdateKind;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HotUpdate {
    pub contract_version: u8,
    pub sequence: u64,
    pub kind: HotUpdateKind,
    pub build_id: Option<String>,
    pub paths: Vec<String>,
    pub boundaries: Vec<String>,
    pub diagnostics: Option<tachyon_diagnostics::DiagnosticReport>,
}

impl HotUpdate {
    #[must_use]
    pub const fn v1(
        sequence: u64,
        kind: HotUpdateKind,
        build_id: Option<String>,
        paths: Vec<String>,
        boundaries: Vec<String>,
        diagnostics: Option<tachyon_diagnostics::DiagnosticReport>,
    ) -> Self {
        Self {
            contract_version: 1,
            sequence,
            kind,
            build_id,
            paths,
            boundaries,
            diagnostics,
        }
    }
}
