use serde_json::Value;

pub(super) struct ScrollPage {
    pub(super) documents: Vec<Value>,
    pub(super) next_cursor: Option<String>,
}
