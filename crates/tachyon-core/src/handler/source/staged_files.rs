use std::fs::DirEntry;
use std::path::{Path, PathBuf};

pub(super) struct StagedFiles;

impl StagedFiles {
    pub(super) fn beneath(source_root: &Path, relative: &Path) -> Vec<PathBuf> {
        let mut found = Self::visit(source_root, relative);
        found.sort();
        found
    }

    fn visit(source_root: &Path, relative: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(source_root.join(relative))
            .into_iter()
            .flatten()
            .flatten()
            .flat_map(|entry| Self::entry_files(source_root, relative, &entry))
            .collect()
    }

    fn entry_files(source_root: &Path, relative: &Path, entry: &DirEntry) -> Vec<PathBuf> {
        let child = relative.join(entry.file_name());
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => Self::visit(source_root, &child),
            Ok(kind) if kind.is_file() => vec![child],
            _ => Vec::new(),
        }
    }
}
