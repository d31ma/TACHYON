use super::super::testing::Unit;
use super::test_clients::TestClients;
use super::{Field, Module, Type};

pub(super) struct GoTestConstruction<'a> {
    module: &'a Module,
    owner: &'a Type,
    unit: &'a Unit,
}

impl<'a> GoTestConstruction<'a> {
    const RECEIVER: &'static str = "__tachyon_receiver";
    const POINTER: char = '*';

    pub(super) fn members(module: &'a Module, owner: &'a Type, unit: &'a Unit) -> String {
        let construction = Self {
            module,
            owner,
            unit,
        };
        let cases = module
            .types
            .iter()
            .filter(|target| construction.constructible(target))
            .map(|target| construction.case(target))
            .collect::<String>();
        format!(
            "\nfunc ({} *{}) construct(target any) {{\n\tswitch service := target.(type) {{{cases}\n\t}}\n}}\n",
            Self::RECEIVER,
            owner.name
        )
    }

    fn constructible(&self, target: &Type) -> bool {
        [
            !target.declared,
            target.name != self.owner.name,
            !TestClients::sends_requests(target),
        ] == [true; 3]
    }

    fn case(&self, target: &Type) -> String {
        let assignments = target
            .fields
            .iter()
            .filter(|field| self.faked(field))
            .map(|field| self.assignment(field))
            .collect::<String>();
        format!("\n\tcase *{}:\n\t\t_ = service{assignments}", target.name)
    }

    fn faked(&self, field: &Field) -> bool {
        let fake = self
            .owner
            .fields
            .iter()
            .find(|candidate| candidate.name == field.name)
            .map(|candidate| candidate.type_source.trim());
        [
            self.unit.mocks.contains(&field.name),
            fake == Some(field.type_source.trim()),
        ] == [true, true]
    }

    fn assignment(&self, field: &Field) -> String {
        let name = &field.name;
        let receiver = Self::RECEIVER;
        let kind = field.type_source.trim();
        let client = kind
            .strip_prefix(Self::POINTER)
            .filter(|client| TestClients::is_client(self.module, client))
            .map(|client| {
                format!(
                    "\n\t\tif service.{name} == nil {{\n\t\t\tservice.{name} = New{client}With({receiver}.behaviors)\n\t\t}}"
                )
            })
            .unwrap_or_default();
        format!("\n\t\tservice.{name} = {receiver}.{name}{client}")
    }
}
