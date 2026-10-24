use super::super::StreamFrame;
use super::SocketOutput;
use crate::Failure;
use crate::handler::frame::FramePrefix;
use serde_json::Value;
use tokio::process::ChildStdout;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub(super) struct SocketReader {
    stdout: ChildStdout,
    output: mpsc::Sender<SocketOutput>,
    request_id: String,
    style: FramePrefix,
}

impl SocketReader {
    pub(super) fn spawn(
        stdout: ChildStdout,
        output: mpsc::Sender<SocketOutput>,
        (request_id, style): (String, FramePrefix),
    ) -> JoinHandle<Result<(), Failure>> {
        let reader = Self {
            stdout,
            output,
            request_id,
            style,
        };
        tokio::spawn(reader.forward_all_outputs())
    }

    async fn forward_all_outputs(mut self) -> Result<(), Failure> {
        while let Some(payload) = StreamFrame::read(&mut self.stdout, self.style).await? {
            self.forward_output(&payload).await?;
        }
        Ok(())
    }

    async fn forward_output(&self, payload: &[u8]) -> Result<(), Failure> {
        let frame: Value = serde_json::from_slice(payload)
            .map_err(|_| SocketOutput::failure("A socket frame is not valid JSON."))?;
        let message = SocketOutput::read(&frame, &self.request_id)?;
        let _delivered = self.output.send(message).await;
        Ok(())
    }
}
