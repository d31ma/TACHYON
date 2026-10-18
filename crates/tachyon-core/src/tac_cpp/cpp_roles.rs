use super::{CppFile, CppLifetime, CppMember};
use crate::Failure;
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) struct CppRoles;

impl CppRoles {
    const ROLES: [&'static str; 13] = [
        "Service",
        "Store",
        "HTTPClient",
        "Helper",
        "Converter",
        "Setup",
        "Entity",
        "Test",
        "TestSetup",
        "RequestRoute",
        "SocketRoute",
        "ErrorHandler",
        "IdentityProvider",
    ];

    pub(crate) fn has_role(path: &Path, source: &str) -> Result<bool, Failure> {
        let file = CppFile::read(path, source)?;
        Ok(file
            .class
            .markers
            .iter()
            .any(|marker| Self::ROLES.contains(&marker.name.as_str())))
    }

    pub(crate) fn either_role(
        path: &Path,
        source: &str,
        marker: &str,
        other: impl Fn(&Path, &str, &str) -> Result<bool, Failure>,
    ) -> Result<bool, Failure> {
        Self::routed_role(super::CppSource::is_cpp(path), path, source, marker, other)
    }

    fn routed_role(
        cpp: bool,
        path: &Path,
        source: &str,
        marker: &str,
        other: impl Fn(&Path, &str, &str) -> Result<bool, Failure>,
    ) -> Result<bool, Failure> {
        if cpp {
            return Self::has_marker(path, source, marker);
        }
        other(path, source, marker)
    }

    fn has_marker(path: &Path, source: &str, marker: &str) -> Result<bool, Failure> {
        let file = CppFile::read(path, source)?;
        Ok(file.class.markers.iter().any(|found| found.name == marker))
    }

    pub(crate) fn service_roles(
        path: &Path,
        source: &str,
    ) -> Result<crate::behavior::ServiceRoles, Failure> {
        let file = CppFile::read(path, source)?;
        let needs_all = CppLifetime::needs_all(&file.class);
        let required = std::iter::once(String::from("*"))
            .filter(|_| needs_all)
            .collect();
        Ok(crate::behavior::ServiceRoles::declared(
            source,
            CppLifetime::is_prototype(&file.class),
            required,
        ))
    }

    pub(crate) fn props(path: &Path, source: &str) -> Vec<String> {
        CppFile::read(path, source).map_or_else(
            |_| Vec::new(),
            |file| {
                CppLifetime::props(&file.class)
                    .into_iter()
                    .map(|member| crate::companion::camel_case(&member.name))
                    .collect()
            },
        )
    }

    pub(crate) fn stored(path: &Path, source: &str) -> Result<BTreeMap<String, String>, Failure> {
        let file = CppFile::read(path, source)?;
        Ok(file
            .class
            .members
            .iter()
            .filter_map(|member| {
                Self::area(member)
                    .map(|area| (crate::companion::camel_case(&member.name), area.to_owned()))
            })
            .collect())
    }

    fn area(member: &CppMember) -> Option<&'static str> {
        [("LocalState", "local"), ("SessionState", "session")]
            .into_iter()
            .find(|(marker, _)| member.marked(marker))
            .map(|(_, area)| area)
    }
}
