use axum::http::header::HeaderName;

pub(in crate::server) struct HeaderNames;

impl HeaderNames {
    pub(in crate::server) const CONTENT_SECURITY_POLICY: HeaderName =
        HeaderName::from_static("content-security-policy");
    pub(in crate::server) const LAST_EVENT_ID: HeaderName =
        HeaderName::from_static("last-event-id");
    pub(in crate::server) const X_FRAME_OPTIONS: HeaderName =
        HeaderName::from_static("x-frame-options");
    pub(in crate::server) const REQUEST_ID: HeaderName =
        HeaderName::from_static("x-tachyon-request-id");
}
