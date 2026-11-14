use super::{BuiltInCase, DispatchRoute, ProtocolRequest};
use crate::handler::HandlerSource;
use crate::routing::RouteMethod;
use crate::yon::built_in_error::BuiltInError;
use crate::yon::plan::{ParameterSource, RouteParameter};
use axum::body::Body;
use axum::http::{HeaderMap, Request, Uri};
use tachyon_contracts::{HandlerRequest, HttpMethod};

pub(in crate::server) struct BuiltInRequest {
    uri: Uri,
    headers: HeaderMap,
}

impl BuiltInRequest {
    const CREDENTIAL_HEADERS: [&'static str; 3] =
        ["authorization", "proxy-authorization", "cookie"];
    const DEADLINE_MILLISECONDS: u64 = 10_000;

    pub(in crate::server) fn of(request: &Request<Body>) -> Self {
        Self {
            uri: request.uri().clone(),
            headers: request.headers().clone(),
        }
    }

    pub(in crate::server) fn protocol(
        &self,
        entry: &DispatchRoute,
        case: &BuiltInCase,
        reference: &str,
        identity: Option<&HandlerSource>,
    ) -> HandlerRequest {
        let method = RouteMethod::parse(&entry.mapping.verb).unwrap_or(HttpMethod::Get);
        let mut request = HandlerRequest::route(reference, BuiltInError::ROUTE, method);
        ProtocolRequest::copy_headers(&mut request, &self.safe_headers(identity));
        request.target = ProtocolRequest::target(&self.uri);
        request.arguments = Some(vec![case.argument(reference)]);
        request.deadline_ms = Some(Self::DEADLINE_MILLISECONDS);
        request
    }

    fn safe_headers(&self, identity: Option<&HandlerSource>) -> HeaderMap {
        let mut headers = self.headers.clone();
        for name in Self::hidden_names(identity) {
            headers.remove(name.as_str());
        }
        headers
    }

    fn hidden_names(identity: Option<&HandlerSource>) -> Vec<String> {
        let read = identity
            .map(Self::identity_header_names)
            .unwrap_or_default();
        let credentials = Self::CREDENTIAL_HEADERS
            .iter()
            .map(|name| String::from(*name));
        credentials.chain(read).collect()
    }

    fn identity_header_names(identity: &HandlerSource) -> Vec<String> {
        identity
            .routes()
            .iter()
            .flat_map(|route| &route.parameters)
            .filter_map(Self::header_name)
            .collect()
    }

    fn header_name(parameter: &RouteParameter) -> Option<String> {
        let is_header = parameter.source == ParameterSource::Header;
        is_header.then(|| parameter.name.to_ascii_lowercase())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::BuiltInRequest;
    use axum::body::Body;
    use axum::http::Request;

    #[test]
    fn credential_headers_never_reach_the_error_handler() {
        let request = Request::builder()
            .uri("/orders?page=2")
            .header("authorization", "Bearer secret")
            .header("proxy-authorization", "Basic secret")
            .header("cookie", "session=secret")
            .header("accept-language", "en")
            .body(Body::empty())
            .expect("request");
        let headers = BuiltInRequest::of(&request).safe_headers(None);
        let names: Vec<&str> = headers.keys().map(|name| name.as_str()).collect();
        assert_eq!(names, ["accept-language"]);
    }
}
