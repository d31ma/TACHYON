use super::RouteCalls;
use crate::yon::plan::{ParameterSource, Plan, Route};
use std::fmt::Write as _;

pub(super) struct KotlinRoutes;

impl KotlinRoutes {
    pub(super) fn functions(plan: &Plan) -> String {
        let mut output = String::new();
        let errors = !plan.errors.is_empty();
        let parameters = super::super::error_calls::ErrorCalls::parameters(plan, "kt");
        for verb in RouteCalls::verbs(plan) {
            let guard = if errors { "    try {\n" } else { "" };
            let _ = write!(
                output,
                "\n@Suppress(\"UNCHECKED_CAST\")\nfun __yon_route_{verb}__(controller: {}, request: __YonFrame{parameters}): Any? {{\n    val values = request.arguments()\n    val route = request.route()\n{guard}",
                plan.controller
            );
            for route in RouteCalls::routes_for(plan, verb) {
                let _ = writeln!(
                    output,
                    "    if (route == {}) return controller.{}({})",
                    RouteCalls::pattern(route),
                    route.method,
                    Self::arguments(&plan.controller, route)
                );
            }
            output.push_str(
                "    throw IllegalStateException(\"No controller method answers this route.\")\n",
            );
            if errors {
                let arguments = super::super::error_calls::ErrorCalls::arguments(plan, "kt");
                let _ = writeln!(
                    output,
                    "    }} catch (error: Exception) {{ return __YonErrors.handle(error, request{arguments}) }}"
                );
            }
            output.push_str("}\n");
        }
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
                "YonModels.argument({controller}::class.java, \"{}\", {count}, {index}, values.getOrNull({index})) as {}",
                route.method, parameter.type_text
            ));
        }
        arguments.join(", ")
    }
}
