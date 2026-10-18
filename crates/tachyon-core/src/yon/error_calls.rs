use super::plan::Plan;
use super::route_calls::RouteCalls;
use std::fmt::Write as _;

mod csharp;
mod dart;
mod go;
mod java;
mod javascript;
mod kotlin;
mod php;
mod python;
mod ruby;
pub(crate) mod rust;

pub(super) struct ErrorCalls;

impl ErrorCalls {
    pub(super) fn parameters(plan: &Plan, extension: &str) -> String {
        let mut parameters = String::new();
        for (index, name) in RouteCalls::error_owners(plan) {
            let parameter = match extension {
                "ts" | "kt" => format!("handler{index}: {name}"),
                "rs" => format!("handler{index}: &{name}"),
                "go" => format!("handler{index} *{name}"),
                "php" => format!("{name} $handler{index}"),
                "java" | "cs" | "dart" => format!("{name} handler{index}"),
                _ => format!("handler{index}"),
            };
            let _ = write!(parameters, ", {parameter}");
        }
        parameters
    }

    pub(super) fn arguments(plan: &Plan, extension: &str) -> String {
        let mut arguments = String::new();
        let prefix = if extension == "php" { "$" } else { "" };
        for (index, _) in RouteCalls::error_owners(plan) {
            let _ = write!(arguments, ", {prefix}handler{index}");
        }
        arguments
    }

    pub(super) fn functions(
        extension: &str,
        plan: &Plan,
        stripped: &str,
    ) -> Result<String, String> {
        if plan.errors.is_empty() {
            return Ok(String::new());
        }
        match extension {
            "js" | "ts" => Ok(javascript::functions(plan, extension == "ts")),
            "py" => Ok(python::functions(plan)),
            "rb" => Ok(ruby::functions(plan)),
            "php" => Ok(php::functions(plan)),
            "java" => Ok(java::functions(plan)),
            "cs" => Ok(csharp::functions(plan)),
            "kt" => Ok(kotlin::functions(plan)),
            "dart" => Ok(dart::functions(plan, stripped)),
            "go" => go::functions(plan, stripped),
            "rs" => Ok(rust::RustErrors::functions(plan, stripped)),
            _ => Err(format!(
                "Error handler execution for .{extension} is not implemented"
            )),
        }
    }
}
