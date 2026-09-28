use super::RouteCalls;
use crate::route_types::{Shape, TypeText};
use crate::yon::plan::{ParameterSource, Plan, Route};
use std::fmt::Write as _;

pub(super) struct RubyRoutes;

impl RubyRoutes {
    pub(super) fn functions(plan: &Plan) -> String {
        let mut output = String::from("module YonGenerated\n");
        let parameters = super::super::error_calls::ErrorCalls::parameters(plan, "rb");
        let arguments = super::super::error_calls::ErrorCalls::arguments(plan, "rb");
        for verb in RouteCalls::verbs(plan) {
            let routes = RouteCalls::routes_for(plan, verb);
            let guarded = !plan.errors.is_empty() && !routes.iter().any(|route| route.streaming);
            let indent = if guarded { "  " } else { "" };
            let _ = writeln!(
                output,
                "  def self.route_{verb}(controller, request{parameters}, &events)\n    values = request[\"arguments\"] || []\n    route = request[\"route\"]"
            );
            if guarded {
                output.push_str("    begin\n");
            }
            for route in routes {
                let _ = writeln!(
                    output,
                    "{indent}    if route == {}\n{indent}      return controller.{}({})\n{indent}    end",
                    RouteCalls::pattern(route),
                    route.method,
                    Self::arguments(route)
                );
            }
            let _ = writeln!(
                output,
                "{indent}    raise YonNoRoute, \"No controller method answers this route.\""
            );
            if guarded {
                let _ = writeln!(
                    output,
                    "    rescue StandardError => error\n      handle_error(error, request{arguments})\n    end"
                );
            }
            output.push_str("  end\n\n");
        }
        output.push_str("end\n");
        output
    }

    fn arguments(route: &Route) -> String {
        let mut arguments = Vec::new();
        for (index, parameter) in route.parameters.iter().enumerate() {
            if parameter.source == ParameterSource::Request {
                arguments.push(String::from("YonHttp.request(request)"));
                continue;
            }
            let value = format!("values[{index}]");
            let descriptor = TypeText::shape("rb", &parameter.type_text)
                .ok()
                .and_then(|shape| Self::descriptor(&shape));
            match descriptor {
                Some(descriptor) => arguments.push(format!(
                    "YonModels.read({descriptor}, {value}, YonGenerated.field_types)"
                )),
                None => arguments.push(value),
            }
        }
        arguments.push(String::from("&events"));
        arguments.join(", ")
    }

    pub(super) fn descriptor(shape: &Shape) -> Option<String> {
        match shape {
            Shape::Model(name) => Some(name.clone()),
            Shape::List(item) => Self::descriptor(item).map(|item| format!("[{item}]")),
            Shape::Optional(inner) => Self::descriptor(inner),
            _ => None,
        }
    }
}
