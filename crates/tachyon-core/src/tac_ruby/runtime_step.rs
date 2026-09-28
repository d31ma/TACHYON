use crate::Failure;
use crate::external_command::ToolOutput;
use crate::failure::diagnostic;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

pub(crate) struct RubyRuntimeStep;

impl RubyRuntimeStep {
    const FAILURE_CODE: u16 = 1605;
    const OUTPUT_LIMIT: usize = 8 * 1_024 * 1_024;
    const SHOWN_OUTPUT: usize = 4_000;

    pub(crate) async fn run(
        command: &mut Command,
        label: &str,
        limit: Duration,
    ) -> Result<(), Failure> {
        command.stdin(Stdio::null());
        let result = crate::external_command::run(command, limit, Self::OUTPUT_LIMIT)
            .await
            .map_err(|error| Self::failure(&format!("Cannot run {label}: {error}.")))?;
        Self::require_success(&result, label)
    }

    pub(crate) fn write_file(path: &Path, bytes: &[u8]) -> Result<(), Failure> {
        Self::create_parent(path)?;
        std::fs::write(path, bytes)
            .map_err(|error| Self::failure(&format!("Cannot write {}: {error}", path.display())))
    }

    pub(crate) fn read_file(path: &Path) -> Result<Vec<u8>, Failure> {
        std::fs::read(path)
            .map_err(|error| Self::failure(&format!("Cannot read {}: {error}", path.display())))
    }

    pub(crate) fn create_folder(path: &Path) -> Result<(), Failure> {
        std::fs::create_dir_all(path)
            .map_err(|error| Self::failure(&format!("Cannot create {}: {error}", path.display())))
    }

    pub(crate) fn work_folder(parent: &Path) -> Result<tempfile::TempDir, Failure> {
        Self::create_folder(parent)?;
        tempfile::Builder::new()
            .prefix(".build-")
            .tempdir_in(parent)
            .map_err(|error| {
                Self::failure(&format!(
                    "Cannot create a build folder in {}: {error}",
                    parent.display()
                ))
            })
    }

    pub(crate) fn publish(built: &Path, cached: &Path) -> Result<(), Failure> {
        Self::create_parent(cached)?;
        std::fs::rename(built, cached).map_err(|error| {
            Self::failure(&format!(
                "Cannot store the Ruby runtime in {}: {error}",
                cached.display()
            ))
        })
    }

    pub(crate) fn failure(message: &str) -> Failure {
        Failure::one(diagnostic(
            Self::FAILURE_CODE,
            message,
            Some(String::from(
                "Tac Ruby services run on a WebAssembly Ruby runtime that Tachyon builds once per machine. Run `ty doctor` to see which tools it needs.",
            )),
            None,
        ))
    }

    fn create_parent(path: &Path) -> Result<(), Failure> {
        let parent = path.parent().unwrap_or(path);
        Self::create_folder(parent)
    }

    fn require_success(result: &ToolOutput, label: &str) -> Result<(), Failure> {
        if result.status.success() {
            return Ok(());
        }
        let shown = Self::last_output(result);
        Err(Self::failure(&format!(
            "{label} failed ({}).\n{shown}",
            result.status
        )))
    }

    fn last_output(result: &ToolOutput) -> String {
        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);
        let text = format!("{stdout}{stderr}");
        let start = text.ceil_char_boundary(text.len().saturating_sub(Self::SHOWN_OUTPUT));
        text[start..].to_owned()
    }
}
