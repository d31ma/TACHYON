use crate::CollisionPolicy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RouteContext {
    pub collision_policy: CollisionPolicy,
    pub static_exports: Vec<String>,
    pub response_exports: Vec<String>,
}

impl Default for RouteContext {
    fn default() -> Self {
        Self {
            collision_policy: CollisionPolicy::Error,
            static_exports: Vec::new(),
            response_exports: Vec::new(),
        }
    }
}
