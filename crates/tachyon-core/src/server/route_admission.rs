use axum::body::Body;
use axum::http::Response;
use serde_json::Value;

pub(in crate::server) enum RouteAdmission {
    Refused(Response<Body>),
    Admitted(Option<Value>),
}

impl RouteAdmission {
    pub(in crate::server) fn new(refusal: Option<Response<Body>>, identity: Option<Value>) -> Self {
        refusal.map_or(Self::Admitted(identity), Self::Refused)
    }
}
