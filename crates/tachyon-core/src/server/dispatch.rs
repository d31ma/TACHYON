use super::controller_dispatch::{
    RouteAnswer, RouteArguments, RouteRequest, RouteResult, RouteVerbs,
};
use super::socket_dispatch::{SocketHub, SocketSession};
use super::{
    DispatchRoute, ForwardedHeaders, HandlerEventBridge, HostGuard, HotUpdateHub, InvocationLog,
    Middleware, MiddlewareOutcome, ProducerTasks, ProtocolRequest, RouteGuard, ServerResponse,
    StaticFiles, TopicHub, TopicStream,
};
use crate::Failure;
use crate::handler::{HandlerCancellation, HandlerSource, HandlerSupervisor};
use crate::routing::{RouteMatch, RouteMethod, match_route};
use axum::Router;
use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::header::CONTENT_LENGTH;
use axum::http::request::Parts;
use axum::http::response::Parts as ResponseParts;
use axum::http::{HeaderMap, Method, Request, Response, StatusCode, Uri};
use std::sync::Arc;
use tachyon_contracts::{
    HandlerBody, HandlerBodyEncoding, HandlerRequest, HandlerResponse, HttpMethod,
};
use tokio_util::sync::CancellationToken;
use tower_http::services::ServeDir;

#[derive(Clone)]
pub(in crate::server) struct Dispatch {
    pub(in crate::server) routes: Arc<Vec<DispatchRoute>>,
    pub(in crate::server) page_routes: Arc<Vec<String>>,
    pub(in crate::server) supervisor: Arc<HandlerSupervisor>,
    pub(in crate::server) files: ServeDir,
    pub(in crate::server) hot_updates: HotUpdateHub,
    pub(in crate::server) watch: bool,
    pub(in crate::server) middleware: Option<Arc<HandlerSource>>,
    pub(in crate::server) identity: Option<Arc<HandlerSource>>,
    pub(in crate::server) topic_hub: TopicHub,
    pub(in crate::server) sockets: SocketHub,
    pub(in crate::server) cancellation: CancellationToken,
    pub(in crate::server) producers: ProducerTasks,
}

impl Dispatch {
    const MAX_REQUEST_BODY_BYTES: usize = 1024 * 1024;
    const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
    pub(in crate::server) const SSE_BRIDGE_BYTES: usize = 16 * 1024;
    pub(in crate::server) const STREAM_EVENT_QUEUE: usize = 2;
    const PAGE_METHODS: [HttpMethod; 2] = [HttpMethod::Get, HttpMethod::Head];

    pub(in crate::server) fn into_router(
        self,
        hosts: &crate::safety::DeclaredHosts,
        guard: HostGuard,
        edge: ForwardedHeaders,
    ) -> Router {
        let routes = Router::new().fallback(axum::routing::any(Self::handle).with_state(self));
        ServerResponse::defensive_headers(edge.protect(guard.protect(routes)), hosts)
    }

    pub(in crate::server) async fn invoke(
        &self,
        source: &HandlerSource,
        request: &HandlerRequest,
    ) -> Result<HandlerResponse, Failure> {
        self.invoke_until_stopped(source, request).await
    }

    async fn handle(State(state): State<Self>, request: Request<Body>) -> Response<Body> {
        let uri = request.uri().clone();
        let method = Self::protocol_method(request.method()).unwrap_or(HttpMethod::Get);
        let request_headers = request.headers().clone();
        let response = state.serve(request).await;
        state
            .after_middleware(response, &uri, method, &request_headers)
            .await
    }

    async fn after_middleware(
        &self,
        response: Response<Body>,
        uri: &Uri,
        method: HttpMethod,
        request_headers: &HeaderMap,
    ) -> Response<Body> {
        match self.middleware.clone() {
            None => response,
            Some(middleware) => {
                Middleware::after(self, &middleware, uri, method, request_headers, response).await
            }
        }
    }

    const fn protocol_method(method: &Method) -> Option<HttpMethod> {
        match *method {
            Method::DELETE => Some(HttpMethod::Delete),
            Method::GET => Some(HttpMethod::Get),
            Method::HEAD => Some(HttpMethod::Head),
            Method::OPTIONS => Some(HttpMethod::Options),
            Method::PATCH => Some(HttpMethod::Patch),
            Method::POST => Some(HttpMethod::Post),
            Method::PUT => Some(HttpMethod::Put),
            _ => None,
        }
    }

