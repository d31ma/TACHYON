use super::{PreparedProbe, ProbeKind, RuntimeRequirementKey};
use crate::Failure;
use crate::failure::diagnostic;
use crate::handler::{FirecrackerIsolation, HandlerRuntimePrograms};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct RuntimeRequirement {
    pub(super) key: RuntimeRequirementKey,
    pub(super) program: PathBuf,
    pub(super) command_label: String,
    pub(super) label: String,
    pub(super) help: String,
    pub(super) probe: ProbeKind,
}

impl RuntimeRequirement {
    pub(super) const PROBE_TIME_LIMIT: Duration = Duration::from_secs(10);
    pub(super) const DOTNET_BUILD_TIME_LIMIT: Duration = Duration::from_mins(1);
    pub(super) const PROBE_OUTPUT_BYTES: usize = 64 * 1024;
    pub(super) const OLDEST_GO_MINOR_VERSION: u32 = 27;
    pub(super) const OLDEST_RUBY_VERSION: (u32, u32) = (3, 3);
    pub(super) const OLDEST_LUA_VERSION: (u32, u32) = (5, 4);
    pub(super) const OLDEST_NODE_PROXY_VERSION: (u32, u32) = (24, 5);
    pub(super) const BUN_IDENTITY: &str = "bun";
    const NODE_PROXY_PROBE: &str = "typeof Bun === 'undefined' ? process.version : 'bun'";
    const JAVASCRIPT_HELP: &str = "Install Node.js or Bun, or correct YON_JAVASCRIPT_RUNTIME.";
    const NODE_PROXY_HELP: &str = "Tachyon sends Yon's outgoing HTTP requests through a local guard \
        that allows only the hosts in outgoing.hosts (ADR 0068). Node.js sends fetch and http \
        requests to the guard only from version 24.5, when NODE_USE_ENV_PROXY=1 is set. An older \
        Node.js connects directly to any host, so Tachyon does not run JavaScript handlers with it. \
        Install Node.js 24.5 or newer, or Bun, or correct YON_JAVASCRIPT_RUNTIME.";
    const LUA_PROBE: &str = "io.write((_VERSION:gsub('^Lua ', '')))";
    pub(crate) const LUA_LABEL: &str = "Yon Lua runtime 'lua'";
    pub(crate) const LUA_HELP: &str =
        "Install Lua 5.4 or newer and put its `lua` program on PATH before starting Tachyon.";
    const RUBY_PROBE: &str = "require \"json\"; JSON.generate(1, strict: true); print RUBY_VERSION";
    pub(super) const GO_LABEL: &str = "Yon Go compiler 'go'";
    pub(super) const GO_HELP: &str = "Install Go 1.27 or newer before starting Tachyon.";
    pub(super) const KOTLIN_LABEL: &str = "Yon Kotlin compiler 'kotlinc'";
    pub(super) const KOTLIN_HELP: &str = "Install the Kotlin compiler before starting Tachyon.";
    const GROOVY_COMMAND: &str = "groovyc";
    pub(super) const GROOVY_LABEL: &str = "Yon Groovy compiler 'groovyc'";
    pub(super) const GROOVY_HELP: &str =
        "Install Apache Groovy 4 or newer, then put its bin folder on PATH or set GROOVY_HOME.";
    pub(super) const DOTNET_LABEL: &str = "Yon C# runtime 'dotnet'";
    pub(super) const DOTNET_HELP: &str = "Install the .NET SDK and Microsoft.NETCore.App runtime.";
    const RUST_LABEL: &str = "Yon Rust compiler 'rustc'";
    pub(super) const CARGO_LABEL: &str = "Yon Rust build tool 'cargo'";
    pub(super) const RUST_HELP: &str = "Install Rust before starting Tachyon.";
    pub(super) const DART_LABEL: &str = "Yon Dart compiler 'dart'";
    pub(super) const DART_HELP: &str = "Install the Dart SDK before starting Tachyon.";
    pub(super) const CPP_LABEL: &str = "Yon C++ compiler 'clang++'";
    pub(super) const CPP_HELP: &str = "Install clang++ with C++20 support (Xcode Command Line Tools on macOS, the clang package on Linux, LLVM on Windows) before starting Tachyon.";
    pub(crate) const PHP_LABEL: &str = "Yon PHP runtime 'php-cgi'";
    pub(crate) const PHP_HELP: &str = "Tachyon runs PHP with php-cgi, because header() only works there. \
        Homebrew's php already includes php-cgi. On Debian or Ubuntu, install the php-cgi package. \
        On Windows, the PHP zip file has php-cgi.exe; add its folder to PATH.";
    const COMPILERS: [(&'static str, &'static str, &'static str); 8] = [
        ("kotlinc", Self::KOTLIN_LABEL, Self::KOTLIN_HELP),
        ("groovyc", Self::GROOVY_LABEL, Self::GROOVY_HELP),
        ("dotnet", Self::DOTNET_LABEL, Self::DOTNET_HELP),
        ("rustc", Self::RUST_LABEL, Self::RUST_HELP),
        ("cargo", Self::CARGO_LABEL, Self::RUST_HELP),
        ("dart", Self::DART_LABEL, Self::DART_HELP),
        ("go", Self::GO_LABEL, Self::GO_HELP),
        ("clang++", Self::CPP_LABEL, Self::CPP_HELP),
    ];

    pub(crate) fn compiler_label_and_help(program: &str) -> Option<(&'static str, &'static str)> {
        Self::COMPILERS
            .into_iter()
            .find(|(name, _, _)| *name == program)
            .map(|(_, label, help)| (label, help))
    }

    pub(crate) const fn key(&self) -> RuntimeRequirementKey {
        self.key
    }

    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    pub(crate) fn command_label(&self) -> &str {
        &self.command_label
    }

    pub(crate) fn help(&self) -> &str {
        &self.help
    }

    pub(super) fn configured(
        key: RuntimeRequirementKey,
        program: &Path,
        default: &str,
        family: &str,
        environment: &str,
        help: &str,
        probe: ProbeKind,
    ) -> Self {
        let (label, command_label) = Self::configured_labels(program, default, family, environment);
        Self {
            key,
            program: program.to_path_buf(),
            command_label: String::from(command_label),
            label,
            help: String::from(help),
            probe,
        }
    }

    pub(super) fn javascript(runtimes: &HandlerRuntimePrograms, typescript: bool) -> Self {
        Self::configured(
            RuntimeRequirementKey::JavaScript,
            &runtimes.javascript,
            HandlerRuntimePrograms::default_javascript_for(&runtimes.javascript),
            "Yon JavaScript runtime",
            "YON_JAVASCRIPT_RUNTIME",
            Self::JAVASCRIPT_HELP,
            ProbeKind::JavaScript { typescript },
        )
    }

    pub(crate) fn node_proxy_switch(runtimes: &HandlerRuntimePrograms) -> Self {
        Self {
            key: RuntimeRequirementKey::NodeProxySwitch,
            help: String::from(Self::NODE_PROXY_HELP),
            probe: ProbeKind::NodeProxySwitch,
            ..Self::javascript(runtimes, false)
        }
    }

    pub(crate) fn includes_javascript(requirements: &[Self]) -> bool {
        requirements
            .iter()
            .any(|requirement| requirement.key == RuntimeRequirementKey::JavaScript)
    }

    pub(super) fn fixed(
        key: RuntimeRequirementKey,
        program: &str,
        label: &str,
        help: &str,
        probe: ProbeKind,
    ) -> Self {
        Self {
            key,
            program: PathBuf::from(program),
            command_label: String::from(program),
            label: String::from(label),
            help: String::from(help),
            probe,
        }
    }

    pub(super) fn groovy_runtime() -> Self {
        let runtime_jar = crate::groovy_home::GroovyHome::find()
            .and_then(|home| home.runtime_jar())
            .unwrap_or_else(|| PathBuf::from(Self::GROOVY_COMMAND));
        Self {
            key: RuntimeRequirementKey::GroovyRuntime,
            program: runtime_jar,
            command_label: String::from(Self::GROOVY_COMMAND),
            label: String::from(Self::GROOVY_LABEL),
            help: String::from(Self::GROOVY_HELP),
            probe: ProbeKind::File,
        }
    }

    pub(super) fn gateway_program(program: &str) -> Self {
        Self {
            key: RuntimeRequirementKey::GatewayProgram,
            program: PathBuf::from(program),
            command_label: program.to_owned(),
            label: format!("gateway program '{program}'"),
            help: format!(
                "Install '{program}' on the PATH, or change the Gateway command that names it."
            ),
            probe: ProbeKind::OnPath,
        }
    }

    pub(super) fn firecracker_driver(policy: &FirecrackerIsolation) -> Self {
        Self {
            key: RuntimeRequirementKey::FirecrackerDriver,
            program: policy.driver().to_path_buf(),
            command_label: String::from("YON_FIRECRACKER_DRIVER"),
            label: String::from("configured Firecracker driver (YON_FIRECRACKER_DRIVER)"),
            help: String::from(
                "Install or correct YON_FIRECRACKER_DRIVER before starting Tachyon.",
            ),
            probe: ProbeKind::File,
        }
    }

    pub(super) fn prepare(&self) -> Result<PreparedProbe, Failure> {
        let mut command = crate::kotlin_home::KotlinHome::command(&self.program);
        let workspace = self.prepare_arguments(&mut command)?;
        Ok(PreparedProbe { command, workspace })
    }

    pub(super) fn prepare_dotnet_build(&self, sdk_major: u32) -> Result<PreparedProbe, Failure> {
        let workspace = Self::probe_workspace()?;
        Self::write_probe(
            &workspace.path().join("Readiness.cs"),
            "internal static class Readiness { private static void Main() {} }\n",
        )?;
        Self::write_probe(
            &workspace.path().join("Readiness.csproj"),
            &format!(
                "<Project Sdk=\"Microsoft.NET.Sdk\">\n  <PropertyGroup>\n    \
                 <OutputType>Exe</OutputType>\n    \
                 <TargetFramework>net{sdk_major}.0</TargetFramework>\n    \
                 <EnableDefaultCompileItems>false</EnableDefaultCompileItems>\n    \
                 <RestoreIgnoreFailedSources>true</RestoreIgnoreFailedSources>\n    \
                 <NuGetAudit>false</NuGetAudit>\n  \
                 </PropertyGroup>\n  <ItemGroup><Compile Include=\"Readiness.cs\" /></ItemGroup>\n\
                 </Project>\n"
            ),
        )?;
        Self::write_probe(
            &workspace.path().join("NuGet.Config"),
            "<configuration><packageSources><clear /></packageSources></configuration>\n",
        )?;
        let mut command = StdCommand::new(&self.program);
        command
            .args([
                "build",
                "Readiness.csproj",
                "--nologo",
                "--verbosity",
                "quiet",
                "--disable-build-servers",
            ])
            .current_dir(workspace.path());
        Ok(PreparedProbe {
            command,
            workspace: Some(workspace),
        })
    }

    fn configured_labels<'a>(
        program: &Path,
        default: &'a str,
        family: &str,
        environment: &'a str,
    ) -> (String, &'a str) {
        if program == Path::new(default) {
            return (format!("{family} '{default}'"), default);
        }
        (format!("configured {family} ({environment})"), environment)
    }

