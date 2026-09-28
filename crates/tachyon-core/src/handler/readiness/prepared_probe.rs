use crate::handler::EnvironmentPolicy;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command as StdCommand;

pub(super) struct PreparedProbe {
    pub(super) command: StdCommand,
    pub(super) workspace: Option<tempfile::TempDir>,
}

impl PreparedProbe {
    pub(super) fn apply_dotnet_build_environment(&mut self, environment: &EnvironmentPolicy) {
        environment.apply_std(&mut self.command);
        let variables = self
            .workspace
            .as_ref()
            .map(|workspace| Self::dotnet_variables(workspace.path()))
            .unwrap_or_default();
        self.command.envs(variables);
    }

    fn dotnet_variables(workspace: &Path) -> Vec<(&'static str, OsString)> {
        vec![
            ("DOTNET_CLI_HOME", workspace.as_os_str().to_os_string()),
            ("DOTNET_CLI_TELEMETRY_OPTOUT", OsString::from("1")),
            ("DOTNET_NOLOGO", OsString::from("1")),
            ("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", OsString::from("1")),
            (
                "NUGET_PACKAGES",
                workspace.join("packages").into_os_string(),
            ),
        ]
    }
}
