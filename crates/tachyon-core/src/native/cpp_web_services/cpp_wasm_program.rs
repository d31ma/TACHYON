use super::super::web_program_build::WebProgramBuild;
use crate::Failure;
use crate::wasm_release_note::WasmReleaseNote;
use sha2::{Digest, Sha256};
use std::ffi::{OsStr, OsString};
use std::path::Path;

pub(super) struct CppWasmProgram {
    binary: Vec<u8>,
}

impl CppWasmProgram {
    const BUILD: WebProgramBuild = WebProgramBuild {
        language: "C++",
        composed: "services.cpp",
        note: WasmReleaseNote {
            optimized: "C++: WebAssembly release build — Emscripten -Oz, optimized by Binaryen",
            unoptimized: "C++: WebAssembly development build — Emscripten -O1",
        },
    };
    const COMPILER_SETTING: &'static str = "TAC_CPP_WASM_COMPILER";
    #[cfg(windows)]
    const DEFAULT_COMPILER: &'static str = "em++.bat";
    #[cfg(not(windows))]
    const DEFAULT_COMPILER: &'static str = "em++";
    const FLAGS: [&'static str; 8] = [
        "-std=c++20",
        "-fwasm-exceptions",
        "-sSTANDALONE_WASM",
        "--no-entry",
        "-sALLOW_MEMORY_GROWTH=1",
        "-sMALLOC=emmalloc",
        "-sINITIAL_MEMORY=4MB",
        "-sSTACK_SIZE=1MB",
    ];
    const RELEASE_LEVEL: &'static str = "-Oz";
    const DEVELOPMENT_LEVEL: &'static str = "-O1";
    const SOURCE: &'static str = "services.cpp";
    const BINARY: &'static str = "services.wasm";
    const PUBLISHED_BINARY: &'static str = "cpp/services.wasm";
    const CACHE_FOLDER: &'static str = "cpp-wasm";
    const MISSING_COMPILER: &'static str = "Install Emscripten (emsdk) and put em++ on PATH, or set TAC_CPP_WASM_COMPILER to its em++. Run ty doctor";

    pub(super) async fn compile(
        program: &str,
        profile: crate::BuildProfile,
    ) -> Result<Self, Failure> {
        let compiler = Self::compiler();
        let version = Self::BUILD
            .tool_version(&compiler, "--version", Self::MISSING_COMPILER)
            .await?;
        let level = Self::level(profile);
        let key = Self::cache_key(&version, level, program);
        let cached = crate::cache::root().join(Self::CACHE_FOLDER).join(key);
        if let Ok(binary) = std::fs::read(cached.join(Self::BINARY)) {
            return Ok(Self { binary });
        }
        Self::built(&compiler, level, program, &cached).await
    }

    pub(super) fn note(profile: crate::BuildProfile) -> Option<String> {
        Self::BUILD.note.for_build(profile, true)
    }

    pub(super) fn published(self) -> Vec<(String, Vec<u8>)> {
        vec![(Self::PUBLISHED_BINARY.to_owned(), self.binary)]
    }

    fn compiler() -> OsString {
        std::env::var_os(Self::COMPILER_SETTING)
            .unwrap_or_else(|| OsString::from(Self::DEFAULT_COMPILER))
    }

    fn level(profile: crate::BuildProfile) -> &'static str {
        match profile {
            crate::BuildProfile::Release => Self::RELEASE_LEVEL,
            crate::BuildProfile::Development => Self::DEVELOPMENT_LEVEL,
        }
    }

    async fn built(
        compiler: &OsStr,
        level: &str,
        program: &str,
        cached: &Path,
    ) -> Result<Self, Failure> {
        let staged = tempfile::Builder::new()
            .prefix(".tachyon-cpp-wasm-")
            .tempdir()
            .map_err(|error| Self::BUILD.failure(&error.to_string()))?;
        std::fs::write(staged.path().join(Self::SOURCE), program)
            .map_err(|error| Self::BUILD.failure(&error.to_string()))?;
        let mut command = tokio::process::Command::new(compiler);
        command
            .arg(level)
            .args(Self::FLAGS)
            .args([Self::SOURCE, "-o", Self::BINARY])
            .current_dir(staged.path())
            .stdin(std::process::Stdio::null());
        Self::BUILD.run(&mut command, staged.path()).await?;
        let _ = WebProgramBuild::save(staged.path(), &[Self::BINARY], cached);
        let binary = std::fs::read(staged.path().join(Self::BINARY))
            .map_err(|error| Self::BUILD.failure(&error.to_string()))?;
        Ok(Self { binary })
    }

    fn cache_key(version: &[u8], level: &str, program: &str) -> String {
        let mut material = version.to_vec();
        material.push(0);
        material.extend_from_slice(level.as_bytes());
        material.push(0);
        material.extend_from_slice(Self::FLAGS.join(" ").as_bytes());
        material.push(0);
        material.extend_from_slice(program.as_bytes());
        crate::compiler::hex_digest(Sha256::digest(material))
    }
}
