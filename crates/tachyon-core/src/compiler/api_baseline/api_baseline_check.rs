use super::{ApiDifference, ApplicationVersion};
use crate::failure::diagnostic;
use crate::{Failure, ProjectDiscovery};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tachyon_diagnostics::Diagnostic;

#[derive(Clone, Copy, Debug, Default)]
pub struct ApiBaselineCheck;

impl ApiBaselineCheck {
    const CODE: u16 = 1338;
    const OUTPUT_CODE: u16 = 1201;
    const BASELINE: &'static str = "server/api-baseline.json";
    const API_FILE: &'static str = "api.json";
    const VERSION_KEY: &'static str = "application_version";
    const HELP: &'static str = "Callers built against the last release still send and expect the old shape. Keep the old route, field or type and add the new one beside it. If the break is on purpose, raise the major number of tac.application.version in client/manifest.json, such as 1.4.0 to 2.0.0. After the release, run `ty api freeze` to save the new baseline (ADR 0074).";

    pub fn freeze(project_root: impl AsRef<Path>) -> Result<PathBuf, Failure> {
        let root = project_root.as_ref();
        let project = ProjectDiscovery::discover(root)?;
        let files = crate::handler::api_reference_files(project.route_graph())?;
        let mut api = Self::current(&files);
        api[Self::VERSION_KEY] = Value::String(ApplicationVersion::current(root));
        let path = root.join(Self::BASELINE);
        let text = format!("{api:#}\n");
        std::fs::write(&path, text).map_err(|error| Self::write_failure(&path, &error))?;
        Ok(path)
    }

    pub(crate) fn verify(project_root: &Path, files: &[(PathBuf, Vec<u8>)]) -> Result<(), Failure> {
        let baseline = Self::baseline(project_root);
        let version = ApplicationVersion::current(project_root);
        let breaks = Self::breaks(&baseline, &Self::current(files), &version);
        Self::result(
            breaks
                .iter()
                .map(String::as_str)
                .map(Self::diagnostic)
                .collect(),
        )
    }

    fn baseline(project_root: &Path) -> Value {
        std::fs::read(project_root.join(Self::BASELINE))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or(Value::Null)
    }

    fn current(files: &[(PathBuf, Vec<u8>)]) -> Value {
        let api = files
            .iter()
            .find(|(path, _)| path == Path::new(Self::API_FILE))
            .and_then(|(_, bytes)| serde_json::from_slice(bytes).ok());
        api.unwrap_or_else(|| serde_json::json!({ "routes": [], "models": {} }))
    }

    fn breaks(baseline: &Value, current: &Value, version: &str) -> Vec<String> {
        let frozen = baseline[Self::VERSION_KEY]
            .as_str()
            .unwrap_or(ApplicationVersion::DEFAULT);
        let raised = ApplicationVersion::major(version) > ApplicationVersion::major(frozen);
        if raised | baseline.is_null() {
            Vec::new()
        } else {
            ApiDifference::breaks(baseline, current)
        }
    }

    fn diagnostic(found: &str) -> Diagnostic {
        let message = format!(
            "The API breaks the last release in {}: {found}.",
            Self::BASELINE
        );
        diagnostic(Self::CODE, message, Some(String::from(Self::HELP)), None)
    }

    fn write_failure(path: &Path, error: &std::io::Error) -> Failure {
        let message = format!(
            "Cannot write the API baseline '{}': {error}",
            path.display()
        );
        Failure::one(diagnostic(Self::OUTPUT_CODE, message, None, None))
    }

    fn result(diagnostics: Vec<Diagnostic>) -> Result<(), Failure> {
        if diagnostics.is_empty() {
            Ok(())
        } else {
            Err(Failure::new(diagnostics))
        }
    }
}
