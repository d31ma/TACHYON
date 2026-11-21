use axum::http::HeaderMap;
use serde_json::Value;
use std::collections::BTreeMap;

pub(in crate::server) struct RouteRequest<'a> {
    pub parameters: &'a BTreeMap<String, String>,
    pub query: Option<&'a str>,
    pub headers: &'a HeaderMap,
    pub body: &'a [u8],
    pub identity: Option<&'a Value>,
}
