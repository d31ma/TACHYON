use crate::doctor::{CommandProbe, Toolchain};
use crate::web_runtime::EmscriptenTool;

pub(crate) struct CppTools;

impl CppTools {
    pub(crate) const EMSCRIPTEN_VERSION: &'static str = "6.0.10";
    pub(crate) const TEST_COMPILER_SETTING: &'static str = "TAC_CPP_TEST_COMPILER";
    const REQUIREMENT: &'static str = "C++ services on the web";
    const TEST_REQUIREMENT: &'static str = "C++ test units";
    const DEFAULT_TEST_COMPILER: &'static str = "clang++";
    const VERSION: [&'static str; 1] = ["--version"];
    const TEST_INSTALL: &'static str = "Install clang++ (Xcode Command Line Tools on macOS, the clang package on Linux, LLVM on Windows) or set TAC_CPP_TEST_COMPILER to a C++20 compiler. `ty test` compiles C++ test units with it.";

    pub(crate) fn toolchains() -> Vec<Toolchain> {
        let emscripten = EmscriptenTool::new(String::from(Self::EMSCRIPTEN_VERSION));
        let test_compiler = Self::test_compiler();
        vec![
            Toolchain {
                requirement: String::from(Self::REQUIREMENT),
                command: String::from(EmscriptenTool::EMCC),
                state: emscripten.state(),
                install: emscripten.install(),
                required: true,
            },
            Toolchain {
                requirement: String::from(Self::TEST_REQUIREMENT),
                state: CommandProbe::version_state(&test_compiler, &Self::VERSION),
                command: test_compiler,
                install: String::from(Self::TEST_INSTALL),
                required: false,
            },
        ]
    }

    pub(crate) fn test_compiler() -> String {
        std::env::var(Self::TEST_COMPILER_SETTING)
            .unwrap_or_else(|_| String::from(Self::DEFAULT_TEST_COMPILER))
    }
}
