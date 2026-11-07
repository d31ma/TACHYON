use super::ProtocolRequest;
use super::controller_dispatch::{RouteArguments, RouteRequest, RouteVerbs};
use crate::Failure;
use crate::failure::diagnostic;
use crate::handler::{HandlerCancellation, HandlerSource, HandlerSupervisor};
use crate::routing::{RouteMatch, match_route};
use crate::yon::plan::Route;
use axum::http::{HeaderMap, HeaderName, HeaderValue, Uri};
use tachyon_contracts::{HandlerRequest, HandlerResponse};

pub struct RouteInvocation<'a> {
    source: &'a HandlerSource,
    supervisor: &'a HandlerSupervisor,
}

impl<'a> RouteInvocation<'a> {
    const INVALID_REQUEST: u16 = 2005;

    #[must_use]
    pub const fn new(source: &'a HandlerSource, supervisor: &'a HandlerSupervisor) -> Self {
        Self { source, supervisor }
    }

    pub async fn invoke(
        &self,
        request: HandlerRequest,
        target: Option<&str>,
        cancellation: &HandlerCancellation,
    ) -> Result<HandlerResponse, Failure> {
        let annotated = !self.source.routes().is_empty();
        let route = annotated
            .then(|| self.route(&request, target))
            .transpose()?
            .flatten();
        let bound = self.bound(request, route)?;
        self.answer(bound, cancellation).await
    }

    async fn answer(
        &self,
        bound: Result<HandlerRequest, HandlerResponse>,
        cancellation: &HandlerCancellation,
    ) -> Result<HandlerResponse, Failure> {
        match bound {
            Ok(request) => self.run(&request, cancellation).await,
            Err(refusal) => Ok(refusal),
        }
    }

    async fn run(
        &self,
        request: &HandlerRequest,
        cancellation: &HandlerCancellation,
    ) -> Result<HandlerResponse, Failure> {
        self.supervisor
            .invoke(self.source, request, cancellation)
            .await
    }

    fn route(
        &self,
        request: &HandlerRequest,
        target: Option<&str>,
    ) -> Result<Option<(&'a Route, RouteMatch, Uri)>, Failure> {
        let uri = Self::target_uri(target.unwrap_or(request.route.as_str()))?;
        let routes = self.source.routes();
        let patterns = routes.iter().filter_map(|route| route.pattern.as_deref());
        let matched =
            match_route(patterns, uri.path()).ok_or_else(|| Self::unanswered(uri.path()))?;
        let wanted = RouteVerbs::wanted(request.method);
        let route = routes.iter().find(|route| {
            let pattern = route.pattern.as_deref() == Some(matched.route.as_str());
            [route.verb == wanted, pattern] == [true, true]
        });
        Ok(route.map(|route| (route, matched, uri)))
    }

    fn bound(
        &self,
        request: HandlerRequest,
        route: Option<(&Route, RouteMatch, Uri)>,
    ) -> Result<Result<HandlerRequest, HandlerResponse>, Failure> {
        match route {
            None => Ok(Ok(request)),
            Some(route) => self.bind(request, route),
        }
    }

    fn bind(
        &self,
        mut request: HandlerRequest,
        (route, matched, uri): (&Route, RouteMatch, Uri),
    ) -> Result<Result<HandlerRequest, HandlerResponse>, Failure> {
        let headers = Self::header_map(&request)?;
        let body = request.body.as_ref().map_or("", |body| body.data.as_str());
        let route_request = RouteRequest {
            parameters: &matched.parameters,
            query: uri.query(),
            headers: &headers,
            body: body.as_bytes(),
        };
        let arguments = RouteArguments::build(route, &route_request, self.source.models());
        let request_id = request.request_id.clone();
        request.route = matched.route;
        request.parameters = matched.parameters;
        request.target = ProtocolRequest::target(&uri);
        request.headers.clear();
        ProtocolRequest::copy_headers(&mut request, &headers);
        Ok(arguments
            .map(|arguments| HandlerRequest {
                arguments: Some(arguments),
                ..request
            })
            .map_err(|rejection| rejection.handler_response(&request_id)))
    }

    fn target_uri(target: &str) -> Result<Uri, Failure> {
        target
            .starts_with('/')
            .then_some(target)
            .and_then(|target| target.parse().ok())
            .ok_or_else(Self::invalid_target)
    }

    fn header_map(request: &HandlerRequest) -> Result<HeaderMap, Failure> {
        let mut headers = HeaderMap::new();
        request
            .headers
            .iter()
            .flat_map(|(name, values)| values.iter().map(move |value| (name, value)))
            .try_for_each(|(name, value)| Self::append_header(&mut headers, name, value))?;
        Ok(headers)
    }

    fn append_header(headers: &mut HeaderMap, name: &str, value: &str) -> Result<(), Failure> {
        let header_name =
            HeaderName::from_bytes(name.as_bytes()).map_err(|_| Self::invalid_header(name))?;
        let header_value = HeaderValue::from_str(value).map_err(|_| Self::invalid_header(name))?;
        headers.append(header_name, header_value);
        Ok(())
    }

    fn invalid_target() -> Failure {
        Failure::one(diagnostic(
            Self::INVALID_REQUEST,
            "The request target is not a path with an optional query.",
            Some(String::from(
                "Pass a target such as /items/42?limit=3, starting with '/'.",
            )),
            None,
        ))
    }

    fn unanswered(path: &str) -> Failure {
        Failure::one(diagnostic(
            Self::INVALID_REQUEST,
            format!("No route of this controller answers the path '{path}'."),
            Some(String::from(
                "Pass --target with a path that one of the controller's routes answers.",
            )),
            None,
        ))
    }

    fn invalid_header(name: &str) -> Failure {
        Failure::one(diagnostic(
            Self::INVALID_REQUEST,
            format!("The header '{name}' is not a valid HTTP header."),
            Some(String::from(
                "Use a header name and value that HTTP allows.",
            )),
            None,
        ))
    }
}
