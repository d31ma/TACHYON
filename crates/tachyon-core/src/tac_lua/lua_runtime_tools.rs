use super::LuaRuntimeSources;
use crate::Failure;
use crate::doctor::{Toolchain, ToolchainState};
use crate::web_runtime::EmscriptenTool;
use std::path::Path;

pub(crate) struct LuaRuntimeTools;

impl LuaRuntimeTools {
    const REQUIREMENT: &'static str = "Lua services on the web";
    const MISSING: &'static str = "Tac Lua services need a WebAssembly Lua runtime, and Tachyon builds it once on this machine with Emscripten";

    pub(crate) fn toolchains() -> Vec<Toolchain> {
        let emscripten = Self::emscripten();
        vec![Self::row(
            EmscriptenTool::EMCC,
            emscripten.state(),
            &emscripten.install(),
        )]
    }

    pub(crate) fn readiness() -> Result<(), Failure> {
        let emscripten = Self::emscripten();
        Self::report(&emscripten.state(), &emscripten.install())
    }

    pub(crate) fn cached_row(path: &Path) -> Toolchain {
        let detail = format!("built in {}", path.display());
        Self::row("tachyon-lua.wasm", ToolchainState::Ready(detail), "")
    }

    fn emscripten() -> EmscriptenTool {
        EmscriptenTool::new(LuaRuntimeSources::minimum_emscripten())
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
        Err(LuaRuntimeSources::STEP.failure(&format!(
            "{}, and emcc is missing or too old. {install}",
            Self::MISSING
        )))
    }
}
