use super::host::{NativeHost, native_io, native_tool_failure};
use crate::Failure;
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub(super) struct LinuxAppImageRuntime;

impl LinuxAppImageRuntime {
    pub(super) const OVERRIDE: &'static str = "TAC_APPIMAGE_RUNTIME";
    const RELEASE: &'static str = "20251108";
    const DOWNLOAD: &'static str =
        "https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-";
    const UNFINISHED_SUFFIX: &'static str = ".unfinished";
    const PINNED: [(&'static str, &'static str); 2] = [
        (
            "x86_64",
            "2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d",
        ),
        (
            "aarch64",
            "00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444",
        ),
    ];

    pub(super) async fn bytes(machine: &str) -> Result<Vec<u8>, Failure> {
        let chosen = Self::chosen(std::env::var_os(Self::OVERRIDE), machine).await?;
        native_io(std::fs::read(&chosen), &chosen)
    }

    async fn chosen(variable: Option<OsString>, machine: &str) -> Result<PathBuf, Failure> {
        let Some(named) = variable.filter(|value| !value.is_empty()) else {
            return Self::cached(machine).await;
        };
        let named = PathBuf::from(named);
        named.is_file().then_some(named.clone()).ok_or_else(|| {
            native_tool_failure(
                1605,
                &format!(
                    "{} names '{}', which is not an AppImage runtime file.",
                    Self::OVERRIDE,
                    named.display()
                ),
            )
        })
    }

    async fn cached(machine: &str) -> Result<PathBuf, Failure> {
        let sha256 = Self::pinned(machine)?;
        let file = Self::cache_folder().join(format!("runtime-{machine}"));
        Self::download_unless_present(&file, machine).await?;
        Self::check_pinned(&file, sha256)?;
        Ok(file)
    }

    async fn download_unless_present(file: &Path, machine: &str) -> Result<(), Failure> {
        if file.is_file() {
            return Ok(());
        }
        let mut unfinished = file.as_os_str().to_owned();
        unfinished.push(Self::UNFINISHED_SUFFIX);
        let unfinished = PathBuf::from(unfinished);
        let folder = file.parent().unwrap_or_else(|| Path::new("."));
        native_io(std::fs::create_dir_all(folder), folder)?;
        let url = format!("{}{machine}", Self::DOWNLOAD);
        let output = unfinished.to_string_lossy().into_owned();
        let arguments = [
            "--fail",
            "--location",
            "--silent",
            "--show-error",
            "--proto-redir",
            "=https",
            "--output",
            output.as_str(),
            url.as_str(),
        ];
        NativeHost::run_long_tool("curl", &arguments).await?;
        native_io(std::fs::rename(&unfinished, file), file)
    }

    fn check_pinned(file: &Path, sha256: &str) -> Result<(), Failure> {
        let bytes = native_io(std::fs::read(file), file)?;
        let actual = crate::compiler::hex_digest(Sha256::digest(&bytes));
        let matches = actual == sha256;
        matches.then_some(()).ok_or_else(|| {
            let _ = std::fs::remove_file(file);
            native_tool_failure(
                1605,
                &format!(
                    "The AppImage runtime at {} has SHA-256 {actual}, but Tachyon pins {sha256}. Tachyon deleted it; build again to download it once more.",
                    file.display()
                ),
            )
        })
    }

    fn pinned(machine: &str) -> Result<&'static str, Failure> {
        Self::PINNED
            .into_iter()
            .find(|(name, _)| *name == machine)
            .map(|(_, sha256)| sha256)
            .ok_or_else(|| {
                native_tool_failure(
                    1605,
                    &format!(
                        "Tachyon pins no AppImage runtime for {machine}. Set {} to a runtime file.",
                        Self::OVERRIDE
                    ),
                )
            })
    }

    pub(super) fn cache_folder() -> PathBuf {
        crate::cache::root()
            .join(crate::cache::CacheStatus::RUNTIME_FOLDER)
            .join(format!("appimage-runtime-{}", Self::RELEASE))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::LinuxAppImageRuntime;
    use std::ffi::OsString;

    #[tokio::test]
    async fn a_named_runtime_must_be_a_file() {
        let root = tempfile::tempdir().expect("root");
        let runtime = root.path().join("runtime");
        std::fs::write(&runtime, b"runtime").expect("runtime");
        let chosen = LinuxAppImageRuntime::chosen(Some(OsString::from(&runtime)), "x86_64")
            .await
            .expect("chosen");
        assert_eq!(chosen, runtime);
        let missing = root.path().join("missing");
        let text = LinuxAppImageRuntime::chosen(Some(OsString::from(&missing)), "x86_64")
            .await
            .expect_err("missing")
            .to_string();
        assert!(text.contains("TY1605"), "{text}");
        assert!(text.contains("TAC_APPIMAGE_RUNTIME names"), "{text}");
    }

    #[test]
    fn a_runtime_that_does_not_match_its_pin_is_deleted() {
        let root = tempfile::tempdir().expect("root");
        let runtime = root.path().join("runtime-x86_64");
        std::fs::write(&runtime, b"not the runtime").expect("runtime");
        let pinned = LinuxAppImageRuntime::pinned("x86_64").expect("pinned");
        let text = LinuxAppImageRuntime::check_pinned(&runtime, pinned)
            .expect_err("mismatch")
            .to_string();
        assert!(text.contains("but Tachyon pins"), "{text}");
        assert!(!runtime.exists());
        let unknown = LinuxAppImageRuntime::pinned("riscv64")
            .expect_err("unknown")
            .to_string();
        assert!(unknown.contains("Set TAC_APPIMAGE_RUNTIME"), "{unknown}");
    }

    #[test]
    fn the_runtime_is_kept_in_the_runtime_cache() {
        let folder = LinuxAppImageRuntime::cache_folder();
        assert!(folder.ends_with("runtime/appimage-runtime-20251108"));
    }
}
