use crate::Failure;
use crate::handler::frame::{FrameLimits, HandlerFrame};
use crate::handler::topic_bus::TopicOutbox;
use serde_json::Value;
use std::sync::Arc;
use tachyon_contracts::{
    HandlerBody, HandlerBodyEncoding, HandlerHeaders, HandlerResponse, HandlerResponseKind,
};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DirectResponse {
    status: u16,
    #[serde(default)]
    headers: HandlerHeaders,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    topics: Vec<Value>,
}

impl DirectResponse {
    const PUBLICATION_STOPPED: &'static str = "Topic publication stopped before it finished.";

    pub(super) async fn parse(
        bytes: &[u8],
        request_id: &str,
        outbox: Option<Arc<TopicOutbox>>,
    ) -> Result<HandlerResponse, Failure> {
        let mut parsed = Self::read(bytes)?;
        let topics = std::mem::take(&mut parsed.topics);
        Self::publish(topics, outbox, request_id).await;
        Ok(parsed.into_response(request_id))
    }

    fn read(bytes: &[u8]) -> Result<Self, Failure> {
        Self::check_size(bytes)?;
        let text = std::str::from_utf8(bytes).map_err(|_| {
            HandlerFrame::protocol_failure(2004, "Direct handler output is not valid UTF-8.")
        })?;
        let trimmed = text.trim();
        let parsed: Self = serde_json::from_str(trimmed).map_err(|error| {
            let preview: String = trimmed.chars().take(200).collect();
            HandlerFrame::protocol_failure(
                2004,
                &format!(
                    "Direct handler must write one JSON object with a status: {error}. \
                     Wrote: {preview}"
                ),
            )
        })?;
        parsed.check_status()?;
        Ok(parsed)
    }

    async fn publish(topics: Vec<Value>, outbox: Option<Arc<TopicOutbox>>, request_id: &str) {
        if topics.is_empty() {
            return;
        }
        let published = tokio::task::spawn_blocking(move || Self::publish_all(&topics, outbox));
        let failure = published
            .await
            .unwrap_or(Err(Self::PUBLICATION_STOPPED))
            .err();
        if let Some(message) = failure {
            Self::print_topic_failure(request_id, message);
        }
    }

    fn publish_all(topics: &[Value], outbox: Option<Arc<TopicOutbox>>) -> Result<(), &'static str> {
        let outbox = outbox.ok_or("This handler has no topic models.")?;
        let failures: Vec<&'static str> = topics
            .iter()
            .filter_map(|command| outbox.publish(command).err())
            .collect();
        failures.first().map_or(Ok(()), |failure| Err(*failure))
    }

    fn print_topic_failure(request_id: &str, message: &str) {
        eprintln!(
            "{}",
            serde_json::json!({
                "event": "handler.topic_failed",
                "request_id": request_id,
                "message": message,
            })
        );
    }

    fn check_size(bytes: &[u8]) -> Result<(), Failure> {
        if bytes.len() > FrameLimits::MAX_BYTES {
            return Err(HandlerFrame::protocol_failure(
                2004,
                "Direct handler wrote more than the frame limit.",
            ));
        }
        Ok(())
    }

    fn check_status(&self) -> Result<(), Failure> {
        if !(100..=599).contains(&self.status) {
            return Err(HandlerFrame::protocol_failure(
                2004,
                "Direct handler status must be a valid HTTP status.",
            ));
        }
        Ok(())
    }

    fn into_response(self, request_id: &str) -> HandlerResponse {
        HandlerResponse {
            protocol_version: 1,
            kind: HandlerResponseKind::Response,
            request_id: String::from(request_id),
            status: self.status,
            headers: self.headers,
            body: self.body.map(|data| HandlerBody {
                encoding: HandlerBodyEncoding::Utf8,
                data,
            }),
            error: None,
        }
    }
}
