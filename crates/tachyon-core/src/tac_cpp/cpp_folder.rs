use super::{CppFailure, CppSource};
use crate::Failure;
use std::path::{Path, PathBuf};

pub(crate) struct CppFolder;

impl CppFolder {
    const MAX_FILES: usize = 512;
    const MAX_BYTES: u64 = 1_048_576;
    const TOO_MANY: &'static str = "A C++ folder holds at most 512 files";
    const TOO_LARGE: &'static str = "A C++ file is at most 1 MiB";

    pub(crate) fn files(root: &Path, folder: &str) -> Result<Vec<PathBuf>, Failure> {
        let found = Self::walk(root.join(folder));
        (found.len() <= Self::MAX_FILES)
            .then_some(found)
            .ok_or_else(|| CppFailure::unreadable(Path::new(folder), 0, Self::TOO_MANY))
    }

    pub(crate) fn read(root: &Path, path: &Path) -> Result<(PathBuf, String), Failure> {
        let relative = path.strip_prefix(root).unwrap_or(path).to_path_buf();
        let size = std::fs::metadata(path).map_or(0, |metadata| metadata.len());
        (size <= Self::MAX_BYTES)
            .then_some(())
            .ok_or_else(|| CppFailure::unreadable(&relative, 0, Self::TOO_LARGE))?;
        let source = std::fs::read_to_string(path).map_err(|error| {
            CppFailure::unreadable(&relative, 0, &format!("Cannot read it: {error}"))
        })?;
        Ok((relative, crate::without_bom(&source).to_owned()))
    }

    fn walk(start: PathBuf) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut pending = vec![start];
        while let Some(directory) = pending.pop() {
            Self::visit(&directory, &mut pending, &mut found);
        }
        found.sort();
        found
    }

    fn visit(directory: &Path, pending: &mut Vec<PathBuf>, found: &mut Vec<PathBuf>) {
        let entries = std::fs::read_dir(directory).into_iter().flatten().flatten();
        for entry in entries {
            Self::sort(entry.path(), pending, found);
        }
    }

    fn sort(path: PathBuf, pending: &mut Vec<PathBuf>, found: &mut Vec<PathBuf>) {
        if path.is_dir() {
            pending.push(path);
        } else if CppSource::is_cpp(&path) {
            found.push(path);
        }
    }
}
