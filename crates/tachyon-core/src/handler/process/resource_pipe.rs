use super::Drained;
use crate::handler::frame::FramePrefix;
use crate::handler::resources::PipedResources;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{ChildStdin, ChildStdout};

pub(super) struct ResourcePipe {
    pub(super) resources: PipedResources,
    pub(super) stdin: ChildStdin,
    pub(super) limit: usize,
    pub(super) style: FramePrefix,
}

impl ResourcePipe {
    pub(super) async fn serve(mut self, request: Vec<u8>, mut stdout: ChildStdout) -> Drained {
        let mut drained = Drained {
            bytes: Vec::new(),
            overflow: false,
            read_error: false,
        };
        self.send(&self.style.reframed(&request)).await;
        drained.bytes = self.relay(&mut stdout, &mut drained).await;
        Self::refuse_trailing_output(&mut stdout, &mut drained).await;
        drained
    }

    async fn relay(&mut self, stdout: &mut ChildStdout, drained: &mut Drained) -> Vec<u8> {
        let mut frame = self.next_frame(stdout, drained).await.unwrap_or_default();
        while let Some(command) = PipedResources::command(&frame) {
            let answer = self.resources.answer(&command).await;
            self.send(&[self.style.encode(answer.len()), answer].concat())
                .await;
            frame = self.next_frame(stdout, drained).await.unwrap_or_default();
        }
        frame
    }

    async fn send(&mut self, bytes: &[u8]) {
        let _sent = self.stdin.write_all(bytes).await;
        let _flushed = self.stdin.flush().await;
    }

    async fn next_frame(&self, stdout: &mut ChildStdout, drained: &mut Drained) -> Option<Vec<u8>> {
        let mut prefix = vec![0_u8; self.style.width()];
        stdout.read_exact(&mut prefix).await.ok()?;
        let length = self.style.decode(&prefix).unwrap_or(usize::MAX);
        drained.overflow = length > self.limit;
        let mut frame = vec![0_u8; length.min(self.limit)];
        let read = stdout.read_exact(&mut frame).await;
        drained.read_error = read.is_err();
        [!drained.overflow, read.is_ok()]
            .iter()
            .all(|kept| *kept)
            .then_some(frame)
    }

    async fn refuse_trailing_output(stdout: &mut ChildStdout, drained: &mut Drained) {
        let mut byte = [0_u8; 1];
        let trailing = stdout.read(&mut byte).await;
        drained.overflow |= matches!(trailing, Ok(1..));
    }
}
