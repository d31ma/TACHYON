use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HandlerResponseKind {
    #[serde(rename = "response")]
    Response,
}