    async fn serve(&self, request: Request<Body>) -> Response<Body> {
        let path = request.uri().path().to_owned();
        let watched = self.watch_response(&path, &request);
        self.serve_unless_watched(watched, &path, request).await
    }

    fn watch_response(&self, path: &str, request: &Request<Body>) -> Option<Response<Body>> {
        if path == HotUpdateHub::LIVE_ENDPOINT && self.watch {
            Some(ServerResponse::text(
                StatusCode::OK,
                &self.hot_updates.sequence().to_string(),
            ))
        } else if path == HotUpdateHub::HOT_ENDPOINT && self.watch {
            Some(
                self.hot_updates
                    .stream(request, &self.producers, &self.cancellation),
            )
        } else if path == HotUpdateHub::DIAGNOSTICS_ENDPOINT && self.watch {
            Some(self.hot_updates.diagnostics_response())
        } else {
            None
        }
    }

    async fn serve_unless_watched(
        &self,
        watched: Option<Response<Body>>,
        path: &str,
        request: Request<Body>,
    ) -> Response<Body> {
        match watched {
            Some(response) => response,
            None => self.serve_with_middleware(path, request).await,
        }
    }

    async fn serve_with_middleware(&self, path: &str, request: Request<Body>) -> Response<Body> {
        let method = Self::protocol_method(request.method()).unwrap_or(HttpMethod::Get);
        let outcome = self
            .before_middleware(request.uri(), request.headers(), method)
            .await;
        self.serve_unless_responded(outcome, path, request).await
    }

    async fn before_middleware(
        &self,
        uri: &Uri,
        headers: &HeaderMap,
        method: HttpMethod,
    ) -> MiddlewareOutcome {
        match self.middleware.clone() {
            None => MiddlewareOutcome::Continue,
            Some(middleware) => Middleware::before(self, &middleware, uri, method, headers).await,
        }
    }

    async fn serve_unless_responded(
        &self,
        outcome: MiddlewareOutcome,
        path: &str,
        request: Request<Body>,
    ) -> Response<Body> {
        match outcome {
            MiddlewareOutcome::Respond(response) => response,
            MiddlewareOutcome::Continue => self.serve_path(path, request).await,
        }
    }

    async fn serve_path(&self, path: &str, request: Request<Body>) -> Response<Body> {
        let answer = self.framework_answer(path, &request);
        self.serve_unless_answered(answer, path, request).await
    }

    fn framework_answer(&self, path: &str, request: &Request<Body>) -> Option<Response<Body>> {
        if let Some(topic) = path.strip_prefix(TopicHub::ENDPOINT) {
            Some(TopicStream::open(self, topic, request))
        } else if path == HotUpdateHub::LIVE_SCRIPT_ENDPOINT && self.watch {
            Some(ServerResponse::live_script())
        } else if HotUpdateHub::is_endpoint(path) {
            Some(ServerResponse::text(StatusCode::NOT_FOUND, "Not found."))
        } else {
            None
        }
    }

    async fn serve_unless_answered(
        &self,
        answer: Option<Response<Body>>,
        path: &str,
        request: Request<Body>,
    ) -> Response<Body> {
        match answer {
            Some(response) => response,
            None => self.serve_route_or_file(path, request).await,
        }
    }

    async fn serve_route_or_file(&self, path: &str, request: Request<Body>) -> Response<Body> {
        let found = self.find_dispatch_route(path);
        self.serve_found(found, request).await
    }

    async fn serve_found(
        &self,
        found: Option<(Vec<&DispatchRoute>, RouteMatch)>,
        request: Request<Body>,
    ) -> Response<Body> {
        match found {
            None => self.clone().serve_static(request).await,
            Some((entries, matched)) => self.serve_mapped_route(&entries, matched, request).await,
        }
    }

    fn find_dispatch_route(&self, path: &str) -> Option<(Vec<&DispatchRoute>, RouteMatch)> {
        let patterns = self.routes.iter().map(|route| route.route.as_str());
        let matched = match_route(patterns, path)?;
        let entries: Vec<&DispatchRoute> = self
            .routes
            .iter()
            .filter(|route| route.route == matched.route)
            .collect();
        let found = !entries.is_empty();
        found.then_some((entries, matched))
    }

