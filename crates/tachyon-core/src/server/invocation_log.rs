use crate::Failure;
use crate::handler::{HandlerSource, YonLanguage};
use serde_json::Value;
use std::path::Path;
use tachyon_contracts::HandlerRequest;

pub(in crate::server) struct InvocationLog;

impl InvocationLog {
    fn runtime_family(source: &HandlerSource) -> Option<&'static str> {
        YonLanguage::from_path(Path::new(source.relative_path())).map(YonLanguage::family)
    }

    pub(in crate::server) fn log_handler_failure(
        operation: &str,
        request: &HandlerRequest,
        source: &HandlerSource,
        failure: &Failure,
    ) {
        eprintln!(
            "{}",
            Self::handler_failure_event(operation, request, source, failure)
        );
    }

    pub(in crate::server) fn handler_failure_event(
        operation: &str,
        request: &HandlerRequest,
        source: &HandlerSource,
        failure: &Failure,
    ) -> String {
        let mut event = Self::failure_value(
            operation,
            &request.request_id,
            Self::runtime_family(source),
            failure,
        );
        event["route"] = serde_json::json!(request.route);
        event["handler"] = serde_json::json!(source.relative_path());
        event.to_string()
    }

    #[cfg(test)]
    pub(in crate::server) fn failure_event(
        operation: &str,
        request_id: &str,
        runtime_family: Option<&str>,
        failure: &Failure,
    ) -> String {
        Self::failure_value(operation, request_id, runtime_family, failure).to_string()
    }

    fn failure_value(
        operation: &str,
        request_id: &str,
        runtime_family: Option<&str>,
        failure: &Failure,
    ) -> Value {
        let diagnostic_codes = failure
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.code.to_string())
            .collect::<Vec<_>>();
        let runtime_missing = diagnostic_codes.iter().any(|code| code == "TY2112");
        let mut event = serde_json::json!({
            "event": "handler.invocation_failed",
            "operation": operation,
            "request_id": request_id,
            "diagnostic_codes": diagnostic_codes,
        });
        Self::mark_missing_runtime(&mut event, runtime_missing, runtime_family);
        event
    }

    fn mark_missing_runtime(
        event: &mut Value,
        runtime_missing: bool,
        runtime_family: Option<&str>,
    ) {
        if runtime_missing {
            event["runtime_family"] = serde_json::json!(runtime_family.unwrap_or("unknown"));
            event["failure_kind"] = serde_json::json!("not_found");
        }
    }
}
