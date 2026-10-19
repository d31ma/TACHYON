use super::ToolchainState;
use std::path::Path;

pub(super) struct SdkBuildTools;

impl SdkBuildTools {
    const FOLDER: &'static str = "build-tools";

    pub(super) fn state(sdk: &Path, files: &[&str]) -> ToolchainState {
        Self::newest(sdk, files).map_or_else(
            || Self::none_found(sdk, files),
            |version| ToolchainState::Ready(format!("{} {version}", Self::FOLDER)),
        )
    }

    fn newest(sdk: &Path, files: &[&str]) -> Option<String> {
        std::fs::read_dir(sdk.join(Self::FOLDER))
            .ok()?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|folder| Self::holds_one_of(folder, files))
            .filter_map(|folder| folder.file_name()?.to_str().map(String::from))
            .max_by_key(|version| Self::version_numbers(version))
    }

    fn holds_one_of(folder: &Path, files: &[&str]) -> bool {
        files.iter().any(|file| folder.join(file).is_file())
    }

    fn version_numbers(version: &str) -> Vec<u64> {
        version
            .split(|character: char| !character.is_ascii_digit())
            .filter_map(|part| part.parse().ok())
            .collect()
    }

    fn none_found(sdk: &Path, files: &[&str]) -> ToolchainState {
        ToolchainState::Incomplete(format!(
            "{} has no {} with {}",
            sdk.display(),
            Self::FOLDER,
            files.first().copied().unwrap_or_default()
        ))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::SdkBuildTools;
    use crate::doctor::ToolchainState;
    use std::fs;

    #[test]
    fn the_newest_build_tools_folder_with_the_tool_is_reported() {
        let sdk = tempfile::tempdir().expect("sdk");
        for (version, file) in [
            ("9.0.0", "apksigner"),
            ("35.0.0", "apksigner.bat"),
            ("36.0.0", "aapt2"),
        ] {
            let folder = sdk.path().join("build-tools").join(version);
            fs::create_dir_all(&folder).expect("version folder");
            fs::write(folder.join(file), "").expect("tool");
        }
        assert_eq!(
            SdkBuildTools::state(sdk.path(), &["apksigner", "apksigner.bat"]),
            ToolchainState::Ready(String::from("build-tools 35.0.0"))
        );
        let ToolchainState::Incomplete(reason) = SdkBuildTools::state(sdk.path(), &["zipalign"])
        else {
            panic!("no build-tools folder holds zipalign");
        };
        assert!(
            reason.ends_with("has no build-tools with zipalign"),
            "{reason}"
        );
    }
}
