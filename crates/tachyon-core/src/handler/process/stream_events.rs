use super::{HandlerEvent, ProcessFailure, StreamReadOutcome};
use crate::Failure;
use serde_json::Value;
use tokio::sync::mpsc;

pub(super) struct StreamEvents {
    events: mpsc::Sender<HandlerEvent>,
    request_id: String,
    sent: usize,
    sent_bytes: usize,
}

impl StreamEvents {
    pub(super) fn new(events: mpsc::Sender<HandlerEvent>, request_id: String) -> Self {
        Self {
            events,
            request_id,
            sent: 0,
            sent_bytes: 0,
        }
    }

    pub(super) fn ends(payload: &[u8], request_id: &str) -> bool {
        let frame = serde_json::from_slice::<Value>(payload).unwrap_or(Value::Null);
        let kind = frame.get("kind").and_then(Value::as_str);
        let owner = frame.get("request_id").and_then(Value::as_str);
        let version = frame.get("protocol_version").and_then(Value::as_u64);
        [kind == Some("end"), owner == Some(request_id), version == Some(1)] == [true, true, true]
    }

    pub(super) async fn forward(
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
