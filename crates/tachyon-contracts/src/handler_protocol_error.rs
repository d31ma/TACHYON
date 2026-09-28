use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandlerProtocolError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}
