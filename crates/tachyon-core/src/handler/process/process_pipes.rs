use command_group::AsyncGroupChild;
use tokio::process::{ChildStderr, ChildStdin, ChildStdout};

pub(super) struct ProcessPipes {
    pub(super) child: AsyncGroupChild,
    pub(super) stdin: ChildStdin,
    pub(super) stdout: ChildStdout,
    pub(super) stderr: ChildStderr,
}
