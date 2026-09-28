use super::super::web_program_build::WebProgramBuild;
use crate::Failure;
use crate::wasm_optimizer::WasmOptimizer;
use sha2::{Digest, Sha256};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

pub(super) struct SwiftWasmProgram {
    binary: Vec<u8>,
    made_smaller: bool,
}

impl SwiftWasmProgram {
    const BUILD: WebProgramBuild = WebProgramBuild {
        language: "Swift",
        composed: "services.swift",
        optimized_note: "Swift: WebAssembly release build — optimized by Binaryen",
        unoptimized_note: "Swift: shipped without Binaryen optimization; install Binaryen or run `ty doctor`",
    };
    const SDK: &'static str = "TAC_SWIFT_WASM_SDK";
    const COMPILER_SETTING: &'static str = "TAC_SWIFT_WASM_COMPILER";
    const DEFAULT_COMPILER: &'static str = "swiftc";
    const TARGET: &'static str = "wasm32-unknown-wasip1";
    const SYSTEM_ROOT: &'static str = "WASI.sdk";
    const SWIFT_RESOURCES: &'static str = "swift.xctoolchain/usr/lib/swift_static";
    const CLANG_RESOURCES: &'static str = "swift.xctoolchain/usr/lib/swift_static/clang";
    const FLAGS: [&'static str; 11] = [
        "-static-stdlib",
        "-Osize",
        "-parse-as-library",
        "-swift-version",
        "5",
        "-module-name",
        "TachyonServices",
        "-enable-experimental-feature",
        "Extern",
        "-Xclang-linker",
        "-mexec-model=reactor",
    ];
    const SOURCE: &'static str = "services.swift";
    const BINARY: &'static str = "services.wasm";
    const PUBLISHED_BINARY: &'static str = "swift/services.wasm";
    const CACHE_FOLDER: &'static str = "swift-wasm";
    const RELEASE_FLAGS: [&'static str; 3] = ["-Oz", "--strip-debug", "--strip-producers"];
    const MISSING_SDK: &'static str = "Swift web services need the Swift SDK for WebAssembly. Install it with `swift sdk install` (the same version as your Swift compiler) and point TAC_SWIFT_WASM_SDK at its wasm32-unknown-wasip1 folder. Run ty doctor";
    const MISSING_COMPILER: &'static str =
        "Install Swift from swift.org, or set TAC_SWIFT_WASM_COMPILER to its swiftc";

    pub(super) async fn compile(
        program: &str,
        profile: crate::BuildProfile,
    ) -> Result<Self, Failure> {
        let sdk = Self::sdk()?;
        let compiler = Self::compiler();
        let version = Self::BUILD
            .tool_version(&compiler, "--version", Self::MISSING_COMPILER)
            .await?;
        let optimizer = WebProgramBuild::release_optimizer(profile).await;
        let key = Self::cache_key(&version, &sdk, optimizer.as_ref(), program);
        let cached = crate::cache::root().join(Self::CACHE_FOLDER).join(key);
        let build = (compiler.as_os_str(), sdk.as_path(), optimizer.as_ref());
        Self::cached_or_built(build, program, &cached).await
    }

    pub(super) fn note(&self, profile: crate::BuildProfile) -> Option<&'static str> {
        Self::BUILD.note(profile, self.made_smaller)
    }

    pub(super) fn published(self) -> Vec<(String, Vec<u8>)> {
        vec![(Self::PUBLISHED_BINARY.to_owned(), self.binary)]
    }

    pub(super) fn sdk() -> Result<PathBuf, Failure> {
        let configured = std::env::var_os(Self::SDK).map(PathBuf::from);
        configured
            .filter(|path| path.join(Self::SYSTEM_ROOT).is_dir())
            .ok_or_else(|| Self::BUILD.failure(Self::MISSING_SDK))
    }

    pub(super) fn compiler() -> OsString {
        std::env::var_os(Self::COMPILER_SETTING)
            .unwrap_or_else(|| OsString::from(Self::DEFAULT_COMPILER))
    }

    async fn cached_or_built(
        build: (&OsStr, &Path, Option<&WasmOptimizer>),
        program: &str,
        cached: &Path,
    ) -> Result<Self, Failure> {
        let (_, _, optimizer) = build;
        if let Ok(binary) = std::fs::read(cached.join(Self::BINARY)) {
            return Ok(Self {
                binary,
                made_smaller: optimizer.is_some(),
            });
        }
        Self::built(build, program, cached).await
    }

    async fn built(
        (compiler, sdk, optimizer): (&OsStr, &Path, Option<&WasmOptimizer>),
        program: &str,
        cached: &Path,
    ) -> Result<Self, Failure> {
        let staged = Self::stage(program)?;
        let mut command = Self::command(compiler, sdk, staged.path());
        Self::BUILD.run(&mut command, staged.path()).await?;
        let binary = staged.path().join(Self::BINARY);
        let made_smaller =
            WebProgramBuild::optimize(optimizer, &binary, &Self::RELEASE_FLAGS).await;
        Self::keep(staged.path(), cached, made_smaller == optimizer.is_some());
        let binary =
            std::fs::read(&binary).map_err(|error| Self::BUILD.failure(&error.to_string()))?;
        Ok(Self {
            binary,
            made_smaller,
        })
    }

    fn keep(built: &Path, cached: &Path, as_expected: bool) {
        if as_expected {
            let _ = WebProgramBuild::save(built, &[Self::BINARY], cached);
        }
    }

    fn cache_key(
        version: &[u8],
        sdk: &Path,
        optimizer: Option<&WasmOptimizer>,
        program: &str,
    ) -> String {
        let mut material = version.to_vec();
        material.push(0);
        material.extend_from_slice(sdk.to_string_lossy().as_bytes());
        material.push(0);
        material.extend_from_slice(optimizer.map_or(&[][..], WasmOptimizer::identity));
        material.push(0);
        material.extend_from_slice(program.as_bytes());
        crate::compiler::hex_digest(Sha256::digest(material))
    }

    fn stage(program: &str) -> Result<tempfile::TempDir, Failure> {
        let staged = tempfile::Builder::new()
            .prefix(".tachyon-swift-wasm-")
            .tempdir()
            .map_err(|error| Self::BUILD.failure(&error.to_string()))?;
        std::fs::write(staged.path().join(Self::SOURCE), program)
            .map_err(|error| Self::BUILD.failure(&error.to_string()))?;
        Ok(staged)
    }

    fn command(compiler: &OsStr, sdk: &Path, staged: &Path) -> tokio::process::Command {
        let mut command = tokio::process::Command::new(compiler);
        command
            .args(["-target", Self::TARGET, "-sdk"])
            .arg(sdk.join(Self::SYSTEM_ROOT))
            .arg("-resource-dir")
            .arg(sdk.join(Self::SWIFT_RESOURCES))
            .args(["-Xclang-linker", "-resource-dir", "-Xclang-linker"])
            .arg(sdk.join(Self::CLANG_RESOURCES))
            .args(Self::FLAGS)
            .args([Self::SOURCE, "-o", Self::BINARY])
            .current_dir(staged)
            .stdin(std::process::Stdio::null());
        command
    }
}

#[cfg(test)]
mod tests {
    use super::SwiftWasmProgram;
    use std::path::Path;

    #[test]
    fn the_cache_key_is_a_short_folder_name_for_any_program() {
        let program = "final class LongService {}\n".repeat(4_000);
        let key = SwiftWasmProgram::cache_key(b"swift 6.3.3", Path::new("/sdk"), None, &program);
        assert_eq!(key.len(), 64, "{key}");
    }
}
