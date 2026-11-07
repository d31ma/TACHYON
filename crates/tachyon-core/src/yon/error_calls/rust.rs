use super::{ErrorCalls, Plan, RouteCalls};
use std::fmt::Write as _;

pub(crate) struct RustErrors;

impl RustErrors {
    const BEHAVIOR_FAILURE: &'static str = "impl YonFailure for BehaviorError {";
    const UNHANDLED_BEHAVIOR_ERROR: &'static str = "    if let Some(error) = error.downcast_ref::<BehaviorError>() {\n        return __YonAnswer::failed(error);\n    }\n";

    pub(crate) fn result_error(returns: &str) -> Option<String> {
        let compact = returns.split_whitespace().collect::<String>();
        let inside = compact
            .strip_prefix("Result<")
            .or_else(|| compact.strip_prefix("std::result::Result<"))?
            .strip_suffix('>')?;
        crate::go_source::split_top_level(inside, b',')
            .get(1)
            .map(|value| (*value).to_owned())
    }

    pub(crate) fn reference(error_type: &str) -> &'static str {
        if error_type.starts_with("Box<dyn") || error_type.starts_with("std::boxed::Box<dyn") {
            "error.as_ref()"
        } else {
            "&error"
        }
    }

    pub(super) fn functions(plan: &Plan, stripped: &str) -> String {
        let parameters = ErrorCalls::parameters(plan, "rs");
        let mut source = format!(
            "\n#[allow(unused_variables)]\nfn __yon_handle_error__(error: &(dyn std::error::Error + 'static), request: &__YonFrame{parameters}) -> __YonAnswer {{\n"
        );
        for method in &plan.errors {
            let Some((index, _)) = RouteCalls::error_owners(plan)
                .into_iter()
                .find(|(_, name)| *name == method.owner)
            else {
                continue;
            };
            let kind = method.error_type.trim_start_matches('&').trim();
            let request = if method.takes_request {
                ", &yon_native_request(request)"
            } else {
                ""
            };
            let call = format!("handler{index}.{}(error{request})", method.method);
            let _ = writeln!(
                source,
                "    if let Some(error) = error.downcast_ref::<{kind}>() {{"
            );
            if Self::result_error(&method.returns).is_some() {
                let _ = writeln!(
                    source,
                    "        let value = match {call} {{ Ok(value) => value, Err(_) => return __yon_error_failure__() }};"
                );
            } else {
                let _ = writeln!(source, "        let value = {call};");
            }
            let _ = writeln!(
                source,
                "        let mut response = yon_response(value);\n        if response.status == 200 {{ response = response.status({}); }}\n        return response;\n    }}",
                method.status
            );
        }
        if stripped.contains(Self::BEHAVIOR_FAILURE) {
            source.push_str(Self::UNHANDLED_BEHAVIOR_ERROR);
        }
        source.push_str("    __yon_error_failure__()\n}\nfn __yon_error_failure__() -> __YonAnswer {\n    __YonAnswer::json(\"{\\\"error\\\":\\\"Yon controller execution failed.\\\"}\").status(500)\n}\n");
        source
    }
}
