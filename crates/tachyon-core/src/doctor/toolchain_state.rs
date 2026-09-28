use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "state", content = "detail")]
pub enum ToolchainState {
    Ready(String),
    Incomplete(String),
    Missing,
}

impl ToolchainState {
    pub(crate) fn is_ready(&self) -> bool {
        self.ready()
    }

    fn ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }
}
