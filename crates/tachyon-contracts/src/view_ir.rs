use crate::ViewNode;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ViewIr {
    pub contract_version: u8,
    pub source: String,
    pub root: ViewNode,
}

impl ViewIr {
    #[must_use]
    pub const fn v1(source: String, root: ViewNode) -> Self {
        Self {
            contract_version: 1,
            source,
            root,
        }
    }
}
