use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HandlerCancelKind {
    #[serde(rename = "cancel")]
    Cancel,
}
