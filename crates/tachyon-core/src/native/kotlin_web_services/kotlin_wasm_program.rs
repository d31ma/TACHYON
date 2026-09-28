use crate::Failure;
use crate::external_command::{ToolOutput, run as run_tool};
use crate::wasm_optimizer::WasmOptimizer;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub(super) struct KotlinWasmProgram {
    binary: Vec<u8>,
    entry: String,
    loader: String,
    made_smaller: bool,
}

impl KotlinWasmProgram {
    const STANDARD_LIBRARY: &'static str = "TAC_KOTLIN_WASM_STDLIB";
    const COMPILER: &'static str = "kotlinc-js";
    const TARGET: [&'static str; 2] = ["-Xwasm", "-Xwasm-target=wasm-js"];
    const LIBRARY_FOLDER: &'static str = "library";
    const OUTPUT_FOLDER: &'static str = "program";
    const NAME: &'static str = "services";
    const BINARY: &'static str = "services.wasm";
    const ENTRY: &'static str = "services.mjs";
    const LOADER: &'static str = "services.uninstantiated.mjs";
    const PUBLISHED_BINARY: &'static str = "kotlin/services.wasm";
    const PUBLISHED_ENTRY: &'static str = "kotlin/services.js";
    const PUBLISHED_LOADER: &'static str = "kotlin/services-loader.js";
    const PUBLISHED_LOADER_IMPORT: &'static str = "./services-loader.js";
    const LOADER_IMPORT: &'static str = "./services.uninstantiated.mjs";
    const CACHE_FOLDER: &'static str = "kotlin-wasm";
    const DEADLINE: Duration = Duration::from_mins(5);
    const VERSION_DEADLINE: Duration = Duration::from_mins(1);
    const OUTPUT_BYTES: usize = 64 * 1024;
    const FAILURE_CODE: u16 = 1605;
    const PRODUCTION_FLAGS: [&'static str; 10] = [
        "--enable-gc",
        "--enable-reference-types",
        "--enable-exception-handling",
        "--enable-bulk-memory",
        "--enable-nontrapping-float-to-int",
        "--closed-world",
        "-O3",
        "--gufa",
        "-O3",
        "--type-merging",
    ];
    const OPTIMIZED_NOTE: &'static str =
        "Kotlin: Kotlin/Wasm production build — optimized by Binaryen";
    const UNOPTIMIZED_NOTE: &'static str = "Kotlin: shipped without Binaryen optimization (≈3× larger); install Binaryen or run `ty doctor`";

    pub(super) async fn compile(
        sources: &[(PathBuf, String)],
        profile: crate::BuildProfile,
    ) -> Result<Self, Failure> {
        let library = Self::standard_library()?;
        let optimizer = Self::production_optimizer(profile).await;
        let version = Self::compiler_version().await?;
        let key = Self::cache_key(&version, &library, optimizer.as_ref(), sources);
        let cached = crate::cache::root().join(Self::CACHE_FOLDER).join(key);
        let build = (sources, library.as_path(), optimizer.as_ref());
        Self::cached_or_built(build, &cached).await
    }

    pub(super) fn note(&self, profile: crate::BuildProfile) -> Option<&'static str> {
        Self::note_for(profile, self.made_smaller)
    }

