use crate::{HttpMethod, RouteContext, RouteHandler, RouteKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteEntry {
    pub route: String,
    pub parameters: Vec<String>,
    pub kind: RouteKind,
    pub methods: Vec<HttpMethod>,
    pub view: Option<String>,
    pub handlers: Vec<RouteHandler>,
    pub context: RouteContext,
}
