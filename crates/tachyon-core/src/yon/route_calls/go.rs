use super::RouteCalls;
use crate::yon::GoStreamResult;
use crate::yon::error_calls::ErrorCalls;
use crate::yon::go_container::{GoContainer, GoRouteResult};
use crate::yon::plan::{ParameterSource, Plan, Route};
use std::fmt::Write as _;

pub(super) struct GoRoutes;

impl GoRoutes {
    pub(super) fn functions(plan: &Plan, stripped: &str) -> Result<String, String> {
        let mut output = String::new();
        let parameters = ErrorCalls::parameters(plan, "go");
        let error_arguments = ErrorCalls::arguments(plan, "go");
        for verb in RouteCalls::verbs(plan) {
            let _ = write!(
                output,
                "\nfunc __yon_route_{verb}__(controller *{}, request *yonRequest{parameters}) (any, error) {{\n\tswitch request.Route() {{\n",
                plan.controller
            );
            for route in RouteCalls::routes_for(plan, verb) {
                let call = format!("controller.{}({})", route.method, Self::arguments(route));
                if route.streaming {
                    let body = Self::stream_body(stripped, plan, route, &call)?;
                    let _ = writeln!(output, "\tcase {}:\n\t\t{body}", RouteCalls::pattern(route));
                    continue;
                }
                let result = GoContainer::route_result(stripped, &plan.controller, &route.method)?;
                let body = match result {
                    GoRouteResult::Failure if !plan.errors.is_empty() => format!(
                        "failure := {call}\n\t\tif failure != nil {{ return __yon_handle_error__(failure, request{error_arguments}) }}\n\t\treturn nil, nil"
                    ),
                    GoRouteResult::ValueAndFailure if !plan.errors.is_empty() => format!(
                        "value, failure := {call}\n\t\tif failure != nil {{ return __yon_handle_error__(failure, request{error_arguments}) }}\n\t\treturn value, nil"
                    ),
                    GoRouteResult::Nothing => format!("{call}\n\t\treturn nil, nil"),
                    GoRouteResult::Value => format!("return {call}, nil"),
                    GoRouteResult::Failure => format!("return nil, {call}"),
                    GoRouteResult::ValueAndFailure => format!("return {call}"),
                };
                let _ = writeln!(output, "\tcase {}:\n\t\t{body}", RouteCalls::pattern(route));
            }
            output.push_str("\t}\n\treturn nil, yonNoRoute()\n}\n");
        }
        Ok(output)
    }

    fn stream_body(
        stripped: &str,
        plan: &Plan,
        route: &Route,
        call: &str,
    ) -> Result<String, String> {
        let results = GoContainer::declared_results(stripped, &plan.controller, &route.method)?;
        let stream = GoStreamResult::of(&results).map_err(|help| {
            format!(
                "{help}; {}.{} returns '{}'",
                plan.controller,
                route.method,
                results.trim()
            )
        })?;
        Ok(format!("return {}({call}), nil", stream.wrapper()))
    }

    fn arguments(route: &Route) -> String {
        let mut arguments = Vec::new();
        for (index, parameter) in route.parameters.iter().enumerate() {
            if parameter.source == ParameterSource::Request {
                arguments.push(String::from("yonNativeRequest(request)"));
            } else {
                arguments.push(format!(
                    "yonArgument[{}](request, {index})",
                    parameter.type_text
                ));
            }
        }
        arguments.join(", ")
    }
}
