use super::controller_dispatch::{RouteArguments, RouteRequest};
use super::{AccessVerdict, Dispatch, IdentityAnswer, InvocationLog, ProtocolRequest};
use crate::Failure;
use crate::handler::HandlerSource;
use crate::yon::plan::Route;
use crate::yon::route_access::RouteAccess;
use axum::body::Body;
use axum::http::{HeaderMap, Response, Uri};
use std::collections::BTreeMap;
use tachyon_contracts::{HandlerRequest, HandlerResponse, HttpMethod};

pub(in crate::server) struct RouteGuard;

impl RouteGuard {
    const LOG_KIND: &'static str = "identity";

    pub(in crate::server) async fn refusal(
        state: &Dispatch,
        access: &RouteAccess,
        uri: &Uri,
        headers: &HeaderMap,
    ) -> Option<Response<Body>> {
        let answer = Self::answer_when_needed(state, access, uri, headers).await;
        AccessVerdict::of(access, &answer).response()
    }

    async fn answer_when_needed(
        state: &Dispatch,
        access: &RouteAccess,
        uri: &Uri,
        headers: &HeaderMap,
    ) -> IdentityAnswer {
        if !access.needs_identity() {
            return IdentityAnswer::NotAsked;
        }
        Self::ask(state, uri, headers).await
    }

    async fn ask(state: &Dispatch, uri: &Uri, headers: &HeaderMap) -> IdentityAnswer {
        let source = state.identity.clone();
        let request = source
            .as_deref()
            .map_or(Err(IdentityAnswer::Failed), |source| {
                Self::request(source, uri, headers)
            });
        Self::invoke_prepared(state, source.as_deref(), request).await
    }

    async fn invoke_prepared(
        state: &Dispatch,
        source: Option<&HandlerSource>,
        request: Result<HandlerRequest, IdentityAnswer>,
    ) -> IdentityAnswer {
        match (source, request) {
            (Some(source), Ok(request)) => Self::invoke(state, source, &request).await,
            (_, Err(answer)) => answer,
            (None, Ok(_)) => IdentityAnswer::Failed,
        }
    }

    async fn invoke(
        state: &Dispatch,
        source: &HandlerSource,
        request: &HandlerRequest,
    ) -> IdentityAnswer {
        let result = state.invoke(source, request).await;
        Self::log(&result, request, source);
        IdentityAnswer::read(result)
    }

    fn request(
        source: &HandlerSource,
        uri: &Uri,
        headers: &HeaderMap,
    ) -> Result<HandlerRequest, IdentityAnswer> {
        let route = source.routes().first().ok_or(IdentityAnswer::Failed)?;
        let arguments = Self::arguments(source, route, uri, headers)?;
        let pattern = route.pattern.clone().unwrap_or_default();
        let mut request = HandlerRequest::route(crate::ttid::generate(), pattern, HttpMethod::Get);
        ProtocolRequest::copy_headers(&mut request, headers);
        request.target = ProtocolRequest::target(uri);
        request.arguments = Some(arguments);
        Ok(request)
    }

    fn arguments(
        source: &HandlerSource,
        route: &Route,
        uri: &Uri,
        headers: &HeaderMap,
    ) -> Result<Vec<serde_json::Value>, IdentityAnswer> {
        let parameters = BTreeMap::new();
        let request = RouteRequest {
            parameters: &parameters,
            query: uri.query(),
            headers,
            body: &[],
        };
        RouteArguments::build(route, &request, source.models()).map_err(|_| IdentityAnswer::Nobody)
    }

    fn log(
        result: &Result<HandlerResponse, Failure>,
        request: &HandlerRequest,
        source: &HandlerSource,
    ) {
        if let Err(failure) = result {
            InvocationLog::log_failure(
                Self::LOG_KIND,
                &request.request_id,
                InvocationLog::runtime_family(source),
                failure,
            );
        }
    }
}
