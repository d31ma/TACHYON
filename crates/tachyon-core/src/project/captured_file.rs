use super::{CaptureProblem, EntryMetadata};
use crate::compiler::PortablePath;
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt as _};
use cap_std::fs::{Dir, Metadata, OpenOptions};
use std::io::Read as _;
use std::path::{Path, PathBuf};
use tachyon_diagnostics::Diagnostic;

#[derive(Clone, Debug)]
pub(super) struct CapturedFile {
    pub(super) relative: PathBuf,
    pub(super) bytes: Vec<u8>,
}

impl CapturedFile {
    pub(super) const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;
    pub(super) const MAX_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
    pub(super) const MAX_FILES: usize = 4_096;

    pub(super) fn portable(&self) -> String {
        PortablePath::of(&self.relative)
    }

    pub(super) fn project_file(
        project: &Dir,
        relative: &Path,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<Self> {
        let metadata = EntryMetadata::existing(project, relative, relative, diagnostics)?;
        CaptureProblem::check(Self::regular_file_problem(&metadata, relative), diagnostics)?;
        let bytes = Self::read_without_following(project, relative)
            .map_err(|error| {
                diagnostics.push(CaptureProblem::error(format!(
                    "Cannot read project input '{}': {error}",
                    PortablePath::of(relative)
                )));
            })
            .ok()?;
        Some(Self {
            relative: relative.to_path_buf(),
            bytes,
        })
    }

    pub(super) fn read_without_following(directory: &Dir, name: &Path) -> std::io::Result<Vec<u8>> {
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let mut source = directory.open_with(name, &options)?;
        let mut bytes = Vec::new();
        source.read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    fn regular_file_problem(metadata: &Metadata, relative: &Path) -> Option<Diagnostic> {
        let path = PortablePath::of(relative);
        metadata
            .is_symlink()
            .then(|| CaptureProblem::unsafe_symlink(relative))
            .or_else(|| {
                (!metadata.is_file()).then(|| {
                    CaptureProblem::error(format!("Project input '{path}' is not a regular file."))
                })
            })
            .or_else(|| {
                (metadata.len() > Self::MAX_FILE_BYTES).then(|| {
                    CaptureProblem::error(format!(
                        "Project input '{path}' exceeds the 16 MiB snapshot limit."
                    ))
                })
            })
    }
}