    async fn serve_mapped_route(
        &self,
        entries: &[&DispatchRoute],
        matched: RouteMatch,
        request: Request<Body>,
    ) -> Response<Body> {
        let socket = entries
            .iter()
            .find(|entry| entry.mapping.socket.is_some())
            .copied();
        self.serve_socket_or_http(socket, entries, matched, request)
            .await
    }

    async fn serve_socket_or_http(
        &self,
        socket: Option<&DispatchRoute>,
        entries: &[&DispatchRoute],
        matched: RouteMatch,
        request: Request<Body>,
    ) -> Response<Body> {
        match socket {
            Some(entry) => self.open_socket(entry, matched, request).await,
            None => self.serve_http_route(entries, matched, request).await,
        }
    }

    async fn open_socket(
        &self,
        entry: &DispatchRoute,
        matched: RouteMatch,
        request: Request<Body>,
    ) -> Response<Body> {
        let refusal = RouteGuard::refusal(
            self,
            &entry.mapping.access,
            request.uri(),
            request.headers(),
        )
        .await;
        match refusal {
            Some(response) => response,
            None => SocketSession::open(self, entry, matched, request).await,
        }
    }

    async fn serve_http_route(
        &self,
        entries: &[&DispatchRoute],
        matched: RouteMatch,
        request: Request<Body>,
    ) -> Response<Body> {
        let method = Self::protocol_method(request.method());
        self.serve_known_method(method, entries, matched, request)
            .await
    }

    async fn serve_known_method(
        &self,
        method: Option<HttpMethod>,
        entries: &[&DispatchRoute],
        matched: RouteMatch,
        request: Request<Body>,
    ) -> Response<Body> {
        match method {
            None => ServerResponse::text(StatusCode::METHOD_NOT_ALLOWED, "Unsupported method."),
            Some(method) => self.serve_method(method, entries, matched, request).await,
        }
    }

    async fn serve_method(
        &self,
        method: HttpMethod,
        entries: &[&DispatchRoute],
        matched: RouteMatch,
        request: Request<Body>,
    ) -> Response<Body> {
        let wanted = RouteVerbs::wanted(method);
        let entry = entries
            .iter()
            .find(|entry| entry.mapping.verb == wanted)
            .copied();
        self.serve_matching_verb(entry, method, entries, matched, request)
            .await
    }

    async fn serve_matching_verb(
        &self,
        entry: Option<&DispatchRoute>,
        method: HttpMethod,
        entries: &[&DispatchRoute],
        matched: RouteMatch,
        request: Request<Body>,
    ) -> Response<Body> {
        match entry {
            Some(entry) => self.serve_route(entry, matched, request).await,
            None => {
                self.answer_other_verb(method, entries, matched, request)
                    .await
            }
        }
    }

    async fn answer_other_verb(
        &self,
        method: HttpMethod,
        entries: &[&DispatchRoute],
        matched: RouteMatch,
        request: Request<Body>,
    ) -> Response<Body> {
        let mappings: Vec<&crate::yon::plan::Route> =
            entries.iter().map(|entry| entry.mapping.as_ref()).collect();
        let typed: Vec<_> = entries
            .iter()
            .map(|entry| (entry.mapping.as_ref(), entry.handler.models()))
            .collect();
        let verbs = RouteVerbs::allowed(&mappings);
        let serves_page = self.page_routes.contains(&matched.route);
        self.answer_by_verb(method, serves_page, &verbs, &typed, &matched.route, request)
            .await
    }

    async fn answer_by_verb(
        &self,
        method: HttpMethod,
        serves_page: bool,
        verbs: &[HttpMethod],
        typed: &[(&crate::yon::plan::Route, &crate::route_types::ModelTable)],
        route: &str,
        request: Request<Body>,
    ) -> Response<Body> {
        if method == HttpMethod::Options {
            let schema = crate::handler::RouteSchema::for_path(route, typed);
            RouteAnswer::options(verbs, &schema)
        } else if serves_page && Self::PAGE_METHODS.contains(&method) {
            self.clone().serve_static(request).await
        } else {
            RouteAnswer::method_not_allowed(verbs)
        }
    }

    async fn serve_route(
        &self,
        entry: &DispatchRoute,
        matched: RouteMatch,
        request: Request<Body>,
    ) -> Response<Body> {
        let refusal = RouteGuard::refusal(
            self,
            &entry.mapping.access,
            request.uri(),
            request.headers(),
        )
        .await;
        match refusal {
            Some(response) => response,
            None => self.serve_allowed_route(entry, matched, request).await,
        }
    }

