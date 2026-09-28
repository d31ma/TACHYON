use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewSourceMapping {
    pub generated_start: u64,
    pub generated_end: u64,
    pub source: String,
    pub source_start: u64,
    pub source_end: u64,
}
