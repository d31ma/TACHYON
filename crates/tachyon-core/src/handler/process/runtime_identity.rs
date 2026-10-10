use super::HandlerRuntimePrograms;
use crate::Failure;
use crate::failure::diagnostic;
use crate::handler::HandlerSource;
use crate::handler::readiness::{RuntimeRequirement, YonLanguage};
use command_group::{AsyncCommandGroup, AsyncGroupChild};
use std::path::Path;
use tokio::process::Command;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum RuntimeIdentity {
    JavaScript { configured: bool },
    Python { configured: bool },
    Ruby { configured: bool },
    Java,
    Php,
    Lua,
    Dotnet,
    Firecracker,
    PreparedArtifact,
}

impl RuntimeIdentity {
    const JAVASCRIPT: [Self; 2] = [
        Self::JavaScript { configured: false },
        Self::JavaScript { configured: true },
    ];
    #[cfg(windows)]
    const DEFAULT_PYTHON_LABEL: &str = "Yon Python runtime 'python'";
    #[cfg(not(windows))]
    const DEFAULT_PYTHON_LABEL: &str = "Yon Python runtime 'python3'";

    pub(super) fn from_source(source: &HandlerSource, runtimes: &HandlerRuntimePrograms) -> Self {
        Self::for_language(
            YonLanguage::from_path(Path::new(source.relative_path())),
            runtimes,
        )
    }

    pub(super) fn runs_javascript(self) -> bool {
        Self::JAVASCRIPT.contains(&self)
    }

    pub(super) fn spawn(self, command: &mut Command) -> Result<AsyncGroupChild, Failure> {
        command
            .group()
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| self.spawn_failure(&error))
    }

    fn for_language(language: Option<YonLanguage>, runtimes: &HandlerRuntimePrograms) -> Self {
        match language {
            Some(YonLanguage::JavaScript | YonLanguage::TypeScript) => Self::JavaScript {
                configured: !HandlerRuntimePrograms::is_default_javascript(&runtimes.javascript),
            },
            Some(YonLanguage::Python) => Self::Python {
                configured: runtimes.python != Path::new(HandlerRuntimePrograms::DEFAULT_PYTHON),
            },
            Some(YonLanguage::Ruby) => Self::Ruby {
                configured: runtimes.ruby != Path::new(HandlerRuntimePrograms::DEFAULT_RUBY),
            },
            Some(YonLanguage::Java | YonLanguage::Groovy | YonLanguage::Kotlin) => Self::Java,
            Some(YonLanguage::Php) => Self::Php,
            Some(YonLanguage::Lua) => Self::Lua,
            Some(YonLanguage::CSharp | YonLanguage::FSharp) => Self::Dotnet,
            Some(YonLanguage::Rust | YonLanguage::Dart | YonLanguage::Go) | None => {
                Self::PreparedArtifact
            }
        }
    }

    fn spawn_failure(self, error: &std::io::Error) -> Failure {
        if error.kind() == std::io::ErrorKind::NotFound && self != Self::PreparedArtifact {
            return self.missing();
        }
        Failure::one(diagnostic(
            2101,
            "Cannot start the selected handler runtime.",
            Some(String::from(
                "Check the runtime installation, permissions, and deployment policy.",
            )),
            None,
        ))
    }

    fn missing(self) -> Failure {
        let (name, help) = self.name_and_help();
        Failure::one(diagnostic(
            2112,
            format!("Required {name} was not found."),
            Some(String::from(help)),
            None,
        ))
    }

    fn name_and_help(self) -> (&'static str, &'static str) {
        match self {
            Self::JavaScript { configured: false } => (
                "Yon JavaScript runtime 'node'",
                "Install Node.js or Bun, or configure YON_JAVASCRIPT_RUNTIME.",
            ),
            Self::JavaScript { configured: true } => (
                "configured Yon JavaScript runtime",
                "Correct YON_JAVASCRIPT_RUNTIME or --javascript-runtime.",
            ),
            Self::Python { configured: false } => (
                Self::DEFAULT_PYTHON_LABEL,
                "Install Python or configure YON_PYTHON_RUNTIME.",
            ),
            Self::Python { configured: true } => (
                "configured Yon Python runtime",
                "Correct YON_PYTHON_RUNTIME or --python-runtime.",
            ),
            Self::Ruby { configured: false } => (
                "Yon Ruby runtime 'ruby'",
                "Install Ruby 3.3 or newer, or configure YON_RUBY_RUNTIME.",
            ),
            Self::Ruby { configured: true } => (
                "configured Yon Ruby runtime",
                "Correct YON_RUBY_RUNTIME or --ruby-runtime.",
            ),
            Self::Java => ("Yon Java runtime 'java'", "Install a supported JDK."),
            Self::Php => (RuntimeRequirement::PHP_LABEL, RuntimeRequirement::PHP_HELP),
            Self::Lua => (RuntimeRequirement::LUA_LABEL, RuntimeRequirement::LUA_HELP),
            Self::Dotnet => (
                "Yon C# runtime 'dotnet'",
                "Install the .NET SDK and Microsoft.NETCore.App runtime.",
            ),
            Self::Firecracker => (
                "configured Firecracker driver",
                "Correct YON_FIRECRACKER_DRIVER.",
            ),
            Self::PreparedArtifact => (
                "prepared Yon handler artifact",
                "Restart Tachyon so the handler artifact can be prepared again.",
            ),
        }
    }
}
