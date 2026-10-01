use super::super::testing::Unit;
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
            .map(Self::assigned)
            .collect::<Vec<_>>()
            .join(", "))
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
