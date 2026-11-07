use crate::{HandlerCancel, HandlerRequest, HandlerResponse};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HandlerEnvelope {
    Request(HandlerRequest),
    Response(HandlerResponse),
    Cancel(HandlerCancel),
}
