use super::super::testing::Unit;
use super::typed_test_fakes::TypedTestFakes;
use super::{Module, Type};

#[cfg(test)]
mod tests;

pub(super) struct KotlinTestConstruction;

impl KotlinTestConstruction {
    pub(super) fn members(module: &Module, owner: &Type, unit: &Unit) -> String {
        let fakes = TypedTestFakes::new(owner, unit, Self::kind);
        Self::source(&Self::cases(module, &fakes))
    }

    fn source(cases: &str) -> String {
        format!(
            "\nfun <T : Any> construct(type: kotlin.reflect.KClass<T>): T {{\n\
             check(behaviors != null) {{ \"Test.construct needs an active test setup.\" }}\n\
             {cases}\
             throw IllegalArgumentException(\"Test.construct needs a class with a declared Needs constructor.\")\n\
             }}\n"
        )
    }

    fn cases(module: &Module, fakes: &TypedTestFakes<'_>) -> String {
        module
            .types
            .iter()
            .filter_map(|target| Self::construction(target, fakes))
            .collect()
    }

    fn construction(target: &Type, fakes: &TypedTestFakes<'_>) -> Option<String> {
        let fields = target.constructor.as_ref()?;
        let values = fakes.names(target, fields).map(Self::arguments);
        Some(Self::factory(&target.name, values))
    }

    fn factory(name: &str, arguments: Result<String, String>) -> String {
        let action = arguments.map_or_else(
            |error| format!("throw IllegalStateException({});", Self::quoted(&error)),
            |values| format!("return type.java.cast({name}({values}))"),
        );
        format!("if (type == {name}::class) {{ {action} }}\n")
    }

    fn arguments(names: Vec<&str>) -> String {
        names
            .into_iter()
            .map(Self::assigned)
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn assigned(name: &str) -> String {
        format!(
            "checkNotNull(this.{name}) {{ {} }}",
            Self::quoted(&format!(
                "Test.construct needs an assigned Fake named {name}."
            ))
        )
    }

    fn kind(source: &str) -> &str {
        source.trim().trim_end_matches('?')
    }

    fn quoted(text: &str) -> String {
        serde_json::Value::String(text.to_owned())
            .to_string()
            .replace('$', "\\$")
    }
}
