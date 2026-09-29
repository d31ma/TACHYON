use serde_json::{Map, Value};

#[derive(Clone, Debug)]
pub(crate) struct PlannedMethod {
    pub(crate) owner: String,
    pub(crate) name: String,
    pub(crate) line: usize,
    pub(crate) parameters: Vec<String>,
    pub(crate) metadata: Map<String, Value>,
    pub(crate) declared: bool,
    pub(crate) asynchronous: bool,
    pub(crate) mapped_argument: Option<usize>,
}
