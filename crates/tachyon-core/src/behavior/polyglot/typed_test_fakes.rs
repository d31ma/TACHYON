use super::super::testing::Unit;
use super::Type;

pub(super) struct TypedTestFakes<'a> {
    owner: &'a Type,
    unit: &'a Unit,
    normalize: fn(&str) -> &str,
}

impl<'a> TypedTestFakes<'a> {
    pub(super) fn new(owner: &'a Type, unit: &'a Unit, normalize: fn(&str) -> &str) -> Self {
        Self {
            owner,
            unit,
            normalize,
        }
    }

    pub(super) fn names<'b>(
        &self,
        target: &Type,
        fields: &'b [String],
    ) -> Result<Vec<&'b str>, String> {
        self.bindings(target, fields)
    }

    fn bindings<'b>(&self, target: &Type, fields: &'b [String]) -> Result<Vec<&'b str>, String> {
        fields
            .iter()
            .map(|name| self.binding(target, name))
            .collect()
    }

    fn binding<'b>(&self, target: &Type, name: &'b str) -> Result<&'b str, String> {
        let expected = target.fields.iter().find(|field| field.name == name);
        let actual = self.owner.fields.iter().find(|field| field.name == name);
        match (
            self.unit.mocks.iter().any(|field| field == name),
            expected,
            actual,
        ) {
            (true, Some(expected), Some(actual))
                if (self.normalize)(&expected.type_source)
                    == (self.normalize)(&actual.type_source) =>
            {
                Ok(name)
            }
            _ => Err(format!(
                "Test.construct needs a matching typed Fake named {name} for {}.",
                target.name
            )),
        }
    }
}
