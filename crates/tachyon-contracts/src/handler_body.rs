use crate::HandlerBodyEncoding;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandlerBody {
    pub encoding: HandlerBodyEncoding,
    pub data: String,
}
