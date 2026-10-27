use super::WarmOutput;
use crate::handler::frame::{FrameLimits, FramePrefix};
use tokio::io::AsyncReadExt;
use tokio::process::ChildStdout;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub(super) struct WarmFrames {
    stdout: ChildStdout,
    style: FramePrefix,
    sender: mpsc::Sender<WarmOutput>,
}

impl WarmFrames {
    const WAITING_FRAMES: usize = 1;

    pub(super) fn spawn(
        stdout: ChildStdout,
        style: FramePrefix,
    ) -> (mpsc::Receiver<WarmOutput>, JoinHandle<()>) {
        let (sender, receiver) = mpsc::channel(Self::WAITING_FRAMES);
        let frames = Self {
            stdout,
            style,
            sender,
        };
        (receiver, tokio::spawn(frames.read_all()))
    }

    async fn read_all(mut self) {
        let mut open = true;
        while open {
            open = self.forward_next().await;
        }
    }

    async fn forward_next(&mut self) -> bool {
        let output = self.read_one().await;
        let more = Self::continues(&output);
        let delivered = self.sender.send(output).await.is_ok();
        [more, delivered] == [true, true]
    }

    fn continues(output: &WarmOutput) -> bool {
        matches!(output, WarmOutput::Frame(_))
    }

    async fn read_one(&mut self) -> WarmOutput {
        let mut prefix = vec![0_u8; self.style.width()];
        let first = self.stdout.read(&mut prefix[..1]).await.unwrap_or(0);
        if first == 0 {
            return WarmOutput::Ended;
        }
        let length = self.rest_of_prefix(&mut prefix).await;
        self.payload_for(length).await
    }

    async fn rest_of_prefix(&mut self, prefix: &mut [u8]) -> Option<usize> {
        let read = self.stdout.read_exact(&mut prefix[1..]).await;
        read.ok().and_then(|_| self.style.decode(prefix))
    }

    async fn payload_for(&mut self, length: Option<usize>) -> WarmOutput {
        let checked = length.ok_or(WarmOutput::Truncated).and_then(Self::fitting);
        self.checked_payload(checked).await
    }

    async fn checked_payload(&mut self, length: Result<usize, WarmOutput>) -> WarmOutput {
        match length {
            Ok(length) => self.payload(length).await,
            Err(output) => output,
        }
    }

    fn fitting(length: usize) -> Result<usize, WarmOutput> {
        (length <= FrameLimits::MAX_BYTES)
            .then_some(length)
            .ok_or(WarmOutput::Oversized)
    }

    async fn payload(&mut self, length: usize) -> WarmOutput {
        let mut payload = vec![0_u8; length];
        let read = self.stdout.read_exact(&mut payload).await;
        read.map_or(WarmOutput::Truncated, |_| WarmOutput::Frame(payload))
    }
}
