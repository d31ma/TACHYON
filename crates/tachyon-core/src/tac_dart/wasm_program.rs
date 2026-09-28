use super::failure;
use crate::Failure;
use crate::external_command::{ToolOutput, run as run_tool};
use std::path::Path;
use std::time::Duration;

pub(super) struct WasmProgram {
    binary: Vec<u8>,
    loader: String,
    support: String,
}

impl WasmProgram {
    pub(super) const SHARED_MODULE: &str = "tachyon-dart.js";
    const PUBLISHED_BINARY: &str = "tachyon-dart.wasm";
    const PUBLISHED_LOADER: &str = "tachyon-dart-loader.js";
    const BINARY: &str = "program.wasm";
    const LOADER: &str = "program.mjs";
    const SUPPORT: &str = "program.support.js";
    const ENTRY: &str = "main.dart";
    const CACHE_FOLDER: &str = "dart-wasm";
    const DEADLINE: Duration = Duration::from_mins(3);
    const VERSION_DEADLINE: Duration = Duration::from_secs(30);
    const OUTPUT_BYTES: usize = 64 * 1024;
    const SHARED_MODULE_TEMPLATE: &str = include_str!("shared_module.js");

    pub(super) async fn compile(
        libraries: &[(String, String)],
        portable: &str,
    ) -> Result<Self, Failure> {
        let cached = Self::cached_folder(libraries, portable).await?;
        Self::cached_or_fresh(libraries, portable, &cached).await
    }

    async fn cached_or_fresh(
        libraries: &[(String, String)],
        portable: &str,
        cached: &Path,
    ) -> Result<Self, Failure> {
        if let Ok(program) = Self::read(cached) {
            return Ok(program);
        }
        Self::compile_fresh(libraries, portable, cached).await
    }

    async fn cached_folder(
        libraries: &[(String, String)],
        portable: &str,
    ) -> Result<std::path::PathBuf, Failure> {
        let mut version = tokio::process::Command::new("dart");
        version.arg("--version").stdin(std::process::Stdio::null());
        let version = run_tool(&mut version, Self::VERSION_DEADLINE, Self::OUTPUT_BYTES)
            .await
            .map_err(|error| {
                failure(
                    portable,
                    format!(
                        "cannot start the Dart SDK: {error}. Install Dart and put `dart` on PATH"
                    ),
                )
            })?;
        let key = Self::cache_key(&version.stdout, &version.stderr, libraries);
        Ok(Self::cache().join(key))
    }

    async fn compile_fresh(
        libraries: &[(String, String)],
        portable: &str,
        cached: &Path,
    ) -> Result<Self, Failure> {
        let staged = Self::stage(libraries, portable)?;
        Self::run_compiler(staged.path(), portable).await?;
        let program = Self::read(staged.path()).map_err(|error| failure(portable, error))?;
        let _ = Self::save(staged.path(), cached);
        Ok(program)
    }

    fn cache() -> std::path::PathBuf {
        crate::cache::root().join(Self::CACHE_FOLDER)
    }

    pub(super) fn published(self) -> Vec<(String, Vec<u8>)> {
        let shared = Self::SHARED_MODULE_TEMPLATE
            .replace("__SUPPORT__", self.support.trim())
            .replace("__LOADER__", Self::PUBLISHED_LOADER)
            .replace("__BINARY__", Self::PUBLISHED_BINARY);
        vec![
            (Self::SHARED_MODULE.to_owned(), shared.into_bytes()),
            (Self::PUBLISHED_LOADER.to_owned(), self.loader.into_bytes()),
            (Self::PUBLISHED_BINARY.to_owned(), self.binary),
        ]
    }

    fn read(folder: &Path) -> std::io::Result<Self> {
        Ok(Self {
            binary: std::fs::read(folder.join(Self::BINARY))?,
            loader: std::fs::read_to_string(folder.join(Self::LOADER))?,
            support: std::fs::read_to_string(folder.join(Self::SUPPORT))?,
        })
    }

    fn cache_key(
        version_stdout: &[u8],
        version_stderr: &[u8],
        libraries: &[(String, String)],
    ) -> String {
        let mut material = [version_stdout, version_stderr].concat();
        for (name, contents) in libraries {
            material.push(0);
            material.extend_from_slice(name.as_bytes());
            material.push(0);
            material.extend_from_slice(contents.as_bytes());
        }
        crate::compiler::hex_digest(material)
    }

    fn stage(libraries: &[(String, String)], portable: &str) -> Result<tempfile::TempDir, Failure> {
        let staged = tempfile::Builder::new()
            .prefix(".tachyon-dart-")
            .tempdir()
            .map_err(|error| failure(portable, error))?;
        for (name, contents) in libraries {
            Self::write_library(staged.path(), name, contents)
                .map_err(|error| failure(portable, error))?;
        }
        Ok(staged)
    }

    fn write_library(staged: &Path, name: &str, contents: &str) -> std::io::Result<()> {
        let path = staged.join(name);
        std::fs::create_dir_all(path.parent().unwrap_or(staged))?;
        std::fs::write(path, contents)
    }

    async fn run_compiler(staged: &Path, portable: &str) -> Result<(), Failure> {
        let mut command = tokio::process::Command::new("dart");
        command
            .args([
                "compile",
                "wasm",
                "-O2",
                "--no-source-maps",
                "-o",
                Self::BINARY,
                Self::ENTRY,
            ])
            .current_dir(staged)
            .stdin(std::process::Stdio::null());
        let output = run_tool(&mut command, Self::DEADLINE, Self::OUTPUT_BYTES)
            .await
            .map_err(|error| failure(portable, error))?;
        if output.status.success() {
            return Ok(());
        }
        Err(Self::compile_failure(&output, staged, portable))
    }

    fn compile_failure(output: &ToolOutput, staged: &Path, portable: &str) -> Failure {
        let detail = String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr);
        let detail = detail.replace(&staged.to_string_lossy().into_owned(), "");
        failure(
            portable,
            format!(
                "{}\n(line numbers are in the composed library of the service named)",
                detail.trim()
            ),
        )
    }

    fn save(staged: &Path, cached: &Path) -> std::io::Result<()> {
        let cache = Self::cache();
        std::fs::create_dir_all(&cache)?;
        let copy = tempfile::Builder::new()
            .prefix(".saving-")
            .tempdir_in(&cache)?;
        for name in [Self::BINARY, Self::LOADER, Self::SUPPORT] {
            std::fs::copy(staged.join(name), copy.path().join(name))?;
        }
        let kept = copy.keep();
        std::fs::rename(&kept, cached).or_else(|_| std::fs::remove_dir_all(&kept))
    }
}
