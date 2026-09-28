use crate::Failure;
use crate::handler::frame::{FrameLimits, HandlerFrame};
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
}

impl DirectResponse {
    pub(super) fn parse(bytes: &[u8], request_id: &str) -> Result<HandlerResponse, Failure> {
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
        Ok(parsed.into_response(request_id))
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
