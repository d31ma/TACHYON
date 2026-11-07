use std::time::Instant;

use serde_json::Value;

pub(super) struct ScrollSnapshot {
    pub(super) scope: String,
    pub(super) documents: Vec<Value>,
    pub(super) expires_at: Instant,
}
