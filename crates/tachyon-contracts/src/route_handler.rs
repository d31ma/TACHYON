use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteHandler {
    pub source: String,
    pub language: String,
    pub runtime: String,
}
