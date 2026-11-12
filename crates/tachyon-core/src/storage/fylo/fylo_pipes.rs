use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};

use super::FyloError;
use super::frame_answer::FrameAnswer;
use super::root_lock_wait::RootLockWait;
use crate::sorted_json::SortedJson;

pub(super) struct FyloPipes {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl FyloPipes {
    const MAX_REQUEST_BYTES: usize = 1024 * 1024;
    const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

    pub(super) fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        Self {
            stdin,
            stdout: BufReader::new(stdout),
        }
    }

    pub(super) async fn request_waiting(&mut self, request: &Value) -> Result<Value, FyloError> {
        let mut wait = RootLockWait::new();
        let mut answer = self.request(request).await;
        while Self::root_was_busy(&answer, &mut wait).await {
            answer = self.request(request).await;
        }
        answer
    }

    async fn root_was_busy(answer: &Result<Value, FyloError>, wait: &mut RootLockWait) -> bool {
        let root_locked = answer.as_ref().err().is_some_and(FyloError::root_locked);
        root_locked && wait.pause().await
    }

    pub(super) async fn request(&mut self, request: &Value) -> Result<Value, FyloError> {
        let mut line = SortedJson::text(request);
        if line.len() > Self::MAX_REQUEST_BYTES {
            return Err(FyloError::protocol("FYLO request exceeds 1 MiB."));
        }
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|_| FyloError::protocol("FYLO subprocess is not accepting requests."))?;
        let mut answer = String::new();
        let mut reader =
            tokio::io::AsyncReadExt::take(&mut self.stdout, Self::MAX_RESPONSE_BYTES as u64 + 1);
        let read = reader
            .read_line(&mut answer)
            .await
            .map_err(|_| FyloError::protocol("FYLO subprocess closed its output."))?;
        if read == 0 {
            return Err(FyloError::protocol("FYLO subprocess exited."));
        }
        if answer.len() > Self::MAX_RESPONSE_BYTES {
            return Err(FyloError::protocol("FYLO response exceeds 8 MiB."));
        }
        FrameAnswer::from_line(&answer)
    }
}
