use super::{CppAttribute, CppClass, CppFailure, CppMember, CppMemberKind, CppValueSource};
use crate::Failure;
use std::path::Path;

pub(crate) struct CppTestCase {
    pub(crate) name: String,
    pub(crate) call: String,
}

impl CppTestCase {
    const PREFIX: &'static str = "test";
    const VALUE_SOURCE: &'static str = "ValueSource";
    const ONE_PARAMETER: &'static str = "A C++ test case with [[ValueSource]] takes the input as its one parameter, for example void testKeepsTheInput(int value)";

    pub(crate) fn read_all(relative: &Path, class: &CppClass) -> Result<Vec<Self>, Failure> {
        let each: Vec<Vec<Self>> = class
            .members
            .iter()
            .filter(|member| Self::is_case(member))
            .map(|member| Self::read(relative, member))
            .collect::<Result<_, _>>()?;
        Ok(each.into_iter().flatten().collect())
    }

    fn is_case(member: &CppMember) -> bool {
        [
            member.is_public_instance(CppMemberKind::Method),
            member.name.starts_with(Self::PREFIX),
        ] == [true, true]
    }

    fn read(relative: &Path, member: &CppMember) -> Result<Vec<Self>, Failure> {
        let marker = member
            .markers
            .iter()
            .find(|marker| marker.name == Self::VALUE_SOURCE);
        marker.map_or_else(
            || Ok(Self::plain(member)),
            |marker| Self::each_input(relative, member, marker),
        )
    }

    fn plain(member: &CppMember) -> Vec<Self> {
        if !member.parameters.is_empty() {
            return Vec::new();
        }
        vec![Self {
            name: member.name.clone(),
            call: format!("{}()", member.name),
        }]
    }

    fn each_input(
        relative: &Path,
        member: &CppMember,
        marker: &CppAttribute,
    ) -> Result<Vec<Self>, Failure> {
        (member.parameters.len() == 1)
            .then_some(())
            .ok_or_else(|| CppFailure::unreadable(relative, member.name_at, Self::ONE_PARAMETER))?;
        let inputs = CppValueSource::inputs(relative, marker)?;
        let mut cases = Vec::new();
        for (index, input) in inputs.iter().enumerate() {
            cases.push(Self {
                name: format!("{}[{}]", member.name, index + 1),
                call: format!("{}({input})", member.name),
            });
        }
        Ok(cases)
    }
}
