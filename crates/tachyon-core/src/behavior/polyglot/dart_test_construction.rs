use super::super::testing::Unit;
use super::string_literal::StringLiteral;
use super::typed_test_fakes::TypedTestFakes;
use super::{Module, Type};

#[cfg(test)]
mod tests;

pub(super) struct DartTestConstruction<'a> {
    module: &'a Module,
    owner: &'a Type,
    unit: &'a Unit,
}

impl<'a> DartTestConstruction<'a> {
    pub(super) fn members(module: &'a Module, owner: &'a Type, unit: &'a Unit) -> String {
        Self {
            module,
            owner,
            unit,
        }
        .source()
    }

    fn source(&self) -> String {
        format!(
            "\n  T construct<T>(Type type) {{\n\
             \x20   if (behaviors == null) throw StateError('Test.construct needs an active test setup.');\n\
             {}\
             \x20   throw ArgumentError('Test.construct needs a class with a declared Needs constructor.');\n\
             \x20 }}\n\
             \x20 T _tachyonAssignedFake<T>(T? value, String message) {{\n\
             \x20   if (value == null) throw StateError(message);\n\
             \x20   return value;\n\
             \x20 }}\n",
            self.cases()
        )
    }

    fn cases(&self) -> String {
        self.module
            .types
            .iter()
            .filter_map(|target| target.constructor.as_ref().map(|fields| (target, fields)))
            .map(|(target, fields)| self.factory(target, fields))
            .collect()
    }

    fn factory(&self, target: &Type, fields: &[String]) -> String {
        let action = self.arguments(target, fields).map_or_else(
            |message| format!("throw StateError({});", StringLiteral::DART.of(&message)),
            |arguments| format!("return {}({arguments}) as T;", target.name),
        );
        format!("    if (type == {}) {{ {action} }}\n", target.name)
    }

    fn arguments(&self, target: &Type, fields: &[String]) -> Result<String, String> {
        let fakes = TypedTestFakes::new(self.owner, self.unit, Self::kind);
        Ok(fakes
            .names(target, fields)?
            .into_iter()
            .map(Self::assigned)
            .collect::<Vec<_>>()
            .join(", "))
    }

    fn assigned(name: &str) -> String {
        let message = StringLiteral::DART.of(&format!(
            "Test.construct needs an assigned Fake named {name}."
        ));
        format!("_tachyonAssignedFake(this.{name}, {message})")
    }

    fn kind(source: &str) -> &str {
        source.trim().trim_end_matches('?')
    }
}
