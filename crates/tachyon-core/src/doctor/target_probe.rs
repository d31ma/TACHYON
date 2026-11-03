use super::ToolchainState;
use super::command_probe::CommandProbe;
use super::keychain_identities::KeychainIdentities;
use super::sdk_build_tools::SdkBuildTools;
use crate::handler::RuntimeProbe;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug)]
pub(super) enum TargetProbe {
    Version(&'static str, &'static [&'static str]),
    OnPath(&'static str),
    Folder {
        variables: &'static [&'static str],
        marker: &'static str,
    },
    Identity(&'static [&'static str]),
    BuildTool {
        variables: &'static [&'static str],
        files: &'static [&'static str],
    },
    NamedFile(&'static str),
    AndroidNdk,
}

impl TargetProbe {
    const PATH: &'static str = "PATH";
    const FOUND_ON_PATH: &'static str = "found on PATH";
    const NO_ANDROID_NDK: &'static str =
        "no NDK in ANDROID_NDK_HOME, ANDROID_NDK_ROOT or the SDK's ndk folder";

    pub(super) fn state(self) -> ToolchainState {
        self.probe()
    }

    fn probe(self) -> ToolchainState {
        match self {
            Self::Version(program, arguments) => Self::version(program, arguments),
            Self::OnPath(program) => Self::on_path(program),
            Self::Folder { variables, marker } => Self::folder(variables, marker),
            Self::Identity(kinds) => KeychainIdentities::state(kinds),
            Self::BuildTool { variables, files } => Self::build_tool(variables, files),
            Self::NamedFile(variable) => Self::named_file(variable),
            Self::AndroidNdk => Self::android_ndk(),
        }
    }

    fn version(program: &str, arguments: &[&str]) -> ToolchainState {
        CommandProbe::output(program, arguments).map_or(ToolchainState::Missing, |text| {
            ToolchainState::Ready(Self::version_line(&text))
        })
    }

    fn version_line(text: &str) -> String {
        text.lines()
            .map(str::trim)
            .find(|line| line.chars().any(char::is_alphanumeric))
            .unwrap_or_default()
            .to_owned()
    }

    fn on_path(program: &str) -> ToolchainState {
        let paths = std::env::var_os(Self::PATH).unwrap_or_default();
        Self::found_state(RuntimeProbe::on_path(&paths, Path::new(program)))
    }

    fn found_state(found: bool) -> ToolchainState {
        if found {
            return ToolchainState::Ready(String::from(Self::FOUND_ON_PATH));
        }
        ToolchainState::Missing
    }

    fn folder(variables: &[&str], marker: &str) -> ToolchainState {
        Self::named_folder(variables).map_or_else(
            || Self::not_set(variables),
            |folder| Self::folder_state(&folder, marker),
        )
    }

    fn build_tool(variables: &[&str], files: &[&str]) -> ToolchainState {
        Self::named_folder(variables).map_or_else(
            || Self::not_set(variables),
            |sdk| SdkBuildTools::state(&sdk, files),
        )
    }

    fn named_file(variable: &'static str) -> ToolchainState {
        Self::named_folder(&[variable]).map_or_else(
            || Self::not_set(&[variable]),
            |file| Self::file_state(&file),
        )
    }

    fn file_state(file: &Path) -> ToolchainState {
        if file.is_file() {
            return ToolchainState::Ready(file.display().to_string());
        }
        ToolchainState::Incomplete(format!("{} is not a file", file.display()))
    }

    fn android_ndk() -> ToolchainState {
        crate::native::AndroidNdk::from_environment(|name| std::env::var_os(name)).map_or_else(
            |_| ToolchainState::Incomplete(String::from(Self::NO_ANDROID_NDK)),
            |ndk| ToolchainState::Ready(ndk.root().display().to_string()),
        )
    }

    fn named_folder(variables: &[&str]) -> Option<PathBuf> {
        variables
            .iter()
            .find_map(std::env::var_os)
            .map(PathBuf::from)
    }

    fn not_set(variables: &[&str]) -> ToolchainState {
        ToolchainState::Incomplete(format!("{} is not set", variables.join(" or ")))
    }

    fn folder_state(folder: &Path, marker: &str) -> ToolchainState {
        if folder.join(marker).exists() {
            return ToolchainState::Ready(folder.display().to_string());
        }
        ToolchainState::Incomplete(format!("{} has no {marker}", folder.display()))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::TargetProbe;
    use crate::doctor::ToolchainState;
    use std::fs;

    #[test]
    fn the_version_is_the_first_line_with_words() {
        let gradle =
            "\n------------------------------------------------------------\nGradle 8.14.5\n";
        assert_eq!(TargetProbe::version_line(gradle), "Gradle 8.14.5");
        assert_eq!(
            TargetProbe::version_line("javac 21.0.12\n"),
            "javac 21.0.12"
        );
    }

    #[test]
    fn an_sdk_folder_is_ready_only_when_it_holds_its_marker() {
        let folder = tempfile::tempdir().expect("folder");
        let marker = "build/native/include/WebView2.h";
        let ToolchainState::Incomplete(reason) = TargetProbe::folder_state(folder.path(), marker)
        else {
            panic!("an empty folder is not ready");
        };
        assert!(
            reason.ends_with("has no build/native/include/WebView2.h"),
            "{reason}"
        );
        let header = folder.path().join(marker);
        fs::create_dir_all(header.parent().expect("parent")).expect("include");
        fs::write(&header, "").expect("header");
        assert_eq!(
            TargetProbe::folder_state(folder.path(), marker),
            ToolchainState::Ready(folder.path().display().to_string())
        );
        assert_eq!(
            TargetProbe::not_set(&["ANDROID_HOME", "ANDROID_SDK_ROOT"]),
            ToolchainState::Incomplete(String::from("ANDROID_HOME or ANDROID_SDK_ROOT is not set"))
        );
    }

    #[test]
    fn a_named_file_is_ready_only_when_it_exists() {
        let folder = tempfile::tempdir().expect("folder");
        let file = folder.path().join("makemsix");
        assert_eq!(
            TargetProbe::file_state(&file),
            ToolchainState::Incomplete(format!("{} is not a file", file.display()))
        );
        fs::write(&file, "").expect("file");
        assert_eq!(
            TargetProbe::file_state(&file),
            ToolchainState::Ready(file.display().to_string())
        );
    }

    #[test]
    fn a_program_that_is_not_on_path_is_missing() {
        assert_eq!(
            TargetProbe::OnPath("tachyon-no-such-program").state(),
            ToolchainState::Missing
        );
    }
}
