use crate::external_command::run as run_tool;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub(crate) struct WasmOptimizer {
    program: PathBuf,
    version: Vec<u8>,
}

impl WasmOptimizer {
    const PROGRAM: &'static str = "wasm-opt";
    const EMSCRIPTEN_LAUNCHERS: [&'static str; 3] = ["emcc", "emcc.exe", "emcc.bat"];
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
        Self::location_in(&folders)
    }

    fn location_in(folders: &[PathBuf]) -> Option<PathBuf> {
        let on_path = folders
            .iter()
            .map(|folder| Self::program_in(folder))
            .find(|candidate| candidate.is_file());
        on_path.or_else(|| {
            folders
                .iter()
                .filter(|folder| Self::has_emscripten(folder))
                .map(|folder| Self::program_in(&folder.join(Self::EMSCRIPTEN_BINARYEN)))
                .find(|candidate| candidate.is_file())
        })
    }

    fn has_emscripten(folder: &Path) -> bool {
        Self::EMSCRIPTEN_LAUNCHERS
            .iter()
            .any(|launcher| folder.join(launcher).is_file())
    }

    fn program_in(folder: &Path) -> PathBuf {
        folder
            .join(Self::PROGRAM)
            .with_extension(std::env::consts::EXE_EXTENSION)
    }
}

#[cfg(test)]
mod tests {
    use super::WasmOptimizer;
    use std::path::Path;

    fn emsdk_with(launcher: &str) -> tempfile::TempDir {
        let emsdk = tempfile::tempdir().expect("emsdk");
        let emscripten = emsdk.path().join("upstream/emscripten");
        let binaryen = emsdk.path().join("upstream/bin");
        std::fs::create_dir_all(&emscripten).expect("emscripten folder");
        std::fs::create_dir_all(&binaryen).expect("binaryen folder");
        std::fs::write(emscripten.join(launcher), "").expect("launcher");
        std::fs::write(WasmOptimizer::program_in(&binaryen), "").expect("wasm-opt");
        emsdk
    }

    fn found_beside(launcher: &str) -> bool {
        let emsdk = emsdk_with(launcher);
        let emscripten = emsdk.path().join("upstream/emscripten");
        let found = WasmOptimizer::location_in(&[emscripten]);
        found.is_some_and(|program| program.starts_with(emsdk.path()))
    }

    #[test]
    fn binaryen_is_found_beside_each_emscripten_launcher() {
        for launcher in WasmOptimizer::EMSCRIPTEN_LAUNCHERS {
            assert!(found_beside(launcher), "{launcher}");
        }
    }

    #[test]
    fn a_folder_without_emscripten_or_binaryen_finds_nothing() {
        let empty = tempfile::tempdir().expect("empty");
        assert!(WasmOptimizer::location_in(&[empty.path().to_path_buf()]).is_none());
        assert!(!WasmOptimizer::has_emscripten(Path::new("/no/such/folder")));
    }
}
