use super::super::web_program_build::WebProgramBuild;
use crate::Failure;
use crate::wasm_release_note::WasmReleaseNote;
use crate::wasm_release_step::WasmReleaseStep;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

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
    const NODE_CHECK: &'static str = "process.release.name === 'node'";
    const PAGE_SAFE_NODE_CHECK: &'static str = "process.release?.name === 'node'";
    const CACHE_FOLDER: &'static str = "kotlin-wasm";
    const BUILD: WebProgramBuild = WebProgramBuild {
        language: "Kotlin",
        composed: "services.kt",
        note: WasmReleaseNote {
            optimized: "Kotlin: Kotlin/Wasm production build — optimized by Binaryen",
            unoptimized: "Kotlin: shipped without Binaryen optimization (≈3× larger); install Binaryen or run `ty doctor`",
        },
    };
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

    pub(super) async fn compile(
        sources: &[(PathBuf, String)],
        profile: crate::BuildProfile,
    ) -> Result<Self, Failure> {
        let library = Self::standard_library()?;
        let release = WasmReleaseStep::prepare(profile).await;
        let version = Self::compiler_version().await?;
        let key = Self::cache_key(&version, &library, release.identity().as_bytes(), sources);
        let cached = crate::cache::root().join(Self::CACHE_FOLDER).join(key);
        let build = (sources, library.as_path(), &release);
        Self::cached_or_built(build, &cached).await
    }

    pub(super) fn note(&self, profile: crate::BuildProfile) -> Option<String> {
        Self::BUILD.note.for_build(profile, self.made_smaller)
    }

    async fn cached_or_built(
        build: (&[(PathBuf, String)], &Path, &WasmReleaseStep),
        cached: &Path,
    ) -> Result<Self, Failure> {
        let (_, _, release) = build;
        if let Ok(program) = Self::read(cached, release.will_shrink()) {
            return Ok(program);
        }
        Self::built(build, cached).await
    }

    async fn built(
        (sources, library, release): (&[(PathBuf, String)], &Path, &WasmReleaseStep),
        cached: &Path,
    ) -> Result<Self, Failure> {
        let staged = Self::stage(sources)?;
        Self::run_compiler(staged.path(), library, sources).await?;
        let output = staged.path().join(Self::OUTPUT_FOLDER);
        let made_smaller = release
            .shrink(&output.join(Self::BINARY), &Self::PRODUCTION_FLAGS)
            .await;
        let program = Self::read(&output, made_smaller)
            .map_err(|error| Self::BUILD.failure(&error.to_string()))?;
        Self::keep(&output, cached, made_smaller == release.will_shrink());
        Ok(program)
    }

    fn keep(built: &Path, cached: &Path, as_expected: bool) {
        if as_expected {
            let _ =
                WebProgramBuild::save(built, &[Self::BINARY, Self::ENTRY, Self::LOADER], cached);
        }
    }

    pub(super) fn published(self) -> Vec<(String, Vec<u8>)> {
        let entry = self
            .entry
            .replace(Self::LOADER_IMPORT, Self::PUBLISHED_LOADER_IMPORT);
        let loader = self
            .loader
            .replace(Self::NODE_CHECK, Self::PAGE_SAFE_NODE_CHECK);
        vec![
            (Self::PUBLISHED_ENTRY.to_owned(), entry.into_bytes()),
            (Self::PUBLISHED_LOADER.to_owned(), loader.into_bytes()),
            (Self::PUBLISHED_BINARY.to_owned(), self.binary),
        ]
    }

    fn standard_library() -> Result<PathBuf, Failure> {
        let configured = std::env::var_os(Self::STANDARD_LIBRARY).map(PathBuf::from);
        configured.filter(|path| path.is_file()).ok_or_else(|| {
            Self::BUILD.failure(
                "Kotlin web services need the Kotlin/Wasm standard library (kotlin-stdlib-wasm-js, the same version as kotlinc). Download it from Maven Central and point TAC_KOTLIN_WASM_STDLIB at the .klib file. Run ty doctor",
            )
        })
    }

    async fn compiler_version() -> Result<Vec<u8>, Failure> {
        Self::BUILD
            .tool_version(
                std::ffi::OsStr::new(Self::COMPILER),
                "-version",
                "Install the Kotlin compiler and put its bin folder on PATH",
            )
            .await
    }

    fn cache_key(
        version: &[u8],
        library: &Path,
        release_identity: &[u8],
        sources: &[(PathBuf, String)],
    ) -> String {
        let mut material = version.to_vec();
        material.extend_from_slice(library.to_string_lossy().as_bytes());
        let library_size = std::fs::metadata(library).map_or(0, |metadata| metadata.len());
        material.extend_from_slice(&library_size.to_le_bytes());
        material.extend_from_slice(release_identity);
        for (path, contents) in sources {
            material.push(0);
            material.extend_from_slice(path.to_string_lossy().as_bytes());
            material.push(0);
            material.extend_from_slice(contents.as_bytes());
        }
        crate::compiler::hex_digest(Sha256::digest(material))
    }

    fn stage(sources: &[(PathBuf, String)]) -> Result<tempfile::TempDir, Failure> {
        let staged = tempfile::Builder::new()
            .prefix(".tachyon-kotlin-wasm-")
            .tempdir()
            .map_err(|error| Self::BUILD.failure(&error.to_string()))?;
        for (path, contents) in sources {
            let target = staged.path().join(path);
            std::fs::create_dir_all(target.parent().unwrap_or(staged.path()))
                .and_then(|()| std::fs::write(&target, contents))
                .map_err(|error| Self::BUILD.failure(&error.to_string()))?;
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
        Self::BUILD.run(&mut compile, staged).await?;
        let mut link = Self::command(staged, library);
        link.args(["-Xir-produce-js", "-Xir-dce"])
            .arg(format!(
                "-Xinclude={}/{}.klib",
                Self::LIBRARY_FOLDER,
                Self::NAME
            ))
            .args(["-ir-output-dir", Self::OUTPUT_FOLDER])
            .args(["-ir-output-name", Self::NAME]);
        Self::BUILD.run(&mut link, staged).await
    }

    fn command(staged: &Path, library: &Path) -> tokio::process::Command {
        let mut command =
            tokio::process::Command::from(crate::kotlin_home::KotlinHome::command(Self::COMPILER));
        command
            .args(Self::TARGET)
            .arg("-libraries")
            .arg(library)
            .current_dir(staged)
            .stdin(std::process::Stdio::null());
        command
    }

    fn read(folder: &Path, made_smaller: bool) -> std::io::Result<Self> {
        Ok(Self {
            binary: std::fs::read(folder.join(Self::BINARY))?,
            entry: std::fs::read_to_string(folder.join(Self::ENTRY))?,
            loader: std::fs::read_to_string(folder.join(Self::LOADER))?,
            made_smaller,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::KotlinWasmProgram;

    #[test]
    fn the_published_loader_starts_beside_the_go_process_stand_in() {
        let program = KotlinWasmProgram {
            binary: Vec::new(),
            entry: String::new(),
            loader: String::from(
                "const isNodeJs = (typeof process !== 'undefined') && (process.release.name === 'node');",
            ),
            made_smaller: true,
        };
        let published = program.published();
        let loader = published
            .iter()
            .find(|(name, _)| name == KotlinWasmProgram::PUBLISHED_LOADER)
            .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
            .unwrap_or_default();
        assert!(
            loader.contains("process.release?.name === 'node'"),
            "{loader}"
        );
    }
}