    async fn serve_allowed_route(
        &self,
        entry: &DispatchRoute,
        matched: RouteMatch,
        request: Request<Body>,
    ) -> Response<Body> {
        let (parts, body) = request.into_parts();
        let body = axum::body::to_bytes(body, Self::MAX_REQUEST_BODY_BYTES).await;
        self.serve_route_body(entry, matched, &parts, body).await
    }

    async fn serve_route_body(
        &self,
        entry: &DispatchRoute,
        matched: RouteMatch,
        parts: &Parts,
        body: Result<Bytes, axum::Error>,
    ) -> Response<Body> {
        match body {
            Err(_) => {
                ServerResponse::text(StatusCode::PAYLOAD_TOO_LARGE, "Request body is too large.")
            }
            Ok(body) => {
                self.serve_route_with_body(entry, matched, parts, &body)
                    .await
            }
        }
    }

    async fn serve_route_with_body(
        &self,
        entry: &DispatchRoute,
        matched: RouteMatch,
        parts: &Parts,
        body: &Bytes,
    ) -> Response<Body> {
        let route_request = RouteRequest {
            parameters: &matched.parameters,
            query: parts.uri.query(),
            headers: &parts.headers,
            body,
        };
        let arguments = RouteArguments::build(
            entry.mapping.as_ref(),
            &route_request,
            entry.handler.models(),
        );
        self.serve_arguments(arguments, entry, matched, parts, body)
            .await
    }

    async fn serve_arguments(
        &self,
        arguments: Result<Vec<serde_json::Value>, super::controller_dispatch::Rejection>,
        entry: &DispatchRoute,
        matched: RouteMatch,
        parts: &Parts,
        body: &Bytes,
    ) -> Response<Body> {
        match arguments {
            Err(rejection) => rejection.response(),
            Ok(arguments) => {
                self.serve_with_arguments(arguments, entry, matched, parts, body)
                    .await
            }
        }
    }

    async fn serve_with_arguments(
        &self,
        arguments: Vec<serde_json::Value>,
        entry: &DispatchRoute,
        matched: RouteMatch,
        parts: &Parts,
        body: &Bytes,
    ) -> Response<Body> {
        let method = RouteMethod::parse(&entry.mapping.verb).unwrap_or(HttpMethod::Get);
        let protocol_request = Self::route_handler_request(matched, method, &parts.headers, body);
        self.serve_protocol_request(protocol_request, arguments, entry, parts)
            .await
    }

    async fn serve_protocol_request(
        &self,
        protocol_request: Option<HandlerRequest>,
        arguments: Vec<serde_json::Value>,
        entry: &DispatchRoute,
        parts: &Parts,
    ) -> Response<Body> {
        match protocol_request {
            None => ServerResponse::text(StatusCode::BAD_REQUEST, "Request body must be UTF-8."),
            Some(protocol_request) => {
                self.run_route(protocol_request, arguments, entry, parts)
                    .await
            }
        }
    }

    async fn run_route(
        &self,
        mut protocol_request: HandlerRequest,
        arguments: Vec<serde_json::Value>,
        entry: &DispatchRoute,
        parts: &Parts,
    ) -> Response<Body> {
        protocol_request.arguments = Some(arguments);
        protocol_request.target = ProtocolRequest::target(&parts.uri);
        self.stream_or_answer(entry, protocol_request).await
    }

    async fn stream_or_answer(
        &self,
        entry: &DispatchRoute,
        protocol_request: HandlerRequest,
    ) -> Response<Body> {
        if entry.mapping.streaming {
            return HandlerEventBridge::start(self, entry, protocol_request);
        }
        self.answer(entry, &protocol_request).await
    }

    async fn answer(
        &self,
        entry: &DispatchRoute,
        protocol_request: &HandlerRequest,
    ) -> Response<Body> {
        let response = self.dispatch_mapped_response(entry, protocol_request).await;
        RouteAnswer::with_status(response, entry.mapping.as_ref())
    }

    async fn dispatch_mapped_response(
        &self,
        entry: &DispatchRoute,
        protocol_request: &HandlerRequest,
    ) -> Response<Body> {
        let result = self
            .invoke_until_stopped(&entry.handler, protocol_request)
            .await;
        Self::mapped_answer(result, entry, &protocol_request.request_id)
    }

