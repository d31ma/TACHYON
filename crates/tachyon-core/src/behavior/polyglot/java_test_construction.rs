use super::super::testing::Unit;
use super::test_clients::TestClients;
use super::{Module, Type};

#[cfg(test)]
mod tests;

pub(super) struct JavaTestConstruction<'a> {
    module: &'a Module,
    owner: &'a Type,
    unit: &'a Unit,
}

impl<'a> JavaTestConstruction<'a> {
    pub(super) fn members(module: &'a Module, owner: &'a Type, unit: &'a Unit) -> String {
        Self {
            module,
            owner,
            unit,
        }
        .source()
    }

    pub(super) fn cleared_fakes(owner: &Type, unit: &Unit) -> String {
        let lines: Vec<String> = unit
            .mocks
            .iter()
            .map(|name| Self::cleared_fake(owner, name))
            .collect();
        lines.concat()
    }

    fn cleared_fake(owner: &Type, name: &str) -> String {
        let empty = Self::empty_value(owner, name);
        format!("this.{name} = {empty};\n")
    }

    fn empty_value(owner: &Type, name: &str) -> &'static str {
        let declared = owner.fields.iter().find(|field| field.name == name);
        let kind = declared.map_or("", |field| field.type_source.trim());
        match kind {
            "boolean" => "false",
            "byte" => "(byte) 0",
            "short" => "(short) 0",
            "char" => "(char) 0",
            "int" => "0",
            "long" => "0L",
            "float" => "0.0f",
            "double" => "0.0d",
            _ => "null",
        }
    }

    fn source(&self) -> String {
        format!(
            "\n<T> T construct(Class<T> type) {{\n\
             if (this.behaviors == null) throw new IllegalStateException(\"Test.construct needs an active test setup.\");\n\
             return type.cast(this.behaviors.operation(execution -> {{\n\
             {}\
             throw new IllegalArgumentException(\"Test.construct needs a class with a declared Needs constructor.\");\n\
             }}));\n\
             }}\n\
             private <T> T __tachyonAssignedFake(T value, String message) {{\n\
             if (value == null) throw new IllegalStateException(message);\n\
             return value;\n\
             }}\n",
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
        let body = self.arguments(target, fields).map_or_else(
            |message| {
                format!(
                    "throw new IllegalStateException({});",
                    Self::quoted(&message)
                )
            },
            |arguments| format!("return type.cast(new {}({arguments}));", target.name),
        );
        format!("if ({}.class.equals(type)) {{ {body} }}\n", target.name)
    }

    fn arguments(&self, target: &Type, fields: &[String]) -> Result<String, String> {
        let names =
            super::typed_test_fakes::TypedTestFakes::new(self.owner, self.unit, |kind| kind)
                .names(target, fields)?;
        Ok(names
            .into_iter()
            .map(|name| self.fake(target, name))
            .collect::<Vec<_>>()
            .join(", "))
    }

    fn fake(&self, target: &Type, name: &str) -> String {
        self.client_kind(target, name).map_or_else(
            || Self::assigned(name),
            |kind| format!("this.{name} != null ? this.{name} : new {kind}(this.behaviors)"),
        )
    }

    fn client_kind<'t>(&self, target: &'t Type, name: &str) -> Option<&'t str> {
        target
            .fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| field.type_source.trim())
            .filter(|kind| TestClients::is_client(self.module, kind))
    }

    fn assigned(name: &str) -> String {
        format!(
            "this.__tachyonAssignedFake(this.{name}, {})",
            Self::quoted(&format!(
                "Test.construct needs an assigned Fake named {name}."
            ))
        )
    }

    fn quoted(text: &str) -> String {
        serde_json::Value::String(text.to_owned()).to_string()
    }
}
