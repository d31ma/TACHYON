use super::RouteCalls;
use crate::route_types::PhpDoc;
use crate::yon::plan::{ParameterSource, Plan, Route};
use std::fmt::Write as _;

pub(super) struct PhpRoutes;

impl PhpRoutes {
    pub(super) fn functions(plan: &Plan) -> String {
        let mut output = String::new();
        let errors = !plan.errors.is_empty();
        let parameters = super::super::error_calls::ErrorCalls::parameters(plan, "php");
        for verb in RouteCalls::verbs(plan) {
            let guard = if errors { "    try {\n" } else { "" };
            let _ = write!(
                output,
                "function __yon_route_{verb}__($controller, $request{parameters}) {{\n    $values = $request->arguments();\n{guard}    switch ($request->route()) {{\n"
            );
            for route in RouteCalls::routes_for(plan, verb) {
                let _ = writeln!(
                    output,
                    "        case {}: return $controller->{}({});",
                    RouteCalls::pattern(route),
                    route.method,
                    Self::arguments(route)
                );
            }
            output.push_str("    }\n    throw new \\RuntimeException('No controller method answers this route.');\n");
            if errors {
                let arguments = super::super::error_calls::ErrorCalls::arguments(plan, "php");
                let _ = writeln!(
                    output,
                    "    }} catch (\\Throwable $error) {{ return __yon_handle_error__($error, $request{arguments}); }}"
                );
            }
            output.push_str("}\n");
        }
        output
    }

    fn arguments(route: &Route) -> String {
        let mut arguments = Vec::new();
        for (index, parameter) in route.parameters.iter().enumerate() {
            if parameter.source == ParameterSource::Request {
                arguments.push(String::from("$request"));
                continue;
            }
            let value = format!("$values[{index}]");
            let converts = RouteCalls::needs_conversion("php", &parameter.type_text);
            let class = parameter.type_text.trim_start_matches('?');
            let single = !class.contains(['<', '[']) && class != "array";
            let argument = match PhpDoc::list_item(&parameter.type_text) {
                Some(item) if converts => format!("YonModels::readList({item}::class, {value})"),
                _ if converts && single => format!("YonModels::read({class}::class, {value})"),
                _ => value,
            };
            arguments.push(argument);
        }
        arguments.join(", ")
    }
}
