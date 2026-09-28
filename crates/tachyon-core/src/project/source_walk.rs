use super::{CaptureProblem, CapturedFile, SourceRootStep};
use crate::compiler::PortablePath;
use cap_fs_ext::DirExt as _;
use cap_std::fs::{Dir, DirEntry, Metadata};
use std::ffi::OsStr;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use tachyon_diagnostics::Diagnostic;

pub(super) struct SourceWalk<'a> {
    diagnostics: &'a mut Vec<Diagnostic>,
    files: Vec<CapturedFile>,
}

impl SourceWalk<'_> {
    pub(super) fn capture(
        project: &Dir,
        relative_root: &Path,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Vec<CapturedFile> {
        let root = SourceRootStep::open_root(project, relative_root, diagnostics);
        let walk = SourceWalk {
            diagnostics,
            files: Vec::new(),
        };
        root.map(|directory| walk.files_beneath(&directory, relative_root))
            .unwrap_or_default()
    }

    fn files_beneath(mut self, directory: &Dir, relative_directory: &Path) -> Vec<CapturedFile> {
        self.visit(directory, relative_directory);
        self.files
    }

    fn visit(&mut self, directory: &Dir, relative_directory: &Path) {
        let entries = self
            .sorted_entries(directory, relative_directory)
            .unwrap_or_default();
        let _ = entries
            .iter()
            .try_for_each(|entry| self.visit_entry(directory, relative_directory, entry));
    }

    fn sorted_entries(
        &mut self,
        directory: &Dir,
        relative_directory: &Path,
    ) -> Option<Vec<DirEntry>> {
        let listed = directory
            .entries()
            .map_err(|error| {
                self.diagnostics.push(CaptureProblem::unreadable_directory(
                    relative_directory,
                    &error,
                ));
            })
            .ok()?;
        let mut entries = listed
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| {
                self.diagnostics.push(CaptureProblem::error(format!(
                    "Cannot enumerate source directory '{}': {error}",
                    PortablePath::of(relative_directory)
                )));
            })
            .ok()?;
        entries.sort_by_key(DirEntry::file_name);
        Some(entries)
    }

    fn visit_entry(
        &mut self,
        directory: &Dir,
        relative_directory: &Path,
        entry: &DirEntry,
    ) -> ControlFlow<()> {
        let name = entry.file_name();
        let relative = relative_directory.join(&name);
        let metadata = directory
            .symlink_metadata(&name)
            .map_err(|error| {
                self.diagnostics.push(CaptureProblem::error(format!(
                    "Cannot inspect '{}': {error}",
                    PortablePath::of(&relative)
                )));
            })
            .ok();
        metadata.map_or(ControlFlow::Continue(()), |metadata| {
            self.visit_found(directory, &name, relative, &metadata)
        })
    }

    fn visit_found(
        &mut self,
        directory: &Dir,
        name: &OsStr,
        relative: PathBuf,
        metadata: &Metadata,
    ) -> ControlFlow<()> {
        let relative_directory = relative.parent().unwrap_or(Path::new(""));
        if metadata.file_type().is_symlink() {
            self.diagnostics
                .push(CaptureProblem::unsafe_symlink(&relative));
        } else if metadata.is_dir() {
            self.visit_child(directory, name, &relative);
        } else if !metadata.is_file() {
            self.diagnostics.push(CaptureProblem::error(format!(
                "Source '{}' is not a regular file or directory.",
                PortablePath::of(&relative)
            )));
        } else if metadata.len() > CapturedFile::MAX_FILE_BYTES {
            self.diagnostics
                .push(CaptureProblem::oversized_source(&relative));
        } else if let Some(problem) = self.tree_limit_problem(metadata.len(), relative_directory) {
            self.diagnostics.push(problem);
            return ControlFlow::Break(());
        } else {
            self.capture_file(directory, name, relative);
        }
        ControlFlow::Continue(())
    }

    fn visit_child(&mut self, directory: &Dir, name: &OsStr, relative: &Path) {
        directory
            .open_dir_nofollow(name)
            .map_err(|error| {
                self.diagnostics
                    .push(CaptureProblem::unreadable_directory(relative, &error));
            })
            .into_iter()
            .for_each(|child| self.visit(&child, relative));
    }

    fn tree_limit_problem(
        &self,
        new_file_bytes: u64,
        relative_directory: &Path,
    ) -> Option<Diagnostic> {
        let path = PortablePath::of(relative_directory);
        let total = self.files.iter().fold(new_file_bytes, |total, file| {
            total.saturating_add(file.bytes.len() as u64)
        });
        (self.files.len() >= CapturedFile::MAX_FILES)
            .then(|| {
                CaptureProblem::error(format!(
                    "Source tree '{path}' exceeds the 4,096-file snapshot limit."
                ))
            })
            .or_else(|| {
                (total > CapturedFile::MAX_TOTAL_BYTES).then(|| {
                    CaptureProblem::error(format!(
                        "Source tree '{path}' exceeds the 64 MiB snapshot limit."
                    ))
                })
            })
    }

    fn capture_file(&mut self, directory: &Dir, name: &OsStr, relative: PathBuf) {
        let bytes =
            CapturedFile::read_without_following(directory, Path::new(name)).map_err(|error| {
                self.diagnostics.push(CaptureProblem::error(format!(
                    "Cannot read source '{}': {error}",
                    PortablePath::of(&relative)
                )));
            });
        self.files
            .extend(bytes.ok().map(|bytes| CapturedFile { relative, bytes }));
    }
}
