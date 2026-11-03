use crate::doctor::{CommandProbe, ToolchainState};

pub(crate) struct EmscriptenTool {
    minimum: String,
}

impl EmscriptenTool {
    pub(crate) const EMCC: &'static str = "emcc";

    pub(crate) const LINK_ARGUMENTS: [&'static str; 9] = [
        "-Oz",
        "-fwasm-exceptions",
        "-sSUPPORT_LONGJMP=wasm",
        "-sSTANDALONE_WASM",
        "--no-entry",
        "-sALLOW_MEMORY_GROWTH=1",
        "-sINITIAL_MEMORY=4MB",
        "-sSTACK_SIZE=1MB",
        "-sEXPORTED_FUNCTIONS=_tac_boot,_tac_invoke,_tac_alloc,_tac_free",
    ];

    pub(crate) fn new(minimum: String) -> Self {
        Self { minimum }
    }

    pub(crate) fn install(&self) -> String {
        format!(
            "Install Emscripten {} or newer. With the official emsdk (https://emscripten.org/docs/getting_started/downloads.html): `emsdk install latest`, `emsdk activate latest`, then load `emsdk_env.sh` (macOS and Linux) or run `emsdk_env.bat` (Windows). On macOS, `brew install emscripten` also works.",
            self.minimum
        )
    }

    pub(crate) fn state(&self) -> ToolchainState {
        CommandProbe::run(Self::EMCC, &["--version"])
            .map_or(ToolchainState::Missing, |line| self.version_state(line))
    }

    pub(crate) fn version_state(&self, line: String) -> ToolchainState {
        let recent = Self::numbers(&line) >= Self::parts(&self.minimum);
        self.state_of(line, recent)
    }

    fn state_of(&self, line: String, recent: bool) -> ToolchainState {
        if recent {
            return ToolchainState::Ready(line);
        }
        ToolchainState::Incomplete(format!("{line}, but {} or newer is required", self.minimum))
    }

    fn numbers(text: &str) -> Vec<u32> {
        let version = text.split_whitespace().find(|word| Self::is_version(word));
        Self::parts(version.unwrap_or_default())
    }

    fn is_version(word: &str) -> bool {
        word.contains('.') && word.split('.').all(|part| part.parse::<u32>().is_ok())
    }

    fn parts(version: &str) -> Vec<u32> {
        version
            .split('.')
            .filter_map(|part| part.parse().ok())
            .collect()
    }
}
