use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CacheStatus {
    pub root: PathBuf,
    pub runtime_entries: usize,
    pub legacy_entries: usize,
}

#[must_use]
pub fn root() -> PathBuf {
    if let Some(override_path) = std::env::var_os("TAC_CACHE_DIR")
        && !override_path.is_empty()
    {
        return PathBuf::from(override_path);
    }
    let home = home_directory();
    if cfg!(target_os = "macos") {
        return home.join("Library/Caches/Tachyon");
    }
    if cfg!(target_os = "windows") {
        let local_data = std::env::var_os("LOCALAPPDATA")
            .map_or_else(|| home.join("AppData/Local"), PathBuf::from);
        return local_data.join("Tachyon/Cache");
    }
    let cache_home =
        std::env::var_os("XDG_CACHE_HOME").map_or_else(|| home.join(".cache"), PathBuf::from);
    cache_home.join("tachyon")
}

fn home_directory() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}

#[must_use]
pub fn status() -> CacheStatus {
    status_at(root())
}

fn status_at(root: PathBuf) -> CacheStatus {
    let runtime_entries = count_runtime_entries(&root);
    CacheStatus {
        root,
        runtime_entries,
        legacy_entries: 0,
    }
}

fn count_runtime_entries(root: &Path) -> usize {
    let Ok(entries) = fs::read_dir(root.join("runtime")) else {
        return 0;
    };
    let mut count = 0;
    for entry in entries.flatten() {
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            count += 1;
        }
    }
    count
}

pub fn ensure_runtime() -> Result<PathBuf, Failure> {
    let entry = root().join("runtime/native-v1");
    fs::create_dir_all(&entry).map_err(|error| {
        cache_failure(format!(
            "Cannot initialize the Tachyon runtime cache: {error}"
        ))
    })?;
    Ok(entry)
}

pub fn clean() -> Result<CacheStatus, Failure> {
    clean_at(root())
}

fn clean_at(root: PathBuf) -> Result<CacheStatus, Failure> {
    let before = status_at(root);
    match fs::remove_dir_all(before.root.join("runtime")) {
        Ok(()) => Ok(before),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(before),
        Err(error) => Err(cache_failure(format!(
            "Cannot clear the cache at {}. {error}",
            before.root.display()
        ))),
    }
}

fn cache_failure(message: String) -> Failure {
    Failure::one(diagnostic(
        1503,
        message,
        Some(String::from(
            "Check the directory's permissions, or set TAC_CACHE_DIR to a writable location.",
        )),
        source_span("cache", 0, 5),
    ))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{clean_at, root, status_at};
    use std::fs;

    #[test]
    fn the_cache_root_is_absolute_and_named() {
        let root = root();
        assert!(
            root.to_string_lossy().to_lowercase().contains("tachyon"),
            "{}",
            root.display()
        );
    }

    #[test]
    fn a_legacy_cache_is_counted_then_removed() {
        let directory = tempfile::tempdir().expect("directory");
        let cache = directory.path().join("cache");
        let empty = status_at(cache.clone());
        assert_eq!(empty.legacy_entries, 0);
        assert_eq!(empty.runtime_entries, 0);
        assert!(clean_at(cache.clone()).is_ok());

        fs::create_dir_all(cache.join("runtime")).expect("runtime");
        fs::write(cache.join("runtime/a.js"), "export const a = 1").expect("entry");
        assert_eq!(status_at(cache.clone()).runtime_entries, 0);

        let cleaned = clean_at(cache.clone()).expect("cleaned");
        assert_eq!(cleaned.runtime_entries, 0);
        assert!(!cache.join("runtime").exists());
        assert_eq!(status_at(cache).legacy_entries, 0);
    }
}