    fn prepare_arguments(
        &self,
        command: &mut StdCommand,
    ) -> Result<Option<tempfile::TempDir>, Failure> {
        match self.probe {
            ProbeKind::JavaScript { typescript } => {
                Self::prepare_javascript_probe(command, typescript).map(Some)
            }
            ProbeKind::NodeProxySwitch => {
                command.args(["-p", Self::NODE_PROXY_PROBE]);
                Ok(None)
            }
            ProbeKind::Python => {
                command.args(["-I", "-B", "-c", "class ReadinessController: pass"]);
                Ok(None)
            }
            ProbeKind::LuaVersion => {
                command.args(["-e", Self::LUA_PROBE]);
                Ok(None)
            }
            ProbeKind::RubyVersion => {
                command.args([
                    "--disable=rubyopt",
                    "-E",
                    "UTF-8:UTF-8",
                    "-e",
                    Self::RUBY_PROBE,
                ]);
                Ok(None)
            }
            ProbeKind::Java {
                source_launcher: true,
            } => Self::prepare_java_source_probe(command).map(Some),
            ProbeKind::Java {
                source_launcher: false,
            }
            | ProbeKind::DotnetBuild => {
                command.arg("--version");
                Ok(None)
            }
            ProbeKind::Version(arguments) => {
                command.args(arguments);
                Ok(None)
            }
            ProbeKind::GoVersion => {
                command.arg("version");
                Ok(None)
            }
            ProbeKind::File | ProbeKind::OnPath => Ok(None),
        }
    }

