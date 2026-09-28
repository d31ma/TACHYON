use super::{HandlerEvent, ProcessFailure, StreamFrame, StreamReadOutcome};
use crate::Failure;
use serde_json::Value;
use tokio::process::ChildStdout;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub(super) struct EventReader {
    stdout: ChildStdout,
    events: mpsc::Sender<HandlerEvent>,
    request_id: String,
    sent: usize,
    sent_bytes: usize,
}

impl EventReader {
    pub(super) fn spawn(
        stdout: ChildStdout,
        events: mpsc::Sender<HandlerEvent>,
        request_id: String,
    ) -> JoinHandle<Result<StreamReadOutcome, Failure>> {
        let reader = Self {
            stdout,
            events,
            request_id,
            sent: 0,
            sent_bytes: 0,
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
        let payload = StreamFrame::read(&mut self.stdout).await?;
        self.handle_payload(payload).await
    }

    async fn handle_payload(
        &mut self,
        payload: Option<Vec<u8>>,
    ) -> Result<Option<StreamReadOutcome>, Failure> {
        match payload {
            None => Ok(Some(StreamReadOutcome::Complete)),
            Some(payload) => self.forward_frame(&payload).await,
        }
    }

    async fn forward_frame(
        &mut self,
        payload: &[u8],
    ) -> Result<Option<StreamReadOutcome>, Failure> {
        let frame: Value = serde_json::from_slice(payload)
            .map_err(|_| ProcessFailure::stream("A streamed frame is not valid JSON."))?;
        let data = Self::event_data(&frame, &self.request_id)?;
        Self::check_event_size(data)?;
        self.count_bytes(data.len())?;
        self.count_event()?;
        let event = HandlerEvent {
            data: String::from(data),
        };
        let sent = self.events.send(event).await;
        Ok(sent.err().map(|_| StreamReadOutcome::SubscriberClosed))
    }

    fn event_data<'frame>(frame: &'frame Value, request_id: &str) -> Result<&'frame str, Failure> {
        Self::check_owner(frame, request_id)?;
        Self::check_no_error(frame)?;
        Self::check_event_envelope(frame)?;
        frame
            .pointer("/body/data")
            .and_then(Value::as_str)
            .ok_or_else(|| ProcessFailure::stream("A streamed event has no UTF-8 body."))
    }

    fn check_owner(frame: &Value, request_id: &str) -> Result<(), Failure> {
        let protocol_version = frame.get("protocol_version").and_then(Value::as_u64);
        let frame_request_id = frame.get("request_id").and_then(Value::as_str);
        if protocol_version != Some(1) || frame_request_id != Some(request_id) {
            return Err(ProcessFailure::stream(
                "A streamed frame violates Handler Protocol v1.",
            ));
        }
        Ok(())
    }

    fn check_no_error(frame: &Value) -> Result<(), Failure> {
        if frame.get("error").is_some_and(|error| !error.is_null()) {
            return Err(ProcessFailure::stream(
                "The handler ended its stream with an error.",
            ));
        }
        Ok(())
    }

    fn check_event_envelope(frame: &Value) -> Result<(), Failure> {
        let kind = frame.get("kind").and_then(Value::as_str);
        let encoding = frame.pointer("/body/encoding").and_then(Value::as_str);
        if kind != Some("event") || encoding != Some("utf8") {
            return Err(ProcessFailure::stream(
                "A streamed event has an invalid envelope.",
            ));
        }
        Ok(())
    }

    fn check_event_size(data: &str) -> Result<(), Failure> {
        if data.len() > HandlerEvent::MAX_BYTES {
            return Err(ProcessFailure::stream(
                "A streamed event exceeds the 256 KiB delivery limit.",
            ));
        }
        Ok(())
    }

    fn count_bytes(&mut self, length: usize) -> Result<(), Failure> {
        self.sent_bytes = self.sent_bytes.saturating_add(length);
        if self.sent_bytes > HandlerEvent::MAX_TOTAL_BYTES_PER_STREAM {
            return Err(ProcessFailure::stream(
                "A streamed handler exceeded the 64 MiB aggregate delivery limit.",
            ));
        }
        Ok(())
    }

    fn count_event(&mut self) -> Result<(), Failure> {
        self.sent += 1;
        if self.sent > HandlerEvent::MAX_EVENTS_PER_STREAM {
            return Err(ProcessFailure::stream(
                "A streamed handler exceeded the 100,000 event limit.",
            ));
        }
        Ok(())
    }
}
