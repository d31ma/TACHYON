use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CacheStatus {
    pub root: PathBuf,
    pub runtime_entries: usize,
    pub legacy_entries: usize,
    pub build_entries: Vec<(&'static str, usize)>,
}

impl CacheStatus {
    pub const RUNTIME_FOLDER: &'static str = "runtime";
    pub const BUILD_FOLDERS: [&'static str; 11] = [
        "dotnet-web",
        "go-wasm",
        "swift-wasm",
        "kotlin-wasm",
        "java-wasm",
        "rust-wasm",
        "dart-wasm",
        "ruby-web",
        "lua-web",
        "cpp-wasm",
        "typescript",
    ];

    fn removable_folders() -> impl Iterator<Item = &'static str> {
        std::iter::once(Self::RUNTIME_FOLDER).chain(Self::BUILD_FOLDERS)
    }

    fn remove_folder(root: &Path, name: &str) -> Result<(), Failure> {
        match fs::remove_dir_all(root.join(name)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(cache_failure(format!(
                "Cannot clear the cache at {}. {error}",
                root.display()
            ))),
        }
    }
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
    let runtime_entries = count_folder_entries(&root.join(CacheStatus::RUNTIME_FOLDER), true);
    let build_entries = CacheStatus::BUILD_FOLDERS
        .into_iter()
        .filter(|name| root.join(name).is_dir())
        .map(|name| (name, count_folder_entries(&root.join(name), false)))
        .collect();
    CacheStatus {
        root,
        runtime_entries,
        legacy_entries: 0,
        build_entries,
    }
}

fn count_folder_entries(folder: &Path, folders_only: bool) -> usize {
    let Ok(entries) = fs::read_dir(folder) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|entry| !folders_only || entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .count()
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
    for name in CacheStatus::removable_folders() {
        CacheStatus::remove_folder(&before.root, name)?;
    }
    Ok(before)
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

    use super::{CacheStatus, clean_at, root, status_at};
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

    #[test]
    fn language_build_caches_are_counted_and_cleaned_but_installed_tools_stay() {
        let directory = tempfile::tempdir().expect("directory");
        let cache = directory.path().join("cache");
        for folder in ["dotnet-web/key-1", "go-wasm/key-2", "go-wasm/key-3"] {
            fs::create_dir_all(cache.join(folder)).expect("build entry");
        }
        for tool in ["python-web/314.0.7", "teavm-0.15.0"] {
            fs::create_dir_all(cache.join(tool)).expect("installed tool");
        }
        let status = status_at(cache.clone());
        assert_eq!(
            status.build_entries,
            vec![("dotnet-web", 1), ("go-wasm", 2)]
        );

        clean_at(cache.clone()).expect("cleaned");
        for name in CacheStatus::BUILD_FOLDERS {
            assert!(!cache.join(name).exists(), "{name}");
        }
        assert!(cache.join("python-web/314.0.7").is_dir());
        assert!(cache.join("teavm-0.15.0").is_dir());
        assert!(status_at(cache).build_entries.is_empty());
    }
}