    fn note_for(profile: crate::BuildProfile, made_smaller: bool) -> Option<&'static str> {
        match (profile, made_smaller) {
            (crate::BuildProfile::Development, _) => None,
            (crate::BuildProfile::Release, true) => Some(Self::OPTIMIZED_NOTE),
            (crate::BuildProfile::Release, false) => Some(Self::UNOPTIMIZED_NOTE),
        }
    }

    async fn production_optimizer(profile: crate::BuildProfile) -> Option<WasmOptimizer> {
        match profile {
            crate::BuildProfile::Release => WasmOptimizer::find().await,
            crate::BuildProfile::Development => None,
        }
    }

    async fn cached_or_built(
        build: (&[(PathBuf, String)], &Path, Option<&WasmOptimizer>),
        cached: &Path,
    ) -> Result<Self, Failure> {
        let (_, _, optimizer) = build;
        if let Ok(program) = Self::read(cached, optimizer.is_some()) {
            return Ok(program);
        }
        Self::built(build, cached).await
    }

    async fn built(
        (sources, library, optimizer): (&[(PathBuf, String)], &Path, Option<&WasmOptimizer>),
        cached: &Path,
    ) -> Result<Self, Failure> {
        let staged = Self::stage(sources)?;
        Self::run_compiler(staged.path(), library, sources).await?;
        let output = staged.path().join(Self::OUTPUT_FOLDER);
        let made_smaller = Self::optimize(optimizer, &output.join(Self::BINARY)).await;
        let program =
            Self::read(&output, made_smaller).map_err(|error| Self::failure(&error.to_string()))?;
        Self::keep(&output, cached, made_smaller == optimizer.is_some());
        Ok(program)
    }

    async fn optimize(optimizer: Option<&WasmOptimizer>, binary: &Path) -> bool {
        match optimizer {
            Some(optimizer) => optimizer.run(binary, &Self::PRODUCTION_FLAGS).await,
            None => false,
        }
    }

    fn keep(built: &Path, cached: &Path, as_expected: bool) {
        if as_expected {
            let _ = Self::save(built, cached);
        }
    }

    pub(super) fn published(self) -> Vec<(String, Vec<u8>)> {
        let entry = self
            .entry
            .replace(Self::LOADER_IMPORT, Self::PUBLISHED_LOADER_IMPORT);
        vec![
            (Self::PUBLISHED_ENTRY.to_owned(), entry.into_bytes()),
            (Self::PUBLISHED_LOADER.to_owned(), self.loader.into_bytes()),
            (Self::PUBLISHED_BINARY.to_owned(), self.binary),
        ]
    }

    fn standard_library() -> Result<PathBuf, Failure> {
        let configured = std::env::var_os(Self::STANDARD_LIBRARY).map(PathBuf::from);
        configured.filter(|path| path.is_file()).ok_or_else(|| {
            Self::failure(
                "Kotlin web services need the Kotlin/Wasm standard library (kotlin-stdlib-wasm-js, the same version as kotlinc). Download it from Maven Central and point TAC_KOTLIN_WASM_STDLIB at the .klib file. Run ty doctor",
            )
        })
    }

    async fn compiler_version() -> Result<Vec<u8>, Failure> {
        let mut command = tokio::process::Command::new(Self::COMPILER);
        command.arg("-version").stdin(std::process::Stdio::null());
        let output = run_tool(&mut command, Self::VERSION_DEADLINE, Self::OUTPUT_BYTES)
            .await
            .map_err(|error| {
                Self::failure(&format!(
                    "cannot start kotlinc-js: {error}. Install the Kotlin compiler and put its bin folder on PATH"
                ))
            })?;
        Ok([output.stdout, output.stderr].concat())
    }

    fn cache_key(
        version: &[u8],
        library: &Path,
        optimizer: Option<&WasmOptimizer>,
        sources: &[(PathBuf, String)],
    ) -> String {
        let mut material = version.to_vec();
        material.extend_from_slice(library.to_string_lossy().as_bytes());
        let library_size = std::fs::metadata(library).map_or(0, |metadata| metadata.len());
        material.extend_from_slice(&library_size.to_le_bytes());
        material.extend_from_slice(optimizer.map_or(&[][..], WasmOptimizer::identity));
        for (path, contents) in sources {
            material.push(0);
            material.extend_from_slice(path.to_string_lossy().as_bytes());
            material.push(0);
            material.extend_from_slice(contents.as_bytes());
        }
        crate::compiler::hex_digest(material)
    }

    fn stage(sources: &[(PathBuf, String)]) -> Result<tempfile::TempDir, Failure> {
        let staged = tempfile::Builder::new()
            .prefix(".tachyon-kotlin-wasm-")
            .tempdir()
            .map_err(|error| Self::failure(&error.to_string()))?;
        for (path, contents) in sources {
            let target = staged.path().join(path);
            std::fs::create_dir_all(target.parent().unwrap_or(staged.path()))
                .and_then(|()| std::fs::write(&target, contents))
                .map_err(|error| Self::failure(&error.to_string()))?;
        }
        Ok(staged)
    }

    async fn run_compiler(
        staged: &Path,
        library: &Path,
        sources: &[(PathBuf, String)],
    ) -> Result<(), Failure> {
        let mut compile = Self::command(staged, library);
        compile
            .args(["-Xir-produce-klib-file", "-ir-output-dir"])
            .arg(Self::LIBRARY_FOLDER)
            .args(["-ir-output-name", Self::NAME]);
        for (path, _) in sources {
            compile.arg(path);
        }
        Self::finish(staged, &mut compile).await?;
        let mut link = Self::command(staged, library);
        link.args(["-Xir-produce-js", "-Xir-dce"])
            .arg(format!(
                "-Xinclude={}/{}.klib",
                Self::LIBRARY_FOLDER,
                Self::NAME
            ))
            .args(["-ir-output-dir", Self::OUTPUT_FOLDER])
            .args(["-ir-output-name", Self::NAME]);
        Self::finish(staged, &mut link).await
    }

    fn command(staged: &Path, library: &Path) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(Self::COMPILER);
        command
            .args(Self::TARGET)
            .arg("-libraries")
            .arg(library)
            .current_dir(staged)
            .stdin(std::process::Stdio::null());
        command
    }

    async fn finish(staged: &Path, command: &mut tokio::process::Command) -> Result<(), Failure> {
        let output = run_tool(command, Self::DEADLINE, Self::OUTPUT_BYTES)
            .await
            .map_err(|error| Self::failure(&error.to_string()))?;
        if output.status.success() {
            return Ok(());
        }
        Err(Self::compile_failure(&output, staged))
    }

    fn compile_failure(output: &ToolOutput, staged: &Path) -> Failure {
        let detail = String::from_utf8_lossy(&output.stdout).into_owned()
            + &String::from_utf8_lossy(&output.stderr);
        let detail = detail.replace(&format!("{}/", staged.to_string_lossy()), "");
        Self::failure(&format!(
            "{}\n(line numbers are in the composed services.kt that Tachyon builds from every Kotlin web service)",
            detail.trim()
        ))
    }

    fn failure(detail: &str) -> Failure {
        super::super::host::native_tool_failure(
            Self::FAILURE_CODE,
            &format!("Cannot build the Kotlin web services: {detail}"),
        )
    }

    fn read(folder: &Path, made_smaller: bool) -> std::io::Result<Self> {
        Ok(Self {
            binary: std::fs::read(folder.join(Self::BINARY))?,
            entry: std::fs::read_to_string(folder.join(Self::ENTRY))?,
            loader: std::fs::read_to_string(folder.join(Self::LOADER))?,
            made_smaller,
        })
    }

    fn save(built: &Path, cached: &Path) -> std::io::Result<()> {
        let cache = cached.parent().unwrap_or(cached);
        std::fs::create_dir_all(cache)?;
        let copy = tempfile::Builder::new()
            .prefix(".saving-")
            .tempdir_in(cache)?;
        for name in [Self::BINARY, Self::ENTRY, Self::LOADER] {
            std::fs::copy(built.join(name), copy.path().join(name))?;
        }
        let kept = copy.keep();
        std::fs::rename(&kept, cached).or_else(|_| std::fs::remove_dir_all(&kept))
    }
}
