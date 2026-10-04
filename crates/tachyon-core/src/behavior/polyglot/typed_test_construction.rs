use super::super::testing::Unit;
use super::test_clients::TestClients;
use super::typed_test_fakes::TypedTestFakes;
use super::{Module, Type};

pub(super) struct TypedTestConstruction;

impl TypedTestConstruction {
    const NO_SETUP: &'static str = "Test.construct needs an active test setup.";

    pub(super) fn members(module: &Module, owner: &Type, unit: &Unit, language: &str) -> String {
        Self::compose(module, owner, unit, language)
    }

    fn compose(module: &Module, owner: &Type, unit: &Unit, language: &str) -> String {
        let fakes = TypedTestFakes::new(owner, unit, Self::kind);
        let cases = Self::cases(module, &fakes, language);
        Self::source(&cases, language)
    }

    fn source(cases: &str, language: &str) -> String {
        match language {
            "cs" => format!(
                "\npublic T construct<T>() where T : class {{\n\
             if (this.behaviors == null) throw new System.InvalidOperationException(\"Test.construct needs an active test setup.\");\n\
             {cases}\
             throw new System.ArgumentException(\"Test.construct needs a class with a declared Needs constructor.\");\n\
             }}\n\
             private T __tachyonAssignedFake<T>(T value, string message) {{\n\
             if (value is null) throw new System.InvalidOperationException(message);\n\
             return value;\n\
             }}\n"
            ),
            _ => format!(
                "\nfun <T : Any> construct(type: kotlin.reflect.KClass<T>): T {{\n\
             check(behaviors != null) {{ \"Test.construct needs an active test setup.\" }}\n\
             {cases}\
             throw IllegalArgumentException(\"Test.construct needs a class with a declared Needs constructor.\")\n\
             }}\n"
            ),
        }
    }

    fn cases(module: &Module, fakes: &TypedTestFakes<'_>, language: &str) -> String {
        module
            .types
            .iter()
            .filter_map(|target| Self::construction(module, target, fakes, language))
            .collect()
    }

    fn construction(
        module: &Module,
        target: &Type,
        fakes: &TypedTestFakes<'_>,
        language: &str,
    ) -> Option<String> {
        let fields = target.constructor.as_ref()?;
        let arguments = fakes
            .names(target, fields)
            .map(|names| Self::arguments(module, target, names, language));
        Some(Self::factory(&target.name, arguments, language))
    }

    fn factory(name: &str, arguments: Result<String, String>, language: &str) -> String {
        let action = arguments.map_or_else(
            |error| Self::refusal(&error, language),
            |values| Self::creation(name, &values, language),
        );
        Self::selection(name, &action, language)
    }

    fn refusal(error: &str, language: &str) -> String {
        let message = Self::quoted(error, language);
        Self::refusal_source(&message, language)
    }

    fn refusal_source(message: &str, language: &str) -> String {
        match language {
            "cs" => format!("throw new System.InvalidOperationException({message});"),
            _ => format!("throw IllegalStateException({message});"),
        }
    }

    fn creation(name: &str, values: &str, language: &str) -> String {
        match language {
            "cs" => format!(
                "return new {name}({values}) as T ?? throw new System.InvalidOperationException(\"Test.construct type mismatch.\");"
            ),
            _ => format!("return type.java.cast({name}({values}))"),
        }
    }

    fn selection(name: &str, action: &str, language: &str) -> String {
        match language {
            "cs" => format!("if (typeof(T) == typeof({name})) {{ {action} }}\n"),
            _ => format!("if (type == {name}::class) {{ {action} }}\n"),
        }
    }

    fn arguments(module: &Module, target: &Type, names: Vec<&str>, language: &str) -> String {
        names
            .into_iter()
            .map(|name| {
                Self::kotlin_client(module, target, name, language)
                    .unwrap_or_else(|| Self::assigned(name, language))
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn kotlin_client(module: &Module, target: &Type, name: &str, language: &str) -> Option<String> {
        let kind = target
            .fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| Self::kind(&field.type_source))
            .filter(|kind| [language == "kt", TestClients::is_client(module, kind)] == [true; 2])?;
        let message = Self::quoted(Self::NO_SETUP, language);
        Some(format!(
            "(this.{name} ?: {kind}(checkNotNull(this.behaviors) {{ {message} }}))"
        ))
    }

    fn assigned(name: &str, language: &str) -> String {
        let message = Self::quoted(
            &format!("Test.construct needs an assigned Fake named {name}."),
            language,
        );
        Self::assigned_source(name, &message, language)
    }

    fn assigned_source(name: &str, message: &str, language: &str) -> String {
        match language {
            "cs" => format!("this.__tachyonAssignedFake(this.{name}, {message})"),
            _ => format!("checkNotNull(this.{name}) {{ {message} }}"),
        }
    }

    fn kind(source: &str) -> &str {
        source.trim().trim_end_matches('?')
    }

    fn quoted(text: &str, language: &str) -> String {
        let quoted = serde_json::Value::String(text.to_owned()).to_string();
        match language {
            "kt" => quoted.replace('$', "\\$"),
            _ => quoted,
        }
    }
}
