use super::AdapterFiles;
use crate::handler::HandlerSource;
use command_group::AsyncGroupChild;
use tokio::process::{ChildStderr, ChildStdin, ChildStdout};

pub(super) struct WarmStart {
    pub(super) source: HandlerSource,
    pub(super) adapter: AdapterFiles,
    pub(super) child: AsyncGroupChild,
    pub(super) stdin: ChildStdin,
    pub(super) stdout: ChildStdout,
    pub(super) stderr: ChildStderr,
    pub(super) stderr_limit: usize,
}
