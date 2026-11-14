use super::built_in_error::BuiltInError;
use super::plan::{ErrorMethod, Plan};

pub(crate) struct BuiltInErrors;

impl BuiltInErrors {
    const JAVASCRIPT: &'static str = include_str!("built_in_errors/built_in_errors.js");
    const TYPESCRIPT: &'static str = include_str!("built_in_errors/built_in_errors.ts");
    const GO: &'static str = include_str!("built_in_errors/built_in_errors.go");
    const PYTHON: &'static str = include_str!("built_in_errors/built_in_errors.py");
    const RUBY: &'static str = include_str!("built_in_errors/built_in_errors.rb");
    const PHP: &'static str = include_str!("built_in_errors/built_in_errors.php");
    const JAVA: &'static str = include_str!("built_in_errors/built_in_errors.java");
    const KOTLIN: &'static str = include_str!("built_in_errors/built_in_errors.kt");
    const CSHARP: &'static str = include_str!("built_in_errors/built_in_errors.cs");
    const DART: &'static str = include_str!("built_in_errors/built_in_errors.dart");
    const RUST: &'static str = include_str!("built_in_errors/built_in_errors.rs.in");

    pub(crate) fn used(plan: &Plan) -> bool {
        plan.errors.iter().any(Self::names_built_in)
    }

    fn names_built_in(method: &ErrorMethod) -> bool {
        BuiltInError::named(&method.error_type).is_some()
    }

    pub(crate) fn source(extension: &str, plan: &Plan) -> &'static str {
        let used = Self::used(plan);
        Self::source_when(used, extension)
    }

    fn source_when(used: bool, extension: &str) -> &'static str {
        if !used {
            return "";
        }
        Self::language_source(extension)
    }

    fn language_source(extension: &str) -> &'static str {
        match extension {
            "js" => Self::JAVASCRIPT,
            "ts" => Self::TYPESCRIPT,
            "go" => Self::GO,
            "py" => Self::PYTHON,
            "rb" => Self::RUBY,
            "php" => Self::PHP,
            "java" => Self::JAVA,
            "kt" => Self::KOTLIN,
            "cs" => Self::CSHARP,
            "dart" => Self::DART,
            "rs" => Self::RUST,
            _ => "",
        }
    }
}
