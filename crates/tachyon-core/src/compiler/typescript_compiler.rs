use super::WebCompiler;
use super::typescript_cache::TypeScriptCache;
use crate::Failure;
use crate::external_command::{ToolError, ToolOutput, run as run_tool};
use crate::failure::diagnostic;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) struct TypeScriptCompiler;

impl TypeScriptCompiler {
    const TOOL_OUTPUT_BYTES: usize = 64 * 1_024;
    const DEADLINE: std::time::Duration = std::time::Duration::from_secs(30);
    const MAX_DETAIL_BYTES: usize = 512;

    pub(super) async fn transpile(
        tools_root: &Path,
        project_root: &Path,
        source: &Path,
        portable_source: &str,
        source_bytes: &[u8],
    ) -> Result<Vec<u8>, Failure> {
        let program = Self::program(tools_root);
        let cached = TypeScriptCache::entry(&program, source, source_bytes);
        let hit = TypeScriptCache::read(cached.as_deref());
        Self::cached_or_transpiled(
            hit,
            cached.as_deref(),
            &program,
            project_root,
            source,
            portable_source,
            source_bytes,
        )
        .await
    }

    pub(super) fn major_version(banner: &str) -> u32 {
        banner
            .split_whitespace()
            .last()
            .and_then(|version| version.split('.').next())
            .and_then(|major| major.parse::<u32>().ok())
            .unwrap_or(0)
    }

    async fn cached_or_transpiled(
        hit: Option<Vec<u8>>,
        cached: Option<&Path>,
        program: &Path,
        project_root: &Path,
        source: &Path,
        portable_source: &str,
        source_bytes: &[u8],
    ) -> Result<Vec<u8>, Failure> {
        if let Some(bytes) = hit {
            return Ok(bytes);
        }
        Self::transpile_uncached(
            cached,
            program,
            project_root,
            source,
            portable_source,
            source_bytes,
        )
        .await
    }

    async fn transpile_uncached(
        cached: Option<&Path>,
        program: &Path,
        project_root: &Path,
        source: &Path,
        portable_source: &str,
        source_bytes: &[u8],
    ) -> Result<Vec<u8>, Failure> {
        Self::check_version(program, portable_source).await?;
        let bytes =
            Self::emit_in_stage(program, project_root, source, portable_source, source_bytes)
                .await?;
        TypeScriptCache::store(cached, &bytes);
        Ok(bytes)
    }

    async fn emit_in_stage(
        program: &Path,
        project_root: &Path,
        source: &Path,
        portable_source: &str,
        source_bytes: &[u8],
    ) -> Result<Vec<u8>, Failure> {
        let staged = tempfile::Builder::new()
            .prefix(".tachyon-typescript-")
            .tempdir()
            .map_err(|error| Self::error(portable_source, &error.to_string()))?;
        let source_name = source
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("tac.ts"));
        let staged_source = staged.path().join(source_name);
        fs::write(&staged_source, source_bytes)
            .map_err(|error| Self::error(portable_source, &error.to_string()))?;
        Self::emit(
            program,
            project_root,
            &staged_source,
            staged.path(),
            portable_source,
        )
        .await?;
        let stem = source.file_stem().map_or_else(
            || String::from("tac"),
            |stem| stem.to_string_lossy().into_owned(),
        );
        let emitted = staged.path().join(format!("{stem}.js"));
        fs::read(&emitted).map_err(|error| {
            Self::error(
                portable_source,
                &format!("the TypeScript compiler emitted no JavaScript: {error}"),
            )
        })
    }

    fn program(tools_root: &Path) -> PathBuf {
        let local = tools_root.join("node_modules/.bin/tsc");
        if local.is_file() {
            local
        } else {
            PathBuf::from("tsc")
        }
    }

    async fn check_version(program: &Path, portable_source: &str) -> Result<(), Failure> {
        let mut command = tokio::process::Command::new(program);
        command.arg("--version").stdin(std::process::Stdio::null());
        let version = run_tool(&mut command, Self::DEADLINE, Self::TOOL_OUTPUT_BYTES)
            .await
            .map_err(|error| Self::start_error(portable_source, program, &error))?;
        let banner = String::from_utf8_lossy(&version.stdout);
        if Self::major_version(&banner) >= WebCompiler::MINIMUM_TYPESCRIPT_MAJOR {
            return Ok(());
        }
        Err(Self::error(
            portable_source,
            &format!(
                "found TypeScript '{}', but emitting tac.ts requires TypeScript 6 \
                 or newer.",
                banner.trim()
            ),
        ))
    }

    async fn emit(
        program: &Path,
        project_root: &Path,
        staged_source: &Path,
        out_dir: &Path,
        portable_source: &str,
    ) -> Result<(), Failure> {
        let source_argument = staged_source.to_string_lossy().into_owned();
        let out_dir = out_dir.to_string_lossy().into_owned();
        let mut command = tokio::process::Command::new(program);
        command
            .args([
                source_argument.as_str(),
                "--outDir",
                out_dir.as_str(),
                "--target",
                "es2022",
                "--module",
                "esnext",
                "--moduleResolution",
                "bundler",
                "--noCheck",
                "--ignoreConfig",
            ])
            .current_dir(project_root)
            .stdin(std::process::Stdio::null());
        let output = run_tool(&mut command, Self::DEADLINE, Self::TOOL_OUTPUT_BYTES)
            .await
            .map_err(|error| Self::start_error(portable_source, program, &error))?;
        let detail = Self::failure_detail(&output);
        Self::check_emitted(output.status.success(), &detail, portable_source)
    }

    fn failure_detail(output: &ToolOutput) -> String {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.trim().is_empty() {
            return String::from_utf8_lossy(&output.stderr).into_owned();
        }
        stdout.into_owned()
    }

    fn check_emitted(success: bool, detail: &str, portable_source: &str) -> Result<(), Failure> {
        if success {
            return Ok(());
        }
        let detail = detail.trim();
        Err(Self::error(
            portable_source,
            detail.get(..Self::MAX_DETAIL_BYTES).unwrap_or(detail),
        ))
    }

    fn start_error(portable_source: &str, program: &Path, error: &ToolError) -> Failure {
        Self::error(
            portable_source,
            &format!(
                "cannot start the TypeScript compiler '{}': {error}. Install \
                 typescript 6 or newer in the project or on PATH.",
                program.display()
            ),
        )
    }

    fn error(source: &str, detail: &str) -> Failure {
        Failure::one(diagnostic(
            1009,
            format!("Cannot compile TypeScript companion '{source}': {detail}"),
            Some(String::from(
                "Tachyon emits tac.ts with the TypeScript compiler. Install \
                 typescript 6 or newer as a project dependency or on PATH.",
            )),
            None,
        ))
    }
}
