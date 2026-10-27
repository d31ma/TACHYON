use crate::{HandlerBody, HandlerHeaders, HandlerRequestKind, HandlerResources, HttpMethod};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandlerRequest {
    pub protocol_version: u8,
    pub kind: HandlerRequestKind,
    pub request_id: String,
    pub operation: String,
    pub route: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    pub method: HttpMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline_ms: Option<u64>,
    pub headers: HandlerHeaders,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<HandlerBody>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<HandlerResources>,
}

impl HandlerRequest {
    #[must_use]
    pub fn route(
        request_id: impl Into<String>,
        route: impl Into<String>,
        method: HttpMethod,
    ) -> Self {
        Self {
            protocol_version: 1,
            kind: HandlerRequestKind::Request,
            request_id: request_id.into(),
            operation: String::from("route.invoke"),
            route: route.into(),
            target: None,
            method,
            deadline_ms: None,
            headers: HandlerHeaders::new(),
            parameters: BTreeMap::new(),
            body: None,
            arguments: None,
            resources: None,
        }
    }
}
