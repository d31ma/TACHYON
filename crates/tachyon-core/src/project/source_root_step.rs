use super::{CaptureProblem, EntryMetadata};
use crate::compiler::PortablePath;
use cap_fs_ext::DirExt as _;
use cap_std::fs::{Dir, Metadata};
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use tachyon_diagnostics::Diagnostic;

pub(super) struct SourceRootStep<'a> {
    name: &'a Path,
    current: &'a Path,
    relative_root: &'a Path,
    is_last: bool,
}

impl SourceRootStep<'_> {
    pub(super) fn open_root(
        project: &Dir,
        relative_root: &Path,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<Dir> {
        let names = Self::normal_names(relative_root)?;
        Self::open_each(project, relative_root, &names, diagnostics)
    }

    fn open_each(
        project: &Dir,
        relative_root: &Path,
        names: &[&OsStr],
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Option<Dir> {
        let mut opened: Option<Dir> = None;
        let mut current = PathBuf::new();
        for (index, name) in names.iter().enumerate() {
            current.push(name);
            let step = SourceRootStep {
                name: Path::new(name),
                current: &current,
                relative_root,
                is_last: index + 1 == names.len(),
            };
            let parent = opened.as_ref().unwrap_or(project);
            opened = Some(step.open(parent, diagnostics)?);
        }
        opened
    }

    fn normal_names(relative_root: &Path) -> Option<Vec<&OsStr>> {
        relative_root.components().map(Self::normal_name).collect()
    }

    fn normal_name(component: Component<'_>) -> Option<&OsStr> {
        match component {
            Component::Normal(name) => Some(name),
            _ => None,
        }
    }

    fn open(&self, parent: &Dir, diagnostics: &mut Vec<Diagnostic>) -> Option<Dir> {
        let metadata = EntryMetadata::existing(parent, self.name, self.current, diagnostics)?;
        CaptureProblem::check(self.directory_problem(&metadata), diagnostics)?;
        parent
            .open_dir_nofollow(self.name)
            .map_err(|error| {
                diagnostics.push(CaptureProblem::unreadable_directory(self.current, &error));
            })
            .ok()
    }

    fn directory_problem(&self, metadata: &Metadata) -> Option<Diagnostic> {
        metadata
            .is_symlink()
            .then(|| CaptureProblem::unsafe_symlink(self.current))
            .or_else(|| {
                (!metadata.is_dir()).then(|| CaptureProblem::error(self.not_a_directory_message()))
            })
    }

    fn not_a_directory_message(&self) -> String {
        if self.is_last {
            return format!(
                "Source root '{}' is not a directory.",
                PortablePath::of(self.relative_root)
            );
        }
        format!(
            "Source root '{}' is blocked by '{}', which is not a directory.",
            PortablePath::of(self.relative_root),
            PortablePath::of(self.current)
        )
    }
}
