use super::HeaderNames;
use axum::http::{HeaderMap, Uri};
use tachyon_contracts::HandlerRequest;

pub(in crate::server) struct ProtocolRequest;

impl ProtocolRequest {
    pub(in crate::server) fn copy_headers(request: &mut HandlerRequest, headers: &HeaderMap) {
        headers
            .iter()
            .filter_map(|(name, value)| value.to_str().ok().map(|value| (name, value)))
            .for_each(|(name, value)| {
                request
                    .headers
                    .entry(name.as_str().to_owned())
                    .or_default()
                    .push(value.to_owned());
            });
        request.headers.insert(
            HeaderNames::REQUEST_ID.as_str().to_owned(),
            vec![request.request_id.clone()],
        );
    }

    pub(in crate::server) fn target(uri: &Uri) -> Option<String> {
        uri.path_and_query()
            .map(|target| target.as_str().to_owned())
    }
}
