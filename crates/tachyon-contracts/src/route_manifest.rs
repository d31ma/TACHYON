use crate::RouteEntry;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteManifest {
    pub contract_version: u8,
    pub routes: Vec<RouteEntry>,
}

impl RouteManifest {
    #[must_use]
    pub const fn v1(routes: Vec<RouteEntry>) -> Self {
        Self {
            contract_version: 1,
            routes,
        }
    }
}
