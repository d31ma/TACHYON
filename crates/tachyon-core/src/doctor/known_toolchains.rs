use super::command_probe::CommandProbe;
use super::{Toolchain, ToolchainState};
use crate::handler::{RuntimeProbeResult, RuntimeProbeState};
use crate::project::NativeCompanion;
use std::fs;
use std::path::Path;

pub(super) struct KnownToolchains;

impl KnownToolchains {
    pub(super) fn runtime(result: &RuntimeProbeResult) -> Toolchain {
        let requirement = result.requirement();
        Toolchain {
            requirement: requirement.label().to_owned(),
            command: requirement.command_label().to_owned(),
            state: Self::runtime_state(result.state()),
            install: requirement.help().to_owned(),
            required: true,
        }
    }

    pub(super) fn typescript(root: &Path) -> Toolchain {
        let local = root.join("node_modules/.bin/tsc");
        let program = Self::typescript_program(&local);
        let state = CommandProbe::run(&program, &["--version"])
            .map_or(ToolchainState::Missing, Self::typescript_state);
        Toolchain {
            requirement: String::from("TypeScript services"),
            command: Self::typescript_command(&program),
            state,
            install: String::from(
                "Install typescript 6 or newer as a project dependency or on PATH.",
            ),
            required: true,
        }
    }

    pub(super) fn fylo(root: &Path) -> Option<Toolchain> {
        Self::uses_fylo(root).then(|| Toolchain {
            requirement: String::from("fylo datasource"),
            command: String::from("fylo"),
            state: CommandProbe::version_state("fylo", &["version"]),
            install: String::from("Install FYLO from https://fylo.del.ma"),
            required: true,
        })
    }

    pub(super) fn dart() -> Toolchain {
        Toolchain {
            requirement: String::from("Dart browser services"),
            command: String::from("dart"),
            state: CommandProbe::version_state("dart", &["--version"]),
            install: String::from(
                "Install the Dart SDK 3.13 or newer from https://dart.dev/get-dart",
            ),
            required: true,
        }
    }

    pub(super) fn native_service(language: NativeCompanion) -> Toolchain {
        let (label, name, arguments, remedy) = Self::native_details(language);
        Toolchain {
            requirement: format!("{label} services for {}", language.target_names()),
            command: String::from(name),
            state: CommandProbe::version_state(name, arguments),
            install: String::from(remedy),
            required: false,
        }
    }

    pub(super) fn web_assembly() -> Toolchain {
        let target = crate::native::RustCompanion::WEB_TARGET;
        let installed =
            CommandProbe::output("rustc", &["--print", "target-libdir", "--target", target])
                .is_some_and(|directory| Path::new(directory.trim()).is_dir());
        Toolchain {
            requirement: String::from("Rust services on the web"),
            command: format!("rustc --target {target}"),
            state: Self::presence_state(installed, target),
            install: format!("rustup target add {target}"),
            required: true,
        }
    }

    pub(super) fn binaryen() -> Toolchain {
        let installed = crate::wasm_optimizer::WasmOptimizer::is_installed();
        Toolchain {
            requirement: String::from("smaller release WebAssembly"),
            command: String::from("wasm-opt"),
            state: Self::presence_state(installed, "wasm-opt"),
            install: String::from(
                "Install Binaryen version_133 or newer from https://github.com/WebAssembly/binaryen/releases and put its bin folder on PATH. Release builds run its wasm-opt on every WebAssembly module to make the download smaller.",
            ),
            required: false,
        }
    }

    pub(super) fn tinygo() -> Toolchain {
        Toolchain {
            requirement: String::from("small Go services on the web"),
            command: String::from("tinygo"),
            state: CommandProbe::version_state("tinygo", &["version"]),
            install: String::from(
                "Install TinyGo 0.42 or newer from https://tinygo.org/getting-started/install/. Release builds compile Go services with TinyGo (several times smaller) and use the standard Go compiler when TinyGo is missing or cannot build them.",
            ),
            required: false,
        }
    }

    pub(super) fn dotnet_web() -> Toolchain {
        let installed =
            CommandProbe::output("dotnet", &["workload", "list"]).is_some_and(|listed| {
                listed
                    .lines()
                    .any(|line| line.trim_start().starts_with("wasm-tools"))
            });
        Toolchain {
            requirement: String::from("C# services on the web"),
            command: String::from("dotnet workload wasm-tools"),
            state: Self::presence_state(installed, "wasm-tools"),
            install: String::from("dotnet workload install wasm-tools"),
            required: true,
        }
    }

