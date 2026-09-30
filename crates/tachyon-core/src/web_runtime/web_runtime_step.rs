use crate::Failure;
use crate::external_command::ToolOutput;
use crate::failure::diagnostic;
use sha2::{Digest, Sha256};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

#[derive(Clone, Copy)]
pub(crate) struct WebRuntimeStep {
    language: &'static str,
}

impl WebRuntimeStep {
    const FAILURE_CODE: u16 = 1605;
    const OUTPUT_LIMIT: usize = 8 * 1_024 * 1_024;
    const SHOWN_OUTPUT: usize = 4_000;

    pub(crate) const fn new(language: &'static str) -> Self {
        Self { language }
    }

    pub(crate) async fn run(
        &self,
        command: &mut Command,
        label: &str,
        limit: Duration,
    ) -> Result<(), Failure> {
        command.stdin(Stdio::null());
        let result = crate::external_command::run(command, limit, Self::OUTPUT_LIMIT)
            .await
            .map_err(|error| self.failure(&format!("Cannot run {label}: {error}.")))?;
        self.require_success(&result, label)
    }

    pub(crate) fn write_file(&self, path: &Path, bytes: &[u8]) -> Result<(), Failure> {
        self.create_parent(path)?;
        std::fs::write(path, bytes)
            .map_err(|error| self.failure(&format!("Cannot write {}: {error}", path.display())))
    }

    pub(crate) fn fingerprint(files: &[(&str, &str)]) -> String {
        let mut hasher = Sha256::new();
        for (name, text) in files {
            hasher.update(name.as_bytes());
            hasher.update(text.len().to_le_bytes());
            hasher.update(text.as_bytes());
        }
        crate::compiler::hex_digest(hasher.finalize())
    }

    pub(crate) fn write_files(&self, folder: &Path, files: &[(&str, &str)]) -> Result<(), Failure> {
        for (name, text) in files {
            self.write_file(&folder.join(name), text.as_bytes())?;
        }
        Ok(())
    }

    pub(crate) fn read_file(&self, path: &Path) -> Result<Vec<u8>, Failure> {
        std::fs::read(path)
            .map_err(|error| self.failure(&format!("Cannot read {}: {error}", path.display())))
    }

    pub(crate) fn create_folder(&self, path: &Path) -> Result<(), Failure> {
        std::fs::create_dir_all(path)
            .map_err(|error| self.failure(&format!("Cannot create {}: {error}", path.display())))
    }

    pub(crate) fn work_folder(&self, parent: &Path) -> Result<tempfile::TempDir, Failure> {
        self.create_folder(parent)?;
        tempfile::Builder::new()
            .prefix(".build-")
            .tempdir_in(parent)
            .map_err(|error| {
                self.failure(&format!(
                    "Cannot create a build folder in {}: {error}",
                    parent.display()
                ))
            })
    }

    pub(crate) fn publish(&self, built: &Path, cached: &Path) -> Result<(), Failure> {
        self.create_parent(cached)?;
        std::fs::rename(built, cached).map_err(|error| {
            self.failure(&format!(
                "Cannot store the {} runtime in {}: {error}",
                self.language,
                cached.display()
            ))
        })
    }

    pub(crate) fn failure(&self, message: &str) -> Failure {
        Failure::one(diagnostic(
            Self::FAILURE_CODE,
            message,
            Some(format!(
                "Tac {0} services run on a WebAssembly {0} runtime that Tachyon builds once per machine. Run `ty doctor` to see which tools it needs.",
                self.language
            )),
            None,
        ))
    }

    fn create_parent(&self, path: &Path) -> Result<(), Failure> {
        let parent = path.parent().unwrap_or(path);
        self.create_folder(parent)
    }

    fn require_success(&self, result: &ToolOutput, label: &str) -> Result<(), Failure> {
        if result.status.success() {
            return Ok(());
        }
        let shown = Self::last_output(result);
        Err(self.failure(&format!("{label} failed ({}).\n{shown}", result.status)))
    }

    fn last_output(result: &ToolOutput) -> String {
        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);
        let text = format!("{stdout}{stderr}");
        let start = text.ceil_char_boundary(text.len().saturating_sub(Self::SHOWN_OUTPUT));
        text[start..].to_owned()
    }
}
