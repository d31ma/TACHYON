use super::rust_imports::RustImports;
use crate::Failure;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct RustServiceScopes {
    services: BTreeMap<String, (PathBuf, RustImports)>,
}

impl RustServiceScopes {
    const START: &'static str = "//__TACHYON_SERVICE_START__";
    const END: &'static str = "//__TACHYON_SERVICE_END__";

    pub(super) fn mark(
        &mut self,
        path: &Path,
        source: &str,
        name: &str,
    ) -> Result<String, Failure> {
        let mut imports = RustImports::default();
        let body = imports.take(path, source)?;
        self.services
            .insert(name.to_owned(), (path.to_path_buf(), imports));
        Ok(format!("{}{body}\n{}", Self::start(name), Self::end(name)))
    }

    pub(super) fn wrap(
        &self,
        source: &str,
        generated: &[(String, String)],
    ) -> Result<String, Failure> {
        let mut wrapped = source.to_owned();
        self.services
            .iter()
            .try_for_each(|(name, (path, imports))| {
                let code = Self::generated_for(name, generated);
                wrapped = Self::wrap_service(&wrapped, (name, path), imports, &code)?;
                Ok::<(), Failure>(())
            })?;
        Ok(wrapped)
    }

    fn generated_for(name: &str, generated: &[(String, String)]) -> String {
        generated
            .iter()
            .filter(|(owner, _)| owner == name)
            .map(|(_, code)| code.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn wrap_service(
        source: &str,
        service: (&str, &Path),
        imports: &RustImports,
        generated: &str,
    ) -> Result<String, Failure> {
        let (name, path) = service;
        let start = Self::start(name);
        let end = Self::end(name);
        let (before, rest) = source.split_once(&start).ok_or_else(Self::missing)?;
        let (body, after) = rest.split_once(&end).ok_or_else(Self::missing)?;
        let module = imports.service_module(path, body, generated, name)?;
        Ok(format!("{before}{module}{after}"))
    }

    fn start(name: &str) -> String {
        format!("{}{name}\n", Self::START)
    }

    fn end(name: &str) -> String {
        format!("{}{name}\n", Self::END)
    }

    fn missing() -> Failure {
        super::host::native_tool_failure(1406, "Native Rust service scope marker is missing.")
    }
}