    pub(super) fn kotlin_web() -> Toolchain {
        let library = std::env::var_os("TAC_KOTLIN_WASM_STDLIB").map(std::path::PathBuf::from);
        let has_library = library.is_some_and(|path| path.is_file());
        let state = match (
            has_library,
            crate::wasm_optimizer::WasmOptimizer::is_installed(),
        ) {
            (false, _) => ToolchainState::Missing,
            (true, false) => ToolchainState::Incomplete(String::from(
                "wasm-opt (Binaryen) was not found, so release builds ship Kotlin about 3x larger. Install Binaryen or put Emscripten on PATH",
            )),
            (true, true) => CommandProbe::version_state("kotlinc-js", &["-version"]),
        };
        Toolchain {
            requirement: String::from("Kotlin services on the web"),
            command: String::from("kotlinc-js + TAC_KOTLIN_WASM_STDLIB"),
            state,
            install: String::from(
                "Install the Kotlin compiler 2.1.10, download kotlin-stdlib-wasm-js 2.1.10 (.klib) from Maven Central and set TAC_KOTLIN_WASM_STDLIB to that file.",
            ),
            required: true,
        }
    }

    pub(super) fn go_web() -> Toolchain {
        let state =
            CommandProbe::run("go", &["version"]).map_or(ToolchainState::Missing, Self::go_state);
        Toolchain {
            requirement: String::from("Go services on the web"),
            command: String::from("go"),
            state,
            install: String::from(
                "Install Go 1.27 or newer; Tachyon builds Go services with GOOS=js GOARCH=wasm.",
            ),
            required: true,
        }
    }

    fn runtime_state(state: RuntimeProbeState) -> ToolchainState {
        match state {
            RuntimeProbeState::Ready => {
                ToolchainState::Ready(String::from("deployment readiness probe succeeded"))
            }
            RuntimeProbeState::Missing => ToolchainState::Missing,
            RuntimeProbeState::Incomplete => ToolchainState::Incomplete(String::from(
                "deployment readiness probe did not complete successfully",
            )),
        }
    }

    fn typescript_program(local: &Path) -> String {
        if local.is_file() {
            return local.to_string_lossy().into_owned();
        }
        String::from("tsc")
    }

    fn typescript_command(program: &str) -> String {
        if program == "tsc" {
            return String::from("tsc");
        }
        String::from("node_modules/.bin/tsc")
    }

    fn typescript_state(version: String) -> ToolchainState {
        let major = Self::major_version(&version).unwrap_or(0);
        if major >= 6 {
            return ToolchainState::Ready(version);
        }
        ToolchainState::Incomplete(format!("{version}, but 6 or newer is required"))
    }

    fn major_version(version: &str) -> Option<u32> {
        let number = version.split_whitespace().next_back()?;
        let major = number.split('.').next()?;
        major.parse::<u32>().ok()
    }

    fn uses_fylo(root: &Path) -> bool {
        Self::datasource_driver(root).is_some_and(|driver| driver == "fylo")
    }

    fn datasource_driver(root: &Path) -> Option<serde_json::Value> {
        let configuration = fs::read_to_string(root.join("server/application.json")).ok()?;
        let configuration: serde_json::Value = serde_json::from_str(&configuration).ok()?;
        Some(configuration["datasource"]["driver"].clone())
    }

    fn native_details(
        language: NativeCompanion,
    ) -> (
        &'static str,
        &'static str,
        &'static [&'static str],
        &'static str,
    ) {
        match language {
            NativeCompanion::Rust => (
                "Rust",
                "rustc",
                &["--version"],
                "Install Rust from https://rustup.rs",
            ),
            NativeCompanion::Swift => (
                "Swift",
                "swiftc",
                &["--version"],
                "Install Xcode, which provides swiftc",
            ),
            NativeCompanion::Kotlin => (
                "Kotlin",
                "kotlinc",
                &["-version"],
                "Install Kotlin, or let the Android Gradle plugin provide it",
            ),
            NativeCompanion::CSharp => (
                "C#",
                "dotnet",
                &["--version"],
                "Install the .NET SDK from https://dotnet.microsoft.com",
            ),
        }
    }

    fn presence_state(installed: bool, detail: &str) -> ToolchainState {
        if installed {
            return ToolchainState::Ready(String::from(detail));
        }
        ToolchainState::Missing
    }

    fn go_state(version: String) -> ToolchainState {
        if crate::handler::RuntimeProbe::supports_go_version(&version) {
            return ToolchainState::Ready(version);
        }
        ToolchainState::Incomplete(version)
    }
}
