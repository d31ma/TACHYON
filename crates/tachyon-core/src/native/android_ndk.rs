use super::host::native_tool_failure;
use crate::Failure;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AndroidNdk {
    root: PathBuf,
}

impl AndroidNdk {
    const FAILURE_CODE: u16 = 1605;
    const NDK_VARIABLES: [&'static str; 2] = ["ANDROID_NDK_HOME", "ANDROID_NDK_ROOT"];
    const SDK_VARIABLES: [&'static str; 2] = ["ANDROID_HOME", "ANDROID_SDK_ROOT"];
    const SDK_FOLDER: &'static str = "ndk";
    const PREBUILT: &'static str = "toolchains/llvm/prebuilt";
    const LOWEST_API_LEVEL: u32 = 21;

    pub(crate) fn from_environment(
        variable: impl Fn(&str) -> Option<OsString>,
    ) -> Result<Self, Failure> {
        let declared = Self::NDK_VARIABLES
            .iter()
            .filter_map(|name| variable(name))
            .map(PathBuf::from);
        let side_by_side = Self::SDK_VARIABLES
            .iter()
            .filter_map(|name| variable(name))
            .filter_map(|sdk| Self::newest_in(&PathBuf::from(sdk).join(Self::SDK_FOLDER)));
        declared
            .chain(side_by_side)
            .find(|root| root.join(Self::PREBUILT).is_dir())
            .map(|root| Self { root })
            .ok_or_else(Self::missing)
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(super) fn linker(&self, triple: &str, api_level: u32) -> Result<PathBuf, Failure> {
        let bin = self
            .root
            .join(Self::PREBUILT)
            .join(Self::host_tag())
            .join("bin");
        (Self::LOWEST_API_LEVEL..=api_level.max(Self::LOWEST_API_LEVEL))
            .rev()
            .map(|level| bin.join(Self::wrapper_name(triple, level)))
            .find(|wrapper| wrapper.is_file())
            .ok_or_else(|| Self::missing_linker(&bin, triple, api_level))
    }

    fn newest_in(folder: &Path) -> Option<PathBuf> {
        let entries = std::fs::read_dir(folder).ok()?;
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.join(Self::PREBUILT).is_dir())
            .max_by_key(|path| Self::version_key(path))
    }

    fn version_key(path: &Path) -> Vec<u64> {
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .split('.')
            .map(|part| part.parse().unwrap_or(0))
            .collect()
    }

    const fn host_tag() -> &'static str {
        if cfg!(windows) {
            "windows-x86_64"
        } else if cfg!(target_os = "macos") {
            "darwin-x86_64"
        } else {
            "linux-x86_64"
        }
    }

    fn wrapper_name(triple: &str, level: u32) -> String {
        let suffix = if cfg!(windows) { ".cmd" } else { "" };
        format!("{triple}{level}-clang{suffix}")
    }

    fn missing() -> Failure {
        native_tool_failure(
            Self::FAILURE_CODE,
            "Rust services in an Android app are linked with the Android NDK, which is not \
             installed. Install it with `sdkmanager \"ndk;<version>\"` (or the SDK Manager in \
             Android Studio), or set ANDROID_NDK_HOME to the NDK folder, then build again.",
        )
    }

    fn missing_linker(bin: &Path, triple: &str, api_level: u32) -> Failure {
        native_tool_failure(
            Self::FAILURE_CODE,
            &format!(
                "The Android NDK in {} has no clang for {triple} at API level {api_level} or \
                 lower. Install a newer NDK, then build again.",
                bin.display()
            ),
        )
    }
}