    fn prepare_javascript_probe(
        command: &mut StdCommand,
        typescript: bool,
    ) -> Result<tempfile::TempDir, Failure> {
        let directory = Self::probe_workspace()?;
        let (name, handler) = Self::javascript_probe_source(typescript);
        let source = directory.path().join(name);
        Self::write_probe(&source, handler)?;
        Self::add_no_warnings(command, typescript);
        command.arg(&source);
        Ok(directory)
    }

    fn javascript_probe_source(typescript: bool) -> (&'static str, &'static str) {
        if typescript {
            return (
                "probe.ts",
                "export class Handler { static GET(): object { return {}; } }\n",
            );
        }
        (
            "probe.js",
            "export class Handler { static GET() { return {}; } }\n",
        )
    }

    fn add_no_warnings(command: &mut StdCommand, typescript: bool) {
        if !typescript {
            command.arg("--no-warnings");
        }
    }

    fn prepare_java_source_probe(command: &mut StdCommand) -> Result<tempfile::TempDir, Failure> {
        let directory = Self::probe_workspace()?;
        let source = directory.path().join("Readiness.java");
        Self::write_probe(
            &source,
            "final class Readiness { public static void main(String[] args) {} }\n",
        )?;
        command.arg(&source);
        Ok(directory)
    }

    fn probe_workspace() -> Result<tempfile::TempDir, Failure> {
        tempfile::Builder::new()
            .prefix("tachyon-readiness-")
            .tempdir()
            .map_err(|_| Self::readiness_internal("Cannot create the runtime readiness workspace."))
    }

    fn write_probe(path: &Path, source: &str) -> Result<(), Failure> {
        fs::write(path, source)
            .map_err(|_| Self::readiness_internal("Cannot write the runtime readiness probe."))
    }

    fn readiness_internal(message: &str) -> Failure {
        Failure::one(diagnostic(
            2101,
            message,
            Some(String::from("Check temporary storage and retry startup.")),
            None,
        ))
    }
}
