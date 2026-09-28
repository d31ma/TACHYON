use crate::Failure;
use crate::external_command::{ToolOutput, run as run_tool};
use crate::wasm_optimizer::WasmOptimizer;
use futures_util::future::OptionFuture;
use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;

pub(super) struct WebProgramBuild {
    pub(super) language: &'static str,
    pub(super) composed: &'static str,
    pub(super) optimized_note: &'static str,
    pub(super) unoptimized_note: &'static str,
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
        let mut command = tokio::process::Command::new(program);
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

    pub(super) fn note(
        &self,
        profile: crate::BuildProfile,
        made_smaller: bool,
    ) -> Option<&'static str> {
        let notes = [self.unoptimized_note, self.optimized_note];
        (profile == crate::BuildProfile::Release).then_some(notes[usize::from(made_smaller)])
    }

    pub(super) async fn release_optimizer(profile: crate::BuildProfile) -> Option<WasmOptimizer> {
        let release = profile == crate::BuildProfile::Release;
        OptionFuture::from(release.then(WasmOptimizer::find))
            .await
            .flatten()
    }

    pub(super) async fn optimize(
        optimizer: Option<&WasmOptimizer>,
        binary: &Path,
        flags: &[&str],
    ) -> bool {
        OptionFuture::from(optimizer.map(|optimizer| optimizer.run(binary, flags)))
            .await
            .unwrap_or(false)
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
