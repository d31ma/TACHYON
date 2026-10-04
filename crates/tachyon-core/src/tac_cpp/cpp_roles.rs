use super::CppFile;
use crate::Failure;
use std::path::Path;

pub(crate) struct CppRoles;

impl CppRoles {
    const ROLES: [&'static str; 11] = [
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
        "ErrorHandler",
    ];

    pub(crate) fn has_role(path: &Path, source: &str) -> Result<bool, Failure> {
        let file = CppFile::read(path, source)?;
        Ok(file
            .class
            .markers
            .iter()
            .any(|marker| Self::ROLES.contains(&marker.name.as_str())))
    }
}
