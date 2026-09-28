use std::collections::BTreeSet;
use std::fs::{self, DirEntry, FileType, Metadata};
use std::path::{Path, PathBuf};

pub(super) struct YonDiscovery;

impl YonDiscovery {
    const SOURCE_ROOTS: [&str; 1] = ["server/controllers"];
    const IGNORED_DIRECTORIES: [&str; 7] = [
        ".git",
        ".tachyon",
        "dist",
        "dist-bin",
        "node_modules",
        "target",
        "__pycache__",
    ];

    pub(super) fn extensions(root: &Path, workers: &crate::Workers) -> BTreeSet<String> {
        let mut extensions = BTreeSet::new();
        Self::insert_source_extensions(root, &mut extensions);
        Self::insert_middleware_extensions(root, &mut extensions);
        extensions.extend(
            workers
                .iter()
                .filter_map(|(worker, _)| Self::extension_of(Path::new(worker))),
        );
        extensions
    }

    fn source_roots(root: &Path) -> Vec<PathBuf> {
        Self::SOURCE_ROOTS
            .iter()
            .map(|relative| root.join(relative))
            .filter(|path| Self::is_real_directory(path))
            .collect()
    }

    fn is_real_directory(path: &Path) -> bool {
        fs::symlink_metadata(path).is_ok_and(|metadata| Self::is_plain_directory(&metadata))
    }

    fn is_plain_directory(metadata: &Metadata) -> bool {
        metadata.is_dir() && !metadata.file_type().is_symlink()
    }

    fn insert_source_extensions(root: &Path, extensions: &mut BTreeSet<String>) {
        let mut pending = Self::source_roots(root);
        while let Some(directory) = pending.pop() {
            Self::read_directory(&directory, &mut pending, extensions);
        }
    }

    fn read_directory(
        directory: &Path,
        pending: &mut Vec<PathBuf>,
        extensions: &mut BTreeSet<String>,
    ) {
        for entry in Self::entries(directory) {
            let kind = Self::real_kind(&entry);
            pending.extend(Self::subdirectory(&entry, kind));
            extensions.extend(Self::yon_extension(&entry, kind));
        }
    }

    fn entries(directory: &Path) -> impl Iterator<Item = DirEntry> {
        fs::read_dir(directory)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
    }

    fn real_kind(entry: &DirEntry) -> Option<FileType> {
        entry.file_type().ok().filter(|kind| !kind.is_symlink())
    }

    fn subdirectory(entry: &DirEntry, kind: Option<FileType>) -> Option<PathBuf> {
        kind.filter(FileType::is_dir)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        Self::is_searched_name(&name).then(|| entry.path())
    }

    fn is_searched_name(name: &str) -> bool {
        !Self::IGNORED_DIRECTORIES.contains(&name) && !name.starts_with('.')
    }

    fn yon_extension(entry: &DirEntry, kind: Option<FileType>) -> Option<String> {
        kind.filter(FileType::is_file)?;
        let path = entry.path();
        Self::extension_of(&path)
            .filter(|_| crate::convention::YonConvention::is_yon_language(&path))
    }

    fn insert_middleware_extensions(root: &Path, extensions: &mut BTreeSet<String>) {
        extensions
            .extend(Self::entries(root).filter_map(|entry| Self::middleware_extension(&entry)));
    }

    fn middleware_extension(entry: &DirEntry) -> Option<String> {
        entry.file_type().ok().filter(FileType::is_file)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        name.strip_prefix("middleware.").map(String::from)
    }

    fn extension_of(path: &Path) -> Option<String> {
        path.extension()?.to_str().map(String::from)
    }
}
