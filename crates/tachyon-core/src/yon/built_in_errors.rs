use super::built_in_error::BuiltInError;
use super::error_calls::ErrorCalls;
use super::plan::{ErrorMethod, Plan};

pub(crate) struct BuiltInErrors;

impl BuiltInErrors {
    pub(crate) fn used(plan: &Plan) -> bool {
        plan.errors.iter().any(Self::names_built_in)
    }

    fn names_built_in(method: &ErrorMethod) -> bool {
        BuiltInError::named(&method.error_type).is_some()
    }

    pub(crate) fn source(extension: &str, plan: &Plan) -> &'static str {
        let used = Self::used(plan);
        let source = Self::language_source(extension);
        Self::text_when(used, source)
    }

    pub(crate) fn route_case(plan: &Plan, extension: &str, template: &str) -> String {
        let arguments = ErrorCalls::arguments(plan, extension);
        let case = template
            .replace("{route}", BuiltInError::ROUTE)
            .replace("{arguments}", &arguments);
        let used = Self::used(plan);
        String::from(Self::text_when(used, &case))
    }

    fn text_when(used: bool, text: &str) -> &str {
        if !used {
            return "";
        }
        text
    }

    fn language_source(extension: &str) -> &'static str {
        match extension {
            "js" => include_str!("built_in_errors/built_in_errors.js"),
            "ts" => include_str!("built_in_errors/built_in_errors.ts"),
            "go" => include_str!("built_in_errors/built_in_errors.go"),
            "py" => include_str!("built_in_errors/built_in_errors.py"),
            "rb" => include_str!("built_in_errors/built_in_errors.rb"),
            "php" => include_str!("built_in_errors/built_in_errors.php"),
            "java" => include_str!("built_in_errors/built_in_errors.java"),
            "kt" => include_str!("built_in_errors/built_in_errors.kt"),
            "cs" => include_str!("built_in_errors/built_in_errors.cs"),
            "dart" => include_str!("built_in_errors/built_in_errors.dart"),
            "rs" => include_str!("built_in_errors/built_in_errors.rs.in"),
            _ => "",
        }
    }
}
