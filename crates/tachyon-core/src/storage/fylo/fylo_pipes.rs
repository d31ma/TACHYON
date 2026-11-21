use std::sync::Arc;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};

use super::FyloError;
use super::fylo_input::FyloInput;
use super::root_lock_wait::RootLockWait;
use crate::sorted_json::SortedJson;
use crate::storage::StoreCall;

pub(super) struct FyloPipes {
    input: Arc<FyloInput>,
    stdout: BufReader<ChildStdout>,
}

impl FyloPipes {
    const MAX_REQUEST_BYTES: usize = 1024 * 1024;
    const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

    pub(super) fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        Self {
            input: Arc::new(FyloInput::new(stdin)),
            stdout: BufReader::new(stdout),
        }
    }

    pub(super) fn input(&self) -> Arc<FyloInput> {
        Arc::clone(&self.input)
    }

    pub(super) async fn drain(&mut self) {
        let mut ignored = tokio::io::sink();
        let _output_is_not_needed = tokio::io::copy(&mut self.stdout, &mut ignored).await;
    }

    pub(super) async fn request_waiting(
        &mut self,
        request: &Value,
        startup: bool,
    ) -> Result<Value, FyloError> {
        let mut wait = RootLockWait::new();
        let mut answer = self.round_trip(request, startup).await;
        while Self::root_was_busy(&answer, &mut wait).await {
            answer = self.round_trip(request, startup).await;
        }
        answer
    }

    async fn round_trip(&mut self, request: &Value, startup: bool) -> Result<Value, FyloError> {
        if startup {
            return StoreCall::startup(self.request(request)).await;
        }
        self.request(request).await
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
        self.input.write_line(line.as_bytes()).await?;
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
        let envelope: Value = serde_json::from_str(answer.trim_end())
            .map_err(|_| FyloError::protocol("FYLO returned a malformed frame."))?;
        if envelope["ok"] == json!(true) {
            return Ok(envelope["result"].clone());
        }
        Err(FyloError {
            code: envelope["error"]["code"]
                .as_str()
                .unwrap_or("EUNKNOWN")
                .to_owned(),
        })
    }
}
