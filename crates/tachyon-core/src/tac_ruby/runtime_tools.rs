use super::runtime_sources::RubyRuntimeSources;
use super::runtime_step::RubyRuntimeStep;
use crate::Failure;
use crate::doctor::{CommandProbe, Toolchain, ToolchainState};
use std::fmt::Write as _;

pub(crate) struct RubyRuntimeTools;

impl RubyRuntimeTools {
    #[cfg(windows)]
    pub(crate) const EMCC: &'static str = "emcc.bat";
    #[cfg(not(windows))]
    pub(crate) const EMCC: &'static str = "emcc";
    #[cfg(target_os = "macos")]
    const HOST_COMPILER: &'static str = "clang";
    #[cfg(not(target_os = "macos"))]
    const HOST_COMPILER: &'static str = "gcc";
    const REQUIREMENT: &'static str = "Ruby services on the web";
    const RUBY_INSTALL: &'static str = "Install Ruby from https://www.ruby-lang.org/en/documentation/installation/. Tachyon runs mruby's build tool with it.";
    const COMPILER_INSTALL: &'static str = "Install a C compiler for mruby's build tools: on macOS run `xcode-select --install`; on Linux install gcc (for example `sudo apt install build-essential`); on Windows run `ridk install` and then `ridk enable` in the RubyInstaller prompt.";

    pub(crate) fn toolchains() -> Vec<Toolchain> {
        vec![
            Self::row(
                Self::EMCC,
                Self::emscripten_state(),
                &Self::emscripten_install(),
            ),
            Self::row(
                "ruby",
                CommandProbe::version_state("ruby", &["--version"]),
                Self::RUBY_INSTALL,
            ),
            Self::row(
                &Self::host_compiler(),
                Self::compiler_state(),
                Self::COMPILER_INSTALL,
            ),
        ]
    }

    pub(crate) fn readiness() -> Result<(), Failure> {
        let missing = Self::not_ready(Self::toolchains());
        Self::report(&missing)
    }

    pub(crate) fn cached_row(path: &std::path::Path) -> Toolchain {
        let detail = format!("built in {}", path.display());
        Self::row("tachyon-ruby.wasm", ToolchainState::Ready(detail), "")
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

    fn emscripten_install() -> String {
        let minimum = RubyRuntimeSources::minimum_emscripten();
        format!(
            "Install Emscripten {minimum} or newer. With the official emsdk (https://emscripten.org/docs/getting_started/downloads.html): `emsdk install latest`, `emsdk activate latest`, then load `emsdk_env.sh` (macOS and Linux) or run `emsdk_env.bat` (Windows). On macOS, `brew install emscripten` also works."
        )
    }

    fn emscripten_state() -> ToolchainState {
        CommandProbe::run(Self::EMCC, &["--version"])
            .map_or(ToolchainState::Missing, Self::version_state)
    }

    fn version_state(line: String) -> ToolchainState {
        let minimum = RubyRuntimeSources::minimum_emscripten();
        if Self::numbers(&line) >= Self::numbers(&minimum) {
            return ToolchainState::Ready(line);
        }
        ToolchainState::Incomplete(format!("{line}, but {minimum} or newer is required"))
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

    fn host_compiler() -> String {
        std::env::var("CC").unwrap_or_else(|_| String::from(Self::HOST_COMPILER))
    }

    fn compiler_state() -> ToolchainState {
        CommandProbe::version_state(&Self::host_compiler(), &["--version"])
    }

    fn not_ready(toolchains: Vec<Toolchain>) -> Vec<Toolchain> {
        toolchains
            .into_iter()
            .filter(|toolchain| !toolchain.state.is_ready())
            .collect()
    }

    fn report(missing: &[Toolchain]) -> Result<(), Failure> {
        if missing.is_empty() {
            return Ok(());
        }
        Err(RubyRuntimeStep::failure(&Self::missing_message(missing)))
    }

    fn missing_message(missing: &[Toolchain]) -> String {
        let mut message = String::from(
            "Tac Ruby services need a WebAssembly Ruby runtime, and Tachyon builds it once on this machine. These tools are missing or too old:",
        );
        for toolchain in missing {
            let _ = write!(message, "\n- {}: {}", toolchain.command, toolchain.install);
        }
        message
    }
}

#[cfg(test)]
mod tests {
    use super::RubyRuntimeTools;
    use crate::doctor::ToolchainState;

    #[test]
    fn the_readiness_failure_lists_each_missing_tool_with_its_install_steps() {
        let rows = vec![
            RubyRuntimeTools::row(
                RubyRuntimeTools::EMCC,
                ToolchainState::Missing,
                &RubyRuntimeTools::emscripten_install(),
            ),
            RubyRuntimeTools::row(
                "ruby",
                ToolchainState::Ready(String::from("ruby 4.0.6")),
                RubyRuntimeTools::RUBY_INSTALL,
            ),
        ];
        let missing = RubyRuntimeTools::not_ready(rows);
        let Err(failure) = RubyRuntimeTools::report(&missing) else {
            panic!("a missing Emscripten must stop the build");
        };
        let text = failure.to_string();
        assert!(
            text.contains("Tachyon builds it once on this machine"),
            "{text}"
        );
        assert!(text.contains("emsdk install latest"), "{text}");
        assert!(text.contains("brew install emscripten"), "{text}");
        assert!(text.contains("Emscripten 6.0.10 or newer"), "{text}");
        assert!(!text.contains("ruby-lang.org"), "{text}");
        assert!(RubyRuntimeTools::report(&[]).is_ok());
    }

    #[test]
    fn emscripten_must_be_at_least_the_verified_version() {
        let line = |version: &str| {
            format!(
                "emcc (Emscripten gcc/clang-like replacement + linker emulating GNU ld) {version} (d6c521a)"
            )
        };
        assert!(matches!(
            RubyRuntimeTools::version_state(line("6.0.10")),
            ToolchainState::Ready(_)
        ));
        assert!(matches!(
            RubyRuntimeTools::version_state(line("7.1.0")),
            ToolchainState::Ready(_)
        ));
        let ToolchainState::Incomplete(reason) = RubyRuntimeTools::version_state(line("6.0.9"))
        else {
            panic!("an older Emscripten must be reported");
        };
        assert!(
            reason.ends_with("but 6.0.10 or newer is required"),
            "{reason}"
        );
    }
}
