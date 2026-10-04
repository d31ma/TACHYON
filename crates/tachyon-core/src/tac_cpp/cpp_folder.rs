use super::{CppFailure, CppSource};
use crate::Failure;
use std::path::{Path, PathBuf};

pub(crate) struct CppFolder;

impl CppFolder {
    const MAX_FILES: usize = 512;
    const MAX_BYTES: u64 = 1_048_576;

    pub(crate) fn files(root: &Path, folder: &str) -> Result<Vec<PathBuf>, Failure> {
        let mut found = Vec::new();
        let mut pending = vec![root.join(folder)];
        while let Some(directory) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else {
                continue;
            };
            for entry in entries.flatten() {
                Self::sort(entry.path(), &mut pending, &mut found);
            }
        }
        if found.len() > Self::MAX_FILES {
            return Err(CppFailure::unreadable(
                Path::new(folder),
                0,
                "A C++ folder holds at most 512 files",
            ));
        }
        found.sort();
        Ok(found)
    }

    fn sort(path: PathBuf, pending: &mut Vec<PathBuf>, found: &mut Vec<PathBuf>) {
        if path.is_dir() {
            pending.push(path);
        } else if CppSource::is_cpp(&path) {
            found.push(path);
        }
    }

    pub(crate) fn read(root: &Path, path: &Path) -> Result<(PathBuf, String), Failure> {
        let relative = path.strip_prefix(root).unwrap_or(path).to_path_buf();
        let size = std::fs::metadata(path).map_or(0, |metadata| metadata.len());
        if size > Self::MAX_BYTES {
            return Err(CppFailure::unreadable(
                &relative,
                0,
                "A C++ file is at most 1 MiB",
            ));
        }
        let source = std::fs::read_to_string(path).map_err(|error| {
            CppFailure::unreadable(&relative, 0, &format!("Cannot read it: {error}"))
        })?;
        Ok((relative, crate::without_bom(&source).to_owned()))
    }
}
