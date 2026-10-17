use crate::Failure;
use crate::external_command::{ToolOutput, run as run_tool};
use crate::wasm_release_note::WasmReleaseNote;
use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;

pub(super) struct WebProgramBuild {
    pub(super) language: &'static str,
    pub(super) composed: &'static str,
    pub(super) note: WasmReleaseNote,
}

impl WebProgramBuild {
    const DEADLINE: Duration = Duration::from_mins(5);
    const VERSION_DEADLINE: Duration = Duration::from_mins(1);
    const OUTPUT_BYTES: usize = 64 * 1024;
    const FAILURE_CODE: u16 = 1605;

    pub(super) async fn tool_version(
        &self,
        program: &OsStr,
        flag: &str,
        install_help: &str,
    ) -> Result<Vec<u8>, Failure> {
        let mut command =
            tokio::process::Command::from(crate::kotlin_home::KotlinHome::command(program));
        command.arg(flag).stdin(std::process::Stdio::null());
        let output = run_tool(&mut command, Self::VERSION_DEADLINE, Self::OUTPUT_BYTES)
            .await
            .map_err(|error| {
                self.failure(&format!(
                    "cannot start {}: {error}. {install_help}",
                    program.to_string_lossy()
                ))
            })?;
        Ok([output.stdout, output.stderr].concat())
    }

    pub(super) async fn run(
        &self,
        command: &mut tokio::process::Command,
        staged: &Path,
    ) -> Result<(), Failure> {
        let output = run_tool(command, Self::DEADLINE, Self::OUTPUT_BYTES)
            .await
            .map_err(|error| self.failure(&error.to_string()))?;
        output
            .status
            .success()
            .then_some(())
            .ok_or_else(|| self.compile_failure(&output, staged))
    }

    pub(super) fn failure(&self, detail: &str) -> Failure {
        super::host::native_tool_failure(
            Self::FAILURE_CODE,
            &format!("Cannot build the {} web services: {detail}", self.language),
        )
    }

    pub(super) fn save(built: &Path, names: &[&str], cached: &Path) -> std::io::Result<()> {
        let cache = cached.parent().unwrap_or(cached);
        std::fs::create_dir_all(cache)?;
        let copy = tempfile::Builder::new()
            .prefix(".saving-")
            .tempdir_in(cache)?;
        names.iter().try_for_each(|name| {
            std::fs::copy(built.join(name), copy.path().join(name)).map(drop)
        })?;
        let kept = copy.keep();
        std::fs::rename(&kept, cached).or_else(|_| std::fs::remove_dir_all(&kept))
    }

    fn compile_failure(&self, output: &ToolOutput, staged: &Path) -> Failure {
        let detail = String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr);
        let detail = detail.replace(&format!("{}/", staged.to_string_lossy()), "");
        self.failure(&format!(
            "{}\n(line numbers are in the composed {} that Tachyon builds from every {} web service)",
            detail.trim(),
            self.composed,
            self.language
        ))
    }
}
