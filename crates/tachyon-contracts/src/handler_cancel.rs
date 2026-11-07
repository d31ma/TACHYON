use crate::HandlerCancelKind;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandlerCancel {
    pub protocol_version: u8,
    pub kind: HandlerCancelKind,
    pub request_id: String,
}

impl HandlerCancel {
    #[must_use]
    pub fn v1(request_id: impl Into<String>) -> Self {
        Self {
            protocol_version: 1,
            kind: HandlerCancelKind::Cancel,
            request_id: request_id.into(),
        }
    }
}
