use super::tool_entry::ToolEntry;

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum NeededTool {
    Bun,
    TypeScript,
    Python,
    Php,
    Ruby,
    Lua,
    CCompiler,
    Emscripten,
    Java,
    Groovy,
    Kotlin,
    Gradle,
    DotNet,
    WasmTools,
    Rust,
    WasmTarget,
    Go,
    TinyGo,
    Binaryen,
    Dart,
    Xcode,
    AndroidSdk,
    LinuxDesktop,
    TeaVm,
}

impl NeededTool {
    pub const ALL: [Self; 24] = [
        Self::Bun,
        Self::TypeScript,
        Self::Python,
        Self::Php,
        Self::Ruby,
        Self::Lua,
        Self::CCompiler,
        Self::Emscripten,
        Self::Java,
        Self::Groovy,
        Self::Kotlin,
        Self::Gradle,
        Self::DotNet,
        Self::WasmTools,
        Self::Rust,
        Self::WasmTarget,
        Self::Go,
        Self::TinyGo,
        Self::Binaryen,
        Self::Dart,
        Self::Xcode,
        Self::AndroidSdk,
        Self::LinuxDesktop,
        Self::TeaVm,
    ];
    const ENTRIES: [ToolEntry; 24] = [
        ToolEntry {
            label: "Bun, the JavaScript and TypeScript runtime",
            commands: &["bun", "node", "YON_JAVASCRIPT_RUNTIME"],
        },
        ToolEntry {
            label: "the TypeScript compiler of this project",
            commands: &["tsc", "node_modules/.bin/tsc"],
        },
        ToolEntry {
            label: "Python",
            commands: &["python3", "python"],
        },
        ToolEntry {
            label: "PHP with php-cgi",
            commands: &["php-cgi", "php"],
        },
        ToolEntry {
            label: "Ruby",
            commands: &["ruby"],
        },
        ToolEntry {
            label: "Lua 5.4 or newer",
            commands: &["lua"],
        },
        ToolEntry {
            label: "a C compiler",
            commands: &["clang", "gcc", "cc"],
        },
        ToolEntry {
            label: "Emscripten",
            commands: &["emcc"],
        },
        ToolEntry {
            label: "a Java JDK",
            commands: &["java", "javac"],
        },
        ToolEntry {
            label: "Apache Groovy",
            commands: &["groovyc"],
        },
        ToolEntry {
            label: "Kotlin",
            commands: &["kotlinc"],
        },
        ToolEntry {
            label: "Gradle",
            commands: &["gradle"],
        },
        ToolEntry {
            label: "the .NET SDK",
            commands: &["dotnet"],
        },
        ToolEntry {
            label: "the .NET wasm-tools workload",
            commands: &["dotnet workload wasm-tools"],
        },
        ToolEntry {
            label: "Rust",
            commands: &["cargo", "rustc"],
        },
        ToolEntry {
            label: "the Rust WebAssembly target",
            commands: &["rustc --target wasm32-unknown-unknown"],
        },
        ToolEntry {
            label: "Go",
            commands: &["go"],
        },
        ToolEntry {
            label: "TinyGo, the small Go compiler for release builds",
            commands: &["tinygo"],
        },
        ToolEntry {
            label: "Binaryen, whose wasm-opt makes release WebAssembly smaller",
            commands: &["wasm-opt"],
        },
        ToolEntry {
            label: "the Dart SDK",
            commands: &["dart"],
        },
        ToolEntry {
            label: "Xcode",
            commands: &["swiftc", "xcrun"],
        },
        ToolEntry {
            label: "the Android SDK",
            commands: &["ANDROID_HOME"],
        },
        ToolEntry {
            label: "the Linux desktop libraries (GTK and WebKitGTK)",
            commands: &["pkg-config"],
        },
        ToolEntry {
            label: "TeaVM 0.15.0, which compiles Java services to WebAssembly",
            commands: &["teavm"],
        },
    ];

    #[must_use]
    pub fn label(self) -> &'static str {
        Self::ENTRIES[self as usize].label
    }

    #[must_use]
    pub fn command(self) -> &'static str {
        self.entry().commands.first().copied().unwrap_or_default()
    }

    #[must_use]
    pub fn for_command(command: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|tool| tool.entry().commands.contains(&command))
    }

    fn entry(self) -> ToolEntry {
        Self::ENTRIES[self as usize]
    }
}
