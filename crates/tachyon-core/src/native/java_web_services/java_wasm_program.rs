use super::super::web_program_build::WebProgramBuild;
use super::tea_vm_runtime::TeaVmRuntime;
use super::tea_vm_toolchain::TeaVmToolchain;
use crate::Failure;
use crate::wasm_release_note::WasmReleaseNote;
use crate::wasm_release_step::WasmReleaseStep;
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub(super) struct JavaWasmProgram {
    binary: Vec<u8>,
    runtime: Vec<u8>,
    made_smaller: bool,
}

impl JavaWasmProgram {
    const BUILD: WebProgramBuild = WebProgramBuild {
        language: "Java",
        composed: "service files",
        note: WasmReleaseNote {
            optimized: "Java: TeaVM WebAssembly release build — optimized by Binaryen",
            unoptimized: "Java: shipped without Binaryen optimization; install Binaryen or run `ty doctor`",
        },
    };
    const JAVA_COMPILER: &'static str = "javac";
    const JAVA_FLAGS: [&'static str; 5] = ["--release", "21", "-encoding", "UTF-8", "-nowarn"];
    const JAVA_RUNTIME: &'static str = "java";
    const TEAVM_RUNNER: &'static str = "org.teavm.cli.TeaVMRunner";
    const TEAVM_FLAGS: [&'static str; 4] = ["-t", "wasm-gc", "-O", "2"];
    const ENTRY_CLASS: &'static str = "TacServices";
    const ARCHIVE_TOOL: &'static str = "jar";
    const RUNTIME_ENTRY: &'static str = "org/teavm/backend/wasm/wasm-gc-module-runtime.min.js";
    const FOLDERS: [&'static str; 3] = ["classes", "program", "java-wasm"];
    const BINARY: &'static str = "services.wasm";
    const RUNTIME: &'static str = "runtime.js";
    const PUBLISHED: [&'static str; 2] = ["java/services.wasm", "java/runtime.js"];
    const SEPARATOR: u8 = 0;
    const RELEASE_FLAGS: [&'static str; 10] = [
        "--enable-gc",
        "--enable-reference-types",
        "--enable-exception-handling",
        "--enable-bulk-memory",
        "--enable-nontrapping-float-to-int",
        "--enable-sign-ext",
        "--enable-mutable-globals",
        "-Oz",
        "--strip-debug",
        "--strip-producers",
    ];
    const MISSING_TEAVM: &'static str = "Java web services need TeaVM 0.15. Download the TeaVM command-line jar (org.teavm:cli:0.15.0, the -all jar) and the teavm-classlib, teavm-jso-impl, teavm-platform and teavm-metaprogramming-impl jars of the same version from Maven Central into one folder, and point TAC_TEAVM_HOME at it. Run ty doctor";
    const UNKNOWN_RUNTIME: &'static str = "The TeaVM runtime.js is not the one of TeaVM 0.15.0, so Tachyon cannot make it run without code made from text, which the page's content security policy forbids. Install TeaVM 0.15.0 and point TAC_TEAVM_HOME at it.";
    const MISSING_JDK: &'static str = "Install a JDK 21 or newer and put its bin folder on PATH";

    pub(super) fn is_installed() -> bool {
        TeaVmToolchain::find().is_some()
    }

    pub(super) async fn compile(
        sources: &[(PathBuf, String)],
        profile: crate::BuildProfile,
    ) -> Result<Self, Failure> {
        let toolchain =
            TeaVmToolchain::find().ok_or_else(|| Self::BUILD.failure(Self::MISSING_TEAVM))?;
        let version = Self::BUILD
            .tool_version(
                OsStr::new(Self::JAVA_COMPILER),
                "-version",
                Self::MISSING_JDK,
            )
            .await?;
        let release = WasmReleaseStep::prepare(profile).await;
        let key = Self::cache_key(&version, &toolchain, release.identity().as_bytes(), sources);
        let cached = crate::cache::root().join(Self::FOLDERS[2]).join(key);
        Self::cached_or_built((&toolchain, &release), sources, &cached).await
    }

    pub(super) fn note(&self, profile: crate::BuildProfile) -> Option<String> {
        Self::BUILD.note.for_build(profile, self.made_smaller)
    }

    pub(super) fn published(self) -> Vec<(String, Vec<u8>)> {
        vec![
            (Self::PUBLISHED[0].to_owned(), self.binary),
            (Self::PUBLISHED[1].to_owned(), self.runtime),
        ]
    }

    async fn cached_or_built(
        build: (&TeaVmToolchain, &WasmReleaseStep),
        sources: &[(PathBuf, String)],
        cached: &Path,
    ) -> Result<Self, Failure> {
        let (_, release) = build;
        if let Ok(program) = Self::read(cached, release.will_shrink()) {
            return Ok(program);
        }
        Self::built(build, sources, cached).await
    }

    async fn built(
        (toolchain, release): (&TeaVmToolchain, &WasmReleaseStep),
        sources: &[(PathBuf, String)],
        cached: &Path,
    ) -> Result<Self, Failure> {
        let staged = Self::stage(sources)?;
        let folder = staged.path();
        Self::BUILD
            .run(&mut Self::java_compiler(folder, toolchain, sources), folder)
            .await?;
        Self::BUILD
            .run(&mut Self::translator(folder, toolchain), folder)
            .await?;
        Self::BUILD
            .run(&mut Self::runtime_extractor(folder, toolchain), folder)
            .await?;
        let output = Self::with_runtime(folder)?;
        let made_smaller = release
            .shrink(&output.join(Self::BINARY), &Self::RELEASE_FLAGS)
            .await;
        Self::keep(&output, cached, made_smaller == release.will_shrink());
        Self::read(&output, made_smaller).map_err(|error| Self::BUILD.failure(&error.to_string()))
    }

    fn keep(built: &Path, cached: &Path, as_expected: bool) {
        if as_expected {
            let _ = WebProgramBuild::save(built, &[Self::BINARY, Self::RUNTIME], cached);
        }
    }

    fn read(folder: &Path, made_smaller: bool) -> std::io::Result<Self> {
        Ok(Self {
            binary: std::fs::read(folder.join(Self::BINARY))?,
            runtime: std::fs::read(folder.join(Self::RUNTIME))?,
            made_smaller,
        })
    }

    fn with_runtime(folder: &Path) -> Result<PathBuf, Failure> {
        let output = folder.join(Self::FOLDERS[1]);
        let runtime = std::fs::read_to_string(folder.join(Self::RUNTIME_ENTRY))
            .map_err(|error| Self::BUILD.failure(&error.to_string()))?;
        let allowed = TeaVmRuntime::without_code_from_text(&runtime)
            .ok_or_else(|| Self::BUILD.failure(Self::UNKNOWN_RUNTIME))?;
        std::fs::write(output.join(Self::RUNTIME), allowed)
            .map_err(|error| Self::BUILD.failure(&error.to_string()))?;
        Ok(output)
    }

    fn cache_key(
        version: &[u8],
        toolchain: &TeaVmToolchain,
        release: &[u8],
        sources: &[(PathBuf, String)],
    ) -> String {
        let identity = toolchain.identity();
        let patch = TeaVmRuntime::identity();
        let mut parts: Vec<&[u8]> = vec![version, &identity, &patch, release];
        parts.extend(sources.iter().flat_map(Self::source_parts));
        crate::compiler::hex_digest(Sha256::digest(parts.join(&Self::SEPARATOR)))
    }

    fn source_parts((path, text): &(PathBuf, String)) -> [&[u8]; 2] {
        [path.as_os_str().as_encoded_bytes(), text.as_bytes()]
    }

    fn stage(sources: &[(PathBuf, String)]) -> Result<tempfile::TempDir, Failure> {
        let staged = tempfile::Builder::new()
            .prefix(".tachyon-java-wasm-")
            .tempdir()
            .map_err(|error| Self::BUILD.failure(&error.to_string()))?;
        for (path, text) in sources {
            Self::write(&staged.path().join(path), text)?;
        }
        Ok(staged)
    }

    fn write(path: &Path, text: &str) -> Result<(), Failure> {
        let folder = path.parent().unwrap_or(path);
        std::fs::create_dir_all(folder)
            .and_then(|()| std::fs::write(path, text))
            .map_err(|error| Self::BUILD.failure(&error.to_string()))
    }

    fn java_compiler(
        folder: &Path,
        toolchain: &TeaVmToolchain,
        sources: &[(PathBuf, String)],
    ) -> tokio::process::Command {
        let mut command = Self::command(Self::JAVA_COMPILER, folder);
        command
            .args(Self::JAVA_FLAGS)
            .arg("-cp")
            .arg(toolchain.compiler())
            .args(["-d", Self::FOLDERS[0]])
            .args(sources.iter().map(|(path, _)| path));
        command
    }

    fn translator(folder: &Path, toolchain: &TeaVmToolchain) -> tokio::process::Command {
        let mut command = Self::command(Self::JAVA_RUNTIME, folder);
        command
            .arg("-cp")
            .arg(toolchain.tool_class_path())
            .arg(Self::TEAVM_RUNNER)
            .args(Self::TEAVM_FLAGS)
            .arg("-p")
            .arg(folder.join(Self::FOLDERS[0]))
            .arg("-p")
            .arg(toolchain.compiler())
            .arg("-d")
            .arg(folder.join(Self::FOLDERS[1]))
            .args(["-f", Self::BINARY, Self::ENTRY_CLASS]);
        command
    }

    fn runtime_extractor(folder: &Path, toolchain: &TeaVmToolchain) -> tokio::process::Command {
        let mut command = Self::command(Self::ARCHIVE_TOOL, folder);
        command
            .arg("xf")
            .arg(toolchain.compiler())
            .arg(Self::RUNTIME_ENTRY);
        command
    }

    fn command(program: &str, folder: &Path) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(program);
        command
            .current_dir(folder)
            .stdin(std::process::Stdio::null());
        command
    }
}
