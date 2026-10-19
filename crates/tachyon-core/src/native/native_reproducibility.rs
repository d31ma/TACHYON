use super::host::{NativeHost, native_io, run_tool_with};
use crate::Failure;
use crate::external_command::ToolOutput;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(super) struct NativeReproducibility;

impl NativeReproducibility {
    const SOURCE_DATE_EPOCH: &'static str = "SOURCE_DATE_EPOCH";
    const ZERO_ARCHIVE_DATES: &'static str = "ZERO_AR_DATE";
    const PORTABLE_FOLDER: &'static str = ".";
    const C_PATH_MAP: &'static str = "-ffile-prefix-map=";
    const RUST_PATH_MAP: &'static str = "--remap-path-prefix=";
    pub(super) const SWIFT_ARGUMENTS: [&'static str; 2] =
        ["-Xfrontend", "-disable-incremental-llvm-codegen"];

    pub(super) fn source_date_epoch() -> u64 {
        std::env::var(Self::SOURCE_DATE_EPOCH)
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(0)
    }

    pub(super) async fn run_tool(program: &str, arguments: &[&str]) -> Result<String, Failure> {
        Self::run_tool_in(program, arguments, None).await
    }

    pub(super) async fn run_tool_in(
        program: &str,
        arguments: &[&str],
        working_directory: Option<&Path>,
    ) -> Result<String, Failure> {
        let epoch = Self::source_date_epoch().to_string();
        let environment = Self::environment(&epoch);
        run_tool_with(program, arguments, working_directory, &environment).await
    }

    pub(super) async fn run_tool_output(
        program: &str,
        arguments: &[&str],
    ) -> Result<ToolOutput, Failure> {
        let epoch = Self::source_date_epoch().to_string();
        let environment = Self::environment(&epoch);
        NativeHost::run_tool_output_with(program, arguments, &environment).await
    }

    fn environment(epoch: &str) -> [(&'static str, &str); 2] {
        [
            (Self::SOURCE_DATE_EPOCH, epoch),
            (Self::ZERO_ARCHIVE_DATES, "1"),
        ]
    }

    pub(super) fn c_path_maps<'path>(
        sources: impl IntoIterator<Item = &'path Path>,
    ) -> Vec<String> {
        Self::folders(sources)
            .map(|folder| format!("{}{folder}={}", Self::C_PATH_MAP, Self::PORTABLE_FOLDER))
            .collect()
    }

    pub(super) fn rust_path_maps<'path>(
        sources: impl IntoIterator<Item = &'path Path>,
    ) -> Vec<String> {
        Self::folders(sources)
            .map(|folder| format!("{}{folder}={}", Self::RUST_PATH_MAP, Self::PORTABLE_FOLDER))
            .collect()
    }

    pub(super) fn absolute(path: &Path) -> Result<PathBuf, Failure> {
        native_io(std::path::absolute(path), path)
    }

    pub(super) fn source_folder(source: &Path) -> Result<PathBuf, Failure> {
        Self::absolute(
            source
                .parent()
                .unwrap_or_else(|| Path::new(Self::PORTABLE_FOLDER)),
        )
    }

    pub(super) fn within(folder: &Path, path: &Path) -> Result<PathBuf, Failure> {
        let absolute = Self::absolute(path)?;
        Ok(absolute
            .strip_prefix(folder)
            .map_or_else(|_| absolute.clone(), Path::to_path_buf))
    }

    fn folders<'path>(
        sources: impl IntoIterator<Item = &'path Path>,
    ) -> impl Iterator<Item = String> {
        sources
            .into_iter()
            .filter_map(Path::parent)
            .filter(|folder| !folder.as_os_str().is_empty())
            .map(|folder| folder.to_string_lossy().into_owned())
            .collect::<BTreeSet<_>>()
            .into_iter()
    }
}
