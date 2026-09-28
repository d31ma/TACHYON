use crate::external_command::run as run_tool;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub(crate) struct WasmOptimizer {
    program: PathBuf,
    version: Vec<u8>,
}

impl WasmOptimizer {
    const PROGRAM: &'static str = "wasm-opt";
    const EMSCRIPTEN: &'static str = "emcc";
    const EMSCRIPTEN_BINARYEN: &'static str = "../bin";
    const DEADLINE: Duration = Duration::from_mins(2);
    const OUTPUT_BYTES: usize = 64 * 1024;

    pub(crate) async fn find() -> Option<Self> {
        let program = Self::location()?;
        let mut command = tokio::process::Command::new(&program);
        command.arg("--version").stdin(std::process::Stdio::null());
        let output = run_tool(&mut command, Self::DEADLINE, Self::OUTPUT_BYTES)
            .await
            .ok()?;
        output.status.success().then_some(Self {
            program,
            version: output.stdout,
        })
    }

    pub(crate) fn is_installed() -> bool {
        Self::location().is_some()
    }

    pub(crate) fn identity(&self) -> &[u8] {
        &self.version
    }

    pub(crate) async fn run(&self, binary: &Path, flags: &[&str]) -> bool {
        let smaller_copy = binary.with_extension("optimized.wasm");
        let mut command = tokio::process::Command::new(&self.program);
        command
            .arg(binary)
            .args(flags)
            .arg("-o")
            .arg(&smaller_copy)
            .stdin(std::process::Stdio::null());
        let finished = run_tool(&mut command, Self::DEADLINE, Self::OUTPUT_BYTES).await;
        Self::replaced(
            finished.is_ok_and(|output| output.status.success()),
            &smaller_copy,
            binary,
        )
    }

    fn replaced(succeeded: bool, smaller_copy: &Path, binary: &Path) -> bool {
        succeeded && std::fs::rename(smaller_copy, binary).is_ok()
    }

    fn location() -> Option<PathBuf> {
        let folders: Vec<PathBuf> = std::env::var_os("PATH")
            .map(|path| std::env::split_paths(&path).collect())
            .unwrap_or_default();
        let on_path = folders
            .iter()
            .map(|folder| Self::program_in(folder))
            .find(|candidate| candidate.is_file());
        on_path.or_else(|| {
            folders
                .iter()
                .filter(|folder| folder.join(Self::EMSCRIPTEN).is_file())
                .map(|folder| Self::program_in(&folder.join(Self::EMSCRIPTEN_BINARYEN)))
                .find(|candidate| candidate.is_file())
        })
    }

    fn program_in(folder: &Path) -> PathBuf {
        folder
            .join(Self::PROGRAM)
            .with_extension(std::env::consts::EXE_EXTENSION)
    }
}
