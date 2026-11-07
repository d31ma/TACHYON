use crate::handler::frame::FramePrefix;
use crate::handler::resources::PipedResources;
use command_group::AsyncGroupChild;
use tokio::process::{ChildStderr, ChildStdin, ChildStdout};

pub(super) struct ProcessPipes {
    pub(super) child: AsyncGroupChild,
    pub(super) stdin: ChildStdin,
    pub(super) stdout: ChildStdout,
    pub(super) stderr: ChildStderr,
    pub(super) resource_pipe: Option<PipedResources>,
    pub(super) style: FramePrefix,
}