    fn mapped_answer(
        result: Result<HandlerResponse, Failure>,
        entry: &DispatchRoute,
        request_id: &str,
    ) -> Response<Body> {
        match result {
            Ok(response) => Self::checked_answer(response, entry, request_id),
            Err(failure) => {
                InvocationLog::log_failure(
                    "handler",
                    request_id,
                    InvocationLog::runtime_family(&entry.handler),
                    &failure,
                );
                ServerResponse::invocation_failure("Handler execution", request_id)
            }
        }
    }

    fn checked_answer(
        response: HandlerResponse,
        entry: &DispatchRoute,
        request_id: &str,
    ) -> Response<Body> {
        let mapping = entry.mapping.as_ref();
        let problem = RouteResult::problem(mapping, entry.handler.models(), &response);
        Self::answer_or_mismatch(problem, response, mapping, request_id)
    }

    fn answer_or_mismatch(
        problem: Option<String>,
        response: HandlerResponse,
        mapping: &crate::yon::plan::Route,
        request_id: &str,
    ) -> Response<Body> {
        match problem {
            Some(problem) => RouteResult::mismatch(mapping, &problem, request_id),
            None => ServerResponse::from_handler(response),
        }
    }

    fn route_handler_request(
        matched: RouteMatch,
        method: HttpMethod,
        headers: &HeaderMap,
        body: &[u8],
    ) -> Option<HandlerRequest> {
        let mut request = HandlerRequest::route(crate::ttid::generate(), matched.route, method);
        request.parameters = matched.parameters;
        ProtocolRequest::copy_headers(&mut request, headers);
        if body.is_empty() {
            return Some(request);
        }
        let text = std::str::from_utf8(body).ok()?;
        request.body = Some(HandlerBody {
            encoding: HandlerBodyEncoding::Utf8,
            data: text.to_owned(),
        });
        Some(request)
    }

    async fn invoke_until_stopped(
        &self,
        source: &HandlerSource,
        request: &HandlerRequest,
    ) -> Result<HandlerResponse, Failure> {
        let cancellation = HandlerCancellation::default();
        let invocation = self.supervisor.invoke(source, request, &cancellation);
        tokio::pin!(invocation);
        tokio::select! {
            result = &mut invocation => result,
            () = self.cancellation.cancelled() => {
                cancellation.cancel();
                invocation.await
            }
        }
    }

    async fn serve_static(self, request: Request<Body>) -> Response<Body> {
        let response = StaticFiles::serve(&self.files, &self.page_routes, request).await;
        self.static_answer(response).await
    }

    async fn static_answer(&self, response: Option<Response<Body>>) -> Response<Body> {
        match response {
            None => ServerResponse::text(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Cannot serve generated output.",
            ),
            Some(response) => self.with_live_reload(response).await,
        }
    }

    async fn with_live_reload(&self, response: Response<Body>) -> Response<Body> {
        let html = StaticFiles::is_html_document(&response);
        self.inject_when_watched(html, response).await
    }

    async fn inject_when_watched(&self, html: bool, response: Response<Body>) -> Response<Body> {
        if !self.watch || !html {
            return response;
        }
        Self::inject(response).await
    }

    async fn inject(response: Response<Body>) -> Response<Body> {
        let (parts, body) = response.into_parts();
        let bytes = axum::body::to_bytes(body, Self::MAX_DOCUMENT_BYTES).await;
        Self::injected(parts, bytes)
    }

    fn injected(parts: ResponseParts, bytes: Result<Bytes, axum::Error>) -> Response<Body> {
        match bytes {
            Err(_) => ServerResponse::text(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Generated document is too large to serve.",
            ),
            Ok(bytes) => Self::injected_document(parts, bytes),
        }
    }

    fn injected_document(parts: ResponseParts, bytes: Bytes) -> Response<Body> {
        let injected = std::str::from_utf8(&bytes)
            .ok()
            .map(StaticFiles::inject_live_reload);
        Self::document_response(parts, bytes, injected)
    }

    fn document_response(
        mut parts: ResponseParts,
        bytes: Bytes,
        injected: Option<String>,
    ) -> Response<Body> {
        match injected {
            None => Response::from_parts(parts, Body::from(bytes)),
            Some(document) => {
                parts.headers.remove(CONTENT_LENGTH);
                Response::from_parts(parts, Body::from(document))
            }
        }
    }
}
