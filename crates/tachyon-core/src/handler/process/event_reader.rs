use super::{HandlerEvent, StreamEvents, StreamFrame, StreamReadOutcome};
use crate::Failure;
use crate::handler::frame::FramePrefix;
use tokio::process::ChildStdout;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub(super) struct EventReader {
    stdout: ChildStdout,
    style: FramePrefix,
    events: StreamEvents,
}

impl EventReader {
    pub(super) fn spawn(
        stdout: ChildStdout,
        events: mpsc::Sender<HandlerEvent>,
        (request_id, style): (String, FramePrefix),
    ) -> JoinHandle<Result<StreamReadOutcome, Failure>> {
        let reader = Self {
            stdout,
            style,
            events: StreamEvents::new(events, request_id),
        };
        tokio::spawn(reader.read_all())
    }

    async fn read_all(mut self) -> Result<StreamReadOutcome, Failure> {
        let mut finished = None;
        while finished.is_none() {
            finished = self.read_next().await?;
        }
        Ok(finished.unwrap_or(StreamReadOutcome::Complete))
    }

    async fn read_next(&mut self) -> Result<Option<StreamReadOutcome>, Failure> {
        let payload = StreamFrame::read(&mut self.stdout, self.style).await?;
        self.handle_payload(payload).await
    }

    async fn handle_payload(
        &mut self,
        payload: Option<Vec<u8>>,
    ) -> Result<Option<StreamReadOutcome>, Failure> {
        match payload {
            None => Ok(Some(StreamReadOutcome::Complete)),
            Some(payload) => self.events.forward(&payload).await,
        }
    }
}
