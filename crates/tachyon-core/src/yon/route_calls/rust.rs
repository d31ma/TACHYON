use super::RouteCalls;
use crate::yon::error_calls::{ErrorCalls, rust::RustErrors};
use crate::yon::plan::{ParameterSource, Plan, Route};
use std::fmt::Write as _;

pub(super) struct RustRoutes;

impl RustRoutes {
    pub(super) fn functions(plan: &Plan) -> String {
        let mut output = String::new();
        let parameters = ErrorCalls::parameters(plan, "rs");
        let error_arguments = ErrorCalls::arguments(plan, "rs");
        for verb in RouteCalls::verbs(plan) {
            let _ = write!(
                output,
                "\n#[allow(non_snake_case, unused_variables)]\nfn __yon_route_{verb}__(controller: &{}, request: &__YonFrame{parameters}) -> __YonAnswer {{\n    let values = request.arguments();\n    let value = |index: usize| values.get(index).cloned().unwrap_or(Json::Null);\n    match request.route().as_str() {{\n",
                plan.controller
            );
            for route in RouteCalls::routes_for(plan, verb) {
                if let Some(error_type) =
                    RustErrors::result_error(&route.returns).filter(|_| !plan.errors.is_empty())
                {
                    let reference = RustErrors::reference(&error_type);
                    let _ = writeln!(
                        output,
                        "        {} => match controller.{}({}) {{ Ok(value) => yon_response(value), Err(error) => __yon_handle_error__({reference}, request{error_arguments}) }},",
                        RouteCalls::pattern(route),
                        route.method,
                        Self::arguments(route)
                    );
                    continue;
                }
                let _ = writeln!(
                    output,
                    "        {} => yon_response(controller.{}({})),",
                    RouteCalls::pattern(route),
                    route.method,
                    Self::arguments(route)
                );
            }
            output.push_str("        _ => __YonAnswer::json(\"{\\\"error\\\":\\\"No controller method answers this route.\\\"}\").status(500),\n    }\n}\n");
        }
        output
    }

    fn arguments(route: &Route) -> String {
        let mut arguments = Vec::new();
        for (index, parameter) in route.parameters.iter().enumerate() {
            if parameter.source == ParameterSource::Request {
                arguments.push(String::from("&yon_native_request(request)"));
            } else {
                arguments.push(format!("YonRead::read(&value({index}))"));
            }
        }
        arguments.join(", ")
    }
}
