use super::AdapterFiles;
use crate::handler::resources::ResourceAccess;
use command_group::AsyncGroupChild;
use tokio::process::{ChildStderr, ChildStdin, ChildStdout};

pub(super) struct StartedProcess {
    pub(super) child: AsyncGroupChild,
    pub(super) stdin: ChildStdin,
    pub(super) stdout: ChildStdout,
    pub(super) stderr: ChildStderr,
    pub(super) resource_access: Option<ResourceAccess>,
    pub(super) _adapter: AdapterFiles,
}
