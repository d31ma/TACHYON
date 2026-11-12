use std::path::Path;
use std::process::Stdio;

use serde_json::Value;
use tokio::process::Child;

use super::FyloError;
use super::fylo_pipes::FyloPipes;
use super::interruption::Interruption;
use super::request_watch::RequestWatch;

pub(super) struct FyloProcess {
    child: Child,
    pipes: FyloPipes,
    interruption: Option<Interruption>,
}

impl FyloProcess {
    pub(super) fn spawn(root: &Path) -> Result<Self, FyloError> {
        let mut child = tokio::process::Command::new("fylo")
            .args(["exec", "--loop", "--root"])
            .arg(root)
            .args(["--max-request-bytes", "1048576", "--max-response-bytes", "8388608"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| {
                FyloError::protocol(
                    "The fylo binary is not on PATH; install FYLO (fylo.del.ma) to use this datasource.",
                )
            })?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| FyloError::protocol("FYLO stdin is unavailable."))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| FyloError::protocol("FYLO stdout is unavailable."))?;
        Ok(Self {
            child,
            pipes: FyloPipes::new(stdin, stdout),
            interruption: None,
        })
    }

    pub(super) async fn handshake(&mut self, request: &Value) -> Result<Value, FyloError> {
        self.pipes.request(request).await
    }

    pub(super) async fn request(
        &mut self,
        operation: &str,
        request: &Value,
    ) -> Result<Value, FyloError> {
        let watch = RequestWatch::start(&mut self.child, &mut self.interruption, operation);
        let answer = self.pipes.request_waiting(request).await;
        watch.finish(&answer);
        answer
    }

    pub(super) async fn close(mut self) {
        let _already_exited = self.child.kill().await;
    }

    pub(super) fn must_stop(&self) -> bool {
        self.interruption.is_some()
    }

    pub(super) fn interrupted_operation(&self) -> Option<&str> {
        self.interruption.as_ref().map(Interruption::operation)
    }

    pub(super) async fn wait_for_exit(&mut self) {
        let _already_exited = self.child.start_kill();
        let _exit_status_is_not_needed = self.child.wait().await;
        self.interruption
            .iter()
            .for_each(Interruption::report_stopped);
    }
}
