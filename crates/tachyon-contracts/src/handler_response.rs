use crate::{HandlerBody, HandlerHeaders, HandlerProtocolError, HandlerResponseKind};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandlerResponse {
    pub protocol_version: u8,
    pub kind: HandlerResponseKind,
    pub request_id: String,
    pub status: u16,
    pub headers: HandlerHeaders,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<HandlerBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<HandlerProtocolError>,
}

impl HandlerResponse {
    #[must_use]
    pub fn success(
        request_id: impl Into<String>,
        status: u16,
        headers: HandlerHeaders,
        body: HandlerBody,
    ) -> Self {
        Self {
            protocol_version: 1,
            kind: HandlerResponseKind::Response,
            request_id: request_id.into(),
            status,
            headers,
            body: Some(body),
            error: None,
        }
    }

    #[must_use]
    pub fn error(request_id: impl Into<String>, status: u16, error: HandlerProtocolError) -> Self {
        Self {
            protocol_version: 1,
            kind: HandlerResponseKind::Response,
            request_id: request_id.into(),
            status,
            headers: HandlerHeaders::new(),
            body: None,
            error: Some(error),
        }
    }
}
