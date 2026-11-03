use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HandlerRequestKind {
    #[serde(rename = "request")]
    Request,
}
