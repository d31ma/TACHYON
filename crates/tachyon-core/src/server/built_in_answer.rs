use super::controller_dispatch::RouteResult;
use super::{
    BuiltInCase, BuiltInRequest, Dispatch, DispatchRoute, HeaderNames, InvocationLog,
    ServerResponse,
};
use crate::Failure;
use crate::sorted_json::SortedJson;
use crate::yon::built_in_error::BuiltInError;
use axum::body::Body;
use axum::http::Response;
use axum::http::header::{CACHE_CONTROL, HeaderValue};
use tachyon_contracts::{HandlerHeaders, HandlerRequest, HandlerResponse};

pub(in crate::server) struct BuiltInAnswer;

impl BuiltInAnswer {
    const KEPT_HEADER: &'static str = "content-type";
    const LOG_KIND: &'static str = "error_handler";
    const CALL_FAILED: &'static str = "the error handler call failed";
    const HANDLER_ERROR: &'static str = "the error handler did not answer";
    const NO_STORE: &'static str = "no-store";

    pub(in crate::server) async fn shape(
        state: &Dispatch,
        entry: &DispatchRoute,
        request: &BuiltInRequest,
        response: Response<Body>,
    ) -> Response<Body> {
        let case = BuiltInCase::of(&response).filter(|case| Self::declares(entry, case));
        match case {
            None => response,
            Some(case) => Self::ask(state, entry, request, &case, response).await,
        }
    }

    fn declares(entry: &DispatchRoute, case: &BuiltInCase) -> bool {
        entry
            .mapping
            .error_responses
            .iter()
            .any(|declared| BuiltInError::named(&declared.error_type) == Some(case.error()))
    }

    async fn ask(
        state: &Dispatch,
        entry: &DispatchRoute,
        request: &BuiltInRequest,
        case: &BuiltInCase,
        fallback: Response<Body>,
    ) -> Response<Body> {
        let reference = Self::reference(&fallback);
        let identity = state.identity.as_deref();
        let protocol = request.protocol(entry, case, &reference, identity);
        let result = state.invoke(&entry.handler, &protocol).await;
        Self::log_call_failure(&result, &protocol, entry);
        let accepted = Self::accepted(result, entry, case);
        Self::answer_or_fallback(accepted, entry, case, &reference, fallback)
    }

    fn log_call_failure(
        result: &Result<HandlerResponse, Failure>,
        protocol: &HandlerRequest,
        entry: &DispatchRoute,
    ) {
        if let Err(failure) = result {
            InvocationLog::log_handler_failure(Self::LOG_KIND, protocol, &entry.handler, failure);
        }
    }

    fn reference(response: &Response<Body>) -> String {
        response
            .headers()
            .get(HeaderNames::REQUEST_ID)
            .and_then(|value| value.to_str().ok())
            .map_or_else(crate::ttid::generate, String::from)
    }

    fn accepted(
        result: Result<HandlerResponse, Failure>,
        entry: &DispatchRoute,
        case: &BuiltInCase,
    ) -> Result<HandlerResponse, String> {
        let answer = result.map_err(|_| String::from(Self::CALL_FAILED))?;
        let mut answer = Self::without_error(answer)?;
        answer.status = case.error().status();
        answer.headers = Self::kept_headers(&answer.headers);
        let problem = RouteResult::problem(&entry.mapping, entry.handler.models(), &answer);
        problem.map_or(Ok(answer), Err)
    }

    fn without_error(answer: HandlerResponse) -> Result<HandlerResponse, String> {
        match &answer.error {
            Some(_) => Err(String::from(Self::HANDLER_ERROR)),
            None => Ok(answer),
        }
    }

    fn kept_headers(headers: &HandlerHeaders) -> HandlerHeaders {
        headers
            .iter()
            .filter(|(name, _)| name.eq_ignore_ascii_case(Self::KEPT_HEADER))
            .map(|(name, values)| (name.clone(), values.clone()))
            .collect()
    }

    fn answer_or_fallback(
        accepted: Result<HandlerResponse, String>,
        entry: &DispatchRoute,
        case: &BuiltInCase,
        reference: &str,
        fallback: Response<Body>,
    ) -> Response<Body> {
        match accepted {
            Ok(answer) => Self::shaped(answer, reference),
            Err(reason) => {
                Self::log_failure(entry, case, reference, &reason);
                fallback
            }
        }
    }

    fn shaped(answer: HandlerResponse, reference: &str) -> Response<Body> {
        let mut response = ServerResponse::from_handler(answer);
        ServerResponse::insert_request_id(&mut response, reference);
        response
            .headers_mut()
            .insert(CACHE_CONTROL, HeaderValue::from_static(Self::NO_STORE));
        response
    }

    fn log_failure(entry: &DispatchRoute, case: &BuiltInCase, reference: &str, reason: &str) {
        let event = serde_json::json!({
            "event": "error_handler.failed",
            "reason": reason,
            "request_id": reference,
            "route": entry.route,
            "handler": entry.handler.relative_path(),
            "type": case.error().name(),
        });
        eprintln!("{}", SortedJson::text(&event));
    }
}
