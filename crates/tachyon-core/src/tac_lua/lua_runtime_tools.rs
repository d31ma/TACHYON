use super::LuaRuntimeSources;
use crate::Failure;
use crate::doctor::{CommandProbe, Toolchain, ToolchainState};
use crate::web_runtime::EmscriptenTool;
use std::path::Path;

pub(crate) struct LuaRuntimeTools;

impl LuaRuntimeTools {
    const REQUIREMENT: &'static str = "Lua services on the web";
    const LUA: &'static str = "lua";
    const LUA_VERSION: [&'static str; 1] = ["-v"];
    const LUA_INSTALL: &'static str = "Install Lua 5.4 or newer and put its `lua` program on PATH. Tac Lua test units run with it.";
    const MISSING: &'static str = "Tac Lua services need a WebAssembly Lua runtime, and Tachyon builds it once on this machine with Emscripten";

    pub(crate) fn toolchains() -> Vec<Toolchain> {
        let emscripten = Self::emscripten();
        vec![
            Self::row(
                EmscriptenTool::EMCC,
                emscripten.state(),
                &emscripten.install(),
            ),
            Self::lua_row(),
        ]
    }

    pub(crate) fn cached_rows(path: &Path) -> Vec<Toolchain> {
        let detail = format!("built in {}", path.display());
        vec![
            Self::row("tachyon-lua.wasm", ToolchainState::Ready(detail), ""),
            Self::lua_row(),
        ]
    }

    pub(crate) fn readiness() -> Result<(), Failure> {
        let emscripten = Self::emscripten();
        Self::report(&emscripten.state(), &emscripten.install())
    }

    fn emscripten() -> EmscriptenTool {
        EmscriptenTool::new(LuaRuntimeSources::SOURCES.minimum_emscripten())
    }

    fn lua_row() -> Toolchain {
        Self::row(
            Self::LUA,
            CommandProbe::version_state(Self::LUA, &Self::LUA_VERSION),
            Self::LUA_INSTALL,
        )
    }

    fn row(command: &str, state: ToolchainState, install: &str) -> Toolchain {
        Toolchain {
            requirement: String::from(Self::REQUIREMENT),
            command: String::from(command),
            state,
            install: String::from(install),
            required: true,
        }
    }

    fn report(state: &ToolchainState, install: &str) -> Result<(), Failure> {
        if state.is_ready() {
            return Ok(());
        }
        Err(LuaRuntimeSources::SOURCES.step.failure(&format!(
            "{}, and emcc is missing or too old. {install}",
            Self::MISSING
        )))
    }
}
