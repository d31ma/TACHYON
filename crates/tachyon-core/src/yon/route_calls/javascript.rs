use super::RouteCalls;
use crate::yon::plan::{ParameterSource, Plan, Route};
use std::fmt::Write as _;

pub(super) struct JavaScriptRoutes;

impl JavaScriptRoutes {
    pub(super) fn functions(plan: &Plan, typed: bool) -> String {
        let mut output = String::new();
        let errors = !plan.errors.is_empty();
        let error_arguments = super::super::error_calls::ErrorCalls::parameters(
            plan,
            if typed { "ts" } else { "js" },
        );
        for verb in RouteCalls::verbs(plan) {
            let signature = if typed {
                let returns = if errors { "Promise<any>" } else { "any" };
                format!(
                    "controller: {}, request: any{error_arguments}): {returns}",
                    plan.controller
                )
            } else {
                format!("controller, request{error_arguments})")
            };
            let asynchronous = if errors { "async " } else { "" };
            let guarded = if errors { "  try {\n" } else { "" };
            let list_type = if typed { ": any[]" } else { "" };
            let _ = write!(
                output,
                "{asynchronous}function __yon_route_{verb}__({signature} {{\n  const values{list_type} = request.arguments ?? [];\n{guarded}  switch (request.route) {{\n"
            );
            for route in RouteCalls::routes_for(plan, verb) {
                let _ = writeln!(
                    output,
                    "    case {}: return {}controller.{}({});",
                    RouteCalls::pattern(route),
                    if errors { "await " } else { "" },
                    route.method,
                    Self::arguments(route, typed)
                );
            }
            output
                .push_str("  }\n  throw new Error('No controller method answers this route.');\n");
            if errors {
                let arguments = super::super::error_calls::ErrorCalls::arguments(plan, "js");
                let _ = writeln!(
                    output,
                    "  }} catch (error) {{ return __yon_handle_error__(error, request{arguments}); }}"
                );
            }
            output.push_str("}\n");
        }
        output
    }

    fn arguments(route: &Route, typed: bool) -> String {
        let mut arguments = Vec::new();
        for (index, parameter) in route.parameters.iter().enumerate() {
            if parameter.source == ParameterSource::Request {
                arguments.push(String::from(if typed {
                    "(globalThis as any).YonHttp.request(request)"
                } else {
                    "YonHttp.request(request)"
                }));
            } else {
                arguments.push(format!("values[{index}]"));
            }
        }
        arguments.join(", ")
    }
}
