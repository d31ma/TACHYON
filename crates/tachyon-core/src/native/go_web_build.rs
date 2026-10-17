use super::go_compiled_with::GoCompiledWith;
use super::go_compiler::GoCompiler;
use super::go_compiler_choice::GoCompilerChoice;
use super::host::{native_io, native_tool_failure, write};
use crate::Failure;
use crate::wasm_release_step::WasmReleaseStep;
use std::path::{Path, PathBuf};

pub(crate) struct GoWebBuild {
    pub(crate) wasm: Vec<u8>,
    pub(crate) glue: Vec<u8>,
    pub(crate) license: Vec<u8>,
    pub(crate) note: Option<String>,
}

impl GoWebBuild {
    const FAILURE_CODE: u16 = 1605;
    const CACHE_FOLDER: &'static str = "go-wasm";
    const GLUE: &'static str = "wasm_exec.js";
    const LICENSE: &'static str = "LICENSE";
    const CACHED_FILES: &'static str = "services.wasm wasm_exec.js LICENSE note.txt";
    const NOTE: &'static str = "note.txt";
    const MAX_CAUSE_CHARACTERS: usize = 512;

    pub(super) async fn load(
        files: &[(PathBuf, String)],
        choice: &GoCompilerChoice,
        release: &WasmReleaseStep,
    ) -> Result<Self, Failure> {
        let identity = Self::identity(choice, release).await;
        let key = super::go::program_key(&identity, files);
        let cached = crate::cache::root().join(Self::CACHE_FOLDER).join(key);
        Self::cached_or_fresh((files, choice, release), &cached).await
    }

    async fn identity(choice: &GoCompilerChoice, release: &WasmReleaseStep) -> String {
        let mut identity = format!("{choice:?} {} {}", release.identity(), Self::CACHED_FILES);
        for compiler in [Some(choice.first), choice.fallback].into_iter().flatten() {
            identity.push_str(&Self::version(compiler).await);
        }
        identity
    }

    async fn version(compiler: GoCompiler) -> String {
        compiler
            .version()
            .await
            .unwrap_or_else(|failure| failure.to_string())
    }

    async fn cached_or_fresh(
        (files, choice, release): (&[(PathBuf, String)], &GoCompilerChoice, &WasmReleaseStep),
        cached: &Path,
    ) -> Result<Self, Failure> {
        if let Ok(build) = Self::read(cached) {
            return Ok(build);
        }
        Self::fresh_and_kept((files, choice, release), cached).await
    }

    async fn fresh_and_kept(
        (files, choice, release): (&[(PathBuf, String)], &GoCompilerChoice, &WasmReleaseStep),
        cached: &Path,
    ) -> Result<Self, Failure> {
        let (build, as_expected) = Self::fresh(files, choice, release).await?;
        Self::keep(&build, cached, as_expected);
        Ok(build)
    }

    async fn fresh(
        files: &[(PathBuf, String)],
        choice: &GoCompilerChoice,
        release: &WasmReleaseStep,
    ) -> Result<(Self, bool), Failure> {
        let staged = Self::stage(files)?;
        let compiled = Self::compiled(staged.path(), choice).await?;
        let output = staged.path().join(GoCompiler::OUTPUT);
        let shrunk = release
            .shrink(&output, compiled.compiler.optimizer_flags)
            .await;
        let note = release.release_only(compiled.note(shrunk));
        let (glue, license) = compiled.compiler.glue_and_license().await?;
        let build = Self {
            wasm: native_io(std::fs::read(&output), &output)?,
            glue,
            license,
            note,
        };
        Ok((build, shrunk == release.will_shrink()))
    }

    fn stage(files: &[(PathBuf, String)]) -> Result<tempfile::TempDir, Failure> {
        let staged = tempfile::Builder::new()
            .prefix(".tachyon-go-wasm-")
            .tempdir()
            .map_err(|error| {
                native_tool_failure(
                    Self::FAILURE_CODE,
                    &format!("Cannot stage Go services: {error}"),
                )
            })?;
        for (name, text) in files {
            write(&staged.path().join(name), text.as_bytes())?;
        }
        Ok(staged)
    }

