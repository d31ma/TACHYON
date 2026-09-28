use crate::Failure;
use crate::handler::{HandlerSource, YonLanguage};
use serde_json::Value;
use std::path::Path;

pub(in crate::server) struct InvocationLog;

impl InvocationLog {
    pub(in crate::server) fn runtime_family(source: &HandlerSource) -> Option<&'static str> {
        YonLanguage::from_path(Path::new(source.relative_path())).map(YonLanguage::family)
    }

    pub(in crate::server) fn log_failure(
        operation: &str,
        request_id: &str,
        runtime_family: Option<&str>,
        failure: &Failure,
    ) {
        eprintln!(
            "{}",
            Self::failure_event(operation, request_id, runtime_family, failure)
        );
    }

    pub(in crate::server) fn failure_event(
        operation: &str,
        request_id: &str,
        runtime_family: Option<&str>,
        failure: &Failure,
    ) -> String {
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
        event.to_string()
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
