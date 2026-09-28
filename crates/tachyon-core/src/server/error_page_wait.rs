use crate::ToolRestart;

#[derive(Debug)]
pub enum ErrorPageWait {
    SourcesChanged,
    ToolUpdated(ToolRestart),
}