    async fn compiled(staged: &Path, choice: &GoCompilerChoice) -> Result<GoCompiledWith, Failure> {
        let first = choice.first.build(staged).await;
        Self::after_first(first, staged, choice).await
    }

    async fn after_first(
        first: Result<(), Failure>,
        staged: &Path,
        choice: &GoCompilerChoice,
    ) -> Result<GoCompiledWith, Failure> {
        match (first, choice.fallback) {
            (Ok(()), _) => Ok(GoCompiledWith {
                compiler: choice.first,
                reason: String::from(choice.reason),
            }),
            (Err(failure), None) => Err(failure),
            (Err(failure), Some(fallback)) => {
                fallback.build(staged).await?;
                Ok(GoCompiledWith {
                    compiler: fallback,
                    reason: Self::fallback_reason(choice.first, &failure),
                })
            }
        }
    }

    fn fallback_reason(first: GoCompiler, failure: &Failure) -> String {
        let cause = Self::cause(failure);
        format!(
            "because {} could not build these services: {cause}; set tac.go.compiler to \"go\" to skip it",
            first.name
        )
    }

    fn cause(failure: &Failure) -> String {
        let messages: Vec<&str> = failure
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect();
        let text = messages.join(" ");
        let words: Vec<&str> = text.split_whitespace().collect();
        let one_line = words.join(" ");
        one_line.chars().take(Self::MAX_CAUSE_CHARACTERS).collect()
    }

    fn read(cached: &Path) -> std::io::Result<Self> {
        let note = std::fs::read_to_string(cached.join(Self::NOTE))?;
        Ok(Self {
            wasm: std::fs::read(cached.join(GoCompiler::OUTPUT))?,
            glue: std::fs::read(cached.join(Self::GLUE))?,
            license: std::fs::read(cached.join(Self::LICENSE))?,
            note: (!note.is_empty()).then_some(note),
        })
    }

    fn keep(build: &Self, cached: &Path, as_expected: bool) {
        if as_expected {
            let _ = build.save(cached);
        }
    }

    fn save(&self, cached: &Path) -> std::io::Result<()> {
        let cache = cached.parent().unwrap_or(cached);
        std::fs::create_dir_all(cache)?;
        let copy = tempfile::Builder::new()
            .prefix(".saving-")
            .tempdir_in(cache)?;
        std::fs::write(copy.path().join(GoCompiler::OUTPUT), &self.wasm)?;
        std::fs::write(copy.path().join(Self::GLUE), &self.glue)?;
        std::fs::write(copy.path().join(Self::LICENSE), &self.license)?;
        std::fs::write(
            copy.path().join(Self::NOTE),
            self.note.as_deref().unwrap_or_default(),
        )?;
        let kept = copy.keep();
        std::fs::rename(&kept, cached).or_else(|_| std::fs::remove_dir_all(&kept))
    }
}

#[cfg(test)]
mod tests {
    use super::GoWebBuild;
    use crate::native::go_compiler::GoCompiler;
    use crate::native::host::native_tool_failure;

    #[test]
    fn the_fallback_reason_keeps_the_tinygo_cause_from_a_later_line() {
        let failure = native_tool_failure(
            1605,
            "Native tool 'tinygo' failed with status exit status: 1: # services\nerror: could not open the cache directory /Users/someone/Library/Caches/tinygo: permission denied",
        );
        let reason = GoWebBuild::fallback_reason(GoCompiler::TINYGO, &failure);
        assert_eq!(
            reason,
            "because TinyGo could not build these services: Native tool 'tinygo' failed with status exit status: 1: # services error: could not open the cache directory /Users/someone/Library/Caches/tinygo: permission denied; set tac.go.compiler to \"go\" to skip it"
        );
    }

    #[test]
    fn the_fallback_reason_bounds_a_long_cause_and_keeps_the_advice() {
        let detail = "x".repeat(10_000);
        let failure = native_tool_failure(1605, &detail);
        let reason = GoWebBuild::fallback_reason(GoCompiler::TINYGO, &failure);
        let cause_length = reason.matches('x').count();
        assert_eq!(cause_length, GoWebBuild::MAX_CAUSE_CHARACTERS);
        assert!(
            reason.ends_with("; set tac.go.compiler to \"go\" to skip it"),
            "{reason}"
        );
    }
}
