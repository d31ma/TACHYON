use super::{LuaRuntimeSources, LuaRuntimeTools};
use crate::Failure;
use crate::doctor::Toolchain;
use crate::web_runtime::EmscriptenTool;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::Mutex;

pub(crate) struct LuaRuntimeBuild;

impl LuaRuntimeBuild {
    const FILE_NAME: &'static str = "tachyon-lua.wasm";
    const CACHE_FOLDER: &'static str = "lua-web";
    const SOURCE_FOLDER: &'static str = "lua";
    const INCLUDE: &'static str = "-Ilua/src";
    const LINK_LIMIT: Duration = Duration::from_mins(30);
    const LUA_FILES: [&'static str; 27] = [
        "lapi", "lauxlib", "lbaselib", "lcode", "lcorolib", "lctype", "ldebug", "ldo", "ldump",
        "lfunc", "lgc", "llex", "lmathlib", "lmem", "lobject", "lopcodes", "lparser", "lstate",
        "lstring", "lstrlib", "ltable", "ltablib", "ltm", "lundump", "lutf8lib", "lvm", "lzio",
    ];

    pub(crate) fn cached_path() -> PathBuf {
        crate::cache::root()
            .join(Self::CACHE_FOLDER)
            .join(LuaRuntimeSources::SOURCES.fingerprint())
            .join(Self::FILE_NAME)
    }

    pub(crate) async fn runtime() -> Result<Vec<u8>, Failure> {
        let _turn = Self::build_turn().lock().await;
        let cached = Self::cached_path();
        Self::build_if_missing(&cached).await?;
        LuaRuntimeSources::SOURCES.step.read_file(&cached)
    }

    pub(crate) fn toolchains() -> Vec<Toolchain> {
        Self::toolchains_for(&Self::cached_path())
    }

    #[cfg(test)]
    pub(crate) fn unavailable_reason() -> Option<String> {
        let ready = [
            Self::cached_path().is_file(),
            LuaRuntimeTools::readiness().is_ok(),
        ];
        (!ready.contains(&true)).then(|| {
            String::from("the Lua runtime is not cached and emcc is not available to build it")
        })
    }

    fn build_turn() -> &'static Mutex<()> {
        static TURN: OnceLock<Mutex<()>> = OnceLock::new();
        TURN.get_or_init(|| Mutex::new(()))
    }

    fn toolchains_for(cached: &Path) -> Vec<Toolchain> {
        if cached.is_file() {
            return LuaRuntimeTools::cached_rows(cached);
        }
        LuaRuntimeTools::toolchains()
    }

    async fn build_if_missing(cached: &Path) -> Result<(), Failure> {
        if cached.is_file() {
            return Ok(());
        }
        Self::build(cached).await
    }

    async fn build(cached: &Path) -> Result<(), Failure> {
        LuaRuntimeTools::readiness()?;
        let sources = LuaRuntimeSources::SOURCES;
        let folder = sources
            .step
            .work_folder(&crate::cache::root().join(Self::CACHE_FOLDER))?;
        let work = folder.path();
        sources.write_into(&work.join("runtime"))?;
        sources
            .archive(Self::SOURCE_FOLDER)?
            .unpack_into(work, Self::SOURCE_FOLDER)
            .await?;
        Self::link(work).await?;
        sources.step.publish(&work.join(Self::FILE_NAME), cached)
    }

    async fn link(work: &Path) -> Result<(), Failure> {
        let mut command = Command::new(EmscriptenTool::EMCC);
        command
            .args(EmscriptenTool::LINK_ARGUMENTS)
            .arg(Self::INCLUDE)
            .args(Self::LUA_FILES.map(|name| format!("lua/src/{name}.c")))
            .arg(format!("runtime/{}", LuaRuntimeSources::RUNTIME_SOURCE))
            .args(["-o", Self::FILE_NAME])
            .current_dir(work);
        LuaRuntimeSources::SOURCES
            .step
            .run(
                &mut command,
                "emcc to build the Lua runtime",
                Self::LINK_LIMIT,
            )
            .await
    }
}
