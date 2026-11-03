use super::RouteCalls;
use crate::yon::plan::{ParameterSource, Plan, Route};
use std::fmt::Write as _;

pub(super) struct CSharpRoutes;

impl CSharpRoutes {
    pub(super) fn functions(plan: &Plan) -> String {
        let mut output = String::from("\nstatic class __YonRoutes\n{\n");
        let errors = !plan.errors.is_empty();
        let parameters = super::super::error_calls::ErrorCalls::parameters(plan, "cs");
        for verb in RouteCalls::verbs(plan) {
            let guard = if errors { "        try {\n" } else { "" };
            let _ = write!(
                output,
                "    internal static object __yon_route_{verb}__({} controller, __YonFrame request{parameters})\n    {{\n        var values = request.Arguments();\n        var route = request.Route();\n{guard}",
                plan.controller
            );
            for route in RouteCalls::routes_for(plan, verb) {
                let call = format!("controller.{}({})", route.method, Self::arguments(route));
                let body = if route.returns == "void" {
                    format!("{{ {call}; return null; }}")
                } else {
                    format!("return YonModels.Answer({call});")
                };
                let _ = writeln!(
                    output,
                    "        if (route == {}) {body}",
                    RouteCalls::pattern(route)
                );
            }
            output.push_str("        throw new InvalidOperationException(\"No controller method answers this route.\");\n");
            if errors {
                let arguments = super::super::error_calls::ErrorCalls::arguments(plan, "cs");
                let _ = writeln!(
                    output,
                    "        }} catch (System.Exception error) {{ return __YonErrors.Handle(error, request{arguments}); }}"
                );
            }
            output.push_str("    }\n");
        }
        output.push_str("}\n");
        output
    }

    fn arguments(route: &Route) -> String {
        let mut arguments = Vec::new();
        for (index, parameter) in route.parameters.iter().enumerate() {
            if parameter.source == ParameterSource::Request {
                arguments.push(String::from("YonHttp.Request(request)"));
                continue;
            }
            arguments.push(format!(
                "YonModels.Argument<{}>(values, {index})",
                parameter.type_text
            ));
        }
        arguments.join(", ")
    }
}
