use crate::model_layout::ModelBlocks;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(crate) struct ClientModelNames;

impl ClientModelNames {
    const MODELS: &'static str = "client/models";
    const MOST_DEPTH: usize = 8;

    pub(crate) fn read(snapshot_root: &Path, extension: &str) -> Vec<String> {
        let mut files = Vec::new();
        Self::collect(snapshot_root, Path::new(Self::MODELS), 0, &mut files);
        files.retain(|file| Self::has_extension(file, extension));
        Self::declared_in_files(snapshot_root, &files)
    }

    fn has_extension(file: &Path, extension: &str) -> bool {
        file.extension() == Some(std::ffi::OsStr::new(extension))
    }

    pub(crate) fn of_files(files: &[(PathBuf, String)]) -> Vec<String> {
        Self::names_in(files)
    }

    fn names_in(files: &[(PathBuf, String)]) -> Vec<String> {
        let mut names = BTreeSet::new();
        for (path, text) in files {
            names.extend(ModelBlocks::names(path, text).unwrap_or_default());
        }
        names.into_iter().collect()
    }

    fn declared_in_files(root: &Path, files: &[PathBuf]) -> Vec<String> {
        let mut names = BTreeSet::new();
        for relative in files {
            names.extend(Self::declared(root, relative));
        }
        names.into_iter().collect()
    }

    fn collect(root: &Path, relative: &Path, depth: usize, files: &mut Vec<PathBuf>) {
        for child in Self::children(root, relative, depth) {
            Self::add_entry(root, child, depth, files);
        }
    }

    fn children(root: &Path, relative: &Path, depth: usize) -> Vec<PathBuf> {
        let entries = (depth <= Self::MOST_DEPTH)
            .then(|| std::fs::read_dir(root.join(relative)).ok())
            .flatten();
        entries
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| relative.join(entry.file_name()))
            .collect()
    }

    fn add_entry(root: &Path, child: PathBuf, depth: usize, files: &mut Vec<PathBuf>) {
        let folder = std::fs::symlink_metadata(root.join(&child))
            .ok()
            .map(|metadata| metadata.is_dir());
        match folder {
            Some(true) => Self::collect(root, &child, depth + 1, files),
            Some(false) => files.push(child),
            None => {}
        }
    }

    fn declared(root: &Path, relative: &Path) -> Vec<String> {
        let Ok(text) = std::fs::read_to_string(root.join(relative)) else {
            return Vec::new();
        };
        ModelBlocks::names(relative, crate::without_bom(&text)).unwrap_or_default()
    }
}
