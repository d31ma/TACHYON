use super::RouteCalls;
use crate::yon::plan::{ParameterSource, Plan, Route};
use std::fmt::Write as _;

pub(super) struct PythonRoutes;

impl PythonRoutes {
    pub(super) fn functions(plan: &Plan) -> String {
        let mut output = String::new();
        let errors = !plan.errors.is_empty();
        let parameters = super::super::error_calls::ErrorCalls::parameters(plan, "py");
        for verb in RouteCalls::verbs(plan) {
            let asynchronous = if errors { "async " } else { "" };
            let guard = if errors { "    try:\n" } else { "" };
            let indent = if errors { "    " } else { "" };
            let _ = write!(
                output,
                "\n\n{asynchronous}def __yon_route_{verb}__(controller, request{parameters}):\n    values = request.get(\"arguments\") or []\n    route = request.get(\"route\")\n{guard}"
            );
            for route in RouteCalls::routes_for(plan, verb) {
                let call = format!("controller.{}({})", route.method, Self::arguments(route));
                let _ = writeln!(
                    output,
                    "{indent}    if route == {}:",
                    RouteCalls::pattern(route)
                );
                if errors {
                    let _ = writeln!(
                        output,
                        "{indent}        result = {call}\n{indent}        if __yon_error_inspect.isawaitable(result):\n{indent}            result = await result\n{indent}        return result"
                    );
                } else {
                    let _ = writeln!(output, "        return {call}");
                }
            }
            let _ = writeln!(
                output,
                "{indent}    raise LookupError(\"No controller method answers this route.\")"
            );
            if errors {
                let arguments = super::super::error_calls::ErrorCalls::arguments(plan, "py");
                let _ = writeln!(
                    output,
                    "    except Exception as error:\n        return await __yon_handle_error__(error, request{arguments})"
                );
            }
        }
        output
    }

    fn arguments(route: &Route) -> String {
        let mut arguments = Vec::new();
        for (index, parameter) in route.parameters.iter().enumerate() {
            if parameter.source == ParameterSource::Request {
                arguments.push(String::from("YonHttp.request(request)"));
            } else if RouteCalls::needs_conversion("py", &parameter.type_text) {
                arguments.push(format!(
                    "YonModels.read({}, values[{index}])",
                    parameter.type_text
                ));
            } else {
                arguments.push(format!("values[{index}]"));
            }
        }
        arguments.join(", ")
    }
}
