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
        let mut variables = vec![
            ("DOTNET_CLI_HOME", workspace.as_os_str().to_os_string()),
            ("DOTNET_CLI_TELEMETRY_OPTOUT", OsString::from("1")),
            ("DOTNET_NOLOGO", OsString::from("1")),
            ("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", OsString::from("1")),
            (
                "NUGET_PACKAGES",
                workspace.join("packages").into_os_string(),
            ),
        ];
        if cfg!(windows) {
            variables.extend(Self::windows_folder_variables(workspace, |name| {
                std::env::var_os(name)
            }));
        }
        variables
    }

    /// `NuGet`'s .NET build reads the Windows special folders from these
    /// variables, not from the shell, so a cleared environment makes restore
    /// fail with a null path. The user folders stay inside the probe workspace,
    /// like `DOTNET_CLI_HOME`, so the probe never reads the user's `NuGet`
    /// configuration or credentials; the machine folders are fixed system paths.
    fn windows_folder_variables(
        workspace: &Path,
        host: impl Fn(&str) -> Option<OsString>,
    ) -> Vec<(&'static str, OsString)> {
        const MACHINE_FOLDERS: [&str; 4] = [
            "ProgramData",
            "ProgramFiles",
            "ProgramFiles(x86)",
            "SystemDrive",
        ];
        let profile = workspace.join("profile");
        let mut variables = vec![
            ("USERPROFILE", profile.clone().into_os_string()),
            (
                "APPDATA",
                profile.join("AppData").join("Roaming").into_os_string(),
            ),
            (
                "LOCALAPPDATA",
                profile.join("AppData").join("Local").into_os_string(),
            ),
        ];
        variables.extend(
            MACHINE_FOLDERS
                .into_iter()
                .filter_map(|name| host(name).map(|value| (name, value))),
        );
        variables
    }
}

#[cfg(test)]
mod tests {
    use super::PreparedProbe;
    use std::ffi::OsString;
    use std::path::Path;

    #[test]
    fn windows_dotnet_probes_get_workspace_user_folders_and_host_machine_folders() {
        let workspace = Path::new("probe-workspace");
        let host = |name: &str| {
            ["ProgramData", "ProgramFiles"]
                .contains(&name)
                .then(|| OsString::from(format!("host-{name}")))
        };
        let variables = PreparedProbe::windows_folder_variables(workspace, host);
        let names: Vec<&str> = variables.iter().map(|(name, _)| *name).collect();
        assert_eq!(
            names,
            [
                "USERPROFILE",
                "APPDATA",
                "LOCALAPPDATA",
                "ProgramData",
                "ProgramFiles"
            ]
        );
        assert!(
            variables[..3]
                .iter()
                .all(|(_, value)| Path::new(value).starts_with(workspace)),
            "user folders must stay inside the probe workspace: {variables:?}"
        );
        assert_eq!(variables[3].1, OsString::from("host-ProgramData"));
    }
}
