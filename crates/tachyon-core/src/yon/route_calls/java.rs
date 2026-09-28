use super::RouteCalls;
use crate::yon::plan::{ParameterSource, Plan, Route};
use std::fmt::Write as _;

pub(super) struct JavaRoutes;

impl JavaRoutes {
    pub(super) fn functions(plan: &Plan) -> String {
        let mut output = String::from("\nfinal class __YonRoutes {\n");
        let errors = !plan.errors.is_empty();
        let parameters = super::super::error_calls::ErrorCalls::parameters(plan, "java");
        for verb in RouteCalls::verbs(plan) {
            let guard = if errors { "        try {\n" } else { "" };
            let _ = write!(
                output,
                "    static Object __yon_route_{verb}__({} controller, __YonFrame request{parameters}) throws Exception {{\n        java.util.List<Object> values = request.arguments();\n        String route = request.route();\n{guard}",
                plan.controller
            );
            for route in RouteCalls::routes_for(plan, verb) {
                let call = format!(
                    "controller.{}({})",
                    route.method,
                    Self::arguments(&plan.controller, route)
                );
                let body = if route.returns == "void" {
                    format!("{{ {call}; return null; }}")
                } else {
                    format!("return {call};")
                };
                let _ = writeln!(
                    output,
                    "        if (route.equals({})) {body}",
                    RouteCalls::pattern(route)
                );
            }
            output.push_str("        throw new IllegalStateException(\"No controller method answers this route.\");\n");
            if errors {
                let arguments = super::super::error_calls::ErrorCalls::arguments(plan, "java");
                let _ = writeln!(
                    output,
                    "        }} catch (Exception error) {{ return __YonErrors.handle(error, request{arguments}); }}"
                );
            }
            output.push_str("    }\n");
        }
        output.push_str("}\n");
        output
    }

    fn arguments(controller: &str, route: &Route) -> String {
        let count = route.parameters.len();
        let mut arguments = Vec::new();
        for (index, parameter) in route.parameters.iter().enumerate() {
            if parameter.source == ParameterSource::Request {
                arguments.push(String::from("Yon.exchange(request)"));
                continue;
            }
            arguments.push(format!(
                "({}) YonModels.argument({controller}.class, \"{}\", {count}, {index}, values.get({index}))",
                parameter.type_text, route.method
            ));
        }
        arguments.join(", ")
    }
}
