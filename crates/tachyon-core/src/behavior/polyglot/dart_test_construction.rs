use super::super::testing::Unit;
use super::string_literal::StringLiteral;
use super::test_clients::TestClients;
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
             \x20   if (type != T) throw ArgumentError('Test.construct needs a target matching its type argument.');\n\
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
            .map(|name| self.fake(target, name))
            .collect::<Vec<_>>()
            .join(", "))
    }

    fn fake(&self, target: &Type, name: &str) -> String {
        self.client_of(target, name).map_or_else(
            || Self::assigned(name),
            |client| Self::defaulted(name, &client),
        )
    }

    fn client_of(&self, target: &Type, name: &str) -> Option<String> {
        target
            .fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| Self::kind(&field.type_source).to_owned())
            .filter(|client| TestClients::is_client(self.module, client))
    }

    fn defaulted(name: &str, client: &str) -> String {
        format!("this.{name} ?? {client}(behaviors!)")
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
