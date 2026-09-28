use super::RouteCalls;
use crate::route_types::{DartValues, TypeText};
use crate::yon::plan::{ParameterSource, Plan, Route};
use std::fmt::Write as _;

pub(super) struct DartRoutes;

impl DartRoutes {
    pub(super) fn functions(plan: &Plan) -> String {
        let mut output = String::new();
        let errors = !plan.errors.is_empty();
        let parameters = super::super::error_calls::ErrorCalls::parameters(plan, "dart");
        for verb in RouteCalls::verbs(plan) {
            let guard = if errors { "  try {\n" } else { "" };
            let _ = write!(
                output,
                "\nFuture<Object?> __yon_route_{verb}__({} controller, __YonFrame request{parameters}) async {{\n  final values = request.arguments();\n  Object? value(int index) => index < values.length ? values[index] : null;\n  final route = request.route();\n{guard}",
                plan.controller
            );
            for route in RouteCalls::routes_for(plan, verb) {
                let call = format!("controller.{}({})", route.method, Self::arguments(route));
                let body = match route.returns.as_str() {
                    "void" => format!("{{ {call}; return null; }}"),
                    "Future<void>" => format!("{{ await {call}; return null; }}"),
                    _ if route.streaming => format!("return {call}.map(__yon_json);"),
                    _ => format!("return __yon_json(await {call});"),
                };
                let _ = writeln!(
                    output,
                    "  if (route == {}) {body}",
                    RouteCalls::pattern(route)
                );
            }
            output.push_str("  throw StateError('No controller method answers this route.');\n");
            if errors {
                let arguments = super::super::error_calls::ErrorCalls::arguments(plan, "dart");
                let _ = writeln!(
                    output,
                    "  }} catch (error) {{ return __yon_handle_error__(error, request{arguments}); }}"
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
                arguments.push(String::from("_yonShelfRequest(request)"));
                continue;
            }
            let shape = TypeText::shape("dart", &parameter.type_text).unwrap_or_default();
            arguments.push(DartValues::read(&shape, &format!("value({index})")));
        }
        arguments.join(", ")
    }
}
