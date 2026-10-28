use super::{WarmOutput, WarmStop};
use crate::Failure;
use crate::handler::frame::HandlerFrame;
use serde_json::Value;

/// The frame a warm runner writes once, when it can read its first request.
pub(super) struct WarmReady;

impl WarmReady {
    const KIND: &'static str = "ready";

    pub(super) fn check(output: Option<WarmOutput>) -> Result<(), WarmStop> {
        Self::announced(output.unwrap_or(WarmOutput::Ended))
    }

    fn announced(output: WarmOutput) -> Result<(), WarmStop> {
        match output {
            WarmOutput::Frame(payload) => Self::ready(&payload),
            WarmOutput::Ended => Err(WarmStop::Ended),
            WarmOutput::Oversized | WarmOutput::Truncated => {
                Err(WarmStop::Broken(Self::unannounced()))
            }
        }
    }

    fn ready(payload: &[u8]) -> Result<(), WarmStop> {
        let parsed = serde_json::from_slice::<Value>(payload).ok();
        let kind = parsed
            .as_ref()
            .and_then(|value| value.get("kind"))
            .and_then(Value::as_str);
        (kind == Some(Self::KIND))
            .then_some(())
            .ok_or_else(|| WarmStop::Broken(Self::unannounced()))
    }

    fn unannounced() -> Failure {
        HandlerFrame::protocol_failure(
            2103,
            "A warm handler wrote a frame before it said it was ready.",
        )
    }
}
