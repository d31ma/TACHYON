use super::runtime_sources::RubyRuntimeSources;
use super::runtime_tools::RubyRuntimeTools;
use crate::Failure;
use crate::doctor::Toolchain;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::Mutex;

pub(crate) struct RubyRuntimeBuild;

impl RubyRuntimeBuild {
    const FILE_NAME: &'static str = "tachyon-ruby.wasm";
    const CACHE_FOLDER: &'static str = "ruby-web";
    const PRISM_FOLDER: &'static str = "mruby/mrbgems/mruby-compiler/lib/prism";
    const LIBRARY: &'static str = "build/tachyon-ruby/lib/libmruby.a";
    const MAXIMUM_JOBS: usize = 4;
    const COMPILE_LIMIT: Duration = Duration::from_mins(40);
    const LINK_LIMIT: Duration = Duration::from_mins(10);

    pub(crate) fn cached_path() -> PathBuf {
        crate::cache::root()
            .join(Self::CACHE_FOLDER)
            .join(RubyRuntimeSources::SOURCES.fingerprint())
            .join(Self::FILE_NAME)
    }

    pub(crate) async fn runtime() -> Result<Vec<u8>, Failure> {
        let _turn = Self::build_turn().lock().await;
        Self::read_or_build(&Self::cached_path()).await
    }

    pub(crate) fn toolchains() -> Vec<Toolchain> {
        Self::toolchains_for(&Self::cached_path())
    }

    #[cfg(test)]
    pub(crate) fn unavailable_reason() -> Option<String> {
        if Self::cached_path().is_file() {
            return None;
        }
        RubyRuntimeTools::readiness()
            .err()
            .map(|failure| format!("the Ruby runtime is not cached and cannot be built: {failure}"))
    }

    fn toolchains_for(cached: &Path) -> Vec<Toolchain> {
        if cached.is_file() {
            return vec![RubyRuntimeTools::cached_row(cached)];
        }
        RubyRuntimeTools::toolchains()
    }

    fn build_turn() -> &'static Mutex<()> {
        static TURN: OnceLock<Mutex<()>> = OnceLock::new();
        TURN.get_or_init(|| Mutex::new(()))
    }

    async fn read_or_build(cached: &Path) -> Result<Vec<u8>, Failure> {
        if let Ok(bytes) = std::fs::read(cached) {
            return Ok(bytes);
        }
        Self::build(cached).await?;
        RubyRuntimeSources::SOURCES.step.read_file(cached)
    }

    async fn build(cached: &Path) -> Result<(), Failure> {
        RubyRuntimeTools::readiness()?;
        let folder = RubyRuntimeSources::SOURCES
            .step
            .work_folder(&crate::cache::root().join(Self::CACHE_FOLDER))?;
        let work = folder.path();
        RubyRuntimeSources::SOURCES.write_into(&work.join("runtime"))?;
        RubyRuntimeSources::SOURCES
            .archive("mruby")?
            .unpack_into(work, "mruby")
            .await?;
        RubyRuntimeSources::SOURCES
            .archive("prism")?
            .unpack_into(work, Self::PRISM_FOLDER)
            .await?;
        Self::compile_library(work).await?;
        Self::link(work).await?;
        RubyRuntimeSources::SOURCES
            .step
            .publish(&work.join(Self::FILE_NAME), cached)
    }

    async fn compile_library(work: &Path) -> Result<(), Failure> {
        let configuration = work.join("runtime").join(RubyRuntimeSources::BUILD_CONFIG);
        let mut command = Command::new("ruby");
        command
            .arg("./minirake")
            .arg(format!("-j{}", Self::jobs()))
            .arg("all")
            .current_dir(work.join("mruby"))
            .env("MRUBY_CONFIG", configuration)
            .env("TAC_RUBY_BUILD_DIR", work.join("build"));
        RubyRuntimeSources::SOURCES
            .step
            .run(&mut command, "the mruby build", Self::COMPILE_LIMIT)
            .await
    }

    async fn link(work: &Path) -> Result<(), Failure> {
        let mut command = Command::new(crate::web_runtime::EmscriptenTool::EMCC);
        command
            .args(crate::web_runtime::EmscriptenTool::LINK_ARGUMENTS)
            .arg(Self::LIBRARY)
            .args(["-o", Self::FILE_NAME])
            .current_dir(work);
        RubyRuntimeSources::SOURCES
            .step
            .run(
                &mut command,
                "emcc to link the Ruby runtime",
                Self::LINK_LIMIT,
            )
            .await
    }

    fn jobs() -> usize {
        std::thread::available_parallelism()
            .map_or(1, NonZeroUsize::get)
            .min(Self::MAXIMUM_JOBS)
    }
}
