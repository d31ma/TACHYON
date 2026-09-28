use crate::ViewSourceMapping;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewSourceMap {
    pub contract_version: u8,
    pub output: String,
    pub sources: Vec<String>,
    pub mappings: Vec<ViewSourceMapping>,
}

impl ViewSourceMap {
    #[must_use]
    pub const fn v1(
        output: String,
        sources: Vec<String>,
        mappings: Vec<ViewSourceMapping>,
    ) -> Self {
        Self {
            contract_version: 1,
            output,
            sources,
            mappings,
        }
    }
}
