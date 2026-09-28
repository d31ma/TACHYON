use crate::{NativeNode, NativeTarget};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeUi {
    pub contract_version: u8,
    pub target: NativeTarget,
    pub root: NativeNode,
}

impl NativeUi {
    #[must_use]
    pub const fn v1(target: NativeTarget, root: NativeNode) -> Self {
        Self {
            contract_version: 1,
            target,
            root,
        }
    }
}
