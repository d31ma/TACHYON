use super::SocketInput;
use super::input_step::InputStep;
use tokio::io::AsyncWriteExt as _;
use tokio::process::ChildStdin;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub(super) struct SocketWriter {
    stdin: ChildStdin,
    input: mpsc::Receiver<SocketInput>,
    request_id: String,
}

impl SocketWriter {
    pub(super) fn spawn(
        stdin: ChildStdin,
        input: mpsc::Receiver<SocketInput>,
        request_id: String,
    ) -> JoinHandle<()> {
        let writer = Self {
            stdin,
            input,
            request_id,
        };
        tokio::spawn(writer.write_all_inputs())
    }

    async fn write_all_inputs(mut self) {
        let step = self.write_until_stopped().await;
        self.close_when_input_ended(step).await;
    }

    async fn write_until_stopped(&mut self) -> InputStep {
        let mut step = InputStep::Open;
        while step == InputStep::Open {
            step = self.write_next_input().await;
        }
        step
    }

    async fn write_next_input(&mut self) -> InputStep {
        let message = self.input.recv().await;
        let sent = self.send_input(message.as_ref()).await;
        Self::step_after(message.as_ref(), sent)
    }

    async fn send_input(&mut self, message: Option<&SocketInput>) -> bool {
        match message {
            Some(message) => self
                .send_frame(&message.frame(&self.request_id))
                .await
                .is_ok(),
            None => false,
        }
    }

    async fn send_frame(&mut self, frame: &[u8]) -> std::io::Result<()> {
        self.stdin.write_all(frame).await?;
        self.stdin.flush().await
    }

    fn step_after(message: Option<&SocketInput>, sent: bool) -> InputStep {
        match (message, sent) {
            (None, _) => InputStep::Ended,
            (Some(SocketInput::Close), _) | (Some(_), false) => InputStep::Stopped,
            (Some(_), true) => InputStep::Open,
        }
    }

    async fn close_when_input_ended(&mut self, step: InputStep) {
        if step == InputStep::Ended {
            let frame = SocketInput::Close.frame(&self.request_id);
            let _written = self.stdin.write_all(&frame).await;
            let _flushed = self.stdin.flush().await;
        }
    }
}
