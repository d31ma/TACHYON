use crate::Failure;
use std::path::{Path, PathBuf};

pub(super) struct NativeTypeProgram;

impl NativeTypeProgram {
    pub(super) fn render(
        services: &[(PathBuf, String)],
        text: String,
        service: impl Fn(&Path, &str) -> Result<String, Failure>,
    ) -> Result<String, Failure> {
        Self::render_services(services, text, service)
    }

    fn render_services(
        services: &[(PathBuf, String)],
        mut text: String,
        service: impl Fn(&Path, &str) -> Result<String, Failure>,
    ) -> Result<String, Failure> {
        for (path, source) in services {
            text.push_str(&service(path, source)?);
        }
        Ok(text)
    }
}
