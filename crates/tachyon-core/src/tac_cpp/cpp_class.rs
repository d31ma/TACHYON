use super::{CppAttribute, CppMember};
use std::ops::Range;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CppClass {
    pub(crate) name: String,
    pub(crate) name_at: usize,
    pub(crate) markers: Vec<CppAttribute>,
    pub(crate) members: Vec<CppMember>,
    pub(crate) enumerators: Vec<String>,
    pub(crate) is_enum: bool,
    pub(crate) body: Range<usize>,
}

impl CppClass {
    pub(crate) fn every_marker(&self) -> Vec<&CppAttribute> {
        let members = self.members.iter().flat_map(|member| member.markers.iter());
        let parameters = self
            .members
            .iter()
            .flat_map(|member| member.parameters.iter())
            .flat_map(|parameter| parameter.markers.iter());
        self.markers
            .iter()
            .chain(members)
            .chain(parameters)
            .collect()
    }
}
