use crate::handler::HandlerSource;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct WarmKey {
    execution_path: PathBuf,
    interpreter: Vec<String>,
}

impl WarmKey {
    pub(super) fn of(source: &HandlerSource) -> Self {
        Self {
            execution_path: source.execution_path(),
            interpreter: source.interpreter().to_vec(),
        }
    }
}
